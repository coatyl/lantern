//! Disk-backed rule-set store.
//!
//! A *rule store* is a single directory holding one `<name>.lantern-rules.toml`
//! file per rule set.  The filesystem layout is intentionally trivial so users
//! can copy, version-control, and share rule sets with ordinary tools.
//!
//! # Layout
//!
//! ```text
//! <rules_dir>/
//!   minimal-clean.lantern-rules.toml
//!   aggressive-scrub.lantern-rules.toml
//!   full-scrub.lantern-rules.toml
//!   my-custom-set.lantern-rules.toml
//! ```
//!
//! The on-disk *file name* is derived from the rule-set *display name* by
//! lower-casing, replacing whitespace with dashes, and stripping any character
//! that is neither ASCII alphanumeric nor `-`/`_`.  The display name inside the
//! TOML stays exactly as the user typed it.
//!
//! # Built-in seeding
//!
//! [`seed_builtins_if_absent`] writes the three shipped rule sets ("Minimal
//! clean", "Aggressive scrub", "Full scrub") to `<rules_dir>` if and only if
//! they do not already exist.  It never overwrites a file the user has
//! customised; callers can restore a built-in by deleting it first.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lantern_core::sanitize::pass::RuleSet;

use crate::error::{IoError, Result};
use crate::ruleset::{build_ruleset, build_ruleset_with_configs, read_ruleset, write_ruleset};

// ---------------------------------------------------------------------------
// File-name convention
// ---------------------------------------------------------------------------

/// Suffix every rule-set file carries (PRD OQ-4).
pub const RULE_FILE_SUFFIX: &str = ".lantern-rules.toml";

/// Derive a safe file stem from a rule-set display name.
///
/// Lower-cases, swaps whitespace for `-`, and drops any character outside
/// `[A-Za-z0-9_-]`.  Two different display names that would collapse to the
/// same stem (e.g. `"My Set"` and `"my_set"`) are not guarded against here;
/// the caller checks for existing files before writing.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() {
            // Compress runs of whitespace into a single `-`.
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "unnamed".into()
    } else {
        trimmed.to_owned()
    }
}

/// Absolute path for the on-disk file of a rule set named `display_name`.
pub fn file_path_for(rules_dir: &Path, display_name: &str) -> PathBuf {
    rules_dir.join(format!("{}{}", slugify(display_name), RULE_FILE_SUFFIX))
}

// ---------------------------------------------------------------------------
// Built-in catalogue
// ---------------------------------------------------------------------------

/// Pairs of (display name, ordered treatment IDs) for the three shipped sets.
///
/// The same table is the canonical source used when seeding the rules
/// directory and when `run_pass` falls back to a built-in if the on-disk file
/// is missing.
pub fn builtin_rule_sets() -> [(&'static str, &'static [&'static str]); 3] {
    [
        (
            "Minimal clean",
            &[
                "url.qp.utm",
                "url.qp.click_ids",
                "url.fragment.tracking",
                "title.whitespace",
                "title.html_entities",
                "folder.whitespace",
                "folder.html_entities",
            ],
        ),
        (
            "Aggressive scrub",
            &[
                "url.qp.utm",
                "url.qp.click_ids",
                "url.qp.session",
                "url.qp.affiliate",
                "url.qp.search_tokens",
                "url.fragment.tracking",
                "url.strip_fragment",
                "title.whitespace",
                "title.html_entities",
                "title.email",
                "title.handle",
                "folder.whitespace",
                "folder.html_entities",
            ],
        ),
        (
            "Full scrub",
            &[
                "url.qp.utm",
                "url.qp.click_ids",
                "url.qp.session",
                "url.qp.affiliate",
                "url.qp.search_tokens",
                "url.path.user_segment",
                "url.fragment.tracking",
                "url.strip_fragment",
                "url.https_upgrade",
                "url.host.demobilize",
                "url.host.unshorten.offline",
                "title.whitespace",
                "title.html_entities",
                "title.email",
                "title.handle",
                "title.author_suffix",
                "folder.whitespace",
                "folder.html_entities",
            ],
        ),
    ]
}

/// True if `name` is one of the three built-in rule sets.
pub fn is_builtin(name: &str) -> bool {
    builtin_rule_sets().iter().any(|(n, _)| *n == name)
}

// ---------------------------------------------------------------------------
// Store operations
// ---------------------------------------------------------------------------

/// Ensure `rules_dir` exists and seed any missing built-in rule sets.
///
/// Idempotent and non-destructive: if the user has edited `minimal-clean`,
/// this call leaves their version alone.  Returns the list of rule-set names
/// that were actually written.
pub fn seed_builtins_if_absent(rules_dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(rules_dir).map_err(|e| IoError::Write {
        path: rules_dir.to_owned(),
        source: e,
    })?;

    let mut written = Vec::new();
    for (name, treatment_ids) in builtin_rule_sets() {
        let path = file_path_for(rules_dir, name);
        if path.exists() {
            continue;
        }
        let rs = build_ruleset(name.to_string(), treatment_ids)?;
        write_ruleset(&path, &rs)?;
        written.push(name.to_string());
    }
    Ok(written)
}

/// A single rule set's headline info, one per file in the store.
#[derive(Debug, Clone)]
pub struct RuleSetSummary {
    pub name: String,
    pub treatment_count: usize,
    pub is_builtin: bool,
    pub path: PathBuf,
}

/// Enumerate all rule sets in `rules_dir`.
///
/// Returns summaries sorted so built-ins come first in their canonical order,
/// followed by user sets in alphabetical order.  Files with the wrong
/// extension are ignored; files that fail to parse are logged to `stderr` but
/// do not abort the listing; the UI shows the rest.
pub fn list_rule_sets(rules_dir: &Path) -> Result<Vec<RuleSetSummary>> {
    if !rules_dir.exists() {
        return Ok(Vec::new());
    }

    let mut builtins: Vec<RuleSetSummary> = Vec::new();
    let mut user_sets: BTreeMap<String, RuleSetSummary> = BTreeMap::new();

    let entries = std::fs::read_dir(rules_dir).map_err(|e| IoError::Read {
        path: rules_dir.to_owned(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(RULE_FILE_SUFFIX))
            .unwrap_or(false)
        {
            continue;
        }

        match read_ruleset(&path) {
            Ok(rs) => {
                let summary = RuleSetSummary {
                    name: rs.name.clone(),
                    treatment_count: rs.treatments.len(),
                    is_builtin: is_builtin(&rs.name),
                    path: path.clone(),
                };
                if summary.is_builtin {
                    builtins.push(summary);
                } else {
                    user_sets.insert(summary.name.clone(), summary);
                }
            }
            Err(e) => {
                eprintln!("warn: could not read rule set {}: {e}", path.display());
            }
        }
    }

    // Put built-ins in their canonical order.
    let order: Vec<&'static str> = builtin_rule_sets().iter().map(|(n, _)| *n).collect();
    builtins.sort_by_key(|s| {
        order
            .iter()
            .position(|n| *n == s.name)
            .unwrap_or(usize::MAX)
    });

    let mut out = builtins;
    out.extend(user_sets.into_values());
    Ok(out)
}

/// Load a single rule set by display name.
///
/// If the on-disk file is missing but `name` is a built-in, falls back to the
/// in-memory catalogue so `run_pass` keeps working when the user has deleted
/// their rules directory.
pub fn load_rule_set(rules_dir: &Path, name: &str) -> Result<RuleSet> {
    let path = file_path_for(rules_dir, name);
    if path.exists() {
        return read_ruleset(&path);
    }
    // Fallback: built-in catalogue.
    for (builtin_name, treatment_ids) in builtin_rule_sets() {
        if builtin_name == name {
            return build_ruleset(name.to_string(), treatment_ids);
        }
    }
    Err(IoError::Read {
        path,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "rule set not found"),
    })
}

/// Save a rule set, whether new or edited, to its canonical file.
///
/// The caller is responsible for supplying an ordered list of treatment IDs.
/// Validation (unknown IDs) happens inside [`build_ruleset`].
pub fn save_rule_set(rules_dir: &Path, name: &str, treatment_ids: &[&str]) -> Result<PathBuf> {
    std::fs::create_dir_all(rules_dir).map_err(|e| IoError::Write {
        path: rules_dir.to_owned(),
        source: e,
    })?;
    let rs = build_ruleset(name.to_string(), treatment_ids)?;
    let path = file_path_for(rules_dir, name);
    write_ruleset(&path, &rs)?;
    Ok(path)
}

/// Save a rule set with per-treatment configuration values.
///
/// Used by the rule-set editor UI when treatments such as `url.qp.custom`
/// or `*.regex` carry user-supplied parameters.  The pairs are
/// `(treatment_id, optional_config)`; pass `None` for stateless treatments.
pub fn save_rule_set_with_configs(
    rules_dir: &Path,
    name: &str,
    entries: &[(String, Option<toml::Value>)],
) -> Result<PathBuf> {
    std::fs::create_dir_all(rules_dir).map_err(|e| IoError::Write {
        path: rules_dir.to_owned(),
        source: e,
    })?;
    let rs = build_ruleset_with_configs(name.to_string(), entries)?;
    let path = file_path_for(rules_dir, name);
    write_ruleset(&path, &rs)?;
    Ok(path)
}

/// Delete a user-created rule set.
///
/// Returns `Err` if `name` is a built-in (built-ins can be reset by editing
/// and re-saving; deletion would mean the next seed recreates them which is
/// confusing).
pub fn delete_rule_set(rules_dir: &Path, name: &str) -> Result<()> {
    if is_builtin(name) {
        return Err(IoError::TomlSer(format!(
            "refusing to delete built-in rule set \"{name}\""
        )));
    }
    let path = file_path_for(rules_dir, name);
    if !path.exists() {
        return Err(IoError::Read {
            path,
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "rule set not found"),
        });
    }
    std::fs::remove_file(&path).map_err(|e| IoError::Write { path, source: e })?;
    Ok(())
}

/// Duplicate an existing rule set under a new name.
///
/// The new file is created via [`save_rule_set`] so it is identical to a
/// user-created set: editable, deletable, and with the new display name
/// recorded inside the TOML.
pub fn duplicate_rule_set(rules_dir: &Path, src_name: &str, dst_name: &str) -> Result<PathBuf> {
    let src = load_rule_set(rules_dir, src_name)?;
    if file_path_for(rules_dir, dst_name).exists() {
        return Err(IoError::TomlSer(format!(
            "rule set \"{dst_name}\" already exists"
        )));
    }
    let treatment_ids: Vec<String> = src.treatments.iter().map(|t| t.id().to_owned()).collect();
    let refs: Vec<&str> = treatment_ids.iter().map(String::as_str).collect();
    save_rule_set(rules_dir, dst_name, &refs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn slugify_normalises_names() {
        assert_eq!(slugify("Minimal clean"), "minimal-clean");
        assert_eq!(slugify("  Whacky Set!!  "), "whacky-set");
        assert_eq!(slugify("Work/Personal"), "workpersonal");
        assert_eq!(slugify(""), "unnamed");
        assert_eq!(slugify("🚀"), "unnamed");
    }

    #[test]
    fn seed_creates_three_builtins() {
        let dir = tempdir();
        let written = seed_builtins_if_absent(dir.path()).unwrap();
        assert_eq!(written.len(), 3);
        assert!(file_path_for(dir.path(), "Minimal clean").exists());
        assert!(file_path_for(dir.path(), "Aggressive scrub").exists());
        assert!(file_path_for(dir.path(), "Full scrub").exists());
    }

    #[test]
    fn seed_is_idempotent() {
        let dir = tempdir();
        assert_eq!(seed_builtins_if_absent(dir.path()).unwrap().len(), 3);
        // Second call writes nothing.
        assert!(seed_builtins_if_absent(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn list_returns_builtins_in_canonical_order() {
        let dir = tempdir();
        seed_builtins_if_absent(dir.path()).unwrap();
        let sets = list_rule_sets(dir.path()).unwrap();
        assert_eq!(sets.len(), 3);
        assert_eq!(sets[0].name, "Minimal clean");
        assert_eq!(sets[1].name, "Aggressive scrub");
        assert_eq!(sets[2].name, "Full scrub");
        for s in &sets {
            assert!(s.is_builtin);
        }
    }

    #[test]
    fn save_then_list_includes_user_set() {
        let dir = tempdir();
        seed_builtins_if_absent(dir.path()).unwrap();
        save_rule_set(
            dir.path(),
            "Custom cleanup",
            &["url.qp.utm", "title.whitespace"],
        )
        .unwrap();
        let sets = list_rule_sets(dir.path()).unwrap();
        assert_eq!(sets.len(), 4);
        assert_eq!(sets[3].name, "Custom cleanup"); // user set last
        assert!(!sets[3].is_builtin);
        assert_eq!(sets[3].treatment_count, 2);
    }

    #[test]
    fn load_round_trips_treatments() {
        let dir = tempdir();
        save_rule_set(
            dir.path(),
            "Round trip",
            &["url.qp.utm", "title.whitespace"],
        )
        .unwrap();
        let rs = load_rule_set(dir.path(), "Round trip").unwrap();
        assert_eq!(rs.name, "Round trip");
        assert_eq!(rs.treatments.len(), 2);
        assert_eq!(rs.treatments[0].id(), "url.qp.utm");
    }

    #[test]
    fn load_falls_back_to_builtin_if_file_missing() {
        let dir = tempdir();
        // Do NOT seed; exercise the fallback.
        let rs = load_rule_set(dir.path(), "Minimal clean").unwrap();
        assert_eq!(rs.name, "Minimal clean");
        assert!(!rs.treatments.is_empty());
    }

    #[test]
    fn load_unknown_name_errors_without_fallback() {
        let dir = tempdir();
        let err = load_rule_set(dir.path(), "Not a set");
        assert!(err.is_err());
    }

    #[test]
    fn delete_user_set_removes_file() {
        let dir = tempdir();
        save_rule_set(dir.path(), "Temp", &["title.whitespace"]).unwrap();
        let path = file_path_for(dir.path(), "Temp");
        assert!(path.exists());
        delete_rule_set(dir.path(), "Temp").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn delete_builtin_is_rejected() {
        let dir = tempdir();
        seed_builtins_if_absent(dir.path()).unwrap();
        assert!(delete_rule_set(dir.path(), "Minimal clean").is_err());
        // File still present.
        assert!(file_path_for(dir.path(), "Minimal clean").exists());
    }

    #[test]
    fn duplicate_creates_new_editable_set() {
        let dir = tempdir();
        seed_builtins_if_absent(dir.path()).unwrap();
        duplicate_rule_set(dir.path(), "Minimal clean", "My clean").unwrap();
        let sets = list_rule_sets(dir.path()).unwrap();
        let user = sets.iter().find(|s| s.name == "My clean").unwrap();
        assert!(!user.is_builtin);
        assert_eq!(user.treatment_count, 7); // matches Minimal clean
    }

    #[test]
    fn duplicate_refuses_collision() {
        let dir = tempdir();
        save_rule_set(dir.path(), "A", &["title.whitespace"]).unwrap();
        save_rule_set(dir.path(), "B", &["title.whitespace"]).unwrap();
        assert!(duplicate_rule_set(dir.path(), "A", "B").is_err());
    }
}
