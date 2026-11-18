//! Mach ports that are used for the whole life of the program.

use std::sync::OnceLock;

#[repr(C)]
struct TimebaseInfo {
    numer: u32,
    denom: u32,
}

#[link(name = "System")]
unsafe extern "C" {
    static mach_task_self_: u32;
    fn mach_host_self() -> u32;
    fn mach_timebase_info(info: *mut TimebaseInfo) -> i32;
}

/// This process's task port.
pub fn task_self() -> u32 {
    // SAFETY: the system initialises `mach_task_self_` before `main` runs and never changes it.
    unsafe { mach_task_self_ }
}

/// The host port. `mach_host_self` hands out a new send right on every call and each one leaks until it is
/// deallocated, so it is fetched once and kept.
pub fn host_self() -> u32 {
    static HOST: OnceLock<u32> = OnceLock::new();
    *HOST.get_or_init(|| {
        // SAFETY: the call has no preconditions.
        unsafe { mach_host_self() }
    })
}

/// The ratio that converts mach time units to nanoseconds: 1/1 on Intel, 125/3 on Apple silicon.
pub fn timebase() -> (u32, u32) {
    let mut info = TimebaseInfo { numer: 0, denom: 0 };
    // SAFETY: `info` is a valid out pointer.
    let rc = unsafe { mach_timebase_info(&mut info) };
    if rc == 0 && info.denom != 0 { (info.numer, info.denom) } else { (1, 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timebase_is_a_usable_ratio() {
        let (numer, denom) = timebase();
        assert!(numer >= 1 && denom >= 1);
    }

    #[test]
    fn ports_are_stable() {
        assert_ne!(task_self(), 0);
        assert_eq!(host_self(), host_self());
    }
}
