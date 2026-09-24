//! Disk-backed rule-set store.
//!
//! A rule store is one directory holding a `<slug>.lantern-rules.toml` file
//! per rule set, so users can copy, version-control, and share rule sets
//! with ordinary tools:
//!
//! ```text
//! <rules_dir>/
//!   minimal-clean.lantern-rules.toml
//!   aggressive-scrub.lantern-rules.toml
//!   my-custom-set.lantern-rules.toml
//! ```
//!
//! The file name is derived from the display name with [`slugify`]; the
//! display name inside the TOML stays exactly as the user typed it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lantern_core::sanitize::pass::RuleSet;

use crate::error::{IoError, Result};
use crate::ruleset::{build_ruleset, build_ruleset_with_configs, read_ruleset, write_ruleset};

/// Suffix every rule-set file carries.
pub const RULE_FILE_SUFFIX: &str = ".lantern-rules.toml";

/// Derive a safe file stem from a rule-set display name.
///
/// Lower-cases, collapses whitespace runs to a single `-`, and drops any
/// character outside `[a-z0-9_-]`; an empty result becomes `"unnamed"`.
/// Distinct names that collapse to the same slug (`"My Set"` / `"my set"`)
/// are not guarded against here.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "unnamed".into()
    } else {
        trimmed.to_owned()
    }
}

/// Path of the file holding the rule set named `display_name`.
pub fn file_path_for(rules_dir: &Path, display_name: &str) -> PathBuf {
    rules_dir.join(format!("{}{RULE_FILE_SUFFIX}", slugify(display_name)))
}

/// The shipped rule sets as `(display name, ordered treatment IDs)`, in
/// canonical display order.
///
/// Deletion treatments (`structure.duplicates.*`, `cross.empty_folders`) stay out of the hygiene sets: duplicate review is
/// its own opt-in rule set.
pub const BUILTIN_RULE_SETS: &[(&str, &[&str])] = &[
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
    ("Find duplicates", &["structure.duplicates.near_url"]),
];

/// Position of `name` in [`BUILTIN_RULE_SETS`], if it is a built-in.
fn builtin_index(name: &str) -> Option<usize> {
    BUILTIN_RULE_SETS.iter().position(|(n, _)| *n == name)
}

/// True if `name` is one of the shipped built-in rule sets.
pub fn is_builtin(name: &str) -> bool {
    builtin_index(name).is_some()
}

fn ensure_dir(rules_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(rules_dir).map_err(|e| IoError::Write {
        path: rules_dir.to_owned(),
        source: e,
    })
}

fn not_found(path: PathBuf) -> IoError {
    IoError::Read {
        path,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "rule set not found"),
    }
}

/// Create `rules_dir` if needed and write every built-in rule set whose file
/// is missing.
///
/// Never overwrites an existing file, so user edits to a built-in survive;
/// deleting the file restores the shipped version on the next call.
/// Returns the names that were written.
pub fn seed_builtins_if_absent(rules_dir: &Path) -> Result<Vec<String>> {
    ensure_dir(rules_dir)?;
    let mut written = Vec::new();
    for &(name, treatment_ids) in BUILTIN_RULE_SETS {
        let path = file_path_for(rules_dir, name);
        if !path.exists() {
            write_ruleset(&path, &build_ruleset(name, treatment_ids)?)?;
            written.push(name.to_owned());
        }
    }
    Ok(written)
}

/// Headline info for one rule-set file in the store.
#[derive(Debug, Clone)]
pub struct RuleSetSummary {
    pub name: String,
    pub treatment_count: usize,
    pub is_builtin: bool,
    pub path: PathBuf,
}

/// Enumerate the rule sets in `rules_dir`: built-ins first in canonical
/// order, then user sets alphabetically.
///
/// Files without [`RULE_FILE_SUFFIX`] are ignored; files that fail to parse
/// are reported on stderr and skipped so one bad file cannot hide the rest.
pub fn list_rule_sets(rules_dir: &Path) -> Result<Vec<RuleSetSummary>> {
    if !rules_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(rules_dir).map_err(|e| IoError::Read {
        path: rules_dir.to_owned(),
        source: e,
    })?;

    let mut builtins = Vec::new();
    let mut user_sets = BTreeMap::new();
    for path in entries.flatten().map(|e| e.path()) {
        let is_rule_file = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(RULE_FILE_SUFFIX));
        if !is_rule_file {
            continue;
        }
        let rs = match read_ruleset(&path) {
            Ok(rs) => rs,
            Err(e) => {
                eprintln!("warn: could not read rule set {}: {e}", path.display());
                continue;
            }
        };
        let builtin = builtin_index(&rs.name);
        let summary = RuleSetSummary {
            treatment_count: rs.treatments.len(),
            is_builtin: builtin.is_some(),
            path,
            name: rs.name,
        };
        match builtin {
            Some(order) => builtins.push((order, summary)),
            None => {
                user_sets.insert(summary.name.clone(), summary);
            }
        }
    }

    builtins.sort_by_key(|(order, _)| *order);
    Ok(builtins
        .into_iter()
        .map(|(_, s)| s)
        .chain(user_sets.into_values())
        .collect())
}

/// Load a rule set by display name.
///
/// A missing file for a built-in falls back to the in-memory catalogue, so
/// passes keep working after the user deletes their rules directory.
pub fn load_rule_set(rules_dir: &Path, name: &str) -> Result<RuleSet> {
    let path = file_path_for(rules_dir, name);
    if path.exists() {
        return read_ruleset(&path);
    }
    match builtin_index(name) {
        Some(i) => build_ruleset(name, BUILTIN_RULE_SETS[i].1),
        None => Err(not_found(path)),
    }
}

/// Save a rule set (new or edited) to its canonical file.  Unknown treatment
/// IDs are rejected before anything is written.
pub fn save_rule_set(rules_dir: &Path, name: &str, treatment_ids: &[&str]) -> Result<PathBuf> {
    save(rules_dir, &build_ruleset(name, treatment_ids)?)
}

/// Save a rule set whose treatments carry configuration, as
/// `(treatment_id, optional_config)` pairs.
pub fn save_rule_set_with_configs(
    rules_dir: &Path,
    name: &str,
    entries: &[(String, Option<toml::Value>)],
) -> Result<PathBuf> {
    save(rules_dir, &build_ruleset_with_configs(name, entries)?)
}

fn save(rules_dir: &Path, rs: &RuleSet) -> Result<PathBuf> {
    ensure_dir(rules_dir)?;
    let path = file_path_for(rules_dir, &rs.name);
    write_ruleset(&path, rs)?;
    Ok(path)
}

/// Delete a user-created rule set.  Built-ins are refused: the next seed
/// would silently recreate them.
pub fn delete_rule_set(rules_dir: &Path, name: &str) -> Result<()> {
    if is_builtin(name) {
        return Err(IoError::BuiltinRuleSet(name.to_owned()));
    }
    let path = file_path_for(rules_dir, name);
    if !path.exists() {
        return Err(not_found(path));
    }
    std::fs::remove_file(&path).map_err(|e| IoError::Write { path, source: e })
}

/// Copy an existing rule set, including per-treatment configuration, under
/// a new name as an ordinary user set.
pub fn duplicate_rule_set(rules_dir: &Path, src_name: &str, dst_name: &str) -> Result<PathBuf> {
    let mut rs = load_rule_set(rules_dir, src_name)?;
    if file_path_for(rules_dir, dst_name).exists() {
        return Err(IoError::RuleSetExists(dst_name.to_owned()));
    }
    rs.id = dst_name.to_owned();
    rs.name = dst_name.to_owned();
    save(rules_dir, &rs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin_ids(name: &str) -> &'static [&'static str] {
        BUILTIN_RULE_SETS[builtin_index(name).unwrap()].1
    }

    #[test]
    fn slugify_normalises_names() {
        for (input, slug) in [
            ("Minimal clean", "minimal-clean"),
            ("  Whacky Set!!  ", "whacky-set"),
            ("Work/Personal", "workpersonal"),
            ("a \t b", "a-b"),
            ("", "unnamed"),
            ("🚀", "unnamed"),
        ] {
            assert_eq!(slugify(input), slug, "slugify({input:?})");
        }
    }

    #[test]
    fn builtin_catalogue_keeps_deletions_opt_in() {
        for name in ["Minimal clean", "Aggressive scrub", "Full scrub"] {
            for delete in [
                "structure.duplicates.exact_url",
                "structure.duplicates.near_url",
                "cross.empty_folders",
            ] {
                assert!(
                    !builtin_ids(name).contains(&delete),
                    "{name} must not silently delete ({delete})"
                );
            }
        }
        assert_eq!(
            builtin_ids("Find duplicates"),
            ["structure.duplicates.near_url"]
        );
    }

    #[test]
    fn seed_writes_missing_builtins_once() {
        let dir = tempfile::tempdir().unwrap();
        let written = seed_builtins_if_absent(dir.path()).unwrap();
        let expected: Vec<_> = BUILTIN_RULE_SETS.iter().map(|(n, _)| *n).collect();
        assert_eq!(written, expected);
        for name in expected {
            assert!(file_path_for(dir.path(), name).exists(), "{name}");
        }
        assert!(seed_builtins_if_absent(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn list_orders_builtins_canonically_then_user_sets_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        seed_builtins_if_absent(dir.path()).unwrap();
        save_rule_set(dir.path(), "Zeta", &["title.whitespace"]).unwrap();
        save_rule_set(dir.path(), "Custom", &["url.qp.utm", "title.whitespace"]).unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not a rule set").unwrap();

        let sets = list_rule_sets(dir.path()).unwrap();
        let names: Vec<_> = sets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Minimal clean",
                "Aggressive scrub",
                "Full scrub",
                "Find duplicates",
                "Custom",
                "Zeta"
            ]
        );
        assert!(sets[..4].iter().all(|s| s.is_builtin));
        assert!(!sets[4].is_builtin);
        assert_eq!(sets[4].treatment_count, 2);
    }

    #[test]
    fn load_round_trips_treatments() {
        let dir = tempfile::tempdir().unwrap();
        save_rule_set(
            dir.path(),
            "Round trip",
            &["url.qp.utm", "title.whitespace"],
        )
        .unwrap();
        let rs = load_rule_set(dir.path(), "Round trip").unwrap();
        assert_eq!(rs.name, "Round trip");
        let ids: Vec<_> = rs.treatments.iter().map(|t| t.id()).collect();
        assert_eq!(ids, ["url.qp.utm", "title.whitespace"]);
    }

    #[test]
    fn load_falls_back_to_builtin_catalogue_only() {
        let dir = tempfile::tempdir().unwrap();
        let rs = load_rule_set(dir.path(), "Minimal clean").unwrap();
        assert_eq!(rs.treatments.len(), builtin_ids("Minimal clean").len());
        assert!(load_rule_set(dir.path(), "Not a set").is_err());
    }

    #[test]
    fn delete_removes_user_sets_and_refuses_builtins() {
        let dir = tempfile::tempdir().unwrap();
        seed_builtins_if_absent(dir.path()).unwrap();
        save_rule_set(dir.path(), "Temp", &["title.whitespace"]).unwrap();

        delete_rule_set(dir.path(), "Temp").unwrap();
        assert!(!file_path_for(dir.path(), "Temp").exists());

        let err = delete_rule_set(dir.path(), "Minimal clean").unwrap_err();
        assert!(matches!(err, IoError::BuiltinRuleSet(_)), "{err:?}");
        assert_eq!(
            err.to_string(),
            "refusing to delete built-in rule set \"Minimal clean\""
        );
        assert!(file_path_for(dir.path(), "Minimal clean").exists());
    }

    #[test]
    fn duplicate_creates_new_editable_set() {
        let dir = tempfile::tempdir().unwrap();
        seed_builtins_if_absent(dir.path()).unwrap();
        duplicate_rule_set(dir.path(), "Minimal clean", "My clean").unwrap();
        let sets = list_rule_sets(dir.path()).unwrap();
        let user = sets.iter().find(|s| s.name == "My clean").unwrap();
        assert!(!user.is_builtin);
        assert_eq!(user.treatment_count, builtin_ids("Minimal clean").len());
    }

    #[test]
    fn duplicate_keeps_treatment_config() {
        // Regression: duplicating used to re-save treatment IDs only, so a
        // configured `url.qp.custom` came back with an empty param list.
        let dir = tempfile::tempdir().unwrap();
        let config = toml::Value::Table(toml::from_str(r#"params = ["ref"]"#).unwrap());
        save_rule_set_with_configs(
            dir.path(),
            "Custom",
            &[
                ("title.whitespace".into(), None),
                ("url.qp.custom".into(), Some(config.clone())),
            ],
        )
        .unwrap();

        duplicate_rule_set(dir.path(), "Custom", "Copy").unwrap();

        let copy = load_rule_set(dir.path(), "Copy").unwrap();
        assert_eq!(copy.name, "Copy");
        assert_eq!(copy.treatments[1].current_config(), Some(config));
    }

    #[test]
    fn duplicate_refuses_collision() {
        let dir = tempfile::tempdir().unwrap();
        save_rule_set(dir.path(), "A", &["title.whitespace"]).unwrap();
        save_rule_set(dir.path(), "B", &["title.whitespace"]).unwrap();
        let err = duplicate_rule_set(dir.path(), "A", "B").unwrap_err();
        assert!(
            matches!(&err, IoError::RuleSetExists(n) if n == "B"),
            "{err:?}"
        );
    }
}
