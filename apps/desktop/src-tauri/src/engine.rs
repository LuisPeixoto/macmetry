//! The sampling thread: measures every interval, keeps the live buffer, records the history, checks the alerts, tells
//! the windows and updates the menu bar title.

use crate::state::{LivePoint, Shared};
use crate::title::menu_bar_title;
use macmetry_core::alerts::{EventKind, Rule};
use macmetry_sys::sampler::{SampleOptions, Sampler};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// How often the history is tidied (aggregated and pruned).
const MAINTENANCE_MS: u64 = 60_000;

/// What the interface hears when an alert fires or clears.
#[derive(Debug, Clone, Serialize)]
pub struct AlertPayload {
    pub rule_id: String,
    pub name: String,
    pub kind: &'static str,
    pub message: String,
    pub value: f64,
    pub at_ms: u64,
}

fn any_window_visible(app: &AppHandle) -> bool {
    app.webview_windows().values().any(|w| w.is_visible().unwrap_or(false))
}

pub fn spawn(app: AppHandle, shared: Arc<Shared>, mut sampler: Sampler) {
    let spawned = std::thread::Builder::new().name("macmetry-sampler".into()).spawn(move || {
        let (mut tick, mut last_maintenance, mut last_title) = (0u64, 0u64, String::new());
        let mut next = Instant::now() + Duration::from_millis(u64::from(shared.settings.read().interval_ms));
        while !shared.stop.load(Ordering::Relaxed) {
            // Wait in short steps so quitting is never held up by a long interval.
            while Instant::now() < next {
                if shared.stop.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep((next - Instant::now()).min(Duration::from_millis(50)));
            }
            let (interval, record, menu_bar, items, notifications) = {
                let s = shared.settings.read();
                (s.interval_ms, s.record_history, s.menu_bar_enabled, s.menu_bar_items.clone(), s.notifications)
            };
            let visible = any_window_visible(&app);
            // Processes cost the most to read, so they are read every other tick, and only when someone can see them.
            let snapshot =
                sampler.sample(SampleOptions { processes: visible && tick.is_multiple_of(2), ..Default::default() });
            tick += 1;

            shared.live.lock().push(LivePoint::from_snapshot(&snapshot));
            *shared.latest.write() = Some(snapshot.clone());

            if record && let Some(store) = shared.store.lock().as_ref() {
                if let Err(e) = store.record(&snapshot) {
                    tracing::warn!("Could not record: {e}");
                }
                if snapshot.timestamp_ms.saturating_sub(last_maintenance) >= MAINTENANCE_MS {
                    last_maintenance = snapshot.timestamp_ms;
                    if let Err(e) = store.maintain(snapshot.timestamp_ms) {
                        tracing::warn!("Could not tidy the history: {e}");
                    }
                }
            }

            let events = shared.alerts.lock().evaluate(&snapshot);
            for event in events {
                let rules: Vec<Rule> = shared.alerts.lock().rules().to_vec();
                let Some(rule) = rules.iter().find(|r| r.id == event.rule_id) else { continue };
                let payload = AlertPayload {
                    rule_id: rule.id.clone(),
                    name: rule.name.clone(),
                    kind: if event.kind == EventKind::Fired { "fired" } else { "resolved" },
                    message: rule.message(event.value),
                    value: event.value,
                    at_ms: event.at_ms,
                };
                let _ = app.emit("alert", &payload);
                if notifications && event.kind == EventKind::Fired {
                    let shown = app.notification().builder().title(&rule.name).body(&payload.message).show();
                    if let Err(e) = shown {
                        tracing::warn!("Could not show a notification: {e}");
                    }
                }
            }

            if visible {
                let _ = app.emit("snapshot", &snapshot);
            }
            if let Some(tray) = app.tray_by_id("main") {
                let title = if menu_bar { menu_bar_title(&snapshot, &items) } else { String::new() };
                if title != last_title {
                    let _ = tray.set_title(if title.is_empty() { None } else { Some(title.as_str()) });
                    last_title = title;
                }
            }
            next += Duration::from_millis(u64::from(interval));
            // After a long pause (sleep, a stalled disk) do not try to catch up with a burst of readings.
            next = next.max(Instant::now());
        }
    });
    if let Err(e) = spawned {
        tracing::error!("Could not start the sampler: {e}");
    }
}
