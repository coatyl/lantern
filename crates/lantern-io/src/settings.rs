//! Application settings: the [`Settings`] struct and its TOML persistence.
//!
//! Settings are stored at `%APPDATA%\Lantern\settings.toml` for installed
//! builds, or alongside the executable for portable builds (PRD F-SET-1).
//! The choice of path is the caller's responsibility; this module only reads
//! and writes whatever path it is given.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// User-facing application preferences (PRD F-SET-2).
///
/// All fields have sane defaults so a missing or empty `settings.toml` is
/// handled gracefully.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// UI colour theme.
    pub theme: Theme,
    /// Default directory for the "Export" file picker (`None` = last used).
    pub default_export_location: Option<PathBuf>,
    /// Whether the user has opted into the dead-link checker for this
    /// installation.  Defaults to `false` (PRD NFR-PR-1).
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

/// Vertical density for the list pane rows (PRD F-UX-12).
///
/// `Compact` is the historical default: ~28 px tall rows that fit the most
/// bookmarks per screen.  `Comfortable` adds breathing room (~36 px) for users
/// who prefer larger touch targets or simply find tight rows fatiguing.
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

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Read settings from `path`.
///
/// Returns `Ok(Settings::default())` for an empty file; returns an error only
/// for I/O failures or TOML syntax errors.
pub fn read_settings(path: &Path) -> Result<Settings> {
    let text = std::fs::read_to_string(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;

    // An empty file is fine; just use defaults.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");

        let original = Settings::default();
        write_settings(&path, &original).unwrap();
        let loaded = read_settings(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn custom_settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");

        let original = Settings {
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

        write_settings(&path, &original).unwrap();
        let loaded = read_settings(&path).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn empty_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, b"").unwrap();

        let settings = read_settings(&path).unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, b"theme = \"light\"\n").unwrap();

        let settings = read_settings(&path).unwrap();
        assert_eq!(settings.theme, Theme::Light);
        assert!(!settings.dead_link_checker_opt_in); // default
        assert_eq!(settings.recent_files_max, 10); // default
        assert!(settings.recent_files.is_empty());
        assert!(settings.crash_recovery_enabled);
        assert!(settings.recoverable_documents.is_empty());
        assert!(!settings.session_was_running);
    }
}
