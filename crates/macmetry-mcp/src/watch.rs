//! Live readings for an assistant: `watch_start` begins collecting a few metrics, `watch_read` returns what is new,
//! `watch_stop` ends it. Everything is bounded: how many watches run, how many readings each keeps, how long one may
//! go unread and how long it may run in all.

use macmetry_core::{Metric, Ring};
use macmetry_sys::sampler::{Sampler, run};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Watches that may run at once.
pub const MAX_WATCHES: usize = 4;
/// A watch nobody reads for this long stops by itself.
pub const IDLE_LIMIT: Duration = Duration::from_secs(5 * 60);
/// No watch runs longer than this, read or not.
pub const RUN_LIMIT: Duration = Duration::from_secs(60 * 60);
pub const MAX_KEPT: usize = 600;
pub const DEFAULT_KEPT: usize = 300;
pub const MAX_METRICS: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    pub timestamp_ms: u64,
    /// One value per watched metric, in the order of the watch's `metrics`; `null` when the Mac does not provide it.
    pub values: Vec<Option<f64>>,
}

struct Watch {
    metrics: Vec<Metric>,
    readings: Arc<Mutex<Ring<Reading>>>,
    stop: Arc<AtomicBool>,
    last_read: Arc<Mutex<Instant>>,
}

#[derive(Default)]
pub struct WatchHub {
    watches: Mutex<HashMap<String, Watch>>,
    counter: AtomicU64,
}

#[derive(Debug, Serialize)]
pub struct Started {
    pub id: String,
    pub metrics: Vec<&'static str>,
    pub interval_secs: f64,
    pub max_points: usize,
}

#[derive(Debug, Serialize)]
pub struct Read {
    pub id: String,
    pub metrics: Vec<&'static str>,
    pub readings: Vec<Reading>,
    /// Pass this as `since_ms` to the next read to get only what is new.
    pub next_since_ms: Option<u64>,
    pub running: bool,
}

impl WatchHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn forget_finished(watches: &mut HashMap<String, Watch>) {
        watches.retain(|_, w| !w.stop.load(Ordering::Relaxed));
    }

    pub fn start(&self, metrics: Vec<Metric>, interval: Duration, keep: usize) -> Result<Started, String> {
        if metrics.is_empty() || metrics.len() > MAX_METRICS {
            return Err(format!("Watch 1 to {MAX_METRICS} metrics."));
        }
        let mut watches = self.watches.lock();
        Self::forget_finished(&mut watches);
        if watches.len() >= MAX_WATCHES {
            return Err(format!("{MAX_WATCHES} watches are already running; stop one with watch_stop first."));
        }
        let id = format!("w{}", self.counter.fetch_add(1, Ordering::Relaxed) + 1);
        let readings = Arc::new(Mutex::new(Ring::new(keep)));
        let stop = Arc::new(AtomicBool::new(false));
        let last_read = Arc::new(Mutex::new(Instant::now()));

        let (thread_metrics, thread_readings, thread_stop, thread_read) =
            (metrics.clone(), readings.clone(), stop.clone(), last_read.clone());
        std::thread::Builder::new()
            .name(format!("macmetry-watch-{id}"))
            .spawn(move || {
                let mut sampler = Sampler::new();
                let begun = Instant::now();
                run(&mut sampler, interval, u32::MAX, &thread_stop.clone(), |snapshot| {
                    let values = thread_metrics.iter().map(|m| m.value(&snapshot)).collect();
                    thread_readings.lock().push(Reading { timestamp_ms: snapshot.timestamp_ms, values });
                    if thread_read.lock().elapsed() > IDLE_LIMIT || begun.elapsed() > RUN_LIMIT {
                        thread_stop.store(true, Ordering::Relaxed);
                    }
                });
                thread_stop.store(true, Ordering::Relaxed);
            })
            .map_err(|e| format!("Could not start the watch: {e}"))?;

        let started = Started {
            id: id.clone(),
            metrics: metrics.iter().map(|m| m.key()).collect(),
            interval_secs: interval.as_secs_f64(),
            max_points: keep,
        };
        watches.insert(id, Watch { metrics, readings, stop, last_read });
        Ok(started)
    }

    pub fn read(&self, id: &str, since_ms: Option<u64>) -> Result<Read, String> {
        let watches = self.watches.lock();
        let watch = watches.get(id).ok_or_else(|| {
            format!(
                "There is no watch `{id}`. It may have stopped on its own after {} minutes unread.",
                IDLE_LIMIT.as_secs() / 60
            )
        })?;
        *watch.last_read.lock() = Instant::now();
        let readings: Vec<Reading> =
            watch.readings.lock().iter().filter(|r| since_ms.is_none_or(|s| r.timestamp_ms > s)).cloned().collect();
        let next_since_ms = readings.last().map(|r| r.timestamp_ms).or(since_ms);
        Ok(Read {
            id: id.to_string(),
            metrics: watch.metrics.iter().map(|m| m.key()).collect(),
            readings,
            next_since_ms,
            running: !watch.stop.load(Ordering::Relaxed),
        })
    }

    pub fn stop(&self, id: &str) -> Result<(), String> {
        let watch = self.watches.lock().remove(id).ok_or_else(|| format!("There is no watch `{id}`."))?;
        watch.stop.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Ends every watch. Called when the client disconnects. Nothing on the Mac was changed, so there is nothing to
    /// put back, only threads to release.
    pub fn shutdown(&self) {
        for (_, watch) in self.watches.lock().drain() {
            watch.stop.store(true, Ordering::Relaxed);
        }
    }

    #[cfg(test)]
    pub fn running(&self) -> usize {
        let mut watches = self.watches.lock();
        Self::forget_finished(&mut watches);
        watches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_readings_and_reads_only_what_is_new() {
        let hub = WatchHub::new();
        let started =
            hub.start(vec![Metric::CpuTotal, Metric::BatteryPercent], Duration::from_millis(100), 50).unwrap();
        assert_eq!(started.metrics, ["cpu.total", "battery.percent"]);
        assert_eq!(started.interval_secs, 0.1);
        std::thread::sleep(Duration::from_millis(700));

        let first = hub.read(&started.id, None).unwrap();
        assert!(first.readings.len() >= 2, "got {}", first.readings.len());
        assert!(first.running);
        assert!(first.readings[0].values[0].is_some(), "the CPU is always there");
        let since = first.next_since_ms.unwrap();

        std::thread::sleep(Duration::from_millis(400));
        let second = hub.read(&started.id, Some(since)).unwrap();
        assert!(!second.readings.is_empty());
        assert!(second.readings.iter().all(|r| r.timestamp_ms > since), "nothing is repeated");

        hub.stop(&started.id).unwrap();
        assert!(hub.read(&started.id, None).is_err());
    }

    #[test]
    fn keeps_only_the_newest_readings() {
        let hub = WatchHub::new();
        let id = hub.start(vec![Metric::CpuTotal], Duration::from_millis(50), 3).unwrap().id;
        std::thread::sleep(Duration::from_millis(900));
        assert_eq!(hub.read(&id, None).unwrap().readings.len(), 3, "a ring of 3");
        hub.shutdown();
    }

    #[test]
    fn limits_how_many_run_at_once() {
        let hub = WatchHub::new();
        let ids: Vec<String> = (0..MAX_WATCHES)
            .map(|_| hub.start(vec![Metric::CpuTotal], Duration::from_secs(1), 10).unwrap().id)
            .collect();
        let refused = hub.start(vec![Metric::CpuTotal], Duration::from_secs(1), 10).unwrap_err();
        assert!(refused.contains("already running"), "{refused}");
        hub.stop(&ids[0]).unwrap();
        assert!(hub.start(vec![Metric::CpuTotal], Duration::from_secs(1), 10).is_ok(), "a slot opened up");
        hub.shutdown();
        assert_eq!(hub.running(), 0);
    }

    #[test]
    fn refuses_no_metrics_and_too_many() {
        let hub = WatchHub::new();
        assert!(hub.start(vec![], Duration::from_secs(1), 10).is_err());
        assert!(hub.start(vec![Metric::CpuTotal; MAX_METRICS + 1], Duration::from_secs(1), 10).is_err());
    }

    #[test]
    fn an_unknown_watch_explains_itself() {
        let hub = WatchHub::new();
        assert!(hub.read("nope", None).unwrap_err().contains("no watch"));
        assert!(hub.stop("nope").is_err());
    }
}
