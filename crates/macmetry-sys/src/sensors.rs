//! Temperatures, fans and power from the SMC, with a catalog that names the sensors.
//!
//! SMC keys differ between models, so sensors are *discovered*: the key list is read once, every `T…` key that holds
//! a plausible temperature becomes a sensor, and the catalog gives it a kind and a name. A key the catalog does not
//! know still shows up, labelled with its key, so a Mac nobody has tested still gets its sensors.

use crate::error::{Result, SysError};
use crate::smc::{Smc, decode};
use macmetry_core::{Fan, SensorKind, Sensors, Temperature};

/// Temperatures outside this range are the SMC saying "no sensor here" (0) or a sentinel, not a reading.
const PLAUSIBLE: std::ops::RangeInclusive<f64> = 1.0..=125.0;

/// A temperature sensor found on this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempSensor {
    pub key: String,
    pub kind: SensorKind,
    pub label: String,
}

/// What a `T…` key measures, going by its prefix.
pub fn kind_of(key: &str) -> SensorKind {
    let b = key.as_bytes();
    if b.len() != 4 || b[0] != b'T' {
        return SensorKind::Other;
    }
    match (b[1], b[2]) {
        (b'C', _) | (b'p', _) | (b'e', _) => SensorKind::Cpu,
        (b'G', _) | (b'g', _) => SensorKind::Gpu,
        (b'M', _) | (b'm', _) => SensorKind::Memory,
        (b'H', _) => SensorKind::Storage,
        (b'B', _) => SensorKind::Battery,
        (b'a', _) | (b'A', _) => SensorKind::Ambient,
        _ => SensorKind::Other,
    }
}

/// Lower-case variants such as `TC0c` and `TC0p` are alternative estimates of the sensor in upper case; showing both
/// would list each core twice.
fn is_variant(key: &str, all: &[&str]) -> bool {
    let b = key.as_bytes();
    if b.len() != 4 || !matches!(b[3], b'c' | b'p') || b[1] != b'C' {
        return false;
    }
    let upper = format!("{}{}", &key[..3], (b[3] as char).to_ascii_uppercase());
    all.iter().any(|k| *k == upper)
}

/// The name shown for a sensor. `ordinal` is its position among the sensors of the same group (1-based), used
/// where the key says "one of several" but not which.
pub fn label_for(key: &str, kind: SensorKind, ordinal: usize) -> String {
    let named = match key {
        "TC0P" => "CPU proximity",
        "TC0D" => "CPU die",
        "TC0H" => "CPU heatsink",
        "TG0P" => "GPU proximity",
        "TG0D" => "GPU die",
        "TG0H" => "GPU heatsink",
        "TM0P" => "Memory proximity",
        "TM0S" => "Memory slot",
        "TB0T" => "Battery 1",
        "TB1T" => "Battery 2",
        "TB2T" => "Battery 3",
        "Ta0P" | "TA0P" => "Ambient",
        "TN0P" | "TN0D" => "Northbridge",
        "TPCD" => "Platform controller",
        "TW0P" => "Wi-Fi module",
        "Ts0P" | "Ts1P" => "Palm rest",
        "Th0H" | "Th1H" => "Heat pipe",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    let b = key.as_bytes();
    // `TC0C`…`TC9C` are the cores of an Intel CPU, numbered from 0.
    if b.len() == 4 && b[1] == b'C' && b[3] == b'C' && b[2].is_ascii_digit() {
        return format!("CPU core {}", u32::from(b[2] - b'0') + 1);
    }
    match (kind, b.get(1)) {
        (SensorKind::Cpu, Some(b'p')) => format!("CPU performance core {ordinal}"),
        (SensorKind::Cpu, Some(b'e')) => format!("CPU efficiency core {ordinal}"),
        (SensorKind::Gpu, Some(b'g')) => format!("GPU cluster {ordinal}"),
        (SensorKind::Memory, Some(b'm')) => format!("Memory {ordinal}"),
        (SensorKind::Cpu, _) => format!("CPU sensor {key}"),
        (SensorKind::Gpu, _) => format!("GPU sensor {key}"),
        (SensorKind::Memory, _) => format!("Memory sensor {key}"),
        (SensorKind::Storage, _) => format!("Storage sensor {key}"),
        (SensorKind::Battery, _) => format!("Battery sensor {key}"),
        (SensorKind::Ambient, _) => format!("Ambient sensor {key}"),
        _ => format!("Sensor {key}"),
    }
}

/// One key of the dump: its name, its type and what it reads right now.
pub type DumpRow = (String, [u8; 4], Option<f64>);

/// Chooses the temperature sensors from a list of keys. Pure, so it is tested with a real dump.
pub fn discover_temperatures(rows: &[DumpRow]) -> Vec<TempSensor> {
    let names: Vec<&str> = rows.iter().map(|(k, _, _)| k.as_str()).collect();
    let mut picked: Vec<(&str, SensorKind)> = rows
        .iter()
        .filter(|(key, kind, value)| {
            key.len() == 4
                && key.starts_with('T')
                && matches!(kind, b"sp78" | b"flt " | b"sp87" | b"sp96" | b"sp5a" | b"sp69")
                && value.is_some_and(|v| PLAUSIBLE.contains(&v))
                && !is_variant(key, &names)
        })
        .map(|(key, _, _)| (key.as_str(), kind_of(key)))
        .collect();
    picked.sort();

    let mut ordinals: std::collections::HashMap<(SensorKind, u8), usize> = std::collections::HashMap::new();
    picked
        .into_iter()
        .map(|(key, kind)| {
            let n = ordinals.entry((kind, key.as_bytes()[1])).or_insert(0);
            *n += 1;
            TempSensor { key: key.to_string(), kind, label: label_for(key, kind, *n) }
        })
        .collect()
}

/// Reads sensors from the SMC. Discovery runs once; each sample reads only the chosen keys.
pub struct SensorCollector {
    smc: Smc,
    temperatures: Vec<TempSensor>,
    fan_count: u32,
    power_key: Option<&'static str>,
}

/// Keys that hold the total power the system draws, in watts. Only `PSTR` is used: other `P…` keys measure parts
/// (the CPU cores, the package) and are not verified well enough to be shown as the system total.
const SYSTEM_POWER_KEYS: [&str; 1] = ["PSTR"];

impl SensorCollector {
    /// [`SysError::Unsupported`] when there is no SMC.
    pub fn new() -> Result<Self> {
        let smc = Smc::open()?;
        let rows: Vec<DumpRow> = smc
            .keys()?
            .into_iter()
            .filter(|k| k.starts_with('T'))
            .filter_map(|key| {
                let (kind, bytes) = smc.read_bytes(&key).ok()?;
                let value = decode(&kind.data_type, &bytes);
                Some((key, kind.data_type, value))
            })
            .collect();
        let temperatures = discover_temperatures(&rows);
        let fan_count = smc.read_number("FNum").map(|n| n as u32).unwrap_or(0).min(16);
        let power_key = SYSTEM_POWER_KEYS.iter().copied().find(|k| smc.read_number(k).is_ok_and(|w| w > 0.0));
        if temperatures.is_empty() && fan_count == 0 && power_key.is_none() {
            return Err(SysError::Unsupported("Temperature and fan sensors"));
        }
        Ok(Self { smc, temperatures, fan_count, power_key })
    }

    pub fn sample(&self) -> Result<(Sensors, Option<f64>)> {
        let temperatures = self
            .temperatures
            .iter()
            .filter_map(|s| {
                let celsius = self.smc.read_number(&s.key).ok().filter(|v| PLAUSIBLE.contains(v))?;
                Some(Temperature { key: s.key.clone(), label: s.label.clone(), kind: s.kind, celsius })
            })
            .collect();
        let fans = (0..self.fan_count)
            .filter_map(|i| {
                let rpm = self.smc.read_number(&format!("F{i}Ac")).ok().filter(|r| (0.0..=30_000.0).contains(r))?;
                let positive = |key: String| self.smc.read_number(&key).ok().filter(|&v| v > 0.0);
                Some(Fan {
                    index: i,
                    label: format!("Fan {}", i + 1),
                    rpm,
                    min_rpm: positive(format!("F{i}Mn")),
                    max_rpm: positive(format!("F{i}Mx")),
                })
            })
            .collect();
        let power = self.power_key.and_then(|k| self.smc.read_number(k).ok()).filter(|w| (0.0..=1000.0).contains(w));
        Ok((Sensors { temperatures, fans }, power))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = include_str!("../tests/fixtures/intel-macpro7-1/smc-keys.jsonl");

    /// The real dump of this project's Intel Mac Pro, as rows.
    fn real_rows() -> Vec<DumpRow> {
        DUMP.lines()
            .map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).unwrap();
                let t = v["type"].as_str().unwrap().as_bytes();
                let mut kind = [b' '; 4];
                kind[..t.len().min(4)].copy_from_slice(&t[..t.len().min(4)]);
                (v["key"].as_str().unwrap().to_string(), kind, v["value"].as_f64())
            })
            .collect()
    }

    #[test]
    fn finds_the_cores_of_a_real_mac_pro_once_each() {
        let sensors = discover_temperatures(&real_rows());
        let cores: Vec<_> = sensors.iter().filter(|s| s.label.starts_with("CPU core")).collect();
        assert_eq!(cores.len(), 8, "a Xeon W with 8 cores: {sensors:?}");
        assert_eq!(cores[0].label, "CPU core 1");
        assert_eq!(cores[7].label, "CPU core 8");
        assert!(sensors.iter().all(|s| s.kind == SensorKind::Cpu), "this Mac's SMC exposes only CPU temperatures");
    }

    #[test]
    fn leaves_out_absent_sensors_and_duplicate_estimates() {
        let keys: Vec<String> = discover_temperatures(&real_rows()).into_iter().map(|s| s.key).collect();
        assert!(!keys.contains(&"TC0G".to_string()), "TC0G reads 0: there is no sensor");
        assert!(!keys.contains(&"TC0c".to_string()), "TC0c duplicates TC0C");
        assert!(!keys.contains(&"TC3c".to_string()));
        assert!(keys.contains(&"TC0P".to_string()));
        assert!(keys.iter().all(|k| k.starts_with("TC")));
    }

    #[test]
    fn names_known_and_unknown_keys() {
        assert_eq!(label_for("TC0P", SensorKind::Cpu, 1), "CPU proximity");
        assert_eq!(label_for("TC4C", SensorKind::Cpu, 1), "CPU core 5");
        assert_eq!(label_for("TG0D", SensorKind::Gpu, 1), "GPU die");
        assert_eq!(label_for("Tp09", SensorKind::Cpu, 3), "CPU performance core 3");
        assert_eq!(label_for("Te05", SensorKind::Cpu, 1), "CPU efficiency core 1");
        assert_eq!(label_for("TC0E", SensorKind::Cpu, 1), "CPU sensor TC0E", "an unknown key keeps its key");
        assert_eq!(label_for("TZ9Z", SensorKind::Other, 1), "Sensor TZ9Z");
    }

    #[test]
    fn classifies_by_prefix() {
        assert_eq!(kind_of("TC0P"), SensorKind::Cpu);
        assert_eq!(kind_of("Tp01"), SensorKind::Cpu);
        assert_eq!(kind_of("Tg05"), SensorKind::Gpu);
        assert_eq!(kind_of("TH0a"), SensorKind::Storage);
        assert_eq!(kind_of("TB0T"), SensorKind::Battery);
        assert_eq!(kind_of("Ta0P"), SensorKind::Ambient);
        assert_eq!(kind_of("FNum"), SensorKind::Other);
        assert_eq!(kind_of("T"), SensorKind::Other);
    }

    #[test]
    fn apple_silicon_sensors_are_numbered_per_group() {
        let flt = *b"flt ";
        let rows = vec![
            ("Tp01".to_string(), flt, Some(41.0)),
            ("Tp05".to_string(), flt, Some(42.0)),
            ("Te05".to_string(), flt, Some(38.0)),
            ("Tg05".to_string(), flt, Some(40.0)),
        ];
        let labels: Vec<_> = discover_temperatures(&rows).into_iter().map(|s| s.label).collect();
        // Sensors come back in key order (Te, Tg, Tp); the numbering restarts for each group.
        assert_eq!(
            labels,
            ["CPU efficiency core 1", "GPU cluster 1", "CPU performance core 1", "CPU performance core 2"]
        );
    }

    #[test]
    fn rejects_implausible_readings_and_other_types() {
        let rows = vec![
            ("TC0P".to_string(), *b"sp78", Some(0.0)),
            ("TC1P".to_string(), *b"sp78", Some(200.0)),
            ("TC2P".to_string(), *b"ui8 ", Some(40.0)),
            ("TC3P".to_string(), *b"sp78", None),
            ("TC4P".to_string(), *b"sp78", Some(40.0)),
        ];
        let found = discover_temperatures(&rows);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "TC4P");
    }

    #[test]
    fn this_mac_reads_its_sensors_or_says_so() {
        match SensorCollector::new() {
            Ok(collector) => {
                let (sensors, _power) = collector.sample().unwrap();
                assert!(sensors.temperatures.iter().all(|t| PLAUSIBLE.contains(&t.celsius)));
                assert!(sensors.fans.iter().all(|f| f.rpm >= 0.0));
            }
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }
}
