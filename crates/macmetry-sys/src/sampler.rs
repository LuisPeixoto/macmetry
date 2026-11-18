//! Runs every collector and assembles one [`Snapshot`] per call.
//!
//! A collector that this Mac cannot support is left out for good; one that fails once is skipped for that sample and
//! logged. Neither stops the others, so a Mac without a battery or a virtual machine without sensors still reports
//! everything it has.

use crate::battery;
use crate::cpu::CpuCollector;
use crate::disk::DiskCollector;
use crate::gpu;
use crate::hid;
use crate::ioreport::{EnergyCollector, IntelEnergyCollector};
use crate::memory::MemoryCollector;
use crate::network::NetworkCollector;
use crate::process::ProcessCollector;
use crate::sensors::SensorCollector;
use crate::system;
use crate::thermal::ThermalCollector;
use macmetry_core::{Arch, Battery, Gpu, PowerDraw, ProcessSort, Snapshot, SystemInfo, top};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How many processes a snapshot carries when [`SampleOptions::processes`] is on. All of them would be hundreds of
/// rows every second; the interface shows the busiest.
pub const TOP_PROCESSES: usize = 100;

/// What this Mac can report. The interface hides the cards of what it cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct Capabilities {
    pub cpu: bool,
    pub memory: bool,
    pub disks: bool,
    pub network: bool,
    pub gpu: bool,
    pub temperatures: bool,
    pub fans: bool,
    pub power: bool,
    pub battery: bool,
    pub thermal_pressure: bool,
    pub processes: bool,
}

/// What to measure in one call.
#[derive(Debug, Clone, Copy)]
pub struct SampleOptions {
    /// Include the busiest processes. They cost the most, so the loop asks for them less often.
    pub processes: bool,
    /// How many processes to keep, sorted by CPU.
    pub top_processes: usize,
}

impl Default for SampleOptions {
    fn default() -> Self {
        Self { processes: true, top_processes: TOP_PROCESSES }
    }
}

pub struct Sampler {
    system: SystemInfo,
    cpu: Option<CpuCollector>,
    memory: Option<MemoryCollector>,
    disk: DiskCollector,
    network: NetworkCollector,
    processes: ProcessCollector,
    sensors: Option<SensorCollector>,
    apple_energy: Option<EnergyCollector>,
    intel_energy: Option<IntelEnergyCollector>,
    thermal: Option<ThermalCollector>,
    has_gpu: bool,
    has_battery: bool,
    last: Instant,
    last_processes: Instant,
    capabilities: Capabilities,
}

impl Sampler {
    /// Starts every collector and takes the baseline the first sample is measured against.
    pub fn new() -> Self {
        let system = system::read().unwrap_or_default();
        let cpu = CpuCollector::new().ok();
        let memory = MemoryCollector::new().ok();
        let sensors = SensorCollector::new().ok();
        let apple_energy = EnergyCollector::new().ok();
        let intel_energy = if apple_energy.is_none() { IntelEnergyCollector::new().ok() } else { None };
        let thermal = ThermalCollector::new();
        let has_gpu = gpu::read().is_ok();
        let has_battery = battery::read().is_ok();

        let (has_temperatures, has_fans) = sensors
            .as_ref()
            .and_then(|s| s.sample().ok())
            .map(|(s, _)| (!s.temperatures.is_empty(), !s.fans.is_empty()))
            .unwrap_or((false, false));
        let has_hid = system.arch == Arch::Arm64 && !hid::read_temperatures().is_empty();
        let capabilities = Capabilities {
            cpu: cpu.is_some(),
            memory: memory.is_some(),
            disks: true,
            network: true,
            gpu: has_gpu,
            temperatures: has_temperatures || has_hid,
            fans: has_fans,
            power: apple_energy.is_some() || intel_energy.is_some(),
            battery: has_battery,
            thermal_pressure: thermal.is_some(),
            processes: true,
        };
        let now = Instant::now();
        Self {
            system,
            cpu,
            memory,
            disk: DiskCollector::new(),
            network: NetworkCollector::new(),
            processes: ProcessCollector::new(),
            sensors,
            apple_energy,
            intel_energy,
            thermal,
            has_gpu,
            has_battery,
            last: now,
            last_processes: now,
            capabilities,
        }
    }

    pub fn system(&self) -> &SystemInfo {
        &self.system
    }

    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    /// Measures everything now. Rates cover the time since the previous call (or since [`Sampler::new`]).
    pub fn sample(&mut self, options: SampleOptions) -> Snapshot {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last).as_millis() as u64;
        self.last = now;

        let mut snapshot = Snapshot {
            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
            elapsed_ms: elapsed_ms.min(u64::from(u32::MAX)) as u32,
            thermal_pressure: self.thermal.as_ref().and_then(ThermalCollector::sample),
            ..Default::default()
        };
        snapshot.cpu = self.cpu.as_mut().and_then(|c| keep("cpu", c.sample()));
        snapshot.memory = self.memory.as_mut().and_then(|m| keep("memory", m.sample(elapsed_ms)));
        snapshot.disks = keep("disk", self.disk.sample(elapsed_ms));
        snapshot.network = keep("network", self.network.sample(elapsed_ms));
        if self.has_gpu {
            snapshot.gpus = keep("gpu", gpu::read()).unwrap_or_default();
        }
        if self.has_battery {
            snapshot.battery = keep::<Battery>("battery", battery::read());
        }

        let (smc, system_w) = self.sensors.as_ref().and_then(|s| keep("sensors", s.sample())).unzip();
        let mut sensors = smc.unwrap_or_default();
        if self.system.arch == Arch::Arm64 {
            sensors.temperatures.extend(hid::read_temperatures());
        }
        if !sensors.temperatures.is_empty() || !sensors.fans.is_empty() {
            snapshot.sensors = Some(sensors);
        }

        let mut power = self
            .apple_energy
            .as_mut()
            .and_then(|e| keep("energy", e.sample()))
            .or_else(|| self.intel_energy.as_mut().and_then(|e| keep("energy", e.sample())));
        if let Some(watts) = system_w.flatten() {
            power.get_or_insert_with(PowerDraw::default).system_w = Some(watts);
        }
        attach_gpu_power(&mut snapshot.gpus, power.as_ref());
        snapshot.power = power;

        if options.processes {
            let process_ms = now.duration_since(self.last_processes).as_millis() as u64;
            self.last_processes = now;
            if let Some(all) = keep("processes", self.processes.sample(process_ms)) {
                snapshot.processes = top(&all, ProcessSort::Cpu, options.top_processes, None);
            }
        }
        snapshot
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

/// On Apple silicon the GPU's power comes from the energy model, not from its driver.
fn attach_gpu_power(gpus: &mut [Gpu], power: Option<&PowerDraw>) {
    if let (Some(gpu), Some(watts)) = (gpus.first_mut(), power.and_then(|p| p.gpu_w))
        && gpu.power_w.is_none()
    {
        gpu.power_w = Some(watts);
    }
}

/// Logs a failure once per kind of message instead of every second, and turns the result into an option.
fn keep<T>(what: &str, result: crate::error::Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            if !e.is_unsupported() {
                tracing::warn!("{what}: {e}");
            }
            None
        }
    }
}

/// Samples forever, calling `on_snapshot` every `interval` until `stop` is set. Processes are included every
/// `process_every` snapshots. The wait before the next sample accounts for the time the last one took, so the
/// interval does not drift.
pub fn run(
    sampler: &mut Sampler,
    interval: Duration,
    process_every: u32,
    stop: &AtomicBool,
    mut on_snapshot: impl FnMut(Snapshot),
) {
    let process_every = process_every.max(1);
    let mut tick = 0u32;
    let mut next = Instant::now() + interval;
    while !stop.load(Ordering::Relaxed) {
        // Sleep in short steps so a stop request is noticed quickly, whatever the interval is.
        while Instant::now() < next {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep((next - Instant::now()).min(Duration::from_millis(50)));
        }
        let options = SampleOptions { processes: tick.is_multiple_of(process_every), ..Default::default() };
        on_snapshot(sampler.sample(options));
        tick = tick.wrapping_add(1);
        next += interval;
        // If sampling took longer than the interval, do not try to catch up with a burst.
        next = next.max(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::Metric;

    #[test]
    fn samples_a_complete_snapshot_of_this_mac() {
        let mut sampler = Sampler::new();
        std::thread::sleep(Duration::from_millis(300));
        let s = sampler.sample(SampleOptions::default());

        assert!(s.timestamp_ms > 1_600_000_000_000);
        assert!(s.elapsed_ms >= 250, "the sample covers the time since the baseline: {}", s.elapsed_ms);
        let caps = sampler.capabilities();
        assert_eq!(s.cpu.is_some(), caps.cpu);
        assert_eq!(s.memory.is_some(), caps.memory);
        assert_eq!(!s.gpus.is_empty(), caps.gpu);
        assert_eq!(s.battery.is_some(), caps.battery);
        assert_eq!(s.thermal_pressure.is_some(), caps.thermal_pressure);
        assert!(s.disks.is_some() && s.network.is_some());
        assert!(!s.processes.is_empty() && s.processes.len() <= TOP_PROCESSES);
        assert!(s.processes.windows(2).all(|w| w[0].cpu_pct >= w[1].cpu_pct), "sorted by CPU");

        // The metrics a Mac provides are readable, and the ones it does not are absent rather than zero.
        assert!(Metric::CpuTotal.value(&s).is_some());
        assert!(Metric::MemoryUsedPct.value(&s).is_some());
        assert_eq!(Metric::BatteryPercent.value(&s).is_some(), caps.battery);
        assert_eq!(Metric::TempMax.value(&s).is_some(), caps.temperatures);
    }

    #[test]
    fn the_snapshot_round_trips_through_json() {
        let mut sampler = Sampler::new();
        let s = sampler.sample(SampleOptions { processes: false, ..Default::default() });
        let json = serde_json::to_string(&s).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
        assert!(s.processes.is_empty(), "processes were not asked for");
    }

    #[test]
    fn the_loop_runs_until_stopped_and_includes_processes_every_n() {
        let mut sampler = Sampler::new();
        let stop = AtomicBool::new(false);
        let mut seen: Vec<bool> = Vec::new();
        run(&mut sampler, Duration::from_millis(60), 2, &stop, |s| {
            seen.push(!s.processes.is_empty());
            if seen.len() == 4 {
                stop.store(true, Ordering::Relaxed);
            }
        });
        assert_eq!(seen, [true, false, true, false]);
    }

    #[test]
    fn a_stop_request_ends_a_long_wait_quickly() {
        let mut sampler = Sampler::new();
        let stop = AtomicBool::new(true);
        let started = Instant::now();
        run(&mut sampler, Duration::from_secs(30), 1, &stop, |_| panic!("no sample should be taken"));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn gpu_power_comes_from_the_energy_model_when_the_driver_has_none() {
        let mut gpus = vec![Gpu::default()];
        attach_gpu_power(&mut gpus, Some(&PowerDraw { gpu_w: Some(3.5), ..Default::default() }));
        assert_eq!(gpus[0].power_w, Some(3.5));
        let mut with_driver = vec![Gpu { power_w: Some(21.0), ..Default::default() }];
        attach_gpu_power(&mut with_driver, Some(&PowerDraw { gpu_w: Some(3.5), ..Default::default() }));
        assert_eq!(with_driver[0].power_w, Some(21.0), "the driver's own figure wins");
        attach_gpu_power(&mut [], None);
    }
}
