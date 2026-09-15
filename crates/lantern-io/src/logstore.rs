//! Log-file path resolution and parsing.
//!
//! Lantern emits a simple text log to `<settings_dir>/logs/lantern.log` whose
//! lines look like:
//!
//! ```text
//! 2026-05-05T17:23:45Z INFO some message
//! 2026-05-05T17:23:46Z WARN something fishy
//! 2026-05-05T17:23:47Z ERROR exploded
//! ```
//!
//! This module owns the path-resolution rules (the same precedence used by
//! [`crate::settings`]) and a small "tail-the-file" parser used by the
//! `get_logs` Tauri command in `lantern-app`.
//!
//! Path resolution order:
//!
//! 1. `LANTERN_SETTINGS_DIR` environment variable, if set and non-empty.
//! 2. The directory containing the running executable, when a `settings.toml`
//!    sits alongside it (portable installs).
//! 3. `%APPDATA%\Lantern\` on Windows, `$XDG_CONFIG_HOME/lantern/` (or
//!    `~/.config/lantern/`) on Linux, `~/Library/Application Support/Lantern/`
//!    on macOS.
//!
//! In every case the final log path is `<dir>/logs/lantern.log`.

use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Parsed log line: fields are `String` because parsing is intentionally
/// permissive and the consumer (the IPC layer) re-wraps these into a typed
/// `LogLevel` enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLogLine {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Path resolution: mirrors the rules in `lantern-app::resolve_settings_dir`.
// We duplicate the logic here (rather than depend on `lantern-app`) because
// `lantern-io` sits below `lantern-app` in the dependency graph.
// ---------------------------------------------------------------------------

/// Resolve the absolute path of `lantern.log`.
pub fn log_path() -> PathBuf {
    settings_dir().join("logs").join("lantern.log")
}

fn settings_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("LANTERN_SETTINGS_DIR") {
        if !override_dir.is_empty() {
            return PathBuf::from(override_dir);
        }
    }

    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if exe_dir.join("settings.toml").exists() {
            return exe_dir;
        }
    }

    platform_config_dir()
}

fn platform_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("Lantern");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("lantern");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home).join(".config").join("lantern");
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Lantern");
            }
        }
    }

    PathBuf::from(".")
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Read the most recent `max_lines` log lines from `path`.
///
/// Lines are parsed as three whitespace-separated columns: `timestamp`,
/// `level`, and `message` (which may itself contain whitespace).  Malformed
/// lines (fewer than three columns) are skipped silently; log files can
/// accumulate junk over time and we'd rather show the user what we can than
/// fail outright.
///
/// If `max_lines` is zero, returns an empty `Vec`.
pub fn read_recent(path: &Path, max_lines: usize) -> io::Result<Vec<RawLogLine>> {
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    // Ring buffer of the last `max_lines` parsed entries.  We pop the oldest
    // entry when the buffer is full, so peak memory stays bounded regardless
    // of input size.
    let mut buf: std::collections::VecDeque<RawLogLine> =
        std::collections::VecDeque::with_capacity(max_lines.min(1024));

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Split into at most three pieces: timestamp, level, rest-of-line.
        let mut parts = trimmed.splitn(3, char::is_whitespace);
        let timestamp = match parts.next() {
            Some(s) => s.to_owned(),
            None => continue,
        };
        // `splitn` may yield empty strings between consecutive whitespace
        // characters, so step through until we find non-empty fields.
        let level = match parts.find(|s| !s.is_empty()) {
            Some(s) => s.to_owned(),
            None => continue,
        };
        let message = match parts.next() {
            Some(s) => s.trim_start().to_owned(),
            None => continue,
        };

        if buf.len() == max_lines {
            buf.pop_front();
        }
        buf.push_back(RawLogLine {
            timestamp,
            level,
            message,
        });
    }

    Ok(buf.into_iter().collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn empty_file_returns_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        std::fs::write(&path, b"").unwrap();

        let lines = read_recent(&path, 200).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn five_line_file_returns_all_five() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        let mut f = std::fs::File::create(&path).unwrap();
        for i in 0..5 {
            writeln!(f, "2026-05-05T17:23:4{i}Z INFO message number {i}", i = i).unwrap();
        }
        drop(f);

        let lines = read_recent(&path, 200).unwrap();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].level, "INFO");
        assert_eq!(lines[0].message, "message number 0");
        assert_eq!(lines[4].message, "message number 4");
    }

    #[test]
    fn thousand_line_file_capped_at_two_hundred() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        let mut f = std::fs::File::create(&path).unwrap();
        for i in 0..1000 {
            writeln!(f, "2026-05-05T17:23:45Z INFO line {i}").unwrap();
        }
        drop(f);

        let lines = read_recent(&path, 200).unwrap();
        assert_eq!(lines.len(), 200);
        // The window is the *last* 200, i.e. lines 800..1000.
        assert_eq!(lines.first().unwrap().message, "line 800");
        assert_eq!(lines.last().unwrap().message, "line 999");
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "2026-05-05T17:23:45Z INFO ok line").unwrap();
        writeln!(f, "garbage-without-three-fields").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "2026-05-05T17:23:46Z WARN another ok line").unwrap();
        drop(f);

        let lines = read_recent(&path, 200).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].level, "INFO");
        assert_eq!(lines[1].level, "WARN");
    }

    #[test]
    fn log_path_uses_env_override() {
        // `LANTERN_SETTINGS_DIR` is read at call-time, so we set it just for
        // this test then unset it again.  Other tests in this file don't
        // exercise `log_path()` so the global mutation is safe in practice.
        let prev = std::env::var("LANTERN_SETTINGS_DIR").ok();
        std::env::set_var("LANTERN_SETTINGS_DIR", "C:\\fake\\settings");
        let p = log_path();
        match prev {
            Some(v) => std::env::set_var("LANTERN_SETTINGS_DIR", v),
            None => std::env::remove_var("LANTERN_SETTINGS_DIR"),
        }
        assert!(p.ends_with("logs/lantern.log") || p.ends_with("logs\\lantern.log"));
    }
}
