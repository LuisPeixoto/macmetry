//! A flat list of the numbers worth charting, storing and alerting on. One stable key per metric is shared by the
//! history database, the alert rules, the command line and the MCP server.

use crate::model::{SensorKind, Snapshot};
use serde::{Deserialize, Serialize};

macro_rules! metrics {
    ($($variant:ident => ($key:literal, $label:literal, $unit:expr)),+ $(,)?) => {
        /// A single number that can be read from a [`Snapshot`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Metric {
            $(
                #[serde(rename = $key)]
                $variant,
            )+
        }

        impl Metric {
            /// Every metric, in display order.
            pub const ALL: &'static [Metric] = &[$(Metric::$variant),+];

            /// The stable key, for example `cpu.total`.
            pub fn key(self) -> &'static str {
                match self { $(Metric::$variant => $key),+ }
            }

            /// A short English name. The desktop app translates its own labels.
            pub fn label(self) -> &'static str {
                match self { $(Metric::$variant => $label),+ }
            }

            pub fn unit(self) -> Unit {
                match self { $(Metric::$variant => $unit),+ }
            }

            /// Looks a metric up by its key.
            pub fn from_key(key: &str) -> Option<Metric> {
                match key { $($key => Some(Metric::$variant),)+ _ => None }
            }
        }
    };
}

/// What a metric's number means, so charts and alerts can format it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Percent,
    Bytes,
    BytesPerSecond,
    Celsius,
    Rpm,
    Watts,
    Level,
    PerSecond,
}

metrics! {
    CpuTotal => ("cpu.total", "CPU usage", Unit::Percent),
    CpuUser => ("cpu.user", "CPU user", Unit::Percent),
    CpuSystem => ("cpu.system", "CPU system", Unit::Percent),
    Load1 => ("cpu.load1", "Load average (1 min)", Unit::Level),
    ThermalPressure => ("thermal.pressure", "Thermal pressure", Unit::Level),
    MemoryUsedPct => ("memory.used_pct", "Memory used", Unit::Percent),
    MemoryUsed => ("memory.used", "Memory used", Unit::Bytes),
    MemoryPressure => ("memory.pressure", "Memory pressure", Unit::Level),
    SwapUsed => ("memory.swap_used", "Swap used", Unit::Bytes),
    PageOuts => ("memory.page_outs", "Page outs", Unit::PerSecond),
    DiskReadBps => ("disk.read_bps", "Disk read", Unit::BytesPerSecond),
    DiskWriteBps => ("disk.write_bps", "Disk write", Unit::BytesPerSecond),
    DiskFreePct => ("disk.free_pct", "Startup disk free", Unit::Percent),
    NetRxBps => ("net.rx_bps", "Network download", Unit::BytesPerSecond),
    NetTxBps => ("net.tx_bps", "Network upload", Unit::BytesPerSecond),
    GpuUtilization => ("gpu.utilization", "GPU usage", Unit::Percent),
    GpuTemperature => ("gpu.temperature", "GPU temperature", Unit::Celsius),
    TempCpu => ("temp.cpu", "CPU temperature", Unit::Celsius),
    TempMax => ("temp.max", "Hottest sensor", Unit::Celsius),
    FanRpm => ("fan.rpm", "Fastest fan", Unit::Rpm),
    PowerCpu => ("power.cpu", "CPU power", Unit::Watts),
    PowerGpu => ("power.gpu", "GPU power", Unit::Watts),
    PowerSystem => ("power.system", "System power", Unit::Watts),
    BatteryPercent => ("battery.percent", "Battery charge", Unit::Percent),
    BatteryTemperature => ("battery.temperature", "Battery temperature", Unit::Celsius),
}

impl Metric {
    /// Reads this metric from a snapshot. `None` means the Mac does not provide it (no battery, no fans...), which
    /// is different from a value of 0.
    pub fn value(self, s: &Snapshot) -> Option<f64> {
        use Metric::*;
        match self {
            CpuTotal => s.cpu.as_ref().map(|c| c.total_pct),
            CpuUser => s.cpu.as_ref().map(|c| c.user_pct),
            CpuSystem => s.cpu.as_ref().map(|c| c.system_pct),
            Load1 => s.cpu.as_ref().map(|c| c.load_average[0]),
            ThermalPressure => s.thermal_pressure.map(|t| f64::from(t.level())),
            MemoryUsedPct => s.memory.as_ref().filter(|m| m.total > 0).map(|m| m.used as f64 / m.total as f64 * 100.0),
            MemoryUsed => s.memory.as_ref().map(|m| m.used as f64),
            MemoryPressure => s.memory.as_ref().map(|m| f64::from(m.pressure.level())),
            SwapUsed => s.memory.as_ref().map(|m| m.swap_used as f64),
            PageOuts => s.memory.as_ref().map(|m| m.page_outs_per_s),
            DiskReadBps => s.disks.as_ref().filter(|d| !d.io.is_empty()).map(|d| d.io.iter().map(|i| i.read_bps).sum()),
            DiskWriteBps => {
                s.disks.as_ref().filter(|d| !d.io.is_empty()).map(|d| d.io.iter().map(|i| i.write_bps).sum())
            }
            DiskFreePct => s
                .disks
                .as_ref()
                .and_then(|d| d.volumes.iter().find(|v| v.mount_point == "/").or(d.volumes.first()))
                .filter(|v| v.total > 0)
                .map(|v| v.free as f64 / v.total as f64 * 100.0),
            NetRxBps => s.network.as_ref().map(|n| external(n).map(|i| i.rx_bps).sum()),
            NetTxBps => s.network.as_ref().map(|n| external(n).map(|i| i.tx_bps).sum()),
            GpuUtilization => max_of(s.gpus.iter().filter_map(|g| g.utilization_pct)),
            GpuTemperature => max_of(s.gpus.iter().filter_map(|g| g.temperature_c)),
            TempCpu => s.sensors.as_ref().and_then(|x| x.hottest_of(SensorKind::Cpu)).map(|t| t.celsius),
            TempMax => s.sensors.as_ref().and_then(|x| x.hottest()).map(|t| t.celsius),
            FanRpm => s.sensors.as_ref().and_then(|x| max_of(x.fans.iter().map(|f| f.rpm))),
            PowerCpu => s.power.as_ref().and_then(|p| p.cpu_w),
            PowerGpu => s.power.as_ref().and_then(|p| p.gpu_w),
            PowerSystem => s.power.as_ref().and_then(|p| p.system_w),
            BatteryPercent => s.battery.as_ref().map(|b| b.percent),
            BatteryTemperature => s.battery.as_ref().and_then(|b| b.temperature_c),
        }
    }
}

/// Interfaces that carry traffic to and from the outside, so loopback traffic does not inflate the totals.
fn external(n: &crate::model::NetworkStats) -> impl Iterator<Item = &crate::model::NetInterface> {
    n.interfaces.iter().filter(|i| i.kind != crate::model::InterfaceKind::Loopback)
}

fn max_of(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |best, v| match best {
        Some(b) if b >= v => Some(b),
        _ => Some(v),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn keys_are_unique_and_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for &m in Metric::ALL {
            assert!(seen.insert(m.key()), "duplicate key {}", m.key());
            assert_eq!(Metric::from_key(m.key()), Some(m));
        }
        assert_eq!(Metric::from_key("nope"), None);
    }

    #[test]
    fn keys_match_the_serde_names() {
        for &m in Metric::ALL {
            assert_eq!(serde_json::to_string(&m).unwrap(), format!("\"{}\"", m.key()));
        }
    }

    #[test]
    fn an_empty_snapshot_has_no_values() {
        let s = Snapshot::default();
        for &m in Metric::ALL {
            assert_eq!(m.value(&s), None, "{} should be unavailable", m.key());
        }
    }

    #[test]
    fn reads_values_from_a_snapshot() {
        let s = Snapshot {
            cpu: Some(CpuUsage { total_pct: 42.0, load_average: [1.5, 0.0, 0.0], ..Default::default() }),
            memory: Some(Memory { total: 1000, used: 250, pressure: MemoryPressure::Warning, ..Default::default() }),
            thermal_pressure: Some(ThermalPressure::Heavy),
            sensors: Some(Sensors {
                temperatures: vec![
                    Temperature { kind: SensorKind::Cpu, celsius: 61.0, ..Default::default() },
                    Temperature { kind: SensorKind::Storage, celsius: 70.0, ..Default::default() },
                ],
                fans: vec![Fan { rpm: 1200.0, ..Default::default() }, Fan { rpm: 1800.0, ..Default::default() }],
            }),
            ..Default::default()
        };
        assert_eq!(Metric::CpuTotal.value(&s), Some(42.0));
        assert_eq!(Metric::Load1.value(&s), Some(1.5));
        assert_eq!(Metric::MemoryUsedPct.value(&s), Some(25.0));
        assert_eq!(Metric::MemoryPressure.value(&s), Some(1.0));
        assert_eq!(Metric::ThermalPressure.value(&s), Some(2.0));
        assert_eq!(Metric::TempCpu.value(&s), Some(61.0));
        assert_eq!(Metric::TempMax.value(&s), Some(70.0));
        assert_eq!(Metric::FanRpm.value(&s), Some(1800.0));
        assert_eq!(Metric::BatteryPercent.value(&s), None);
    }

    #[test]
    fn network_totals_skip_loopback() {
        let s = Snapshot {
            network: Some(NetworkStats {
                interfaces: vec![
                    NetInterface { kind: InterfaceKind::Loopback, rx_bps: 9_999.0, ..Default::default() },
                    NetInterface { kind: InterfaceKind::Ethernet, rx_bps: 100.0, tx_bps: 7.0, ..Default::default() },
                    NetInterface { kind: InterfaceKind::Wifi, rx_bps: 50.0, tx_bps: 3.0, ..Default::default() },
                ],
            }),
            ..Default::default()
        };
        assert_eq!(Metric::NetRxBps.value(&s), Some(150.0));
        assert_eq!(Metric::NetTxBps.value(&s), Some(10.0));
    }

    #[test]
    fn startup_disk_prefers_the_root_volume() {
        let s = Snapshot {
            disks: Some(DiskStats {
                volumes: vec![
                    Volume { mount_point: "/Volumes/Data".into(), total: 100, free: 90, ..Default::default() },
                    Volume { mount_point: "/".into(), total: 200, free: 50, ..Default::default() },
                ],
                io: vec![],
            }),
            ..Default::default()
        };
        assert_eq!(Metric::DiskFreePct.value(&s), Some(25.0));
        assert_eq!(Metric::DiskReadBps.value(&s), None);
    }

    #[test]
    fn a_missing_total_never_divides_by_zero() {
        let s = Snapshot { memory: Some(Memory::default()), ..Default::default() };
        assert_eq!(Metric::MemoryUsedPct.value(&s), None);
    }
}
