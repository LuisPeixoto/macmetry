//! The text next to the menu bar icon, such as `CPU 23%  61°`.

use macmetry_core::{Metric, Snapshot};

/// A rate in a few characters: `812K`, `3.4M`, `1.2G`.
pub fn compact_rate(bytes_per_second: f64) -> String {
    let b = bytes_per_second.max(0.0);
    if b >= 1e9 {
        format!("{:.1}G", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1}M", b / 1e6)
    } else if b >= 1e3 {
        format!("{:.0}K", b / 1e3)
    } else {
        format!("{b:.0}B")
    }
}

/// The title for the chosen items. An item the Mac cannot provide is left out, so a desktop without a power reading
/// simply shows less.
pub fn menu_bar_title(snapshot: &Snapshot, items: &[String]) -> String {
    let parts: Vec<String> = items
        .iter()
        .filter_map(|item| match item.as_str() {
            "cpu" => Metric::CpuTotal.value(snapshot).map(|v| format!("CPU {v:.0}%")),
            "memory" => Metric::MemoryUsedPct.value(snapshot).map(|v| format!("MEM {v:.0}%")),
            "temp" => {
                Metric::TempCpu.value(snapshot).or_else(|| Metric::TempMax.value(snapshot)).map(|v| format!("{v:.0}°"))
            }
            "power" => Metric::PowerCpu.value(snapshot).map(|v| format!("{v:.0}W")),
            "network" => snapshot.network.as_ref().map(|_| {
                format!(
                    "↓{} ↑{}",
                    compact_rate(Metric::NetRxBps.value(snapshot).unwrap_or(0.0)),
                    compact_rate(Metric::NetTxBps.value(snapshot).unwrap_or(0.0))
                )
            }),
            "gpu" => Metric::GpuUtilization.value(snapshot).map(|v| format!("GPU {v:.0}%")),
            _ => None,
        })
        .collect();
    parts.join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            cpu: Some(CpuUsage { total_pct: 23.4, ..Default::default() }),
            memory: Some(Memory { total: 100, used: 61, ..Default::default() }),
            sensors: Some(Sensors {
                temperatures: vec![Temperature { kind: SensorKind::Cpu, celsius: 61.2, ..Default::default() }],
                fans: vec![],
            }),
            power: Some(PowerDraw { cpu_w: Some(23.6), ..Default::default() }),
            network: Some(NetworkStats {
                interfaces: vec![NetInterface {
                    kind: InterfaceKind::Ethernet,
                    rx_bps: 3_400_000.0,
                    tx_bps: 812_000.0,
                    ..Default::default()
                }],
            }),
            gpus: vec![Gpu { utilization_pct: Some(7.0), ..Default::default() }],
            ..Default::default()
        }
    }

    fn items(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn shows_the_chosen_readings_in_order() {
        assert_eq!(menu_bar_title(&snapshot(), &items(&["cpu", "temp"])), "CPU 23%  61°");
        assert_eq!(menu_bar_title(&snapshot(), &items(&["temp", "cpu"])), "61°  CPU 23%");
        assert_eq!(menu_bar_title(&snapshot(), &items(&["memory", "power", "gpu"])), "MEM 61%  24W  GPU 7%");
        assert_eq!(menu_bar_title(&snapshot(), &items(&["network"])), "↓3.4M ↑812K");
    }

    #[test]
    fn leaves_out_what_the_mac_cannot_provide() {
        let bare = Snapshot { cpu: Some(CpuUsage { total_pct: 5.0, ..Default::default() }), ..Default::default() };
        assert_eq!(menu_bar_title(&bare, &items(&["cpu", "temp", "power", "network", "gpu"])), "CPU 5%");
        assert_eq!(menu_bar_title(&Snapshot::default(), &items(&["cpu"])), "");
    }

    #[test]
    fn nothing_chosen_means_an_empty_title() {
        assert_eq!(menu_bar_title(&snapshot(), &[]), "");
        assert_eq!(menu_bar_title(&snapshot(), &items(&["fans"])), "", "an unknown item is ignored");
    }

    #[test]
    fn compacts_rates() {
        assert_eq!(compact_rate(0.0), "0B");
        assert_eq!(compact_rate(999.0), "999B");
        assert_eq!(compact_rate(1_500.0), "2K");
        assert_eq!(compact_rate(2_500_000.0), "2.5M");
        assert_eq!(compact_rate(1.2e9), "1.2G");
        assert_eq!(compact_rate(-5.0), "0B");
    }
}
