//! Temperature sensors that Apple silicon publishes through the HID event system instead of the SMC.
//!
//! The functions are private. On a Mac without these sensors (Intel desktops, virtual machines) the list is empty.

use crate::cf::{CFTypeRef, Owned, string_from};
use macmetry_core::{SensorKind, Temperature};

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDEventSystemClientCreate(allocator: CFTypeRef) -> CFTypeRef;
    fn IOHIDEventSystemClientSetMatching(client: CFTypeRef, matching: CFTypeRef) -> i32;
    fn IOHIDEventSystemClientCopyServices(client: CFTypeRef) -> CFTypeRef;
    fn IOHIDServiceClientCopyProperty(service: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn IOHIDServiceClientCopyEvent(service: CFTypeRef, kind: i64, options: i32, timestamp: i64) -> CFTypeRef;
    fn IOHIDEventGetFloatValue(event: CFTypeRef, field: i32) -> f64;
}

/// `kHIDPage_AppleVendor` and the usage of its temperature sensors.
const USAGE_PAGE: i64 = 0xff00;
const USAGE: i64 = 5;
/// `kIOHIDEventTypeTemperature`; the field of its value is the event type shifted left by 16.
const EVENT_TEMPERATURE: i64 = 15;
const FIELD_TEMPERATURE: i32 = (EVENT_TEMPERATURE as i32) << 16;

const PLAUSIBLE: std::ops::RangeInclusive<f64> = 1.0..=125.0;

/// What a sensor measures, going by the name its driver gives it. A name this does not recognise is `Other`; the
/// name itself is always shown, so nothing is mislabelled.
pub fn kind_of(name: &str) -> SensorKind {
    let n = name.to_lowercase();
    if n.contains("gpu") {
        SensorKind::Gpu
    } else if n.contains("nand") || n.contains("ssd") {
        SensorKind::Storage
    } else if n.contains("battery") || n.contains("gas gauge") {
        SensorKind::Battery
    } else if n.contains("tdie") || n.contains("cpu") || n.contains("soc") {
        SensorKind::Cpu
    } else {
        SensorKind::Other
    }
}

/// Reads every HID temperature sensor now.
pub fn read_temperatures() -> Vec<Temperature> {
    // SAFETY: a null allocator means the default one; the result is a new object or null.
    let Some(client) = (unsafe { Owned::from_create(IOHIDEventSystemClientCreate(std::ptr::null())) }) else {
        return Vec::new();
    };
    let Some(matching) = Owned::int_dictionary(&[("PrimaryUsagePage", USAGE_PAGE), ("PrimaryUsage", USAGE)]) else {
        return Vec::new();
    };
    // SAFETY: both objects are valid.
    unsafe { IOHIDEventSystemClientSetMatching(client.as_ptr(), matching.as_ptr()) };
    // SAFETY: `client` is valid; the result is a new array or null.
    let Some(services) = (unsafe { Owned::from_create(IOHIDEventSystemClientCopyServices(client.as_ptr())) }) else {
        return Vec::new();
    };
    let Some(product) = Owned::string("Product") else { return Vec::new() };

    let mut out: Vec<Temperature> = services
        .array_items()
        .into_iter()
        .filter_map(|service| {
            // SAFETY: `service` is borrowed from `services`, which lives through this closure; each copy function
            // returns a new object (or null) that `Owned` releases.
            let (name, celsius) = unsafe {
                let name = Owned::from_create(IOHIDServiceClientCopyProperty(service, product.as_ptr()))
                    .and_then(|n| string_from(n.as_ptr()))?;
                let event = Owned::from_create(IOHIDServiceClientCopyEvent(service, EVENT_TEMPERATURE, 0, 0))?;
                (name, IOHIDEventGetFloatValue(event.as_ptr(), FIELD_TEMPERATURE))
            };
            PLAUSIBLE.contains(&celsius).then(|| Temperature {
                key: name.clone(),
                kind: kind_of(&name),
                label: name,
                celsius,
            })
        })
        .collect();
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_name() {
        assert_eq!(kind_of("PMU tdie3"), SensorKind::Cpu);
        assert_eq!(kind_of("GPU MTR Temp Sensor1"), SensorKind::Gpu);
        assert_eq!(kind_of("NAND CH0 temp"), SensorKind::Storage);
        assert_eq!(kind_of("gas gauge battery"), SensorKind::Battery);
        assert_eq!(kind_of("mystery"), SensorKind::Other);
    }

    #[test]
    fn the_temperature_field_is_the_event_type_shifted() {
        assert_eq!(FIELD_TEMPERATURE, 0x000f_0000);
    }

    #[test]
    fn reading_never_crashes_and_stays_plausible() {
        // An Intel desktop has none of these sensors; Apple silicon has several. Either way the values are sane.
        for t in read_temperatures() {
            assert!(PLAUSIBLE.contains(&t.celsius), "{} reads {}", t.label, t.celsius);
            assert!(!t.label.is_empty());
        }
    }
}
