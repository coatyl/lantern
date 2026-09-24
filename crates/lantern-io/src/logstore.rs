//! Log-file location and tail parsing.
//!
//! The log lives at `<settings_dir>/logs/lantern.log` (see
//! [`crate::settings_dir`]) and holds one entry per line:
//!
//! ```text
//! 2026-05-05T17:23:45Z INFO some message
//! 2026-05-05T17:23:46Z WARN something fishy
//! 2026-05-05T17:23:47Z ERROR exploded
//! ```

use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

/// One parsed log line.  Fields stay strings because parsing is deliberately
/// permissive; the IPC layer maps `level` onto its own typed enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLogLine {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Absolute path of `lantern.log`.
pub fn log_path() -> PathBuf {
    crate::settings_dir().join("logs").join("lantern.log")
}

/// Read the most recent `max_lines` entries from `path`.
///
/// Each line is split into `timestamp`, `level`, and `message` (the rest of
/// the line).  Lines with fewer than three columns are skipped rather than
/// failing the whole read: a log can accumulate junk, and showing what we
/// can beats showing nothing.  Memory stays bounded by `max_lines`
/// regardless of file size.
pub fn read_recent(path: &Path, max_lines: usize) -> io::Result<Vec<RawLogLine>> {
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    let reader = BufReader::new(std::fs::File::open(path)?);
    let mut recent = VecDeque::with_capacity(max_lines.min(1024));
    for line in reader.lines() {
        let Some(entry) = parse_line(&line?) else {
            continue;
        };
        if recent.len() == max_lines {
            recent.pop_front();
        }
        recent.push_back(entry);
    }
    Ok(recent.into())
}

/// Split `timestamp level message`, tolerating runs of whitespace between
/// columns (e.g. right-aligned levels: `...Z  INFO msg`).
fn parse_line(line: &str) -> Option<RawLogLine> {
    let (timestamp, rest) = line.trim().split_once(char::is_whitespace)?;
    let (level, message) = rest.trim_start().split_once(char::is_whitespace)?;
    let message = message.trim_start();
    Some(RawLogLine {
        timestamp: timestamp.to_owned(),
        level: level.to_owned(),
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn line(timestamp: &str, level: &str, message: &str) -> RawLogLine {
        RawLogLine {
            timestamp: timestamp.into(),
            level: level.into(),
            message: message.into(),
        }
    }

    #[test]
    fn parses_columns_and_skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        std::fs::write(
            &path,
            "2026-05-05T17:23:45Z INFO ok line\n\
             garbage-without-three-fields\n\
             2026-05-05T17:23:45Z INFO\n\
             \n\
             2026-05-05T17:23:46Z WARN   padded message\n\
             2026-05-05T17:23:47Z  INFO right-aligned level\n\
             2026-05-05T17:23:48Z\tERROR\ttab separated\n",
        )
        .unwrap();

        assert_eq!(
            read_recent(&path, 200).unwrap(),
            [
                line("2026-05-05T17:23:45Z", "INFO", "ok line"),
                line("2026-05-05T17:23:46Z", "WARN", "padded message"),
                line("2026-05-05T17:23:47Z", "INFO", "right-aligned level"),
                line("2026-05-05T17:23:48Z", "ERROR", "tab separated"),
            ]
        );
    }

    #[test]
    fn keeps_only_the_most_recent_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        let mut f = std::fs::File::create(&path).unwrap();
        for i in 0..1000 {
            writeln!(f, "2026-05-05T17:23:45Z INFO line {i}").unwrap();
        }
        drop(f);

        let lines = read_recent(&path, 200).unwrap();
        assert_eq!(lines.len(), 200);
        assert_eq!(lines.first().unwrap().message, "line 800");
        assert_eq!(lines.last().unwrap().message, "line 999");
        assert!(read_recent(&path, 0).unwrap().is_empty());
    }

    #[test]
    fn log_path_lives_under_the_settings_dir_override() {
        // The only test in this crate that touches the environment.
        let prev = std::env::var_os("LANTERN_SETTINGS_DIR");
        std::env::set_var("LANTERN_SETTINGS_DIR", "fake-settings");
        let p = log_path();
        match prev {
            Some(v) => std::env::set_var("LANTERN_SETTINGS_DIR", v),
            None => std::env::remove_var("LANTERN_SETTINGS_DIR"),
        }
        assert_eq!(
            p,
            Path::new("fake-settings").join("logs").join("lantern.log")
        );
    }
}
