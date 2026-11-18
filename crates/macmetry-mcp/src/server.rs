//! The tools. Each one is a thin wrapper over the same collectors, history and formatting the app and the command
//! line use, so a number means the same thing everywhere. Every tool only reads.

use crate::args;
use crate::privacy::for_assistant;
use crate::watch::{DEFAULT_KEPT, MAX_KEPT, WatchHub};
use macmetry_core::alerts::format_value;
use macmetry_core::timespec::parse_duration;
use macmetry_core::{Metric, ProcessSort, Snapshot, top};
use macmetry_store::Store;
use macmetry_sys::battery;
use macmetry_sys::process::ProcessCollector;
use macmetry_sys::sampler::{Capabilities, SampleOptions, Sampler};
use parking_lot::Mutex;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, GetPromptRequestParams, GetPromptResponse, InitializeResult, ListPromptsResult,
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, tool, tool_handler, tool_router};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The longest text a tool returns. Assistants read what is sent, so keep it focused.
pub(crate) const MAX_TEXT: usize = 60_000;
const MAX_WAIT_MS: u64 = 5_000;
const MIN_WAIT_MS: u64 = 200;
const MAX_PROCESSES: usize = 100;

/// The parts of a snapshot that `sections` can pick.
pub(crate) const SECTIONS: [&str; 10] =
    ["cpu", "memory", "disks", "network", "gpus", "sensors", "power", "battery", "thermal", "processes"];

struct Processes {
    collector: ProcessCollector,
    last: Instant,
}

#[derive(Clone)]
pub struct MacmetryServer {
    sampler: Arc<Mutex<Option<Sampler>>>,
    processes: Arc<Mutex<Option<Processes>>>,
    watches: Arc<WatchHub>,
    history: PathBuf,
    tool_router: ToolRouter<MacmetryServer>,
}

pub(crate) fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map(|t| cap(t, MAX_TEXT)).map_err(|e| e.to_string())
}

/// Cuts text that is too long, and says so. Used for what is already text; structured results are bounded by items
/// before they are serialized, so they are never cut in the middle of JSON.
pub(crate) fn cap(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n… (cut: ask for fewer sections or points)");
    }
    text
}

fn reply(result: Result<String, String>) -> CallToolResult {
    match result {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
    }
}

/// Keeps only the named parts of a snapshot. An unknown name is an error that lists the valid ones.
pub(crate) fn pick_sections(mut snapshot: Snapshot, sections: &[String]) -> Result<Snapshot, String> {
    if let Some(bad) = sections.iter().find(|s| !SECTIONS.contains(&s.as_str())) {
        return Err(format!("`{bad}` is not a section. Choose from: {}.", SECTIONS.join(", ")));
    }
    let wants = |name: &str| sections.iter().any(|s| s == name);
    if !wants("cpu") {
        snapshot.cpu = None;
    }
    if !wants("memory") {
        snapshot.memory = None;
    }
    if !wants("disks") {
        snapshot.disks = None;
    }
    if !wants("network") {
        snapshot.network = None;
    }
    if !wants("gpus") {
        snapshot.gpus.clear();
    }
    if !wants("sensors") {
        snapshot.sensors = None;
    }
    if !wants("power") {
        snapshot.power = None;
    }
    if !wants("battery") {
        snapshot.battery = None;
    }
    if !wants("thermal") {
        snapshot.thermal_pressure = None;
    }
    if !wants("processes") {
        snapshot.processes.clear();
    }
    Ok(snapshot)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn parse_metrics(keys: &[String]) -> Result<Vec<Metric>, String> {
    keys.iter()
        .map(|key| {
            Metric::from_key(key).ok_or_else(|| {
                format!("`{key}` is not a metric. list_metrics shows them all (for example cpu.total, temp.max).")
            })
        })
        .collect()
}

impl MacmetryServer {
    /// A server that reads the history at `history`.
    pub fn new(history: PathBuf) -> Self {
        let tool_router = Self::metrics_router() + Self::history_router() + Self::watches_router();
        Self {
            sampler: Arc::new(Mutex::new(None)),
            processes: Arc::new(Mutex::new(None)),
            watches: WatchHub::new(),
            history,
            tool_router,
        }
    }

    /// The tools this server offers, without starting any collector. Used to generate the reference.
    pub fn tool_definitions() -> Vec<rmcp::model::Tool> {
        (Self::metrics_router() + Self::history_router() + Self::watches_router()).list_all()
    }

    /// Ends every watch the session started.
    pub fn shutdown(&self) {
        self.watches.shutdown();
    }

    /// Measures with the shared sampler, waiting until at least `wait` has passed since its last reading so rates mean
    /// something.
    pub(crate) async fn measure_public(
        &self,
        wait: Duration,
        processes: usize,
    ) -> Result<(macmetry_core::SystemInfo, Capabilities, Snapshot), String> {
        self.measure(wait, processes).await
    }

    async fn measure(
        &self,
        wait: Duration,
        processes: usize,
    ) -> Result<(macmetry_core::SystemInfo, Capabilities, Snapshot), String> {
        let sampler = self.sampler.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = sampler.lock();
            let sampler = guard.get_or_insert_with(Sampler::new);
            if let Some(missing) = wait.checked_sub(sampler.idle_for()) {
                std::thread::sleep(missing);
            }
            let snapshot = sampler.sample(SampleOptions { processes: processes > 0, top_processes: processes });
            (sampler.system().clone(), sampler.capabilities(), snapshot)
        })
        .await
        .map_err(|e| format!("The measurement failed: {e}"))
    }

    fn open_history(&self) -> Result<Store, String> {
        Store::open_read_only(&self.history).map_err(|e| e.to_string())
    }
}

#[tool_router(router = metrics_router, vis = "pub(crate)")]
impl MacmetryServer {
    #[tool(
        description = "Describe this Mac: model, chip, architecture (Intel or Apple silicon), cores, memory and macOS version, and which kinds of readings it can provide (a desktop has no battery; Intel has no efficiency cores). Start here.",
        title = "About this Mac",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn system_info(&self) -> CallToolResult {
        let sampler = self.sampler.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut guard = sampler.lock();
            let sampler = guard.get_or_insert_with(Sampler::new);
            (sampler.system().clone(), sampler.capabilities())
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|(system, capabilities)| json(&serde_json::json!({ "system": system, "can_read": capabilities })));
        reply(result)
    }

    #[tool(
        description = "Measure everything now: CPU (total, per core), memory (used, wired, compressed, swap, pressure), disks (space and throughput), network (speed per interface, no addresses), GPU, temperatures and fans, power, battery, thermal pressure and the busiest processes. Parts this Mac does not have are left out. Use `sections` to ask for only what you need.",
        title = "Measure now",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn snapshot(&self, Parameters(a): Parameters<args::Snapshot>) -> CallToolResult {
        reply(
            async {
                let wait = a.wait_ms.unwrap_or(1000);
                if !(MIN_WAIT_MS..=MAX_WAIT_MS).contains(&wait) {
                    return Err(format!("wait_ms must be between {MIN_WAIT_MS} and {MAX_WAIT_MS}."));
                }
                let processes = a.processes.unwrap_or(5);
                if processes > 50 {
                    return Err("processes can be at most 50; use the processes tool for a longer list.".into());
                }
                let sections = a.sections.filter(|s| !s.is_empty());
                if let Some(bad) = sections.iter().flatten().find(|s| !SECTIONS.contains(&s.as_str())) {
                    return Err(format!("`{bad}` is not a section. Choose from: {}.", SECTIONS.join(", ")));
                }
                let wants_processes = sections.as_ref().is_none_or(|s| s.iter().any(|x| x == "processes"));
                let (_, _, snapshot) =
                    self.measure(Duration::from_millis(wait), if wants_processes { processes } else { 0 }).await?;
                let snapshot = for_assistant(snapshot);
                let snapshot = match &sections {
                    Some(s) => pick_sections(snapshot, s)?,
                    None => snapshot,
                };
                json(&snapshot)
            }
            .await,
        )
    }

    #[tool(
        description = "The busiest processes, sorted by CPU, memory or disk use, with their CPU percentage (of one core, so 200 means two cores), memory footprint, threads, user and disk throughput. Only processes of the current user are visible without administrator rights.",
        title = "Busiest processes",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn processes(&self, Parameters(a): Parameters<args::Processes>) -> CallToolResult {
        reply(
            async {
                let sort = match a.sort.as_deref() {
                    None => ProcessSort::Cpu,
                    Some(key) => ProcessSort::from_key(key).ok_or_else(|| {
                        format!("`{key}` is not a sort. Use cpu, memory, disk_read, disk_write or name.")
                    })?,
                };
                let limit = a.limit.unwrap_or(15);
                if !(1..=MAX_PROCESSES).contains(&limit) {
                    return Err(format!("limit must be between 1 and {MAX_PROCESSES}."));
                }
                let state = self.processes.clone();
                let all = tokio::task::spawn_blocking(move || {
                    let mut guard = state.lock();
                    let state = guard
                        .get_or_insert_with(|| Processes { collector: ProcessCollector::new(), last: Instant::now() });
                    // CPU use is a rate; the first reading needs something to compare with.
                    let wait = Duration::from_millis(800).saturating_sub(state.last.elapsed());
                    std::thread::sleep(wait);
                    let elapsed = state.last.elapsed().as_millis() as u64;
                    state.last = Instant::now();
                    state.collector.sample(elapsed)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
                json(&top(&all, sort, limit, a.filter.as_deref()))
            }
            .await,
        )
    }

    #[tool(
        description = "Temperatures (CPU cores, GPU, storage, battery...), fan speeds, power draw by component and thermal pressure. A hot Mac that is not slowing down has `nominal` pressure; `moderate`, `heavy` and `trapping` mean macOS is reducing speed to cool off.",
        title = "Temperatures, fans and power",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn sensors(&self) -> CallToolResult {
        reply(
            async {
                let (_, capabilities, snapshot) = self.measure(Duration::from_millis(500), 0).await?;
                json(&serde_json::json!({
                    "temperatures": snapshot.sensors.as_ref().map(|s| &s.temperatures),
                    "fans": snapshot.sensors.as_ref().map(|s| &s.fans),
                    "power": snapshot.power,
                    "gpu_temperatures_c": snapshot.gpus.iter().map(|g| (g.name.clone(), g.temperature_c)).collect::<Vec<_>>(),
                    "thermal_pressure": snapshot.thermal_pressure,
                    "has_temperature_sensors": capabilities.temperatures,
                    "has_fans": capabilities.fans,
                }))
            }
            .await,
        )
    }

    #[tool(
        description = "The battery: charge, whether it is charging, health (maximum capacity against design), cycle count, temperature and power. A Mac without a battery says so.",
        title = "Battery",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn battery(&self) -> CallToolResult {
        reply(match tokio::task::spawn_blocking(battery::read).await {
            Ok(Ok(battery)) => json(&battery),
            Ok(Err(e)) if e.is_unsupported() => Ok("This Mac has no battery.".to_string()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(e) => Err(e.to_string()),
        })
    }

    #[tool(
        description = "Every metric the history and the watches understand: its key, name and unit.",
        title = "List metrics",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn list_metrics(&self) -> CallToolResult {
        reply(json(
            &Metric::ALL
                .iter()
                .map(|m| serde_json::json!({ "key": m.key(), "name": m.label(), "unit": m.unit() }))
                .collect::<Vec<_>>(),
        ))
    }
}

#[tool_router(router = history_router, vis = "pub(crate)")]
impl MacmetryServer {
    #[tool(
        description = "How one metric changed over the last minutes, hours or days, from the history macmetry recorded (the app, or `macmetry record`, must have been running). Each point has the average, lowest and highest value of the time it covers. The history keeps 24 hours at one second and 30 days at one minute.",
        title = "History of a metric",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn history_query(&self, Parameters(a): Parameters<args::History>) -> CallToolResult {
        reply(
            async {
                let metric = parse_metrics(std::slice::from_ref(&a.metric))?[0];
                let since = parse_duration(a.since.as_deref().unwrap_or("1h"))?;
                let points = a.points.unwrap_or(60);
                if !(2..=500).contains(&points) {
                    return Err("points must be between 2 and 500.".into());
                }
                let store = self.open_history()?;
                let end = now_ms();
                let series = store.query(metric, end.saturating_sub(since), end, points).map_err(|e| e.to_string())?;
                let round = |v: f64| (v * 100.0).round() / 100.0;
                let (min, max) = series.points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.min), hi.max(p.max)));
                let mean = series.points.iter().map(|p| p.avg).sum::<f64>() / series.points.len().max(1) as f64;
                json(&serde_json::json!({
                    "metric": metric.key(),
                    "name": metric.label(),
                    "unit": metric.unit(),
                    "each_point_covers_seconds": series.resolution_ms / 1000,
                    "summary": if series.points.is_empty() { serde_json::Value::Null } else { serde_json::json!({
                        "lowest": format_value(metric, min), "average": format_value(metric, mean), "highest": format_value(metric, max),
                    }) },
                    "points": series.points.iter().map(|p| serde_json::json!({
                        "time": macmetry_core::series::rfc3339(p.ts_ms), "avg": round(p.avg), "min": round(p.min), "max": round(p.max),
                    })).collect::<Vec<_>>(),
                }))
            }
            .await,
        )
    }
}

#[tool_router(router = watches_router, vis = "pub(crate)")]
impl MacmetryServer {
    #[tool(
        description = "Start following a few metrics live, for example while you ask the person to open an app. Readings are kept in a short buffer; call watch_read to get them and watch_stop when done. At most 4 watches run at once, and one nobody reads for 5 minutes stops by itself.",
        title = "Start a watch",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn watch_start(&self, Parameters(a): Parameters<args::WatchStart>) -> CallToolResult {
        reply(
            async {
                let metrics = parse_metrics(&a.metrics)?;
                let interval = a.interval_secs.unwrap_or(2.0);
                if !(1.0..=60.0).contains(&interval) {
                    return Err("interval_secs must be between 1 and 60.".into());
                }
                let keep = a.max_points.unwrap_or(DEFAULT_KEPT);
                if !(10..=MAX_KEPT).contains(&keep) {
                    return Err(format!("max_points must be between 10 and {MAX_KEPT}."));
                }
                json(&self.watches.start(metrics, Duration::from_secs_f64(interval), keep)?)
            }
            .await,
        )
    }

    #[tool(
        description = "Read what a watch collected. Pass the previous call's `next_since_ms` as `since_ms` to get only the new readings.",
        title = "Read a watch",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = false, open_world_hint = false)
    )]
    async fn watch_read(&self, Parameters(a): Parameters<args::WatchRead>) -> CallToolResult {
        reply(self.watches.read(&a.id, a.since_ms).and_then(|r| json(&r)))
    }

    #[tool(
        description = "Stop a watch started with watch_start and free it. Call it when you have the readings you need; a watch left running keeps measuring until it goes unread for 5 minutes.",
        title = "Stop a watch",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn watch_stop(&self, Parameters(a): Parameters<args::WatchStop>) -> CallToolResult {
        reply(self.watches.stop(&a.id).map(|()| format!("Stopped {}.", a.id)))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MacmetryServer {
    fn get_info(&self) -> ServerConfig {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().enable_prompts().enable_resources().build())
            .with_instructions(
                "macmetry reads the performance of the Mac it runs on: CPU, memory, disk, network, GPU, temperatures, fans, \
                 power, battery and processes. Every tool only reads; nothing here changes the Mac. Start with system_info \
                 to see what this Mac can report, then snapshot (use `sections` to save tokens). For trends use \
                 history_query; to follow something while the person does it, use watch_start and watch_read. \
                 Read the resource macmetry://guide for the tools by question, and the prompts such as why_is_my_mac_slow \
                 for a ready plan.",
            )
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(crate::prompts::list()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, McpError> {
        crate::prompts::get(&request.name, request.arguments.as_ref())
            .map(Into::into)
            .map_err(|e| McpError::invalid_params(e, None))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(crate::resources::list())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(crate::resources::templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        self.read_uri(&request.uri).await.map(Into::into).map_err(|e| McpError::invalid_params(e, None))
    }
}
