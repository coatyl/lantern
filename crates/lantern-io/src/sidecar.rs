//! Optional session sidecar file (PRD F-EXP-6).
//!
//! When the user opts in, a `.lantern.json` file is written alongside an
//! export.  It records which rule sets were applied, the Lantern version, and
//! the export timestamp.  The sidecar is off by default and never written
//! unless explicitly requested by the caller.

use std::path::Path;

use serde::Serialize;

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

// ---------------------------------------------------------------------------
// Sidecar
// ---------------------------------------------------------------------------

/// Metadata written alongside a bookmark export (PRD F-EXP-6).
#[derive(Debug, Serialize)]
pub struct Sidecar {
    /// Sidecar schema version.  Always `"1"` for this implementation.
    pub version: &'static str,
    /// Lantern version string from `CARGO_PKG_VERSION`.
    pub lantern_version: &'static str,
    /// ISO 8601 UTC timestamp of the export.
    pub exported_at: String,
    /// Names of the rule sets that were applied during this session before
    /// the export, in the order they were applied.
    pub rule_sets_applied: Vec<String>,
}

impl Sidecar {
    /// Construct a sidecar with the current UTC timestamp.
    pub fn new(rule_sets_applied: Vec<String>) -> Self {
        Self {
            version: "1",
            lantern_version: env!("CARGO_PKG_VERSION"),
            exported_at: chrono::Utc::now().to_rfc3339(),
            rule_sets_applied,
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Serialise `sidecar` to JSON and write it to `path` atomically.
///
/// The conventional path is `<export_basename>.lantern.json` alongside the
/// exported bookmark file.
pub fn write_sidecar(path: &Path, sidecar: &Sidecar) -> Result<()> {
    let json =
        serde_json::to_string_pretty(sidecar).map_err(|e| IoError::JsonSer(e.to_string()))?;
    write_atomic(path, json.as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.lantern.json");

        let sidecar = Sidecar::new(vec!["Aggressive scrub".into()]);
        write_sidecar(&path, &sidecar).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(value["version"], "1");
        assert_eq!(value["rule_sets_applied"][0], "Aggressive scrub");
        // exported_at must be a non-empty string
        assert!(!value["exported_at"].as_str().unwrap().is_empty());
    }
}
