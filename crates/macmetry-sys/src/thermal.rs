//! The thermal pressure level that macOS publishes through the notification center.

use macmetry_core::ThermalPressure;
use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn notify_register_check(name: *const c_char, out_token: *mut c_int) -> u32;
    fn notify_get_state(token: c_int, state: *mut u64) -> u32;
    fn notify_cancel(token: c_int) -> u32;
}

/// `kOSThermalNotificationPressureLevelName`.
const NAME: &std::ffi::CStr = c"com.apple.system.thermalpressurelevel";
const NOTIFY_STATUS_OK: u32 = 0;

/// 0 nominal, 1 moderate, 2 heavy, 3 trapping, 4 sleeping.
pub fn from_state(state: u64) -> Option<ThermalPressure> {
    match state {
        0 => Some(ThermalPressure::Nominal),
        1 => Some(ThermalPressure::Moderate),
        2 => Some(ThermalPressure::Heavy),
        3 => Some(ThermalPressure::Trapping),
        4 => Some(ThermalPressure::Sleeping),
        _ => None,
    }
}

/// A registration for the thermal pressure state. Reading it is cheap.
pub struct ThermalCollector {
    token: c_int,
}

impl ThermalCollector {
    /// `None` when the system does not publish the level.
    pub fn new() -> Option<Self> {
        let mut token: c_int = 0;
        // SAFETY: `NAME` is a valid C string and `token` a valid out pointer.
        let status = unsafe { notify_register_check(NAME.as_ptr(), &mut token) };
        (status == NOTIFY_STATUS_OK).then_some(Self { token })
    }

    pub fn sample(&self) -> Option<ThermalPressure> {
        let mut state: u64 = 0;
        // SAFETY: the token came from a successful registration and `state` is a valid out pointer.
        let status = unsafe { notify_get_state(self.token, &mut state) };
        if status == NOTIFY_STATUS_OK { from_state(state) } else { None }
    }
}

impl Drop for ThermalCollector {
    fn drop(&mut self) {
        // SAFETY: the token came from a successful registration and is cancelled once.
        unsafe { notify_cancel(self.token) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_known_level() {
        assert_eq!(from_state(0), Some(ThermalPressure::Nominal));
        assert_eq!(from_state(2), Some(ThermalPressure::Heavy));
        assert_eq!(from_state(4), Some(ThermalPressure::Sleeping));
        assert_eq!(from_state(5), None);
    }

    #[test]
    fn this_mac_reports_a_level() {
        let collector = ThermalCollector::new().expect("the thermal pressure notification exists");
        assert!(collector.sample().is_some());
    }
}
