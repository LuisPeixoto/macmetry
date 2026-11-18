//! What each command does. Collectors and the database are used as they are; this module only wires them up and
//! prints.

use crate::args::{Command, Format, resolve_range};
use crate::render::*;
use anyhow::{Context, Result, bail};
use macmetry_core::series::{to_csv, to_json};
use macmetry_core::{Metric, ProcessSort, Series, Snapshot, top};
use macmetry_store::Store;
use macmetry_sys::process::ProcessCollector;
use macmetry_sys::sampler::{SampleOptions, Sampler, run};
use macmetry_sys::{battery, system};
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Starts the collectors, waits, and measures. Rates are meaningless without the wait.
fn measure(wait: Duration, processes: usize) -> (Sampler, Snapshot) {
    let mut sampler = Sampler::new();
    std::thread::sleep(wait);
    let snapshot = sampler.sample(SampleOptions { processes: processes > 0, top_processes: processes });
    (sampler, snapshot)
}

pub fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Info { json } => {
            let info = system::read().context("Could not read this Mac's details")?;
            if json {
                print_json(&info)
            } else {
                print!("{}", render_info(&info));
                Ok(())
            }
        }
        Command::Snapshot { json, wait_ms, processes } => {
            let (sampler, snapshot) = measure(Duration::from_millis(wait_ms), processes as usize);
            if json {
                print_json(
                    &serde_json::json!({ "system": sampler.system(), "capabilities": sampler.capabilities(), "snapshot": snapshot }),
                )
            } else {
                print!("{}", render_snapshot(&snapshot, processes as usize));
                Ok(())
            }
        }
        Command::Watch { interval, count, processes } => watch(interval, count, processes as usize),
        Command::Top { sort, limit, filter, json } => {
            let mut collector = ProcessCollector::new();
            std::thread::sleep(Duration::from_millis(1000));
            let all = collector.sample(1000).context("Could not read the processes")?;
            let picked = top(&all, ProcessSort::from(sort), limit as usize, filter.as_deref());
            if json {
                print_json(&picked)
            } else {
                print!("{}", render_processes(&picked));
                Ok(())
            }
        }
        Command::Sensors { json } => {
            let (_, snapshot) = measure(Duration::from_millis(300), 0);
            match snapshot.sensors {
                Some(sensors) if json => print_json(&sensors),
                Some(sensors) => {
                    print!("{}", render_sensors(&sensors));
                    Ok(())
                }
                None if json => print_json(&serde_json::Value::Null),
                None => {
                    println!("This Mac reports no temperature or fan sensors.");
                    Ok(())
                }
            }
        }
        Command::Battery { json } => match battery::read() {
            Ok(b) if json => print_json(&b),
            Ok(b) => {
                print!("{}", render_battery(&b));
                Ok(())
            }
            Err(e) if e.is_unsupported() && json => print_json(&serde_json::Value::Null),
            Err(e) if e.is_unsupported() => {
                println!("This Mac has no battery.");
                Ok(())
            }
            Err(e) => Err(e.into()),
        },
        Command::History { metric, since, points, json } => {
            let store = open_history()?;
            let now = now_ms();
            let series = store.query(metric, now.saturating_sub(since), now, points as usize)?;
            if json {
                print_json(&to_json(&[series]))
            } else {
                print!("{}", render_series(&series));
                Ok(())
            }
        }
        Command::Export { metrics, from, to, format, range, points, output, force } => {
            let store = open_history()?;
            let (from, to) = resolve_range(&from, &to, now_ms()).map_err(anyhow::Error::msg)?;
            let series = metrics
                .iter()
                .map(|&m| store.query(m, from, to, points as usize))
                .collect::<Result<Vec<Series>, _>>()?;
            let text = match format {
                Format::Csv => to_csv(&series, range),
                Format::Json => serde_json::to_string_pretty(&to_json(&series))? + "\n",
            };
            write_output(output.as_deref(), &text, force)
        }
        Command::Record { interval } => record(interval),
        Command::Metrics => {
            for m in Metric::ALL {
                println!("{:<22} {:<28} {:?}", m.key(), m.label(), m.unit());
            }
            Ok(())
        }
    }
}

fn open_history() -> Result<Store> {
    Ok(Store::open_read_only(&macmetry_store::history_path())?)
}

/// Writes to a file, or to the terminal when there is no path. An existing file is never replaced unless asked.
fn write_output(path: Option<&Path>, text: &str, force: bool) -> Result<()> {
    let Some(path) = path else {
        print!("{text}");
        return Ok(());
    };
    if path.is_dir() {
        bail!("{} is a folder; give a file name.", path.display());
    }
    if path.exists() && !force {
        bail!("{} already exists. Use --force to replace it.", path.display());
    }
    std::fs::write(path, text).with_context(|| format!("Could not write {}", path.display()))?;
    eprintln!("Wrote {}", path.display());
    Ok(())
}

fn watch(interval: f64, count: Option<u32>, processes: usize) -> Result<()> {
    let mut sampler = Sampler::new();
    let stop = AtomicBool::new(false);
    let interactive = std::io::stdout().is_terminal();
    let mut shown = 0u32;
    run(&mut sampler, Duration::from_secs_f64(interval), 1, &stop, |snapshot| {
        let mut out = std::io::stdout().lock();
        if interactive {
            // Home the cursor and clear below, so the picture redraws in place.
            let _ = write!(out, "\x1b[H\x1b[J");
        }
        let _ = write!(out, "{}", render_snapshot(&snapshot, processes));
        let _ = writeln!(out, "\n  every {interval} s · Ctrl-C to leave");
        let _ = out.flush();
        shown += 1;
        if count.is_some_and(|c| shown >= c) {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });
    Ok(())
}

fn record(interval: f64) -> Result<()> {
    let store = Store::open_default()?;
    eprintln!("Recording to {} every {interval} s. Press Ctrl-C to stop.", store.path().display());
    let mut sampler = Sampler::new();
    let stop = AtomicBool::new(false);
    let (mut recorded, mut last_maintenance) = (0u64, now_ms());
    run(&mut sampler, Duration::from_secs_f64(interval), 1_000_000, &stop, |snapshot| {
        if let Err(e) = store.record(&snapshot) {
            eprintln!("macmetry: {e}");
            return;
        }
        recorded += 1;
        if snapshot.timestamp_ms.saturating_sub(last_maintenance) >= 60_000 {
            last_maintenance = snapshot.timestamp_ms;
            if let Err(e) = store.maintain(snapshot.timestamp_ms) {
                eprintln!("macmetry: {e}");
            }
        }
        if recorded % 60 == 0 {
            eprintln!("{recorded} samples recorded");
        }
    });
    Ok(())
}
