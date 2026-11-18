//! How the windows look and behave: the frosted glass, the theme, the Dock icon and showing the main window.

use crate::settings::Settings;
use tauri::{ActivationPolicy, AppHandle, Manager, Theme, WebviewWindow};
use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

/// Gives a window the macOS frosted-glass background. The page must be transparent where the glass should show.
pub fn apply_glass(window: &WebviewWindow, material: NSVisualEffectMaterial, radius: Option<f64>) {
    if let Err(e) = apply_vibrancy(window, material, Some(NSVisualEffectState::Active), radius) {
        tracing::warn!("Could not apply the frosted glass: {e}");
    }
}

/// Applies the settings that change how the app is presented. Safe to call again whenever they change.
pub fn apply_settings(app: &AppHandle, settings: &Settings) {
    let theme = match settings.theme.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    };
    for window in app.webview_windows().values() {
        let _ = window.set_theme(theme);
    }
    let policy = if settings.hide_dock_icon { ActivationPolicy::Accessory } else { ActivationPolicy::Regular };
    let _ = app.set_activation_policy(policy);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_visible(settings.menu_bar_enabled);
    }
    crate::tray::set_language(app, &settings.language);
}

/// Brings the main window forward, creating nothing: it always exists, hidden or not.
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(popover) = app.get_webview_window("popover") {
        let _ = popover.hide();
    }
}
