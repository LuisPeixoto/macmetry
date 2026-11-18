//! What the interface can ask for. Every command returns a plain `Result<T, String>` whose error is a sentence a
//! person can read.

use crate::settings::{self, Settings};
use crate::state::{AppState, LivePoint};
use macmetry_core::series::{to_csv, to_json};
use macmetry_core::{Metric, Series, Snapshot, SystemInfo, Unit};
use macmetry_sys::sampler::Capabilities;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
pub struct MetricInfo {
    key: &'static str,
    label: &'static str,
    unit: Unit,
}

#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    system: SystemInfo,
    capabilities: Capabilities,
    metrics: Vec<MetricInfo>,
    data_dir: String,
}

#[derive(Serialize)]
pub struct HistoryStats {
    raw_rows: u64,
    minute_rows: u64,
    oldest_ms: Option<u64>,
    newest_ms: Option<u64>,
    file_bytes: u64,
}

#[tauri::command]
pub fn app_info(state: State<AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        system: state.shared.system.clone(),
        capabilities: state.shared.capabilities,
        metrics: Metric::ALL.iter().map(|m| MetricInfo { key: m.key(), label: m.label(), unit: m.unit() }).collect(),
        data_dir: macmetry_store::data_dir().display().to_string(),
    }
}

#[tauri::command]
pub fn latest(state: State<AppState>) -> Option<Snapshot> {
    state.shared.latest.read().clone()
}

/// The last `count` live points (at most the buffer holds), oldest first.
#[tauri::command]
pub fn recent(state: State<AppState>, count: usize) -> Vec<LivePoint> {
    let live = state.shared.live.lock();
    let skip = live.len().saturating_sub(count.min(crate::state::LIVE_CAPACITY));
    live.iter().skip(skip).cloned().collect()
}

fn parse_metrics(keys: &[String]) -> Result<Vec<Metric>, String> {
    if keys.is_empty() || keys.len() > 12 {
        return Err("Choose between 1 and 12 metrics.".into());
    }
    keys.iter().map(|k| Metric::from_key(k).ok_or_else(|| format!("`{k}` is not a metric."))).collect()
}

fn query_all(
    state: &AppState,
    metrics: &[Metric],
    from_ms: u64,
    to_ms: u64,
    max_points: usize,
) -> Result<Vec<Series>, String> {
    let guard = state.shared.store.lock();
    let store = guard.as_ref().ok_or("The history is not available.")?;
    metrics.iter().map(|&m| store.query(m, from_ms, to_ms, max_points).map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub fn query_history(
    state: State<AppState>,
    metrics: Vec<String>,
    from_ms: u64,
    to_ms: u64,
    max_points: usize,
) -> Result<Vec<Series>, String> {
    query_all(&state, &parse_metrics(&metrics)?, from_ms, to_ms, max_points)
}

#[tauri::command]
pub fn history_stats(state: State<AppState>) -> Result<HistoryStats, String> {
    let guard = state.shared.store.lock();
    let stats = guard.as_ref().ok_or("The history is not available.")?.stats().map_err(|e| e.to_string())?;
    Ok(HistoryStats {
        raw_rows: stats.raw_rows,
        minute_rows: stats.minute_rows,
        oldest_ms: stats.oldest_ms,
        newest_ms: stats.newest_ms,
        file_bytes: stats.file_bytes,
    })
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) -> Result<(), String> {
    let guard = state.shared.store.lock();
    guard.as_ref().ok_or("The history is not available.")?.clear().map_err(|e| e.to_string())
}

/// Asks where to save, then writes the file. Returns the path, or nothing when the person cancelled. The file is
/// only written after that explicit choice.
#[tauri::command]
pub async fn export_history(
    app: AppHandle,
    state: State<'_, AppState>,
    metrics: Vec<String>,
    from_ms: u64,
    to_ms: u64,
    format: String,
    include_range: bool,
) -> Result<Option<String>, String> {
    let metrics = parse_metrics(&metrics)?;
    let (extension, label) = match format.as_str() {
        "csv" => ("csv", "CSV"),
        "json" => ("json", "JSON"),
        other => return Err(format!("`{other}` is not a format. Use csv or json.")),
    };
    let series = query_all(&state, &metrics, from_ms, to_ms, 10_000)?;
    let text = if extension == "csv" {
        to_csv(&series, include_range)
    } else {
        serde_json::to_string_pretty(&to_json(&series)).map_err(|e| e.to_string())? + "\n"
    };
    let chosen = app
        .dialog()
        .file()
        .add_filter(label, &[extension])
        .set_file_name(format!("macmetry-history.{extension}"))
        .blocking_save_file();
    let Some(path) = chosen else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.shared.settings.read().clone()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let settings = settings.validated()?;
    settings::save(&state.shared.settings_path, &settings).map_err(|e| format!("Could not save the settings: {e}"))?;
    state.shared.alerts.lock().set_rules(settings.alerts.clone());
    *state.shared.settings.write() = settings.clone();
    crate::window::apply_settings(&app, &settings);
    Ok(settings)
}

/// The rules that are firing right now.
#[tauri::command]
pub fn alert_status(state: State<AppState>) -> Vec<String> {
    state.shared.alerts.lock().firing().into_iter().map(str::to_string).collect()
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    crate::window::show_main(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.state::<AppState>().shared.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    app.exit(0);
}
