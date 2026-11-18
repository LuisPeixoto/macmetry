//! Turns readings into text for the terminal. Pure functions: the tests feed them snapshots and read the output.

use macmetry_core::units::{format_bytes, format_duration, format_rate};
use macmetry_core::{
    Battery, ChargeState, Gpu, InterfaceKind, ProcessSample, Series, Snapshot, SystemInfo, ThermalPressure,
};
use std::fmt::Write;

/// `████░░░░░░` for 40%, `width` cells wide.
pub fn bar(percent: f64, width: usize) -> String {
    let filled = ((percent.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// A one-line chart of values, scaled between the lowest and the highest of them.
pub fn sparkline(values: &[f64]) -> String {
    const STEPS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let (min, max) = values.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    values
        .iter()
        .map(|&v| if max > min { STEPS[(((v - min) / (max - min)) * 7.0).round() as usize] } else { STEPS[0] })
        .collect()
}

pub fn thermal_name(t: ThermalPressure) -> &'static str {
    match t {
        ThermalPressure::Nominal => "nominal",
        ThermalPressure::Moderate => "moderate",
        ThermalPressure::Heavy => "heavy",
        ThermalPressure::Trapping => "trapping",
        ThermalPressure::Sleeping => "sleeping",
    }
}

pub fn render_info(info: &SystemInfo) -> String {
    let mut out = String::new();
    let cores = match (info.performance_cores, info.efficiency_cores) {
        (Some(p), Some(e)) => format!("{} cores ({p} performance + {e} efficiency)", info.physical_cores),
        _ => format!("{} cores, {} threads", info.physical_cores, info.logical_cores),
    };
    let _ = writeln!(out, "Model     {}", info.model);
    let _ = writeln!(out, "Chip      {} ({})", info.chip, info.arch_name());
    let _ = writeln!(out, "Cores     {cores}");
    let _ = writeln!(out, "Memory    {}", format_bytes(info.memory_bytes));
    let _ = writeln!(out, "macOS     {} ({})", info.os_version, info.os_build);
    if info.boot_time_ms > 0 {
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let _ = writeln!(out, "Uptime    {}", format_duration(now.saturating_sub(info.boot_time_ms) / 1000));
    }
    out
}

/// Small extension so `render_info` can name the architecture without the core depending on text.
trait ArchName {
    fn arch_name(&self) -> &'static str;
}

impl ArchName for SystemInfo {
    fn arch_name(&self) -> &'static str {
        match self.arch {
            macmetry_core::Arch::X86_64 => "Intel",
            macmetry_core::Arch::Arm64 => "Apple silicon",
            macmetry_core::Arch::Unknown => "unknown architecture",
        }
    }
}

fn gpu_line(g: &Gpu) -> String {
    let mut parts = vec![g.name.clone()];
    if let Some(u) = g.utilization_pct {
        parts.push(format!("{u:.0}%"));
    }
    if let (Some(used), Some(total)) = (g.memory_used, g.memory_total) {
        parts.push(format!("{} / {}", format_bytes(used), format_bytes(total)));
    } else if let Some(used) = g.memory_used {
        parts.push(format_bytes(used));
    }
    if let Some(t) = g.temperature_c {
        parts.push(format!("{t:.0} °C"));
    }
    if let Some(w) = g.power_w {
        parts.push(format!("{w:.0} W"));
    }
    parts.join("  ·  ")
}

pub fn render_battery(b: &Battery) -> String {
    let state = match b.state {
        ChargeState::Charging => "charging",
        ChargeState::Discharging => "on battery",
        ChargeState::Charged => "charged",
        ChargeState::NotCharging => "plugged in, not charging",
        ChargeState::Unknown => "unknown",
    };
    let mut out = format!("{} {:>3.0}%  {state}", bar(b.percent, 20), b.percent);
    if let Some(m) = b.minutes_remaining {
        let _ = write!(out, "  ({}h {:02}m)", m / 60, m % 60);
    }
    out.push('\n');
    let mut facts = Vec::new();
    if let Some(h) = b.health_pct {
        facts.push(format!("health {h:.0}%"));
    }
    if let Some(c) = b.cycle_count {
        facts.push(format!("{c} cycles"));
    }
    if let Some(t) = b.temperature_c {
        facts.push(format!("{t:.1} °C"));
    }
    if let Some(w) = b.power_w {
        facts.push(format!("{w:+.1} W"));
    }
    if let Some(w) = b.adapter_w {
        facts.push(format!("{w:.0} W adapter"));
    }
    if !facts.is_empty() {
        let _ = writeln!(out, "  {}", facts.join("  ·  "));
    }
    out
}

pub fn render_processes(processes: &[ProcessSample]) -> String {
    let mut out = format!(
        "{:>7}  {:<30} {:>7} {:>10} {:>5}  {:>10} {:>10}\n",
        "PID", "NAME", "CPU", "MEMORY", "THR", "READ", "WRITE"
    );
    for p in processes {
        let name: String = p.name.chars().take(30).collect();
        let _ = writeln!(
            out,
            "{:>7}  {:<30} {:>6.1}% {:>10} {:>5}  {:>10} {:>10}",
            p.pid,
            name,
            p.cpu_pct,
            format_bytes(p.memory_bytes),
            p.threads,
            format_rate(p.disk_read_bps),
            format_rate(p.disk_write_bps)
        );
    }
    out
}

pub fn render_sensors(s: &macmetry_core::Sensors) -> String {
    let mut out = String::new();
    if s.temperatures.is_empty() && s.fans.is_empty() {
        return "This Mac reports no temperature or fan sensors.\n".to_string();
    }
    for t in &s.temperatures {
        let _ = writeln!(out, "{:<28} {:>6.1} °C   {}", t.label, t.celsius, t.key);
    }
    for f in &s.fans {
        let max = f.max_rpm.map(|m| format!(" of {m:.0}")).unwrap_or_default();
        let _ = writeln!(out, "{:<28} {:>6.0} rpm{max}", f.label, f.rpm);
    }
    out
}

pub fn render_series(series: &Series) -> String {
    if series.points.is_empty() {
        return format!("{}: no data in this range\n", series.metric.label());
    }
    let avgs: Vec<f64> = series.points.iter().map(|p| p.avg).collect();
    let lowest = series.points.iter().map(|p| p.min).fold(f64::MAX, f64::min);
    let highest = series.points.iter().map(|p| p.max).fold(f64::MIN, f64::max);
    let mean = avgs.iter().sum::<f64>() / avgs.len() as f64;
    let m = series.metric;
    format!(
        "{}\n{}\n  lowest {}   average {}   highest {}   ({} points, {} each)\n",
        m.label(),
        sparkline(&avgs),
        macmetry_core::alerts::format_value(m, lowest),
        macmetry_core::alerts::format_value(m, mean),
        macmetry_core::alerts::format_value(m, highest),
        series.points.len(),
        format_duration(series.resolution_ms / 1000),
    )
}

/// The whole picture, in sections. Sections the Mac cannot provide are left out.
pub fn render_snapshot(s: &Snapshot, top_processes: usize) -> String {
    let mut out = String::new();
    if let Some(c) = &s.cpu {
        let _ = writeln!(
            out,
            "CPU       {} {:>5.1}%   user {:.1}%  system {:.1}%",
            bar(c.total_pct, 24),
            c.total_pct,
            c.user_pct,
            c.system_pct
        );
        let _ = writeln!(
            out,
            "          load {:.2} {:.2} {:.2}{}",
            c.load_average[0],
            c.load_average[1],
            c.load_average[2],
            c.frequency_mhz
                .map(|f| format!("   {f:.0} MHz{}", if c.frequency_is_measured { "" } else { " nominal" }))
                .unwrap_or_default()
        );
        if !c.cores.is_empty() {
            let bars: Vec<f64> = c.cores.iter().map(|core| core.usage_pct).collect();
            let _ = writeln!(out, "          cores {}", sparkline(&bars));
        }
    }
    if let Some(t) = s.thermal_pressure {
        let _ = writeln!(out, "Thermal   {}", thermal_name(t));
    }
    if let Some(m) = &s.memory {
        let pct = if m.total > 0 { m.used as f64 / m.total as f64 * 100.0 } else { 0.0 };
        let _ = writeln!(
            out,
            "Memory    {} {:>5.1}%   {} of {}   pressure {:?}",
            bar(pct, 24),
            pct,
            format_bytes(m.used),
            format_bytes(m.total),
            m.pressure
        );
        let _ = writeln!(
            out,
            "          app {}  wired {}  compressed {}  cached {}  swap {}",
            format_bytes(m.app),
            format_bytes(m.wired),
            format_bytes(m.compressed),
            format_bytes(m.cached),
            format_bytes(m.swap_used)
        );
    }
    if let Some(d) = &s.disks {
        for v in &d.volumes {
            let used = v.total.saturating_sub(v.free);
            let pct = if v.total > 0 { used as f64 / v.total as f64 * 100.0 } else { 0.0 };
            let _ = writeln!(
                out,
                "Disk      {} {:>5.1}%   {}: {} free of {}",
                bar(pct, 24),
                pct,
                v.name,
                format_bytes(v.free),
                format_bytes(v.total)
            );
        }
        let (read, write) =
            (d.io.iter().map(|i| i.read_bps).sum::<f64>(), d.io.iter().map(|i| i.write_bps).sum::<f64>());
        let _ = writeln!(out, "          read {}   write {}", format_rate(read), format_rate(write));
    }
    if let Some(n) = &s.network {
        for i in n
            .interfaces
            .iter()
            .filter(|i| i.up && i.kind != InterfaceKind::Loopback && i.kind != InterfaceKind::Virtual)
        {
            let _ = writeln!(
                out,
                "Network   {:<8} down {:>12}   up {:>12}",
                i.name,
                format_rate(i.rx_bps),
                format_rate(i.tx_bps)
            );
        }
    }
    for g in &s.gpus {
        let _ = writeln!(out, "GPU       {}", gpu_line(g));
    }
    if let Some(sensors) = &s.sensors {
        if let Some(hot) = sensors.hottest() {
            let _ = writeln!(out, "Sensors   hottest {:.0} °C ({})", hot.celsius, hot.label);
        }
        if !sensors.fans.is_empty() {
            let rpm: Vec<String> = sensors.fans.iter().map(|f| format!("{:.0}", f.rpm)).collect();
            let _ = writeln!(out, "          fans {} rpm", rpm.join(" / "));
        }
    }
    if let Some(p) = &s.power {
        let mut parts = Vec::new();
        for (label, value) in
            [("CPU", p.cpu_w), ("GPU", p.gpu_w), ("ANE", p.ane_w), ("DRAM", p.dram_w), ("system", p.system_w)]
        {
            if let Some(w) = value {
                parts.push(format!("{label} {w:.1} W"));
            }
        }
        if !parts.is_empty() {
            let _ = writeln!(out, "Power     {}", parts.join("   "));
        }
    }
    if let Some(b) = &s.battery {
        let _ = write!(out, "Battery   {}", render_battery(b));
    }
    if top_processes > 0 && !s.processes.is_empty() {
        out.push('\n');
        out.push_str(&render_processes(&s.processes[..s.processes.len().min(top_processes)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::*;

    #[test]
    fn draws_bars() {
        assert_eq!(bar(0.0, 10), "░░░░░░░░░░");
        assert_eq!(bar(40.0, 10), "████░░░░░░");
        assert_eq!(bar(100.0, 10), "██████████");
        assert_eq!(bar(250.0, 10), "██████████", "out of range is clamped");
        assert_eq!(bar(-5.0, 4), "░░░░");
        assert_eq!(bar(f64::NAN.max(0.0), 4).chars().count(), 4);
    }

    #[test]
    fn draws_sparklines() {
        assert_eq!(sparkline(&[0.0, 5.0, 10.0]), "▁▅█");
        assert_eq!(sparkline(&[3.0, 3.0]), "▁▁", "a flat line is the lowest step, not a division by zero");
        assert_eq!(sparkline(&[]), "");
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            cpu: Some(CpuUsage {
                total_pct: 42.5,
                user_pct: 30.0,
                system_pct: 12.5,
                load_average: [1.0, 2.0, 3.0],
                frequency_mhz: Some(3400.0),
                cores: vec![
                    CoreUsage { usage_pct: 10.0, ..Default::default() },
                    CoreUsage { usage_pct: 90.0, ..Default::default() },
                ],
                ..Default::default()
            }),
            memory: Some(Memory { total: 32 << 30, used: 16 << 30, ..Default::default() }),
            thermal_pressure: Some(ThermalPressure::Nominal),
            gpus: vec![Gpu {
                name: "AMD Radeon".into(),
                utilization_pct: Some(12.0),
                temperature_c: Some(69.0),
                power_w: Some(21.0),
                ..Default::default()
            }],
            sensors: Some(Sensors {
                temperatures: vec![Temperature { label: "CPU die".into(), celsius: 61.0, ..Default::default() }],
                fans: vec![Fan { rpm: 1200.0, ..Default::default() }, Fan { rpm: 1800.0, ..Default::default() }],
            }),
            power: Some(PowerDraw { cpu_w: Some(23.4), ..Default::default() }),
            processes: vec![ProcessSample {
                pid: 7,
                name: "Safari".into(),
                cpu_pct: 12.0,
                memory_bytes: 1 << 30,
                threads: 9,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_snapshot_reads_as_sections() {
        let text = render_snapshot(&snapshot(), 5);
        for expected in [
            "CPU ",
            "42.5%",
            "3400 MHz nominal",
            "Memory",
            "50.0%",
            "16.0 GB of 32.0 GB",
            "Thermal   nominal",
            "GPU       AMD Radeon  ·  12%",
            "69 °C",
            "21 W",
            "hottest 61 °C (CPU die)",
            "fans 1200 / 1800 rpm",
            "CPU 23.4 W",
            "Safari",
        ] {
            assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
        }
        assert!(!text.contains("Battery"), "a Mac without a battery has no battery line");
        assert!(!text.contains("Network"));
    }

    #[test]
    fn processes_can_be_left_out() {
        assert!(!render_snapshot(&snapshot(), 0).contains("Safari"));
    }

    #[test]
    fn long_process_names_are_cut() {
        let p = ProcessSample { name: "x".repeat(80), ..Default::default() };
        let line = render_processes(&[p]).lines().nth(1).unwrap().to_string();
        assert!(line.contains(&"x".repeat(30)) && !line.contains(&"x".repeat(31)));
    }

    #[test]
    fn describes_a_battery() {
        let b = Battery {
            percent: 72.0,
            state: ChargeState::Discharging,
            health_pct: Some(81.0),
            cycle_count: Some(214),
            power_w: Some(-15.6),
            minutes_remaining: Some(283),
            ..Default::default()
        };
        let text = render_battery(&b);
        assert!(text.contains("72%") && text.contains("on battery") && text.contains("(4h 43m)"));
        assert!(text.contains("health 81%") && text.contains("214 cycles") && text.contains("-15.6 W"));
    }

    #[test]
    fn a_series_summarises_itself() {
        let series = Series {
            metric: Metric::CpuTotal,
            resolution_ms: 60_000,
            points: vec![
                Point { ts_ms: 0, avg: 10.0, min: 5.0, max: 20.0 },
                Point { ts_ms: 60_000, avg: 30.0, min: 25.0, max: 90.0 },
            ],
        };
        let text = render_series(&series);
        assert!(
            text.contains("lowest 5%")
                && text.contains("average 20%")
                && text.contains("highest 90%")
                && text.contains("2 points, 1m 00s each")
        );
        let empty = Series { metric: Metric::CpuTotal, resolution_ms: 1000, points: vec![] };
        assert!(render_series(&empty).contains("no data"));
    }

    #[test]
    fn describes_the_system() {
        let info = SystemInfo {
            model: "MacPro7,1".into(),
            chip: "Intel Xeon".into(),
            arch: Arch::X86_64,
            physical_cores: 8,
            logical_cores: 16,
            memory_bytes: 32 << 30,
            os_version: "26.2".into(),
            os_build: "25C56".into(),
            ..Default::default()
        };
        let text = render_info(&info);
        assert!(
            text.contains("MacPro7,1")
                && text.contains("Intel Xeon (Intel)")
                && text.contains("8 cores, 16 threads")
                && text.contains("32.0 GB")
        );
        let silicon = SystemInfo {
            arch: Arch::Arm64,
            performance_cores: Some(6),
            efficiency_cores: Some(4),
            physical_cores: 10,
            ..info
        };
        assert!(render_info(&silicon).contains("10 cores (6 performance + 4 efficiency)"));
    }

    #[test]
    fn sensors_without_hardware_say_so() {
        assert!(render_sensors(&Sensors::default()).contains("no temperature or fan sensors"));
    }
}
