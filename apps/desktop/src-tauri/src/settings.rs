//! What the person has chosen: language, theme, refresh rate, the menu bar, history and alerts. Saved as one JSON
//! file in the data folder, written atomically so a crash cannot leave half a file.

use macmetry_core::alerts::{Rule, presets};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The readings the menu bar title can show, in the order they are offered.
pub const MENU_ITEMS: [&str; 6] = ["cpu", "memory", "temp", "power", "network", "gpu"];
const MAX_MENU_ITEMS: usize = 4;
const MAX_RULES: usize = 50;
pub const MIN_INTERVAL_MS: u32 = 500;
pub const MAX_INTERVAL_MS: u32 = 5_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `system`, `en` or `pt-BR`.
    pub language: String,
    /// `system`, `dark` or `light`.
    pub theme: String,
    /// Milliseconds between readings.
    pub interval_ms: u32,
    /// Keep a history in the database.
    pub record_history: bool,
    /// Show the menu bar item.
    pub menu_bar_enabled: bool,
    /// What the menu bar title shows (see [`MENU_ITEMS`]).
    pub menu_bar_items: Vec<String>,
    /// Hide the Dock icon and live only in the menu bar.
    pub hide_dock_icon: bool,
    /// Closing the window keeps macmetry running (recording and alerting) in the menu bar.
    pub keep_running: bool,
    /// Show system notifications for alerts.
    pub notifications: bool,
    pub alerts: Vec<Rule>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            theme: "system".into(),
            interval_ms: 1_000,
            record_history: true,
            menu_bar_enabled: true,
            menu_bar_items: vec!["cpu".into(), "temp".into()],
            hide_dock_icon: false,
            keep_running: true,
            notifications: true,
            alerts: presets(),
        }
    }
}

impl Settings {
    /// Checks settings that came from outside (the interface, or a file edited by hand) and fixes what can be fixed.
    /// What cannot is an error that says what to change.
    pub fn validated(mut self) -> Result<Self, String> {
        if !matches!(self.language.as_str(), "system" | "en" | "pt-BR") {
            return Err(format!("`{}` is not a language. Use system, en or pt-BR.", self.language));
        }
        if !matches!(self.theme.as_str(), "system" | "dark" | "light") {
            return Err(format!("`{}` is not a theme. Use system, dark or light.", self.theme));
        }
        // With no Dock icon and no menu bar item there would be no way to reach the app at all.
        if self.hide_dock_icon && !self.menu_bar_enabled {
            return Err(
                "The Dock icon can only be hidden while the menu bar item is shown, or macmetry could not be opened."
                    .into(),
            );
        }
        self.interval_ms = self.interval_ms.clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS);
        if let Some(bad) = self.menu_bar_items.iter().find(|i| !MENU_ITEMS.contains(&i.as_str())) {
            return Err(format!("`{bad}` cannot be shown in the menu bar. Choose from: {}.", MENU_ITEMS.join(", ")));
        }
        let mut seen = std::collections::HashSet::new();
        self.menu_bar_items.retain(|i| seen.insert(i.clone()));
        self.menu_bar_items.truncate(MAX_MENU_ITEMS);
        if self.alerts.len() > MAX_RULES {
            return Err(format!("There can be at most {MAX_RULES} alert rules."));
        }
        let mut ids = std::collections::HashSet::new();
        for rule in &self.alerts {
            rule.validate()?;
            if !ids.insert(rule.id.clone()) {
                return Err(format!("Two alert rules share the id `{}`.", rule.id));
            }
        }
        Ok(self)
    }
}

/// Reads the settings. A missing or unreadable file gives the defaults, so a damaged file never keeps the app from
/// starting; the next save replaces it.
pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .and_then(|s| s.validated().ok())
        .unwrap_or_default()
}

/// Writes the settings to a temporary file and renames it over the real one.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macmetry_core::Metric;

    #[test]
    fn defaults_are_valid_and_safe() {
        let s = Settings::default().validated().unwrap();
        assert_eq!(s.interval_ms, 1_000);
        assert!(s.keep_running && s.record_history);
        assert_eq!(s.alerts.iter().filter(|r| r.enabled).count(), 2, "only the protective alerts start on");
    }

    #[test]
    fn rejects_unknown_choices_with_a_reason() {
        assert!(
            Settings { language: "fr".into(), ..Default::default() }
                .validated()
                .unwrap_err()
                .contains("not a language")
        );
        assert!(
            Settings { theme: "neon".into(), ..Default::default() }.validated().unwrap_err().contains("not a theme")
        );
        assert!(
            Settings { menu_bar_items: vec!["fans".into()], ..Default::default() }
                .validated()
                .unwrap_err()
                .contains("cpu, memory")
        );
    }

    #[test]
    fn the_app_always_stays_reachable() {
        let hidden = Settings { hide_dock_icon: true, menu_bar_enabled: false, ..Default::default() };
        assert!(hidden.validated().unwrap_err().contains("menu bar"));
        assert!(Settings { hide_dock_icon: true, menu_bar_enabled: true, ..Default::default() }.validated().is_ok());
        assert!(
            Settings { hide_dock_icon: false, menu_bar_enabled: false, ..Default::default() }.validated().is_ok(),
            "the Dock icon is then the way in"
        );
    }

    #[test]
    fn clamps_the_interval_and_trims_the_menu_bar() {
        assert_eq!(Settings { interval_ms: 1, ..Default::default() }.validated().unwrap().interval_ms, MIN_INTERVAL_MS);
        assert_eq!(
            Settings { interval_ms: 999_999, ..Default::default() }.validated().unwrap().interval_ms,
            MAX_INTERVAL_MS
        );
        let many = ["cpu", "cpu", "memory", "temp", "power", "network", "gpu"].map(String::from).to_vec();
        let s = Settings { menu_bar_items: many, ..Default::default() }.validated().unwrap();
        assert_eq!(s.menu_bar_items, ["cpu", "memory", "temp", "power"], "duplicates dropped, at most four");
    }

    #[test]
    fn rejects_bad_alert_rules() {
        let mut rule = presets().remove(0);
        rule.threshold = f64::NAN;
        assert!(Settings { alerts: vec![rule], ..Default::default() }.validated().is_err());
        let a = presets().remove(0);
        assert!(
            Settings { alerts: vec![a.clone(), a], ..Default::default() }
                .validated()
                .unwrap_err()
                .contains("share the id")
        );
        let flood: Vec<Rule> =
            (0..60).map(|i| Rule { id: format!("r{i}"), metric: Metric::CpuTotal, ..presets().remove(0) }).collect();
        assert!(Settings { alerts: flood, ..Default::default() }.validated().unwrap_err().contains("at most"));
    }

    #[test]
    fn saves_and_loads_what_was_chosen() {
        let dir = tempfile_dir();
        let path = dir.join("settings.json");
        let chosen = Settings {
            language: "pt-BR".into(),
            theme: "dark".into(),
            interval_ms: 2_000,
            hide_dock_icon: true,
            ..Default::default()
        };
        save(&path, &chosen).unwrap();
        assert_eq!(load(&path), chosen);
        assert!(!path.with_extension("json.tmp").exists(), "no temporary file is left behind");
    }

    #[test]
    fn a_missing_or_damaged_file_gives_the_defaults() {
        let dir = tempfile_dir();
        assert_eq!(load(&dir.join("nope.json")), Settings::default());
        let broken = dir.join("broken.json");
        std::fs::write(&broken, "{ not json").unwrap();
        assert_eq!(load(&broken), Settings::default());
        let invalid = dir.join("invalid.json");
        std::fs::write(&invalid, r#"{ "language": "klingon" }"#).unwrap();
        assert_eq!(load(&invalid), Settings::default(), "settings that fail validation are not used");
    }

    #[test]
    fn a_file_from_an_older_version_gets_the_new_fields() {
        let dir = tempfile_dir();
        let old = dir.join("old.json");
        std::fs::write(&old, r#"{ "theme": "light", "interval_ms": 2000 }"#).unwrap();
        let s = load(&old);
        assert_eq!((s.theme.as_str(), s.interval_ms), ("light", 2000));
        assert!(s.keep_running && !s.alerts.is_empty(), "fields the file lacks take their defaults");
    }

    /// A folder that is removed when the test ends (the desktop crate has no `tempfile` dependency to keep it small).
    fn tempfile_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "macmetry-settings-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
