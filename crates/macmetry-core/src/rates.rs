//! Turning counters that only grow into rates and percentages.

/// Rate per second between two readings of a counter.
///
/// A counter that went backwards (the device was reset, or a 32-bit counter wrapped) yields 0 instead of a huge
/// number, and a zero interval yields 0 instead of infinity.
pub fn per_second(previous: u64, current: u64, elapsed_ms: u64) -> f64 {
    if elapsed_ms == 0 || current < previous {
        return 0.0;
    }
    (current - previous) as f64 * 1000.0 / elapsed_ms as f64
}

/// Like [`per_second`] for a 32-bit counter that is allowed to wrap, as the kernel's tick counters do.
pub fn per_second_wrapping_u32(previous: u32, current: u32, elapsed_ms: u64) -> f64 {
    if elapsed_ms == 0 {
        return 0.0;
    }
    current.wrapping_sub(previous) as f64 * 1000.0 / elapsed_ms as f64
}

/// `part` as a percentage of `whole`, kept between 0 and 100. Returns 0 when `whole` is 0.
pub fn percent(part: f64, whole: f64) -> f64 {
    if whole <= 0.0 || !part.is_finite() || !whole.is_finite() {
        return 0.0;
    }
    (part / whole * 100.0).clamp(0.0, 100.0)
}

/// Cumulative CPU time of one core, in scheduler ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTicks {
    pub user: u32,
    pub system: u32,
    pub idle: u32,
    pub nice: u32,
}

/// Where a core's time went between two readings, in percent. The four values add up to 100 (or all 0 when no
/// time passed).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CpuShare {
    pub user: f64,
    pub system: f64,
    pub idle: f64,
}

impl CpuShare {
    /// Everything that was not idle. Computed from the busy states, not as `100 - idle`, so an interval in which no
    /// time passed reads 0% busy instead of 100%.
    pub fn busy(&self) -> f64 {
        (self.user + self.system).clamp(0.0, 100.0)
    }
}

/// The share of time each state took between `previous` and `current`. Tick counters wrap, so subtraction wraps.
pub fn cpu_share(previous: CpuTicks, current: CpuTicks) -> CpuShare {
    let user =
        u64::from(current.user.wrapping_sub(previous.user)) + u64::from(current.nice.wrapping_sub(previous.nice));
    let system = u64::from(current.system.wrapping_sub(previous.system));
    let idle = u64::from(current.idle.wrapping_sub(previous.idle));
    let total = user + system + idle;
    if total == 0 {
        return CpuShare::default();
    }
    let total = total as f64;
    CpuShare {
        user: user as f64 / total * 100.0,
        system: system as f64 / total * 100.0,
        idle: idle as f64 / total * 100.0,
    }
}

/// The average of per-core shares, weighting every core equally.
pub fn average_share(shares: &[CpuShare]) -> CpuShare {
    if shares.is_empty() {
        return CpuShare::default();
    }
    let n = shares.len() as f64;
    CpuShare {
        user: shares.iter().map(|s| s.user).sum::<f64>() / n,
        system: shares.iter().map(|s| s.system).sum::<f64>() / n,
        idle: shares.iter().map(|s| s.idle).sum::<f64>() / n,
    }
}

/// Converts the CPU time a process used between two readings into a percentage of one core.
///
/// `previous_ns` and `current_ns` are cumulative CPU nanoseconds. The result can exceed 100 for a multi-threaded
/// process, as in Activity Monitor.
pub fn process_cpu_percent(previous_ns: u64, current_ns: u64, elapsed_ms: u64) -> f64 {
    if elapsed_ms == 0 || current_ns < previous_ns {
        return 0.0;
    }
    (current_ns - previous_ns) as f64 / (elapsed_ms as f64 * 1_000_000.0) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_is_per_second() {
        assert_eq!(per_second(1_000, 3_000, 500), 4_000.0);
    }

    #[test]
    fn rate_ignores_a_reset_counter_and_a_zero_interval() {
        assert_eq!(per_second(5_000, 100, 1_000), 0.0);
        assert_eq!(per_second(1, 2, 0), 0.0);
    }

    #[test]
    fn wrapping_counter_keeps_counting() {
        // 10 ticks before the wrap and 5 after: 15 ticks in one second.
        assert_eq!(per_second_wrapping_u32(u32::MAX - 9, 5, 1_000), 15.0);
    }

    #[test]
    fn percent_is_clamped_and_safe() {
        assert_eq!(percent(50.0, 200.0), 25.0);
        assert_eq!(percent(300.0, 200.0), 100.0);
        assert_eq!(percent(1.0, 0.0), 0.0);
        assert_eq!(percent(f64::NAN, 10.0), 0.0);
    }

    #[test]
    fn cpu_share_splits_the_interval() {
        let a = CpuTicks { user: 100, system: 50, idle: 800, nice: 0 };
        let b = CpuTicks { user: 130, system: 60, idle: 860, nice: 10 };
        let share = cpu_share(a, b);
        // user 30 + nice 10 = 40, system 10, idle 60 -> 110 ticks.
        assert!((share.user - 40.0 / 110.0 * 100.0).abs() < 1e-9);
        assert!((share.system - 10.0 / 110.0 * 100.0).abs() < 1e-9);
        assert!((share.idle - 60.0 / 110.0 * 100.0).abs() < 1e-9);
        assert!((share.user + share.system + share.idle - 100.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_share_survives_a_tick_wrap() {
        let a = CpuTicks { user: u32::MAX - 4, system: 0, idle: 0, nice: 0 };
        let b = CpuTicks { user: 5, system: 0, idle: 10, nice: 0 };
        let share = cpu_share(a, b);
        assert!((share.user - 50.0).abs() < 1e-9);
        assert!((share.idle - 50.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_share_with_no_time_is_zero() {
        let a = CpuTicks { user: 1, system: 2, idle: 3, nice: 4 };
        assert_eq!(cpu_share(a, a), CpuShare::default());
    }

    #[test]
    fn no_elapsed_time_is_not_a_busy_cpu() {
        let a = CpuTicks { user: 1, system: 2, idle: 3, nice: 4 };
        assert_eq!(cpu_share(a, a).busy(), 0.0);
    }

    #[test]
    fn average_weights_cores_equally() {
        let busy = CpuShare { user: 100.0, system: 0.0, idle: 0.0 };
        let idle = CpuShare { user: 0.0, system: 0.0, idle: 100.0 };
        let avg = average_share(&[busy, idle]);
        assert_eq!(avg.user, 50.0);
        assert_eq!(avg.busy(), 50.0);
        assert_eq!(average_share(&[]), CpuShare::default());
    }

    #[test]
    fn process_cpu_can_exceed_one_core() {
        // 2 seconds of CPU time in 1 second of wall time is 200% of a core.
        assert_eq!(process_cpu_percent(0, 2_000_000_000, 1_000), 200.0);
        assert_eq!(process_cpu_percent(10, 5, 1_000), 0.0);
    }
}
