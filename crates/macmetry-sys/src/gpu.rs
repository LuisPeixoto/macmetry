//! Graphics processors, from the `PerformanceStatistics` that each driver publishes on its accelerator.
//!
//! Discrete AMD GPUs publish temperature, power, fan speed and clocks there too. Apple silicon GPUs publish
//! utilization and memory; their frequency and power come from the reporting framework in `apple_silicon`.

use crate::error::{Result, SysError};
use crate::iokit;
use crate::value::Value;
use macmetry_core::Gpu;
use std::collections::HashSet;

/// The text of a registry property that is either a string or NUL-terminated bytes (the `model` of a PCI device).
fn text(value: &Value) -> Option<String> {
    let raw = match value {
        Value::Str(s) => s.clone(),
        Value::Data(d) => String::from_utf8_lossy(d).into_owned(),
        _ => return None,
    };
    let trimmed = raw.trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Builds a [`Gpu`] from an accelerator's properties and, when known, the properties of the device that owns it
/// (which carry the marketing name and the amount of video memory). `None` when the driver publishes no statistics.
pub fn gpu_from(accelerator: &Value, owner: Option<&Value>) -> Option<Gpu> {
    let stats = accelerator.get("PerformanceStatistics")?;
    let stat = |key: &str| stats.get(key).and_then(Value::as_f64);
    let percent = |key: &str| stat(key).filter(|p| (0.0..=100.0).contains(p));

    let name = owner
        .and_then(|o| o.get("model"))
        .and_then(text)
        .or_else(|| accelerator.get("IOClass").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "GPU".to_string());

    // AMD reports video memory in use; Apple's GPU shares system memory and reports that instead.
    let memory_used =
        stats.get("inUseVidMemoryBytes").or_else(|| stats.get("In use system memory")).and_then(Value::as_u64);
    let memory_total = owner.and_then(|o| o.get("VRAM,totalMB")).and_then(Value::as_u64).map(|mb| mb << 20);

    Some(Gpu {
        name,
        utilization_pct: percent("Device Utilization %").or_else(|| percent("GPU Activity(%)")),
        memory_used,
        memory_total,
        core_clock_mhz: stat("Core Clock(MHz)"),
        memory_clock_mhz: stat("Memory Clock(MHz)"),
        temperature_c: stat("Temperature(C)").filter(|t| (0.0..=150.0).contains(t)),
        power_w: stat("Total Power(W)").filter(|w| *w >= 0.0),
        fan_rpm: stat("Fan Speed(RPM)").filter(|r| *r >= 0.0),
    })
}

/// Reads every GPU. [`SysError::Unsupported`] when no driver publishes statistics.
pub fn read() -> Result<Vec<Gpu>> {
    let mut seen = HashSet::new();
    let mut gpus = Vec::new();
    for accelerator in iokit::services("IOAccelerator")? {
        let owner = accelerator.parent();
        // A driver can register more than one accelerator for the same GPU; they share an owner.
        let identity = owner.as_ref().and_then(iokit::Service::entry_id).or_else(|| accelerator.entry_id());
        if let Some(id) = identity
            && !seen.insert(id)
        {
            continue;
        }
        let Ok(props) = accelerator.properties() else { continue };
        let owner_props = owner.as_ref().and_then(|o| o.properties().ok());
        if let Some(gpu) = gpu_from(&props, owner_props.as_ref()) {
            gpus.push(gpu);
        }
    }
    if gpus.is_empty() { Err(SysError::Unsupported("A graphics processor")) } else { Ok(gpus) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/intel-macpro7-1/ioaccelerator.plist");

    fn accelerators() -> Vec<Value> {
        Value::from_plist_bytes(FIXTURE).unwrap().as_array().expect("a list").to_vec()
    }

    #[test]
    fn reads_a_real_amd_radeon() {
        let gpu = gpu_from(&accelerators()[0], None).unwrap();
        assert_eq!(gpu.name, "AMDRadeonX6000_AMDNavi23GraphicsAccelerator", "the class is the fallback name");
        assert_eq!(gpu.temperature_c, Some(69.0));
        assert_eq!(gpu.power_w, Some(21.0));
        assert_eq!(gpu.fan_rpm, Some(683.0));
        assert_eq!(gpu.memory_clock_mhz, Some(1990.0));
        assert!(gpu.utilization_pct.is_some_and(|u| (0.0..=100.0).contains(&u)));
        assert!(gpu.memory_used.is_some_and(|m| m > 1 << 30), "gigabytes of video memory are in use");
        assert_eq!(gpu.memory_total, None, "the total comes from the owning PCI device");
    }

    #[test]
    fn the_owner_supplies_the_name_and_the_memory_size() {
        let owner = Value::from_plist_bytes(
            br#"<plist version="1.0"><dict><key>model</key><data>QU1EIFJhZGVvbiBSWCA2NjAwIFhUAA==</data><key>VRAM,totalMB</key><integer>8176</integer></dict></plist>"#,
        )
        .unwrap();
        let gpu = gpu_from(&accelerators()[0], Some(&owner)).unwrap();
        assert_eq!(gpu.name, "AMD Radeon RX 6600 XT");
        assert_eq!(gpu.memory_total, Some(8176 << 20));
    }

    #[test]
    fn the_two_listings_in_the_fixture_are_one_gpu() {
        let list = accelerators();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].get("IORegistryEntryID"), list[1].get("IORegistryEntryID"));
    }

    #[test]
    fn out_of_range_readings_are_dropped_not_shown() {
        let accel = Value::from_plist_bytes(
            br#"<plist version="1.0"><dict><key>PerformanceStatistics</key><dict>
                <key>Device Utilization %</key><integer>250</integer>
                <key>Temperature(C)</key><integer>-300</integer>
                <key>Total Power(W)</key><integer>-1</integer></dict></dict></plist>"#,
        )
        .unwrap();
        let gpu = gpu_from(&accel, None).unwrap();
        assert_eq!(gpu.utilization_pct, None);
        assert_eq!(gpu.temperature_c, None);
        assert_eq!(gpu.power_w, None);
    }

    #[test]
    fn an_accelerator_without_statistics_is_not_a_gpu() {
        assert!(gpu_from(&Value::Null, None).is_none());
        let bare = Value::from_plist_bytes(
            br#"<plist version="1.0"><dict><key>IOClass</key><string>X</string></dict></plist>"#,
        )
        .unwrap();
        assert!(gpu_from(&bare, None).is_none());
    }

    #[test]
    fn apple_silicon_reports_shared_memory() {
        let accel = Value::from_plist_bytes(
            br#"<plist version="1.0"><dict><key>PerformanceStatistics</key><dict>
                <key>Device Utilization %</key><integer>12</integer>
                <key>In use system memory</key><integer>734003200</integer></dict></dict></plist>"#,
        )
        .unwrap();
        let gpu = gpu_from(&accel, None).unwrap();
        assert_eq!(gpu.utilization_pct, Some(12.0));
        assert_eq!(gpu.memory_used, Some(734_003_200));
        assert_eq!(gpu.temperature_c, None);
    }

    #[test]
    fn reads_this_macs_gpu_once() {
        match read() {
            Ok(gpus) => {
                assert!(!gpus.is_empty());
                let names: HashSet<_> = gpus.iter().map(|g| g.name.clone()).collect();
                assert_eq!(names.len(), gpus.len(), "no GPU is listed twice");
            }
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }
}
