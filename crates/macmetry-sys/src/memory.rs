//! Memory use, broken down the way Activity Monitor does.

use crate::error::{Result, SysError};
use crate::{mach, sysctl};
use macmetry_core::rates::per_second;
use macmetry_core::{Memory, MemoryPressure};

/// Page counts from the kernel's virtual memory statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VmCounters {
    pub free: u64,
    pub speculative: u64,
    pub wired: u64,
    pub purgeable: u64,
    pub external: u64,
    pub internal: u64,
    pub compressor: u64,
    pub page_ins: u64,
    pub page_outs: u64,
}

/// Swap space in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Swap {
    pub total: u64,
    pub used: u64,
}

/// The `xsw_usage` structure behind `vm.swapusage`.
#[repr(C)]
#[derive(Clone, Copy)]
struct XswUsage {
    total: u64,
    available: u64,
    used: u64,
    page_size: u32,
    encrypted: bool,
}

/// `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical.
pub fn pressure_from_level(level: u64) -> MemoryPressure {
    match level {
        2 => MemoryPressure::Warning,
        4 => MemoryPressure::Critical,
        _ => MemoryPressure::Normal,
    }
}

/// Builds the breakdown from raw counters. Pure, so it is tested with made-up numbers.
///
/// - **App memory** is the anonymous pages that are not purgeable.
/// - **Wired** cannot be paged out. **Compressed** is what the compressor holds, counted at its compressed size.
/// - **Used** is the sum of those three, which is what Activity Monitor calls "Memory Used".
/// - **Cached** is file-backed and purgeable memory the system gives back when needed.
/// - **Free** leaves out speculative pages, as `vm_stat` does.
pub fn memory_from(counters: &VmCounters, page_size: u64, total: u64, swap: Swap, pressure: MemoryPressure) -> Memory {
    let bytes = |pages: u64| pages.saturating_mul(page_size);
    let app = bytes(counters.internal.saturating_sub(counters.purgeable));
    let wired = bytes(counters.wired);
    let compressed = bytes(counters.compressor);
    Memory {
        total,
        used: (app + wired + compressed).min(total),
        app,
        wired,
        compressed,
        cached: bytes(counters.external + counters.purgeable),
        free: bytes(counters.free.saturating_sub(counters.speculative)),
        swap_total: swap.total,
        swap_used: swap.used,
        pressure,
        page_ins_per_s: 0.0,
        page_outs_per_s: 0.0,
    }
}

pub fn read_counters() -> Result<VmCounters> {
    // SAFETY: a zeroed `vm_statistics64` is a valid value of plain integers.
    let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: `stats` is at least `count` integers long and `count` is a valid in/out pointer.
    let rc = unsafe {
        libc::host_statistics64(
            mach::host_self(),
            libc::HOST_VM_INFO64,
            (&mut stats as *mut libc::vm_statistics64).cast(),
            &mut count,
        )
    };
    if rc != 0 {
        return Err(SysError::kern("host_statistics64", rc));
    }
    Ok(VmCounters {
        free: u64::from(stats.free_count),
        speculative: u64::from(stats.speculative_count),
        wired: u64::from(stats.wire_count),
        purgeable: u64::from(stats.purgeable_count),
        external: u64::from(stats.external_page_count),
        internal: u64::from(stats.internal_page_count),
        compressor: u64::from(stats.compressor_page_count),
        page_ins: stats.pageins,
        page_outs: stats.pageouts,
    })
}

pub fn read_swap() -> Swap {
    match sysctl::structure::<XswUsage>("vm.swapusage") {
        Ok(x) => Swap { total: x.total, used: x.used },
        Err(_) => Swap::default(),
    }
}

/// Keeps the previous paging counters so each call returns rates since the last one.
pub struct MemoryCollector {
    page_size: u64,
    total: u64,
    previous: VmCounters,
}

impl MemoryCollector {
    pub fn new() -> Result<Self> {
        let page_size = sysctl::number("vm.pagesize").or_else(|_| sysctl::number("hw.pagesize"))?;
        Ok(Self { page_size, total: sysctl::number("hw.memsize")?, previous: read_counters()? })
    }

    pub fn sample(&mut self, elapsed_ms: u64) -> Result<Memory> {
        let counters = read_counters()?;
        let pressure =
            sysctl::number("kern.memorystatus_vm_pressure_level").map(pressure_from_level).unwrap_or_default();
        let mut memory = memory_from(&counters, self.page_size, self.total, read_swap(), pressure);
        memory.page_ins_per_s = per_second(self.previous.page_ins, counters.page_ins, elapsed_ms);
        memory.page_outs_per_s = per_second(self.previous.page_outs, counters.page_outs, elapsed_ms);
        self.previous = counters;
        Ok(memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: u64 = 4096;

    #[test]
    fn breaks_memory_down() {
        let counters = VmCounters {
            free: 1_000,
            speculative: 200,
            wired: 2_000,
            purgeable: 100,
            external: 3_000,
            internal: 5_100,
            compressor: 500,
            ..Default::default()
        };
        let m =
            memory_from(&counters, PAGE, 100 << 20, Swap { total: 1 << 30, used: 1 << 20 }, MemoryPressure::Warning);
        assert_eq!(m.app, 5_000 * PAGE, "internal minus purgeable");
        assert_eq!(m.wired, 2_000 * PAGE);
        assert_eq!(m.compressed, 500 * PAGE);
        assert_eq!(m.used, 7_500 * PAGE);
        assert_eq!(m.cached, 3_100 * PAGE, "external plus purgeable");
        assert_eq!(m.free, 800 * PAGE, "free minus speculative");
        assert_eq!(m.swap_used, 1 << 20);
        assert_eq!(m.pressure, MemoryPressure::Warning);
    }

    #[test]
    fn used_never_exceeds_total() {
        let counters = VmCounters { internal: 1_000_000, ..Default::default() };
        let m = memory_from(&counters, PAGE, 1 << 20, Swap::default(), MemoryPressure::Normal);
        assert_eq!(m.used, 1 << 20);
    }

    #[test]
    fn odd_counters_do_not_underflow() {
        // More purgeable than internal, and more speculative than free, happen briefly while pages move.
        let counters = VmCounters { internal: 10, purgeable: 50, free: 5, speculative: 9, ..Default::default() };
        let m = memory_from(&counters, PAGE, 1 << 30, Swap::default(), MemoryPressure::Normal);
        assert_eq!(m.app, 0);
        assert_eq!(m.free, 0);
    }

    #[test]
    fn maps_the_pressure_levels() {
        assert_eq!(pressure_from_level(1), MemoryPressure::Normal);
        assert_eq!(pressure_from_level(2), MemoryPressure::Warning);
        assert_eq!(pressure_from_level(4), MemoryPressure::Critical);
        assert_eq!(pressure_from_level(99), MemoryPressure::Normal);
    }

    #[test]
    fn reads_this_macs_memory() {
        let mut collector = MemoryCollector::new().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let m = collector.sample(100).unwrap();
        assert!(m.total >= 1 << 30);
        assert!(m.used > 0 && m.used <= m.total);
        assert!(m.wired > 0);
        assert!(m.page_ins_per_s >= 0.0 && m.page_outs_per_s >= 0.0);
        assert!(m.swap_used <= m.swap_total.max(m.swap_used));
    }
}
