//! Per-process CPU, memory, threads and disk I/O.
//!
//! Without root a program can read only its own user's processes: the kernel refuses the others with `EPERM`, and
//! those are skipped. Activity Monitor shows them through a privileged helper that macmetry does not have.

use crate::error::{Result, SysError};
use macmetry_core::ProcessSample;
use macmetry_core::rates::{per_second, process_cpu_percent};
use std::collections::HashMap;
use std::ffi::CStr;

/// Identifies a process across samples. The start time tells apart two processes that got the same pid in turn.
pub type ProcessKey = (i32, u64);

/// Cumulative figures for one process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessCounters {
    pub pid: i32,
    /// When the process started, in mach time units. Zero is allowed; it only weakens pid-reuse detection.
    pub start: u64,
    pub name: String,
    pub uid: u32,
    /// CPU time (user + system) in nanoseconds.
    pub cpu_ns: u64,
    /// Physical memory footprint in bytes, the figure Activity Monitor shows as "Memory".
    pub footprint: u64,
    pub threads: u32,
    pub disk_read: u64,
    pub disk_write: u64,
}

impl ProcessCounters {
    pub fn key(&self) -> ProcessKey {
        (self.pid, self.start)
    }
}

/// Converts mach time units to nanoseconds. On Intel one unit is one nanosecond; on Apple silicon it is 125/3.
pub fn mach_to_ns(ticks: u64, numer: u32, denom: u32) -> u64 {
    if denom == 0 {
        return ticks;
    }
    (u128::from(ticks) * u128::from(numer) / u128::from(denom)).min(u128::from(u64::MAX)) as u64
}

/// Turns two readings into rates. A process that has no earlier reading (just started, or its pid was reused) shows
/// 0 for the rates this time. Pure, so it is tested with made-up counters.
pub fn samples_from(
    previous: &HashMap<ProcessKey, ProcessCounters>,
    current: &[ProcessCounters],
    elapsed_ms: u64,
    mut user_name: impl FnMut(u32) -> String,
) -> Vec<ProcessSample> {
    current
        .iter()
        .map(|now| {
            let before = previous.get(&now.key());
            ProcessSample {
                pid: now.pid,
                name: now.name.clone(),
                user: user_name(now.uid),
                cpu_pct: before.map_or(0.0, |b| process_cpu_percent(b.cpu_ns, now.cpu_ns, elapsed_ms)),
                memory_bytes: now.footprint,
                threads: now.threads,
                disk_read_bps: before.map_or(0.0, |b| per_second(b.disk_read, now.disk_read, elapsed_ms)),
                disk_write_bps: before.map_or(0.0, |b| per_second(b.disk_write, now.disk_write, elapsed_ms)),
            }
        })
        .collect()
}

fn list_pids() -> Result<Vec<i32>> {
    // SAFETY: a null buffer asks only for the number of processes.
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Err(SysError::os("proc_listallpids"));
    }
    // Room for processes that start between the two calls.
    let mut pids = vec![0i32; count as usize + 64];
    let size = (pids.len() * std::mem::size_of::<i32>()) as libc::c_int;
    // SAFETY: `pids` has `size` writable bytes.
    let filled = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), size) };
    if filled <= 0 {
        return Err(SysError::os("proc_listallpids"));
    }
    pids.truncate(filled as usize);
    pids.retain(|&p| p > 0);
    Ok(pids)
}

fn executable_name(pid: i32) -> String {
    let mut buf = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: `buf` has the size passed.
    let len = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if len > 0 {
        let path = String::from_utf8_lossy(&buf[..len as usize]).into_owned();
        if let Some(name) = path.rsplit('/').next().filter(|n| !n.is_empty()) {
            return name.to_string();
        }
    }
    let mut short = [0u8; 64];
    // SAFETY: `short` has the size passed.
    let len = unsafe { libc::proc_name(pid, short.as_mut_ptr().cast(), short.len() as u32) };
    if len > 0 { String::from_utf8_lossy(&short[..len as usize]).into_owned() } else { format!("pid {pid}") }
}

/// Reads one process, or `None` when the kernel refuses (another user's process) or it has already exited.
fn read_process(pid: i32, numer: u32, denom: u32) -> Option<ProcessCounters> {
    // SAFETY: a zeroed `rusage_info_v4` is a valid value of plain integers.
    let mut usage: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    // SAFETY: `usage` is a writable `rusage_info_v4`, which is what the V4 flavor fills.
    let rc =
        unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V4, (&mut usage as *mut libc::rusage_info_v4).cast()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: a zeroed `proc_taskinfo` is a valid value of plain integers.
    let mut task: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let task_size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
    // SAFETY: `task` has `task_size` writable bytes.
    let got = unsafe {
        libc::proc_pidinfo(pid, libc::PROC_PIDTASKINFO, 0, (&mut task as *mut libc::proc_taskinfo).cast(), task_size)
    };
    let threads = if got == task_size { u32::try_from(task.pti_threadnum).unwrap_or(0) } else { 0 };

    // SAFETY: a zeroed `proc_bsdinfo` is a valid value of plain integers and arrays.
    let mut bsd: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let bsd_size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: `bsd` has `bsd_size` writable bytes.
    let got = unsafe {
        libc::proc_pidinfo(pid, libc::PROC_PIDTBSDINFO, 0, (&mut bsd as *mut libc::proc_bsdinfo).cast(), bsd_size)
    };
    let uid = if got == bsd_size { bsd.pbi_uid } else { u32::MAX };

    Some(ProcessCounters {
        pid,
        start: usage.ri_proc_start_abstime,
        name: executable_name(pid),
        uid,
        cpu_ns: mach_to_ns(usage.ri_user_time.saturating_add(usage.ri_system_time), numer, denom),
        footprint: usage.ri_phys_footprint,
        threads,
        disk_read: usage.ri_diskio_bytesread,
        disk_write: usage.ri_diskio_byteswritten,
    })
}

/// The name of a user, or the number when it has no name.
fn lookup_user(uid: u32) -> String {
    if uid == u32::MAX {
        return String::new();
    }
    let mut buf = vec![0 as libc::c_char; 1024];
    // SAFETY: a zeroed `passwd` is a valid value to be filled in.
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: all pointers are valid and `buf` has the length passed.
    let rc = unsafe { libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buf.len(), &mut out) };
    if rc == 0 && !out.is_null() {
        // SAFETY: on success `pw_name` points to a NUL-terminated string inside `buf`.
        return unsafe { CStr::from_ptr(pwd.pw_name) }.to_string_lossy().into_owned();
    }
    uid.to_string()
}

/// Reads every process the kernel lets this user see.
pub fn read_all() -> Result<Vec<ProcessCounters>> {
    let (numer, denom) = crate::mach::timebase();
    Ok(list_pids()?.into_iter().filter_map(|pid| read_process(pid, numer, denom)).collect())
}

/// Keeps the previous readings so each call returns rates since the last one.
pub struct ProcessCollector {
    previous: HashMap<ProcessKey, ProcessCounters>,
    users: HashMap<u32, String>,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let previous = read_all().unwrap_or_default().into_iter().map(|p| (p.key(), p)).collect();
        Self { previous, users: HashMap::new() }
    }

    /// Every readable process, in no particular order. Use [`macmetry_core::top`] to sort and trim.
    pub fn sample(&mut self, elapsed_ms: u64) -> Result<Vec<ProcessSample>> {
        let current = read_all()?;
        let users = &mut self.users;
        let mut name_of = |uid: u32| users.entry(uid).or_insert_with(|| lookup_user(uid)).clone();
        let samples = samples_from(&self.previous, &current, elapsed_ms, &mut name_of);
        self.previous = current.into_iter().map(|p| (p.key(), p)).collect();
        Ok(samples)
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counters(pid: i32, start: u64, cpu_ns: u64, read: u64) -> ProcessCounters {
        ProcessCounters {
            pid,
            start,
            name: format!("p{pid}"),
            uid: 501,
            cpu_ns,
            footprint: 1 << 20,
            threads: 3,
            disk_read: read,
            disk_write: 0,
        }
    }

    #[test]
    fn converts_mach_time() {
        assert_eq!(mach_to_ns(1_000, 1, 1), 1_000, "Intel");
        assert_eq!(mach_to_ns(24, 125, 3), 1_000, "Apple silicon");
        assert_eq!(mach_to_ns(7, 5, 0), 7, "a bad timebase does not divide by zero");
        assert_eq!(mach_to_ns(u64::MAX, 125, 3), u64::MAX, "no overflow");
    }

    #[test]
    fn computes_cpu_and_disk_rates() {
        let before: HashMap<_, _> = [counters(10, 1, 1_000_000_000, 1_000)].into_iter().map(|c| (c.key(), c)).collect();
        let now = [counters(10, 1, 2_500_000_000, 5_000)];
        let out = samples_from(&before, &now, 1_000, |_| "luis".into());
        assert_eq!(out[0].cpu_pct, 150.0, "1.5 s of CPU in 1 s is 150% of a core");
        assert_eq!(out[0].disk_read_bps, 4_000.0);
        assert_eq!(out[0].user, "luis");
        assert_eq!(out[0].threads, 3);
    }

    #[test]
    fn a_new_process_has_no_rates_yet() {
        let out = samples_from(&HashMap::new(), &[counters(11, 1, 9_000_000_000, 9_000)], 1_000, |_| String::new());
        assert_eq!(out[0].cpu_pct, 0.0);
        assert_eq!(out[0].disk_read_bps, 0.0);
    }

    #[test]
    fn a_reused_pid_is_a_different_process() {
        let before: HashMap<_, _> = [counters(12, 100, 50_000_000_000, 0)].into_iter().map(|c| (c.key(), c)).collect();
        // Same pid, later start: the old process's big CPU total must not be subtracted from the new one's.
        let out = samples_from(&before, &[counters(12, 200, 1_000, 0)], 1_000, |_| String::new());
        assert_eq!(out[0].cpu_pct, 0.0);
    }

    #[test]
    fn lists_this_process() {
        let me = std::process::id() as i32;
        let all = read_all().unwrap();
        assert!(all.len() > 10, "a Mac runs many processes");
        let mine = all.iter().find(|p| p.pid == me).expect("this test process is readable");
        assert!(mine.footprint > 0 && mine.threads >= 1 && mine.cpu_ns > 0);
        assert!(!mine.name.is_empty());
    }

    #[test]
    fn samples_with_names_and_users() {
        let mut collector = ProcessCollector::new();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let out = collector.sample(200).unwrap();
        let me = std::process::id() as i32;
        let mine = out.iter().find(|p| p.pid == me).unwrap();
        assert!(!mine.user.is_empty());
        assert!(out.iter().all(|p| p.cpu_pct >= 0.0));
    }
}
