//! The menu bar item and the popover that opens under it.

use crate::window::{apply_glass, show_main};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use window_vibrancy::NSVisualEffectMaterial;

const POPOVER_WIDTH: f64 = 340.0;
const POPOVER_HEIGHT: f64 = 560.0;
/// A click on the icon while the popover is open first takes focus away from it (which hides it) and then arrives
/// as a click; this window of time keeps that click from opening it again.
const REOPEN_GUARD_MS: u64 = 250;

static LAST_HIDDEN_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// The labels of the menu, in the chosen language.
fn labels(language: &str) -> (&'static str, &'static str) {
    if language == "pt-BR" { ("Abrir macmetry", "Sair do macmetry") } else { ("Open macmetry", "Quit macmetry") }
}

fn build_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (open, quit) = labels(language);
    let open = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, Some("Cmd+Q"))?;
    Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])
}

/// Switches the menu to another language.
pub fn set_language(app: &AppHandle, language: &str) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), build_menu(app, language)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn setup(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let popover = WebviewWindowBuilder::new(app, "popover", WebviewUrl::App("popover.html".into()))
        .title("macmetry")
        .inner_size(POPOVER_WIDTH, POPOVER_HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    apply_glass(&popover, NSVisualEffectMaterial::HudWindow, Some(14.0));
    let hiding = popover.clone();
    popover.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            LAST_HIDDEN_MS.store(now_ms(), Ordering::Relaxed);
            let _ = hiding.hide();
        }
    });

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .menu(&build_menu(app, language)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left, button_state: MouseButtonState::Up, rect, ..
            } = event
            {
                toggle_popover(tray.app_handle(), rect);
            }
        })
        .build(app)?;
    Ok(())
}

/// The left edge that puts a popover of `width` centred under an icon, kept inside the screen.
pub fn popover_left(icon_left: f64, icon_width: f64, width: f64, screen_left: f64, screen_width: f64) -> f64 {
    let centred = icon_left + icon_width / 2.0 - width / 2.0;
    let margin = 8.0;
    centred.clamp(screen_left + margin, (screen_left + screen_width - width - margin).max(screen_left + margin))
}

fn toggle_popover(app: &AppHandle, rect: Rect) {
    let Some(popover) = app.get_webview_window("popover") else { return };
    if popover.is_visible().unwrap_or(false) {
        let _ = popover.hide();
        return;
    }
    if now_ms().saturating_sub(LAST_HIDDEN_MS.load(Ordering::Relaxed)) < REOPEN_GUARD_MS {
        return;
    }
    let scale = popover.scale_factor().unwrap_or(2.0);
    let (icon, size) = (rect.position.to_physical::<f64>(scale), rect.size.to_physical::<f64>(scale));
    let width = POPOVER_WIDTH * scale;
    let (screen_left, screen_width) = app
        .monitor_from_point(icon.x, icon.y)
        .ok()
        .flatten()
        .map(|m| (f64::from(m.position().x), f64::from(m.size().width)))
        .unwrap_or((0.0, f64::MAX / 4.0));
    let x = popover_left(icon.x, size.width, width, screen_left, screen_width);
    let y = icon.y + size.height + 6.0 * scale;
    let _ = popover.set_position(PhysicalPosition::new(x, y));
    let _ = popover.show();
    let _ = popover.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centres_the_popover_under_the_icon() {
        assert_eq!(popover_left(1000.0, 40.0, 340.0, 0.0, 1920.0), 850.0);
    }

    #[test]
    fn keeps_it_on_the_screen() {
        // An icon near the right edge: the popover would hang off it, so it stops 8 points short.
        assert_eq!(popover_left(1900.0, 20.0, 340.0, 0.0, 1920.0), 1920.0 - 340.0 - 8.0);
        // An icon at the left edge of a second screen that starts at 1920.
        assert_eq!(popover_left(1925.0, 20.0, 340.0, 1920.0, 1920.0), 1928.0);
    }

    #[test]
    fn a_screen_narrower_than_the_popover_does_not_panic() {
        assert_eq!(popover_left(50.0, 10.0, 340.0, 0.0, 200.0), 8.0);
    }

    #[test]
    fn the_menu_speaks_the_chosen_language() {
        assert_eq!(labels("pt-BR"), ("Abrir macmetry", "Sair do macmetry"));
        assert_eq!(labels("en"), ("Open macmetry", "Quit macmetry"));
        assert_eq!(labels("system"), ("Open macmetry", "Quit macmetry"));
    }
}
