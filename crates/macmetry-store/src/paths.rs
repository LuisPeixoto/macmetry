//! Where macmetry keeps its data.

use std::path::PathBuf;

/// The environment variable that moves everything macmetry writes, so tests and demos never touch real data.
pub const HOME_ENV: &str = "MACMETRY_HOME";

/// The folder for macmetry's own files: `$MACMETRY_HOME`, or `~/Library/Application Support/macmetry`.
pub fn data_dir() -> PathBuf {
    if let Some(home) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
        return PathBuf::from(home);
    }
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("macmetry")
}

/// The history database.
pub fn history_path() -> PathBuf {
    data_dir().join("history.sqlite")
}
