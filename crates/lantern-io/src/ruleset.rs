//! Rule set persistence: TOML serialisation and deserialisation.
//!
//! # File format
//!
//! ```toml
//! name = "Aggressive scrub"
//! version = 1
//!
//! [[treatments]]
//! id = "url.qp.utm"
//!
//! [[treatments]]
//! id = "url.qp.click_ids"
//!
//! [[treatments]]
//! id = "url.qp.session"
//!
//! [[treatments]]
//! id = "title.whitespace"
//! ```
//!
//! Rule set files conventionally use the `.lantern-rules.toml` extension
//! (PRD OQ-4 / F-SET-3).
//!
//! # Treatment registry
//!
//! Every treatment ID present in a file must be known to the registry
//! ([`treatment_from_id`]).  Unknown IDs produce [`IoError::UnknownTreatment`].
//! This is intentional: silently skipping an unknown treatment could give the
//! user a false sense of completeness.

use std::path::Path;

use serde::{Deserialize, Serialize};

use lantern_core::sanitize::pass::RuleSet;
use lantern_core::sanitize::treatment::Treatment;
use lantern_core::sanitize::treatments::{
    AffiliateTreatment, AuthorSuffixTreatment, ClickIdsTreatment, CustomQpTreatment,
    DeduplicateTreatment, DemobilizeTreatment, EmailTreatment, EmptyFoldersTreatment,
    FolderHtmlEntitiesTreatment, FolderWhitespaceTreatment, FragmentTrackingTreatment,
    HandleTreatment, HtmlEntitiesTreatment, HttpsUpgradeTreatment, RegexFolderTreatment,
    RegexTitleTreatment, SearchTokensTreatment, SessionTreatment, StripFragmentTreatment,
    UnshortenOfflineTreatment, UserSegmentTreatment, UtmTreatment, WhitespaceTreatment,
};

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

// ---------------------------------------------------------------------------
// Serialisable spec types
// ---------------------------------------------------------------------------

/// The TOML-serialisable representation of a rule set.
#[derive(Debug, Serialize, Deserialize)]
pub struct RuleSetSpec {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub treatments: Vec<TreatmentEntry>,
}

fn default_version() -> u32 {
    1
}

/// One entry in the `[[treatments]]` array.
#[derive(Debug, Serialize, Deserialize)]
pub struct TreatmentEntry {
    /// Must match one of the IDs returned by a built-in [`Treatment::id`].
    pub id: String,
    /// Optional per-treatment configuration table (M5).
    ///
    /// Absent entries deserialise as `None`; present entries are passed to
    /// [`Treatment::configure`] after the treatment is instantiated.
    #[serde(default)]
    pub config: Option<toml::Value>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Read a rule set from a `.lantern-rules.toml` file.
///
/// Returns [`IoError::UnknownTreatment`] if any treatment ID in the file is
/// not recognised.
pub fn read_ruleset(path: &Path) -> Result<RuleSet> {
    let text = std::fs::read_to_string(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;

    let spec: RuleSetSpec = toml::from_str(&text).map_err(|e| IoError::TomlDe {
        path: path.to_owned(),
        reason: e.to_string(),
    })?;

    spec_into_ruleset(spec)
}

/// Build a [`RuleSet`] programmatically from a name and a slice of treatment
/// ID strings without touching the filesystem.
///
/// Used by `lantern-app` to construct the hardcoded default rule sets at
/// startup without round-tripping through TOML.
pub fn build_ruleset(name: impl Into<String>, treatment_ids: &[&str]) -> Result<RuleSet> {
    let name = name.into();
    let spec = RuleSetSpec {
        name: name.clone(),
        version: 1,
        treatments: treatment_ids
            .iter()
            .map(|id| TreatmentEntry {
                id: id.to_string(),
                config: None,
            })
            .collect(),
    };
    spec_into_ruleset(spec)
}

/// Build a [`RuleSet`] from `(id, optional-config)` pairs.
///
/// The richer cousin of [`build_ruleset`].  Used when persisting from the
/// rule-set editor UI, where parameterised treatments (`url.qp.custom`,
/// `title.regex`, `folder.regex`) carry user-supplied configuration.
pub fn build_ruleset_with_configs(
    name: impl Into<String>,
    entries: &[(String, Option<toml::Value>)],
) -> Result<RuleSet> {
    let name = name.into();
    let spec = RuleSetSpec {
        name: name.clone(),
        version: 1,
        treatments: entries
            .iter()
            .map(|(id, cfg)| TreatmentEntry {
                id: id.clone(),
                config: cfg.clone(),
            })
            .collect(),
    };
    spec_into_ruleset(spec)
}

/// Serialise `rs` to `.lantern-rules.toml` format and write it to `path`
/// atomically.
pub fn write_ruleset(path: &Path, rs: &RuleSet) -> Result<()> {
    let spec = RuleSetSpec {
        name: rs.name.clone(),
        version: 1,
        treatments: rs
            .treatments
            .iter()
            .map(|t| TreatmentEntry {
                id: t.id().to_owned(),
                config: t.current_config(),
            })
            .collect(),
    };

    let text = toml::to_string_pretty(&spec).map_err(|e| IoError::TomlSer(e.to_string()))?;

    write_atomic(path, text.as_bytes())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a [`RuleSetSpec`] into a [`RuleSet`] by resolving each treatment
/// ID against the built-in registry.
fn spec_into_ruleset(spec: RuleSetSpec) -> Result<RuleSet> {
    let mut treatments: Vec<Box<dyn Treatment>> = Vec::with_capacity(spec.treatments.len());

    for entry in spec.treatments {
        let mut t = treatment_from_id(&entry.id)
            .ok_or_else(|| IoError::UnknownTreatment(entry.id.clone()))?;
        if let Some(config) = &entry.config {
            t.configure(config)
                .map_err(|reason| IoError::TreatmentConfig {
                    id: entry.id.clone(),
                    reason,
                })?;
        }
        treatments.push(t);
    }

    Ok(RuleSet {
        // Use the name as the ID for v0.0.1 (no UUID generation yet).
        id: spec.name.clone(),
        name: spec.name,
        treatments,
    })
}

/// Look up a built-in treatment by its canonical ID string.
///
/// Returns `None` for any ID not registered here.  To add a new treatment,
/// implement the [`Treatment`] trait and add a match arm.
fn treatment_from_id(id: &str) -> Option<Box<dyn Treatment>> {
    match id {
        // ── URL query params ────────────────────────────────────────────────
        "url.qp.utm" => Some(Box::new(UtmTreatment)),
        "url.qp.click_ids" => Some(Box::new(ClickIdsTreatment)),
        "url.qp.session" => Some(Box::new(SessionTreatment)),
        "url.qp.affiliate" => Some(Box::new(AffiliateTreatment)),
        "url.qp.search_tokens" => Some(Box::new(SearchTokensTreatment)),
        // Parameterised treatments register with empty defaults; user rule
        // sets supply the param list via the (planned) per-treatment config
        // block in `.lantern-rules.toml`.
        "url.qp.custom" => Some(Box::new(CustomQpTreatment::empty())),

        // ── URL path / fragment / host ─────────────────────────────────────
        "url.path.user_segment" => Some(Box::new(UserSegmentTreatment)),
        "url.strip_fragment" => Some(Box::new(StripFragmentTreatment)),
        "url.fragment.tracking" => Some(Box::new(FragmentTrackingTreatment)),
        "url.https_upgrade" => Some(Box::new(HttpsUpgradeTreatment)),
        "url.host.demobilize" => Some(Box::new(DemobilizeTreatment)),
        "url.host.unshorten.offline" => Some(Box::new(UnshortenOfflineTreatment)),

        // ── Title ──────────────────────────────────────────────────────────
        "title.whitespace" => Some(Box::new(WhitespaceTreatment)),
        "title.html_entities" => Some(Box::new(HtmlEntitiesTreatment)),
        "title.email" => Some(Box::new(EmailTreatment)),
        "title.handle" => Some(Box::new(HandleTreatment)),
        "title.author_suffix" => Some(Box::new(AuthorSuffixTreatment)),
        "title.regex" => Some(Box::new(RegexTitleTreatment::empty())),

        // ── Folder name ────────────────────────────────────────────────────
        "folder.whitespace" => Some(Box::new(FolderWhitespaceTreatment)),
        "folder.html_entities" => Some(Box::new(FolderHtmlEntitiesTreatment)),
        "folder.regex" => Some(Box::new(RegexFolderTreatment::empty())),

        // ── Cross-field ────────────────────────────────────────────────────
        "cross.dedupe" => Some(Box::new(DeduplicateTreatment)),
        "cross.empty_folders" => Some(Box::new(EmptyFoldersTreatment)),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r#"
name = "Aggressive scrub"
version = 1

[[treatments]]
id = "url.qp.utm"

[[treatments]]
id = "url.qp.click_ids"

[[treatments]]
id = "title.whitespace"
"#;

    #[test]
    fn parse_sample_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lantern-rules.toml");
        std::fs::write(&path, SAMPLE_TOML).unwrap();

        let rs = read_ruleset(&path).unwrap();
        assert_eq!(rs.name, "Aggressive scrub");
        assert_eq!(rs.treatments.len(), 3);
        assert_eq!(rs.treatments[0].id(), "url.qp.utm");
        assert_eq!(rs.treatments[1].id(), "url.qp.click_ids");
        assert_eq!(rs.treatments[2].id(), "title.whitespace");
    }

    #[test]
    fn round_trip_ruleset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rs.toml");

        let original = RuleSet {
            id: "test".into(),
            name: "Test set".into(),
            treatments: vec![Box::new(UtmTreatment), Box::new(SessionTreatment)],
        };

        write_ruleset(&path, &original).unwrap();
        let loaded = read_ruleset(&path).unwrap();

        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.treatments.len(), 2);
        assert_eq!(loaded.treatments[0].id(), "url.qp.utm");
        assert_eq!(loaded.treatments[1].id(), "url.qp.session");
    }

    #[test]
    fn unknown_treatment_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "name = \"x\"\n[[treatments]]\nid = \"not.real\"\n").unwrap();

        let result = read_ruleset(&path);
        assert!(matches!(result, Err(IoError::UnknownTreatment(id)) if id == "not.real"));
    }
}
