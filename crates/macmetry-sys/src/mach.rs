//! Mach ports that are used for the whole life of the program.

use std::sync::OnceLock;

#[link(name = "System")]
unsafe extern "C" {
    static mach_task_self_: u32;
    fn mach_host_self() -> u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_are_stable() {
        assert_ne!(task_self(), 0);
        assert_eq!(host_self(), host_self());
    }
}
