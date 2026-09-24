//! Application settings: the [`Settings`] struct, its TOML persistence, and
//! the settings-directory resolution rules.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

/// User-facing application preferences.
///
/// Every field has a default, so a missing, empty, or partial
/// `settings.toml` loads cleanly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// UI colour theme.
    pub theme: Theme,
    /// Default directory for the "Export" file picker (`None` = last used).
    pub default_export_location: Option<PathBuf>,
    /// Whether the user has opted into the dead-link checker.  Off by default:
    /// it is the only feature that touches the network.
    pub dead_link_checker_opt_in: bool,
    /// Maximum number of entries in the Recent Files list.
    pub recent_files_max: usize,
    /// Recently opened bookmark files (most-recent-last).
    pub recent_files: Vec<PathBuf>,
    /// Whether crash recovery is enabled for this installation.
    pub crash_recovery_enabled: bool,
    /// Document paths captured while the previous session was running.
    pub recoverable_documents: Vec<PathBuf>,
    /// True while Lantern is running; left set after a crash.
    pub session_was_running: bool,
    /// List-row density for the bookmark list pane.
    pub list_density: ListDensity,
}

/// Vertical density for the list pane rows.
///
/// `Compact` (~28 px rows) fits the most bookmarks per screen;
/// `Comfortable` (~36 px) trades density for larger targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListDensity {
    #[default]
    Compact,
    Comfortable,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            default_export_location: None,
            dead_link_checker_opt_in: false,
            recent_files_max: 10,
            recent_files: Vec::new(),
            crash_recovery_enabled: true,
            recoverable_documents: Vec::new(),
            session_was_running: false,
            list_density: ListDensity::default(),
        }
    }
}

/// UI colour theme.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow the OS light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

/// Read settings from `path`.
///
/// An empty file yields `Settings::default()`; only I/O failures and TOML
/// syntax errors are reported.
pub fn read_settings(path: &Path) -> Result<Settings> {
    let text = std::fs::read_to_string(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;
    if text.trim().is_empty() {
        return Ok(Settings::default());
    }
    toml::from_str(&text).map_err(|e| IoError::TomlDe {
        path: path.to_owned(),
        reason: e.to_string(),
    })
}

/// Serialise `settings` to TOML and write it to `path` atomically.
pub fn write_settings(path: &Path, settings: &Settings) -> Result<()> {
    let text = toml::to_string_pretty(settings).map_err(|e| IoError::TomlSer(e.to_string()))?;
    write_atomic(path, text.as_bytes())
}

/// Directory holding `settings.toml`, the rule store, and the log.
///
/// Resolution order:
///
/// 1. `LANTERN_SETTINGS_DIR`, if set and non-empty (tests, portable zip).
/// 2. The executable's directory, when a `settings.toml` sits next to it
///    (portable install).
/// 3. The platform config directory: `%APPDATA%\Lantern` on Windows,
///    `$XDG_CONFIG_HOME/lantern` (or `~/.config/lantern`) on Linux,
///    `~/Library/Application Support/Lantern` on macOS.  Falls back to `.`.
pub fn settings_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("LANTERN_SETTINGS_DIR") {
        return dir;
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        if exe_dir.join("settings.toml").exists() {
            return exe_dir;
        }
    }
    platform_config_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn platform_config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        non_empty_env("APPDATA").map(|d| d.join("Lantern"))
    } else if cfg!(target_os = "macos") {
        non_empty_env("HOME").map(|h| h.join("Library/Application Support/Lantern"))
    } else if cfg!(target_os = "linux") {
        non_empty_env("XDG_CONFIG_HOME")
            .map(|d| d.join("lantern"))
            .or_else(|| non_empty_env("HOME").map(|h| h.join(".config/lantern")))
    } else {
        None
    }
}

fn non_empty_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let custom = Settings {
            theme: Theme::Dark,
            default_export_location: Some(PathBuf::from(r"C:\Users\test\exports")),
            dead_link_checker_opt_in: true,
            recent_files_max: 20,
            recent_files: vec![
                PathBuf::from(r"C:\Users\test\Bookmarks\one.html"),
                PathBuf::from(r"C:\Users\test\Bookmarks\two.html"),
            ],
            crash_recovery_enabled: false,
            recoverable_documents: vec![PathBuf::from(r"C:\Users\test\Bookmarks\recover.html")],
            session_was_running: true,
            list_density: ListDensity::Comfortable,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");

        for original in [Settings::default(), custom] {
            write_settings(&path, &original).unwrap();
            assert_eq!(read_settings(&path).unwrap(), original);
        }
    }

    #[test]
    fn empty_and_partial_files_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");

        std::fs::write(&path, "").unwrap();
        assert_eq!(read_settings(&path).unwrap(), Settings::default());

        std::fs::write(&path, "theme = \"light\"\n").unwrap();
        assert_eq!(
            read_settings(&path).unwrap(),
            Settings {
                theme: Theme::Light,
                ..Settings::default()
            }
        );
    }
}
