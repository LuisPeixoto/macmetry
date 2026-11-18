//! Checks that need a real Mac, and that say what *must* be true on each kind: an Apple silicon Mac has to report
//! energy and efficiency cores, an Intel Mac has to have an SMC. They are ignored in the normal run, because a virtual
//! machine (such as a CI runner) has none of this hardware. Run them with `tools/live-tests.sh`, or
//! `cargo test -p macmetry-sys --test live -- --ignored --test-threads=1`.

#![cfg(target_os = "macos")]

use macmetry_core::{Arch, CoreKind};
use macmetry_sys::ioreport::EnergyCollector;
use macmetry_sys::sampler::{SampleOptions, Sampler};
use macmetry_sys::{hid, sensors::SensorCollector, system};
use std::time::Duration;

fn arch() -> Arch {
    system::read().expect("the Mac describes itself").arch
}

#[test]
#[ignore = "needs a real Mac"]
fn apple_silicon_reports_energy_and_the_two_kinds_of_core() {
    if arch() != Arch::Arm64 {
        eprintln!("not Apple silicon: skipped");
        return;
    }
    let info = system::read().unwrap();
    assert!(info.performance_cores.is_some_and(|n| n > 0), "performance cores: {info:?}");
    assert!(info.efficiency_cores.is_some_and(|n| n > 0), "efficiency cores: {info:?}");

    let mut energy = EnergyCollector::new().expect("Apple silicon publishes an Energy Model");
    std::thread::sleep(Duration::from_millis(800));
    let power = energy.sample().unwrap();
    assert!(power.cpu_w.is_some_and(|w| (0.0..=300.0).contains(&w)), "CPU power: {power:?}");

    let mut sampler = Sampler::new();
    std::thread::sleep(Duration::from_millis(500));
    let snapshot = sampler.sample(SampleOptions { processes: false, ..Default::default() });
    let cpu = snapshot.cpu.expect("a CPU reading");
    assert!(
        cpu.cores.iter().any(|c| c.kind == CoreKind::Efficiency)
            && cpu.cores.iter().any(|c| c.kind == CoreKind::Performance)
    );
    assert_eq!(cpu.clusters.len(), 2, "efficiency and performance clusters");
}

#[test]
#[ignore = "needs a real Mac"]
fn apple_silicon_has_hid_temperature_sensors() {
    if arch() != Arch::Arm64 {
        eprintln!("not Apple silicon: skipped");
        return;
    }
    let sensors = hid::read_temperatures();
    assert!(!sensors.is_empty(), "no HID temperature sensor was readable without root");
    for s in &sensors {
        assert!((1.0..=125.0).contains(&s.celsius), "{}: {}", s.label, s.celsius);
    }
}

#[test]
#[ignore = "needs a real Mac"]
fn a_mac_that_is_not_a_virtual_machine_has_sensors_or_says_so() {
    match SensorCollector::new() {
        Ok(collector) => {
            let (sensors, _) = collector.sample().unwrap();
            assert!(!sensors.temperatures.is_empty() || !sensors.fans.is_empty(), "an SMC with nothing to read");
        }
        Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
    }
}

#[test]
#[ignore = "needs a real Mac"]
fn an_intel_mac_reports_package_power_and_has_no_core_split() {
    if arch() != Arch::X86_64 {
        eprintln!("not Intel: skipped");
        return;
    }
    let info = system::read().unwrap();
    assert!(info.performance_cores.is_none() && info.efficiency_cores.is_none(), "{info:?}");
    let mut sampler = Sampler::new();
    std::thread::sleep(Duration::from_millis(800));
    let snapshot = sampler.sample(SampleOptions { processes: false, ..Default::default() });
    let cpu = snapshot.cpu.expect("a CPU reading");
    assert!(cpu.clusters.is_empty());
    assert!(!cpu.frequency_is_measured, "Intel's frequency is nominal without root");
}

#[test]
#[ignore = "needs a real Mac"]
fn a_laptop_reports_a_battery_and_a_desktop_does_not() {
    let model = system::read().unwrap().model;
    let mut sampler = Sampler::new();
    std::thread::sleep(Duration::from_millis(300));
    let snapshot = sampler.sample(SampleOptions { processes: false, ..Default::default() });
    let is_laptop = model.contains("Book");
    assert_eq!(snapshot.battery.is_some(), is_laptop, "{model}: battery {:?}", snapshot.battery);
}

#[test]
#[ignore = "needs a real Mac"]
fn the_snapshot_costs_little_to_take() {
    let mut sampler = Sampler::new();
    std::thread::sleep(Duration::from_millis(500));
    let started = std::time::Instant::now();
    for _ in 0..10 {
        sampler.sample(SampleOptions { processes: false, ..Default::default() });
    }
    let each = started.elapsed() / 10;
    assert!(each < Duration::from_millis(100), "a reading took {each:?}; it should be a few milliseconds");
}
