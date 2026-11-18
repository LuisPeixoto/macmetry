//! Prints what every collector sees on this Mac, as JSON. Used to compare against Apple's own tools and to capture
//! fixtures: `cargo run -p macmetry-sys --example probe`.

use macmetry_sys::{
    cpu::CpuCollector, disk::DiskCollector, memory::MemoryCollector, network::NetworkCollector, system,
    thermal::ThermalCollector,
};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() {
    let mut cpu = CpuCollector::new().ok();
    let mut memory = MemoryCollector::new().ok();
    let mut disk = DiskCollector::new();
    let mut network = NetworkCollector::new();
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
        "thermal_pressure": thermal.and_then(|t| t.sample()),
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("the snapshot serializes"));
}
