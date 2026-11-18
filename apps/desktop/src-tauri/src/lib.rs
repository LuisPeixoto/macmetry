//! The macmetry desktop shell: it owns the sampler, the history and the alerts, and gives the interface a handful of
//! commands and two events (`snapshot` and `alert`).

mod commands;
mod engine;
mod settings;
mod state;
mod title;
mod tray;
mod window;

use crate::state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};
use window_vibrancy::NSVisualEffectMaterial;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_env("MACMETRY_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let (state, sampler) = AppState::new();
    let shared = state.shared.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::latest,
            commands::recent,
            commands::query_history,
            commands::history_stats,
            commands::clear_history,
            commands::export_history,
            commands::get_settings,
            commands::set_settings,
            commands::alert_status,
            commands::show_main_window,
            commands::quit_app,
        ])
        .setup(move |app| {
            let settings = shared.settings.read().clone();
            if let Some(main) = app.get_webview_window("main") {
                window::apply_glass(&main, NSVisualEffectMaterial::Sidebar, None);
            }
            tray::setup(app.handle(), &settings.language)?;
            window::apply_settings(app.handle(), &settings);
            engine::spawn(app.handle().clone(), shared.clone(), sampler);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides it: macmetry keeps recording and alerting from the menu bar.
            if let (WindowEvent::CloseRequested { api, .. }, "main") = (event, window.label()) {
                let keep = window.app_handle().state::<AppState>().shared.settings.read().keep_running;
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("macmetry could not start");

    app.run(|app, event| match event {
        // Clicking the Dock icon brings the window back.
        RunEvent::Reopen { .. } => window::show_main(app),
        RunEvent::Exit => app.state::<AppState>().shared.stop.store(true, std::sync::atomic::Ordering::Relaxed),
        _ => {}
    });
}
