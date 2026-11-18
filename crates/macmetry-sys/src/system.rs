//! Facts about the Mac that do not change while it runs.

use crate::error::{Result, SysError};
use crate::sysctl;
use macmetry_core::{Arch, SystemInfo};

/// Reads the static facts. Only the model is required; anything else that is missing stays at its default.
pub fn read() -> Result<SystemInfo> {
    let model = sysctl::string("hw.model").map_err(|_| SysError::Unsupported("The Mac's model"))?;
    // `hw.optional.arm64` is 1 on Apple silicon even for a process that Rosetta translates.
    let arm64 = sysctl::number("hw.optional.arm64").unwrap_or(0) == 1;
    let levels = sysctl::number("hw.nperflevels").unwrap_or(1);
    let (performance_cores, efficiency_cores) = if arm64 && levels >= 2 {
        (
            sysctl::number("hw.perflevel0.physicalcpu").ok().map(|n| n as u32),
            sysctl::number("hw.perflevel1.physicalcpu").ok().map(|n| n as u32),
        )
    } else {
        (None, None)
    };
    let boot: Option<libc::timeval> = sysctl::structure("kern.boottime").ok();
    Ok(SystemInfo {
        model,
        chip: sysctl::string("machdep.cpu.brand_string").unwrap_or_default(),
        arch: if arm64 { Arch::Arm64 } else { Arch::X86_64 },
        os_version: sysctl::string("kern.osproductversion").unwrap_or_default(),
        os_build: sysctl::string("kern.osversion").unwrap_or_default(),
        physical_cores: sysctl::number("hw.physicalcpu").unwrap_or(0) as u32,
        logical_cores: sysctl::number("hw.logicalcpu").unwrap_or(0) as u32,
        performance_cores,
        efficiency_cores,
        memory_bytes: sysctl::number("hw.memsize").unwrap_or(0),
        boot_time_ms: boot.map(|t| timeval_ms(t.tv_sec, i64::from(t.tv_usec))).unwrap_or(0),
    })
}

/// A `timeval` as milliseconds. Negative parts (not expected) become 0.
fn timeval_ms(seconds: i64, micros: i64) -> u64 {
    u64::try_from(seconds).unwrap_or(0).saturating_mul(1000) + u64::try_from(micros).unwrap_or(0) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_a_timeval() {
        assert_eq!(timeval_ms(2, 500_000), 2_500);
        assert_eq!(timeval_ms(-1, -5), 0);
    }

    #[test]
    fn describes_this_mac() {
        let info = read().unwrap();
        assert!(info.model.contains(','), "model looks like MacPro7,1: {}", info.model);
        assert!(info.logical_cores >= info.physical_cores && info.physical_cores >= 1);
        assert!(info.memory_bytes >= 1 << 30);
        assert!(info.boot_time_ms > 1_600_000_000_000, "booted after 2020");
        assert!(!info.os_version.is_empty());
        assert_ne!(info.arch, Arch::Unknown);
        if info.arch == Arch::X86_64 {
            assert!(info.performance_cores.is_none(), "Intel has no performance/efficiency split");
        }
    }
}
