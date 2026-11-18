//! Prints what every collector sees on this Mac, as JSON. Used to compare against Apple's own tools and to capture
//! fixtures: `cargo run -p macmetry-sys --example probe`.

use macmetry_sys::{
    cpu::CpuCollector,
    disk::DiskCollector,
    gpu, hid,
    ioreport::{EnergyCollector, IntelEnergyCollector},
    memory::MemoryCollector,
    network::NetworkCollector,
    process::ProcessCollector,
    sensors::SensorCollector,
    system,
    thermal::ThermalCollector,
};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() {
    let mut cpu = CpuCollector::new().ok();
    let mut memory = MemoryCollector::new().ok();
    let mut disk = DiskCollector::new();
    let mut network = NetworkCollector::new();
    let mut processes = ProcessCollector::new();
    let mut apple_energy = EnergyCollector::new().ok();
    let mut intel_energy = IntelEnergyCollector::new().ok();
    let discovery = Instant::now();
    let sensors = SensorCollector::new();
    eprintln!("sensor discovery took {:?}", discovery.elapsed());
    let reading = Instant::now();
    let sensor_reading = sensors.as_ref().ok().and_then(|s| s.sample().ok());
    eprintln!("one sensor reading took {:?}", reading.elapsed());
    let thermal = ThermalCollector::new();
    let started = Instant::now();
    std::thread::sleep(Duration::from_secs(1));
    let elapsed = started.elapsed().as_millis() as u64;

    let out = json!({
        "system": system::read().ok(),
        "cpu": cpu.as_mut().and_then(|c| c.sample().ok()),
        "memory": memory.as_mut().and_then(|m| m.sample(elapsed).ok()),
        "disks": disk.sample(elapsed).ok(),
        "network": network.sample(elapsed).ok(),
        "gpus": gpu::read().ok(),
        "power": apple_energy.as_mut().and_then(|e| e.sample().ok()).or_else(|| intel_energy.as_mut().and_then(|e| e.sample().ok())),
        "hid_temperatures": hid::read_temperatures(),
        "sensors": sensor_reading.as_ref().map(|(s, _)| s),
        "system_power_w": sensor_reading.as_ref().and_then(|(_, w)| *w),
        "thermal_pressure": thermal.and_then(|t| t.sample()),
        "processes": processes.sample(elapsed).ok().map(|all| {
            json!({ "readable": all.len(), "top_cpu": macmetry_core::top(&all, macmetry_core::ProcessSort::Cpu, 5, None),
                    "top_memory": macmetry_core::top(&all, macmetry_core::ProcessSort::Memory, 5, None) })
        }),
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("the snapshot serializes"));
}
