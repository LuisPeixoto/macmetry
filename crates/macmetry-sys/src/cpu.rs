//! Processor usage per core, from the kernel's tick counters.

use crate::error::{Result, SysError};
use crate::{mach, sysctl};
use macmetry_core::rates::{CpuShare, CpuTicks, average_share, cpu_share};
use macmetry_core::{ClusterUsage, CoreKind, CoreUsage, CpuUsage};

/// How the logical cores split between efficiency and performance cores.
///
/// On Apple silicon the kernel numbers the efficiency cores first. That ordering is an observed convention and is
/// confirmed with `tools/capture-fixtures.sh` on real hardware; Intel has no split.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoreLayout {
    pub efficiency: u32,
    pub performance: u32,
}

impl CoreLayout {
    pub fn kind_of(&self, index: u32) -> CoreKind {
        if self.efficiency == 0 || self.performance == 0 {
            CoreKind::Unknown
        } else if index < self.efficiency {
            CoreKind::Efficiency
        } else if index < self.efficiency + self.performance {
            CoreKind::Performance
        } else {
            CoreKind::Unknown
        }
    }
}

/// Usage between two tick readings. Pure, so it is tested with made-up ticks.
///
/// When the number of cores changed between the readings, the result is an empty usage rather than a wrong one.
pub fn usage_between(
    previous: &[CpuTicks],
    current: &[CpuTicks],
    layout: CoreLayout,
    load_average: [f64; 3],
    nominal_mhz: Option<f64>,
) -> CpuUsage {
    let shares: Vec<CpuShare> = if previous.len() == current.len() {
        previous.iter().zip(current).map(|(&a, &b)| cpu_share(a, b)).collect()
    } else {
        vec![CpuShare::default(); current.len()]
    };
    let overall = average_share(&shares);
    let cores: Vec<CoreUsage> = shares
        .iter()
        .enumerate()
        .map(|(i, s)| CoreUsage {
            index: i as u32,
            kind: layout.kind_of(i as u32),
            usage_pct: s.busy(),
            frequency_mhz: None,
        })
        .collect();
    let cluster = |kind: CoreKind, name: &str| {
        let members: Vec<CpuShare> =
            cores.iter().zip(&shares).filter(|(c, _)| c.kind == kind).map(|(_, s)| *s).collect();
        (!members.is_empty()).then(|| ClusterUsage {
            name: name.to_string(),
            usage_pct: average_share(&members).busy(),
            frequency_mhz: None,
        })
    };
    let clusters = [cluster(CoreKind::Efficiency, "Efficiency"), cluster(CoreKind::Performance, "Performance")]
        .into_iter()
        .flatten()
        .collect();
    CpuUsage {
        total_pct: overall.busy(),
        user_pct: overall.user,
        system_pct: overall.system,
        idle_pct: overall.idle,
        cores,
        clusters,
        load_average,
        frequency_mhz: nominal_mhz,
        frequency_is_measured: false,
    }
}

/// Reads the cumulative ticks of every logical core.
pub fn read_ticks() -> Result<Vec<CpuTicks>> {
    let mut count: libc::natural_t = 0;
    let mut info: libc::processor_info_array_t = std::ptr::null_mut();
    let mut info_count: libc::mach_msg_type_number_t = 0;
    // SAFETY: all three out parameters are valid pointers; the host port is valid for the life of the process.
    let rc = unsafe {
        libc::host_processor_info(
            mach::host_self(),
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut count,
            &mut info,
            &mut info_count,
        )
    };
    if rc != 0 || info.is_null() {
        return Err(SysError::kern("host_processor_info", rc));
    }
    // SAFETY: on success the kernel returned `info_count` integers at `info`, which stay valid until deallocated.
    let raw = unsafe { std::slice::from_raw_parts(info.cast::<u32>(), info_count as usize) };
    let ticks = raw
        .as_chunks::<{ libc::CPU_STATE_MAX as usize }>()
        .0
        .iter()
        .map(|c| CpuTicks {
            user: c[libc::CPU_STATE_USER as usize],
            system: c[libc::CPU_STATE_SYSTEM as usize],
            idle: c[libc::CPU_STATE_IDLE as usize],
            nice: c[libc::CPU_STATE_NICE as usize],
        })
        .collect();
    // SAFETY: the array was allocated in this task by the call above and is not used after this point.
    unsafe {
        libc::vm_deallocate(
            mach::task_self(),
            info as libc::vm_address_t,
            info_count as usize * std::mem::size_of::<libc::integer_t>(),
        );
    }
    Ok(ticks)
}

/// The three load averages.
pub fn load_average() -> [f64; 3] {
    let mut loads = [0f64; 3];
    // SAFETY: `loads` has room for the 3 samples requested.
    let n = unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    if n == 3 { loads } else { [0.0; 3] }
}

/// Keeps the previous reading so each call returns the usage since the last one.
pub struct CpuCollector {
    previous: Vec<CpuTicks>,
    layout: CoreLayout,
    nominal_mhz: Option<f64>,
}

impl CpuCollector {
    /// Takes the first reading. The first [`sample`](Self::sample) covers the time since this call.
    pub fn new() -> Result<Self> {
        let arm64 = sysctl::number("hw.optional.arm64").unwrap_or(0) == 1;
        let levels = sysctl::number("hw.nperflevels").unwrap_or(1);
        let layout = if arm64 && levels >= 2 {
            CoreLayout {
                performance: sysctl::number("hw.perflevel0.logicalcpu").unwrap_or(0) as u32,
                efficiency: sysctl::number("hw.perflevel1.logicalcpu").unwrap_or(0) as u32,
            }
        } else {
            CoreLayout::default()
        };
        // Only Intel exposes a nominal frequency; Apple silicon's is read from the reporting framework instead.
        let nominal_mhz = sysctl::number("hw.cpufrequency").ok().filter(|&hz| hz > 0).map(|hz| hz as f64 / 1e6);
        Ok(Self { previous: read_ticks()?, layout, nominal_mhz })
    }

    pub fn layout(&self) -> CoreLayout {
        self.layout
    }

    pub fn sample(&mut self) -> Result<CpuUsage> {
        let current = read_ticks()?;
        let usage = usage_between(&self.previous, &current, self.layout, load_average(), self.nominal_mhz);
        self.previous = current;
        Ok(usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(user: u32, system: u32, idle: u32) -> CpuTicks {
        CpuTicks { user, system, idle, nice: 0 }
    }

    #[test]
    fn per_core_and_overall_usage() {
        let before = [ticks(0, 0, 0), ticks(0, 0, 0)];
        // Core 0 is busy 75% (50 user + 25 system); core 1 is idle.
        let after = [ticks(50, 25, 25), ticks(0, 0, 100)];
        let u = usage_between(&before, &after, CoreLayout::default(), [1.0, 2.0, 3.0], Some(3400.0));
        assert_eq!(u.cores.len(), 2);
        assert!((u.cores[0].usage_pct - 75.0).abs() < 1e-9);
        assert_eq!(u.cores[1].usage_pct, 0.0);
        assert!((u.total_pct - 37.5).abs() < 1e-9);
        assert!((u.user_pct - 25.0).abs() < 1e-9);
        assert!((u.idle_pct - 62.5).abs() < 1e-9);
        assert_eq!(u.load_average, [1.0, 2.0, 3.0]);
        assert_eq!(u.frequency_mhz, Some(3400.0));
        assert!(!u.frequency_is_measured);
        assert!(u.clusters.is_empty(), "Intel has no clusters");
    }

    #[test]
    fn efficiency_cores_come_first_on_apple_silicon() {
        let layout = CoreLayout { efficiency: 2, performance: 2 };
        assert_eq!(layout.kind_of(0), CoreKind::Efficiency);
        assert_eq!(layout.kind_of(1), CoreKind::Efficiency);
        assert_eq!(layout.kind_of(2), CoreKind::Performance);
        assert_eq!(layout.kind_of(3), CoreKind::Performance);
        assert_eq!(layout.kind_of(4), CoreKind::Unknown);
        assert_eq!(CoreLayout::default().kind_of(0), CoreKind::Unknown);
    }

    #[test]
    fn clusters_average_their_cores() {
        let layout = CoreLayout { efficiency: 1, performance: 2 };
        let before = [ticks(0, 0, 0); 3];
        let after = [ticks(100, 0, 0), ticks(0, 0, 100), ticks(50, 0, 50)];
        let u = usage_between(&before, &after, layout, [0.0; 3], None);
        assert_eq!(u.clusters.len(), 2);
        assert_eq!(u.clusters[0].name, "Efficiency");
        assert_eq!(u.clusters[0].usage_pct, 100.0);
        assert_eq!(u.clusters[1].name, "Performance");
        assert_eq!(u.clusters[1].usage_pct, 25.0);
    }

    #[test]
    fn a_changed_core_count_gives_zeros_not_garbage() {
        let u =
            usage_between(&[ticks(0, 0, 0)], &[ticks(9, 9, 9), ticks(9, 9, 9)], CoreLayout::default(), [0.0; 3], None);
        assert_eq!(u.cores.len(), 2);
        assert_eq!(u.total_pct, 0.0);
    }

    #[test]
    fn reads_this_macs_cores() {
        let ticks = read_ticks().unwrap();
        let logical = sysctl::number("hw.logicalcpu").unwrap() as usize;
        assert_eq!(ticks.len(), logical);
        assert!(ticks.iter().any(|t| t.idle > 0));
    }

    #[test]
    fn samples_a_plausible_usage() {
        let mut collector = CpuCollector::new().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let u = collector.sample().unwrap();
        assert!((0.0..=100.0).contains(&u.total_pct));
        assert!((u.user_pct + u.system_pct + u.idle_pct - 100.0).abs() < 0.5);
        assert!(u.cores.iter().all(|c| (0.0..=100.0).contains(&c.usage_pct)));
    }

    #[test]
    fn load_average_is_not_negative() {
        assert!(load_average().iter().all(|&l| l >= 0.0));
    }
}
