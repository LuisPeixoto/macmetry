//! The arguments of every tool. The doc comments become the descriptions an assistant reads, so they say what a value
//! means and what the limits are.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct Snapshot {
    /// How long to measure before answering, in milliseconds (200 to 5000, default 1000). Rates such as CPU use and
    /// network speed need a little time to mean anything.
    pub wait_ms: Option<u64>,
    /// How many of the busiest processes to include (0 to 50, default 5).
    pub processes: Option<usize>,
    /// Only these parts: any of cpu, memory, disks, network, gpus, sensors, power, battery, thermal, processes.
    /// Leave out for everything. Asking for fewer parts saves tokens.
    pub sections: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct Processes {
    /// What to sort by: cpu, memory, disk_read, disk_write or name (default cpu).
    pub sort: Option<String>,
    /// How many processes to return (1 to 100, default 15).
    pub limit: Option<usize>,
    /// Only processes whose name contains this text (case-insensitive).
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct History {
    /// The metric's key, for example cpu.total or temp.max. list_metrics shows them all.
    pub metric: String,
    /// How far back to look: 30s, 15m, 6h or 7d (default 1h; the history keeps 30 days).
    pub since: Option<String>,
    /// How many points to return at most (2 to 500, default 60). Each point has the average, lowest and highest value
    /// of the time it covers, so a short spike is not lost.
    pub points: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchStart {
    /// The metrics to follow (1 to 10 keys, see list_metrics).
    pub metrics: Vec<String>,
    /// Seconds between readings (1 to 60, default 2).
    pub interval_secs: Option<f64>,
    /// How many readings to keep (10 to 600, default 300). Older ones are dropped.
    pub max_points: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchRead {
    /// The id watch_start returned.
    pub id: String,
    /// Only readings after this time, in milliseconds since the epoch (the `next_since` of the previous read). Leave
    /// out for everything kept.
    pub since_ms: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WatchStop {
    /// The id watch_start returned.
    pub id: String,
}
