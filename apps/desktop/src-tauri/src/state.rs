//! What the app shares between its threads: the latest reading, a short live buffer, the settings, the history
//! database and the alert engine.

use crate::settings::{self, Settings};
use macmetry_core::alerts::Engine;
use macmetry_core::{Metric, Ring, Snapshot, SystemInfo};
use macmetry_store::Store;
use macmetry_sys::sampler::{Capabilities, Sampler};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// How many live points are kept: ten minutes at one a second.
pub const LIVE_CAPACITY: usize = 600;

/// One reading reduced to the metric values, in the order of [`Metric::ALL`]. Small enough to send hundreds at once
/// to draw the charts when the window opens.
#[derive(Debug, Clone, Serialize)]
pub struct LivePoint {
    pub ts: u64,
    pub v: Vec<Option<f64>>,
}

impl LivePoint {
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        Self { ts: snapshot.timestamp_ms, v: Metric::ALL.iter().map(|m| m.value(snapshot)).collect() }
    }
}

pub struct Shared {
    pub system: SystemInfo,
    pub capabilities: Capabilities,
    pub latest: RwLock<Option<Snapshot>>,
    pub live: Mutex<Ring<LivePoint>>,
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub store: Mutex<Option<Store>>,
    pub alerts: Mutex<Engine>,
    pub stop: AtomicBool,
}

pub struct AppState {
    pub shared: Arc<Shared>,
}

impl AppState {
    /// Starts the collectors (which takes a moment) and opens the settings and the history. The sampler is returned
    /// separately because the sampling thread owns it.
    pub fn new() -> (Self, Sampler) {
        let sampler = Sampler::new();
        let data_dir = macmetry_store::data_dir();
        let settings_path = data_dir.join("settings.json");
        let loaded = settings::load(&settings_path);
        let store = match Store::open_default() {
            Ok(store) => Some(store),
            Err(e) => {
                tracing::warn!("The history is unavailable: {e}");
                None
            }
        };
        let shared = Arc::new(Shared {
            system: sampler.system().clone(),
            capabilities: sampler.capabilities(),
            latest: RwLock::new(None),
            live: Mutex::new(Ring::new(LIVE_CAPACITY)),
            alerts: Mutex::new(Engine::new(loaded.alerts.clone())),
            settings: RwLock::new(loaded),
            settings_path,
            store: Mutex::new(store),
            stop: AtomicBool::new(false),
        });
        (Self { shared }, sampler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::CpuUsage;

    #[test]
    fn a_live_point_follows_the_metric_order() {
        let snapshot = Snapshot {
            timestamp_ms: 7,
            cpu: Some(CpuUsage { total_pct: 42.0, ..Default::default() }),
            ..Default::default()
        };
        let point = LivePoint::from_snapshot(&snapshot);
        assert_eq!(point.ts, 7);
        assert_eq!(point.v.len(), Metric::ALL.len());
        let at = Metric::ALL.iter().position(|m| *m == Metric::CpuTotal).unwrap();
        assert_eq!(point.v[at], Some(42.0));
        let battery = Metric::ALL.iter().position(|m| *m == Metric::BatteryPercent).unwrap();
        assert_eq!(point.v[battery], None, "a missing part is null, not zero");
    }
}
