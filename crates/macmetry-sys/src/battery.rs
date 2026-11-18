//! The internal battery, from the `AppleSmartBattery` registry entry.
//!
//! Desktops have no battery and report [`SysError::Unsupported`], so the interface hides the card.

use crate::error::{Result, SysError};
use crate::iokit;
use crate::value::Value;
use macmetry_core::{Battery, ChargeState};

/// `TimeRemaining` reports 65535 while the system is still working the figure out.
const TIME_UNKNOWN: u64 = 0xFFFF;

/// Reads the battery from its registry properties. Pure, so it is tested with fixtures.
///
/// Two layouts exist. Newer Macs report `CurrentCapacity` as a percentage (and `MaxCapacity` as 100); older Intel
/// ones report both in mAh. The percentage is derived the same way for both.
pub fn battery_from(props: &Value) -> Option<Battery> {
    if props.get("BatteryInstalled").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let number = |key: &str| props.get(key).and_then(Value::as_f64);
    let flag = |key: &str| props.get(key).and_then(Value::as_bool).unwrap_or(false);

    let current = number("CurrentCapacity")?;
    let max = number("MaxCapacity").filter(|&m| m > 0.0)?;
    let percent = if max <= 100.0 { current } else { current / max * 100.0 }.clamp(0.0, 100.0);

    // Apple's "Maximum Capacity" is the nominal charge capacity over the design capacity. Older Macs have only the
    // raw maximum, and the legacy mAh layout has it in `MaxCapacity`.
    let design = number("DesignCapacity").filter(|&d| d > 0.0);
    let full = number("NominalChargeCapacity").or(number("AppleRawMaxCapacity")).or((max > 100.0).then_some(max));
    let health_pct = design.zip(full).map(|(d, f)| (f / d * 100.0).clamp(0.0, 100.0));

    let charging = flag("IsCharging");
    let plugged = flag("ExternalConnected");
    let full_charge = flag("FullyCharged") || (plugged && !charging && percent >= 100.0);
    let state = if charging {
        ChargeState::Charging
    } else if full_charge {
        ChargeState::Charged
    } else if plugged {
        ChargeState::NotCharging
    } else {
        ChargeState::Discharging
    };

    let voltage_v = number("Voltage").map(|mv| mv / 1000.0);
    // Positive while charging, negative while discharging; the registry stores it as a signed number.
    let current_a = number("Amperage").map(|ma| ma / 1000.0);
    let minutes_key = if charging { "AvgTimeToFull" } else { "TimeRemaining" };
    let minutes_remaining = props
        .get(minutes_key)
        .or_else(|| props.get("TimeRemaining"))
        .and_then(Value::as_u64)
        .filter(|&m| m != TIME_UNKNOWN)
        .and_then(|m| u32::try_from(m).ok());

    Some(Battery {
        percent,
        state,
        on_ac_power: plugged,
        health_pct,
        cycle_count: props.get("CycleCount").and_then(Value::as_u64).and_then(|c| u32::try_from(c).ok()),
        // Reported in hundredths of a degree.
        temperature_c: number("Temperature").map(|t| t / 100.0).filter(|t| (-20.0..=100.0).contains(t)),
        voltage_v,
        current_a,
        power_w: voltage_v.zip(current_a).map(|(v, a)| v * a),
        minutes_remaining,
        adapter_w: props.path(&["AdapterDetails", "Watts"]).and_then(Value::as_f64),
    })
}

/// Reads the battery. [`SysError::Unsupported`] when this Mac has none.
pub fn read() -> Result<Battery> {
    let found = iokit::properties_of("AppleSmartBattery")?;
    found.iter().find_map(|(_, props)| battery_from(props)).ok_or(SysError::Unsupported("A battery"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(bytes: &[u8]) -> Value {
        Value::from_plist_bytes(bytes).unwrap()
    }

    const PERCENT: &[u8] = include_bytes!("../tests/fixtures/synthetic/applesmartbattery-percent.plist");
    const MAH: &[u8] = include_bytes!("../tests/fixtures/synthetic/applesmartbattery-charging-mah.plist");

    #[test]
    fn reads_the_percentage_layout_while_discharging() {
        let b = battery_from(&load(PERCENT)).unwrap();
        assert_eq!(b.percent, 72.0);
        assert_eq!(b.state, ChargeState::Discharging);
        assert!(!b.on_ac_power);
        assert_eq!(b.cycle_count, Some(214));
        assert!((b.health_pct.unwrap() - 5079.0 / 6249.0 * 100.0).abs() < 1e-9);
        assert!((b.temperature_c.unwrap() - 30.51).abs() < 1e-9);
        assert!((b.voltage_v.unwrap() - 12.48).abs() < 1e-9);
        assert!((b.current_a.unwrap() + 1.25).abs() < 1e-9, "negative while discharging");
        assert!((b.power_w.unwrap() + 15.6).abs() < 1e-9);
        assert_eq!(b.minutes_remaining, Some(283));
        assert_eq!(b.adapter_w, None);
    }

    #[test]
    fn reads_the_milliamp_hour_layout_while_charging() {
        let b = battery_from(&load(MAH)).unwrap();
        assert_eq!(b.percent, 50.0, "2900 of 5800 mAh");
        assert_eq!(b.state, ChargeState::Charging);
        assert!(b.on_ac_power);
        assert!((b.health_pct.unwrap() - 5800.0 / 6559.0 * 100.0).abs() < 1e-9);
        assert_eq!(b.minutes_remaining, Some(95), "time to full while charging; the 65535 placeholder is ignored");
        assert_eq!(b.adapter_w, Some(87.0));
        assert!(b.power_w.unwrap() > 0.0);
    }

    fn with(extra: &str) -> Value {
        load(format!(r#"<plist version="1.0"><dict><key>CurrentCapacity</key><integer>{}</integer><key>MaxCapacity</key><integer>100</integer>{extra}</dict></plist>"#, 80).as_bytes())
    }

    #[test]
    fn tells_charging_states_apart() {
        let plugged_holding = with("<key>ExternalConnected</key><true/><key>IsCharging</key><false/>");
        assert_eq!(battery_from(&plugged_holding).unwrap().state, ChargeState::NotCharging);
        let full = with("<key>ExternalConnected</key><true/><key>FullyCharged</key><true/>");
        assert_eq!(battery_from(&full).unwrap().state, ChargeState::Charged);
        let unplugged = with("");
        assert_eq!(battery_from(&unplugged).unwrap().state, ChargeState::Discharging);
    }

    #[test]
    fn an_unknown_time_is_none_and_missing_keys_are_none() {
        let b = battery_from(&with("<key>TimeRemaining</key><integer>65535</integer>")).unwrap();
        assert_eq!(b.minutes_remaining, None);
        assert_eq!(b.health_pct, None);
        assert_eq!(b.cycle_count, None);
        assert_eq!(b.temperature_c, None);
        assert_eq!(b.power_w, None);
    }

    #[test]
    fn rejects_entries_that_are_not_a_battery() {
        assert!(battery_from(&Value::Null).is_none());
        let none_installed = load(br#"<plist version="1.0"><dict><key>BatteryInstalled</key><false/></dict></plist>"#);
        assert!(battery_from(&none_installed).is_none());
        let zero_max = load(br#"<plist version="1.0"><dict><key>CurrentCapacity</key><integer>1</integer><key>MaxCapacity</key><integer>0</integer></dict></plist>"#);
        assert!(battery_from(&zero_max).is_none(), "no division by zero");
    }

    #[test]
    fn an_absurd_temperature_is_dropped() {
        let b = battery_from(&with("<key>Temperature</key><integer>99999</integer>")).unwrap();
        assert_eq!(b.temperature_c, None);
    }

    #[test]
    fn this_mac_either_has_a_battery_or_says_so() {
        match read() {
            Ok(b) => assert!((0.0..=100.0).contains(&b.percent)),
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }
}
