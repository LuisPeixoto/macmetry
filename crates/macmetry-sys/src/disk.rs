//! Volumes (space) and physical disks (throughput).

use crate::error::{Result, SysError};
use crate::iokit;
use crate::value::Value;
use macmetry_core::rates::per_second;
use macmetry_core::{DiskIo, DiskStats, Volume};
use std::collections::HashMap;
use std::ffi::CStr;

/// One mounted file system, as `getfsstat` reports it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsEntry {
    pub from: String,
    pub on: String,
    pub fs_type: String,
    pub block_size: u64,
    pub blocks: u64,
    pub available: u64,
    pub flags: u32,
}

/// File systems that are not storage a person thinks of as a volume.
const IGNORED_TYPES: &[&str] = &["devfs", "autofs", "nullfs", "fdesc", "synthfs"];

/// The volumes worth showing: local, with space, and either the startup volume or one that the Finder would show.
///
/// macOS mounts many hidden system volumes (Preboot, VM, Update, asset images) that all share one APFS container, so
/// listing them would repeat the same space many times. `root_name` is the name of the startup volume.
pub fn volumes_from(entries: &[FsEntry], root_name: &str) -> Vec<Volume> {
    let local = libc::MNT_LOCAL as u32;
    let hidden = libc::MNT_DONTBROWSE as u32;
    entries
        .iter()
        .filter(|e| e.flags & local != 0)
        .filter(|e| !IGNORED_TYPES.contains(&e.fs_type.as_str()))
        .filter(|e| e.on == "/" || e.flags & hidden == 0)
        .filter(|e| e.blocks > 0)
        .map(|e| Volume {
            name: if e.on == "/" {
                root_name.to_string()
            } else {
                e.on.rsplit('/').next().filter(|n| !n.is_empty()).unwrap_or(&e.on).to_string()
            },
            mount_point: e.on.clone(),
            fs_type: e.fs_type.clone(),
            total: e.blocks.saturating_mul(e.block_size),
            free: e.available.saturating_mul(e.block_size),
        })
        .collect()
}

fn c_string(chars: &[libc::c_char]) -> String {
    // SAFETY: the kernel NUL-terminates these fixed-size fields; `from_ptr` stops at the first NUL.
    let text = unsafe { CStr::from_ptr(chars.as_ptr()) };
    text.to_string_lossy().into_owned()
}

pub fn read_filesystems() -> Result<Vec<FsEntry>> {
    // SAFETY: a null buffer asks only for the number of mounted file systems.
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    if count < 0 {
        return Err(SysError::os("getfsstat"));
    }
    // Room for a few more in case one is mounted between the two calls.
    let capacity = count as usize + 8;
    // SAFETY: a zeroed `statfs` is a valid value of plain integers and arrays.
    let mut buf: Vec<libc::statfs> = vec![unsafe { std::mem::zeroed() }; capacity];
    let bytes = (capacity * std::mem::size_of::<libc::statfs>()) as libc::c_int;
    // SAFETY: `buf` has `bytes` writable bytes.
    let filled = unsafe { libc::getfsstat(buf.as_mut_ptr(), bytes, libc::MNT_NOWAIT) };
    if filled < 0 {
        return Err(SysError::os("getfsstat"));
    }
    buf.truncate(filled as usize);
    Ok(buf
        .iter()
        .map(|s| FsEntry {
            from: c_string(&s.f_mntfromname),
            on: c_string(&s.f_mntonname),
            fs_type: c_string(&s.f_fstypename),
            block_size: u64::from(s.f_bsize),
            blocks: s.f_blocks,
            available: s.f_bavail,
            flags: s.f_flags,
        })
        .collect())
}

/// The name of the startup volume: `/Volumes` holds a symbolic link to `/` with that name.
pub fn startup_volume_name() -> String {
    std::fs::read_dir("/Volumes")
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .find(|e| std::fs::read_link(e.path()).is_ok_and(|target| target == std::path::Path::new("/")))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .unwrap_or_else(|| "Startup disk".to_string())
}

/// Cumulative counters of one physical disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiskCounters {
    pub name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

/// Reads one `IOBlockStorageDriver`: its `Statistics` and the whole-disk media below it, which carries the BSD name.
///
/// Drivers without a whole-disk media (nothing a person can name) return `None`.
pub fn counters_from(driver: &Value, media: &[Value]) -> Option<DiskCounters> {
    let stats = driver.get("Statistics")?;
    let name = media
        .iter()
        .find(|m| m.get("Whole").and_then(Value::as_bool) == Some(true))?
        .get("BSD Name")?
        .as_str()?
        .to_string();
    let counter = |key: &str| stats.get(key).and_then(Value::as_u64).unwrap_or(0);
    Some(DiskCounters {
        name,
        read_bytes: counter("Bytes (Read)"),
        write_bytes: counter("Bytes (Write)"),
        read_ops: counter("Operations (Read)"),
        write_ops: counter("Operations (Write)"),
    })
}

pub fn read_counters() -> Result<Vec<DiskCounters>> {
    let mut out = Vec::new();
    for driver in iokit::services("IOBlockStorageDriver")? {
        let Ok(props) = driver.properties() else { continue };
        let media: Vec<Value> = driver.children().iter().filter_map(|c| c.properties().ok()).collect();
        if let Some(counters) = counters_from(&props, &media) {
            out.push(counters);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Keeps the previous counters of each disk so each call returns throughput since the last one.
pub struct DiskCollector {
    previous: HashMap<String, DiskCounters>,
    root_name: String,
}

impl DiskCollector {
    pub fn new() -> Self {
        let previous = read_counters().unwrap_or_default().into_iter().map(|c| (c.name.clone(), c)).collect();
        Self { previous, root_name: startup_volume_name() }
    }

    pub fn sample(&mut self, elapsed_ms: u64) -> Result<DiskStats> {
        let volumes = volumes_from(&read_filesystems()?, &self.root_name);
        let current = read_counters().unwrap_or_default();
        let io = current
            .iter()
            .map(|now| {
                let before = self.previous.get(&now.name).unwrap_or(now);
                DiskIo {
                    name: now.name.clone(),
                    read_bps: per_second(before.read_bytes, now.read_bytes, elapsed_ms),
                    write_bps: per_second(before.write_bytes, now.write_bytes, elapsed_ms),
                    read_ops: per_second(before.read_ops, now.read_ops, elapsed_ms),
                    write_ops: per_second(before.write_ops, now.write_ops, elapsed_ms),
                }
            })
            .collect();
        self.previous = current.into_iter().map(|c| (c.name.clone(), c)).collect();
        Ok(DiskStats { volumes, io })
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(on: &str, fs: &str, flags: i32, blocks: u64, avail: u64) -> FsEntry {
        FsEntry {
            from: format!("/dev/{on}"),
            on: on.into(),
            fs_type: fs.into(),
            block_size: 4096,
            blocks,
            available: avail,
            flags: flags as u32,
        }
    }

    #[test]
    fn keeps_the_startup_volume_and_visible_local_ones() {
        let local = libc::MNT_LOCAL;
        let hidden = libc::MNT_DONTBROWSE;
        let entries = [
            entry("/", "apfs", local, 1000, 400),
            entry("/dev", "devfs", local | hidden, 1, 0),
            entry("/System/Volumes/VM", "apfs", local | hidden, 1000, 400),
            entry("/System/Volumes/Data", "apfs", local | hidden, 1000, 400),
            entry("/Volumes/SSD512", "apfs", local, 500, 100),
            entry("/Volumes/Share", "smbfs", 0, 500, 100),
            entry("/home", "autofs", local, 0, 0),
        ];
        let volumes = volumes_from(&entries, "Macintosh HD");
        assert_eq!(volumes.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["Macintosh HD", "SSD512"]);
        assert_eq!(volumes[0].total, 1000 * 4096);
        assert_eq!(volumes[0].free, 400 * 4096);
        assert_eq!(volumes[1].mount_point, "/Volumes/SSD512");
    }

    #[test]
    fn an_empty_file_system_is_not_a_volume() {
        assert!(volumes_from(&[entry("/Volumes/Empty", "apfs", libc::MNT_LOCAL, 0, 0)], "x").is_empty());
    }

    const DRIVERS: &[u8] = include_bytes!("../tests/fixtures/intel-macpro7-1/iostorage.plist");

    #[test]
    fn reads_real_block_storage_statistics() {
        let tree = Value::from_plist_bytes(DRIVERS).unwrap();
        let drivers = tree.as_array().expect("a list of drivers");
        assert!(!drivers.is_empty());
        let found: Vec<DiskCounters> = drivers
            .iter()
            .filter_map(|d| {
                let media: Vec<Value> = d.get("IORegistryEntryChildren")?.as_array()?.to_vec();
                counters_from(d, &media)
            })
            .collect();
        assert!(!found.is_empty(), "at least one whole disk");
        assert!(found.iter().all(|c| c.name.starts_with("disk")));
        assert!(found.iter().any(|c| c.read_bytes > 0), "a disk has read something since boot");
    }

    #[test]
    fn a_driver_without_a_whole_disk_is_skipped() {
        let driver = Value::from_plist_bytes(
            br#"<plist version="1.0"><dict><key>Statistics</key><dict><key>Bytes (Read)</key><integer>5</integer></dict></dict></plist>"#,
        )
        .unwrap();
        assert!(counters_from(&driver, &[]).is_none());
    }

    #[test]
    fn reads_this_macs_volumes_and_disks() {
        let mut collector = DiskCollector::new();
        std::thread::sleep(std::time::Duration::from_millis(150));
        let stats = collector.sample(150).unwrap();
        let root = stats.volumes.iter().find(|v| v.mount_point == "/").expect("the startup volume");
        assert!(root.total > 0 && root.free <= root.total);
        assert!(stats.io.iter().all(|d| d.read_bps >= 0.0 && d.write_bps >= 0.0));
    }
}
