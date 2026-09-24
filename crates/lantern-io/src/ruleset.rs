//! Rule-set persistence (`.lantern-rules.toml`) and the treatment registry.
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
//! id = "url.qp.custom"
//! config = { params = ["ref", "source"] }
//! ```
//!
//! Every treatment ID must be known to [`treatment_from_id`]; unknown IDs are
//! an error ([`IoError::UnknownTreatment`]) rather than silently skipped, so
//! a rule set never looks more complete than it is.

use std::path::Path;

use serde::{Deserialize, Serialize};

use lantern_core::sanitize::pass::RuleSet;
use lantern_core::sanitize::treatment::Treatment;
use lantern_core::sanitize::treatments::{
    AffiliateTreatment, AuthorSuffixTreatment, ClickIdsTreatment, CustomQpTreatment,
    DemobilizeTreatment, EmailTreatment, EmptyFoldersTreatment, ExactUrlDuplicatesTreatment,
    FolderHtmlEntitiesTreatment, FolderWhitespaceTreatment, FragmentTrackingTreatment,
    HandleTreatment, HtmlEntitiesTreatment, HttpsUpgradeTreatment, NearUrlDuplicatesTreatment,
    RegexFolderTreatment, RegexTitleTreatment, SearchTokensTreatment, SessionTreatment,
    StripFragmentTreatment, UnshortenOfflineTreatment, UserSegmentTreatment, UtmTreatment,
    WhitespaceTreatment,
};

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

/// On-disk shape of a rule-set file.
#[derive(Serialize, Deserialize)]
struct RuleSetSpec {
    name: String,
    #[serde(default = "default_version")]
    version: u32,
    treatments: Vec<TreatmentEntry>,
}

fn default_version() -> u32 {
    1
}

/// One entry in the `[[treatments]]` array.
#[derive(Serialize, Deserialize)]
struct TreatmentEntry {
    id: String,
    /// Passed to [`Treatment::configure`] when present.
    #[serde(default)]
    config: Option<toml::Value>,
}

/// Read a rule set from a `.lantern-rules.toml` file.
pub fn read_ruleset(path: &Path) -> Result<RuleSet> {
    let text = std::fs::read_to_string(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;
    let spec: RuleSetSpec = toml::from_str(&text).map_err(|e| IoError::TomlDe {
        path: path.to_owned(),
        reason: e.to_string(),
    })?;
    resolve(
        spec.name,
        spec.treatments
            .iter()
            .map(|e| (e.id.as_str(), e.config.as_ref())),
    )
}

/// Build a [`RuleSet`] from a name and ordered treatment IDs, without
/// touching the filesystem.
pub fn build_ruleset(name: impl Into<String>, treatment_ids: &[&str]) -> Result<RuleSet> {
    resolve(name.into(), treatment_ids.iter().map(|&id| (id, None)))
}

/// Build a [`RuleSet`] from `(id, optional config)` pairs, for parameterised
/// treatments (`url.qp.custom`, `title.regex`, `folder.regex`).
pub fn build_ruleset_with_configs(
    name: impl Into<String>,
    entries: &[(String, Option<toml::Value>)],
) -> Result<RuleSet> {
    resolve(
        name.into(),
        entries.iter().map(|(id, cfg)| (id.as_str(), cfg.as_ref())),
    )
}

/// Serialise `rs` (including each treatment's current config) to `path`
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

/// Instantiate and configure each treatment against the registry.
fn resolve<'a>(
    name: String,
    entries: impl Iterator<Item = (&'a str, Option<&'a toml::Value>)>,
) -> Result<RuleSet> {
    let treatments = entries
        .map(|(id, config)| {
            let mut t =
                treatment_from_id(id).ok_or_else(|| IoError::UnknownTreatment(id.to_owned()))?;
            if let Some(config) = config {
                t.configure(config)
                    .map_err(|reason| IoError::TreatmentConfig {
                        id: id.to_owned(),
                        reason,
                    })?;
            }
            Ok(t)
        })
        .collect::<Result<Vec<_>>>()?;

    // Rule sets are identified by name.
    Ok(RuleSet {
        id: name.clone(),
        name,
        treatments,
    })
}

/// Look up a built-in treatment by its canonical ID.
///
/// To add a treatment, implement [`Treatment`] and add a match arm.
/// Parameterised treatments start empty and are filled in by `configure`.
fn treatment_from_id(id: &str) -> Option<Box<dyn Treatment>> {
    let t: Box<dyn Treatment> = match id {
        // URL query params
        "url.qp.utm" => Box::new(UtmTreatment),
        "url.qp.click_ids" => Box::new(ClickIdsTreatment),
        "url.qp.session" => Box::new(SessionTreatment),
        "url.qp.affiliate" => Box::new(AffiliateTreatment),
        "url.qp.search_tokens" => Box::new(SearchTokensTreatment),
        "url.qp.custom" => Box::new(CustomQpTreatment::empty()),

        // URL path / fragment / host
        "url.path.user_segment" => Box::new(UserSegmentTreatment),
        "url.strip_fragment" => Box::new(StripFragmentTreatment),
        "url.fragment.tracking" => Box::new(FragmentTrackingTreatment),
        "url.https_upgrade" => Box::new(HttpsUpgradeTreatment),
        "url.host.demobilize" => Box::new(DemobilizeTreatment),
        "url.host.unshorten.offline" => Box::new(UnshortenOfflineTreatment),

        // Title
        "title.whitespace" => Box::new(WhitespaceTreatment),
        "title.html_entities" => Box::new(HtmlEntitiesTreatment),
        "title.email" => Box::new(EmailTreatment),
        "title.handle" => Box::new(HandleTreatment),
        "title.author_suffix" => Box::new(AuthorSuffixTreatment),
        "title.regex" => Box::new(RegexTitleTreatment::empty()),

        // Folder name
        "folder.whitespace" => Box::new(FolderWhitespaceTreatment),
        "folder.html_entities" => Box::new(FolderHtmlEntitiesTreatment),
        "folder.regex" => Box::new(RegexFolderTreatment::empty()),

        // Cross-field / structure
        "cross.empty_folders" => Box::new(EmptyFoldersTreatment),
        "structure.duplicates.exact_url" => Box::new(ExactUrlDuplicatesTreatment),
        // `cross.dedupe` was folded into the near-duplicate pass; rule sets
        // saved with the old id load the new pass.
        "structure.duplicates.near_url" | "cross.dedupe" => Box::new(NearUrlDuplicatesTreatment),

        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_treatments_in_file_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lantern-rules.toml");
        std::fs::write(
            &path,
            r#"
name = "Aggressive scrub"
version = 1

[[treatments]]
id = "url.qp.utm"

[[treatments]]
id = "url.qp.click_ids"

[[treatments]]
id = "title.whitespace"
"#,
        )
        .unwrap();

        let rs = read_ruleset(&path).unwrap();
        assert_eq!(rs.name, "Aggressive scrub");
        let ids: Vec<_> = rs.treatments.iter().map(|t| t.id()).collect();
        assert_eq!(ids, ["url.qp.utm", "url.qp.click_ids", "title.whitespace"]);
    }

    #[test]
    fn write_then_read_preserves_order_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rs.toml");
        let config = toml::Value::Table(toml::from_str(r#"params = ["ref", "source"]"#).unwrap());
        let original = build_ruleset_with_configs(
            "Test set",
            &[
                ("url.qp.utm".into(), None),
                ("url.qp.custom".into(), Some(config.clone())),
            ],
        )
        .unwrap();

        write_ruleset(&path, &original).unwrap();
        let loaded = read_ruleset(&path).unwrap();

        assert_eq!(loaded.name, "Test set");
        let ids: Vec<_> = loaded.treatments.iter().map(|t| t.id()).collect();
        assert_eq!(ids, ["url.qp.utm", "url.qp.custom"]);
        assert_eq!(loaded.treatments[1].current_config(), Some(config));
    }

    #[test]
    fn rejects_unknown_ids_and_invalid_config() {
        let err = build_ruleset("x", &["url.qp.utm", "not.real"]).err();
        assert!(
            matches!(&err, Some(IoError::UnknownTreatment(id)) if id == "not.real"),
            "{err:?}"
        );

        let bad = toml::Value::Table(toml::map::Map::new());
        let err = build_ruleset_with_configs("x", &[("url.qp.custom".into(), Some(bad))]).err();
        assert!(
            matches!(&err, Some(IoError::TreatmentConfig { id, .. }) if id == "url.qp.custom"),
            "{err:?}"
        );
    }
}
