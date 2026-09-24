//! Folder-name treatments (mirrors of the corresponding title treatments).
//!
//! | ID                          | Name                              |
//! |-----------------------------|-----------------------------------|
//! | `folder.whitespace`         | Normalize whitespace in folder names |
//! | `folder.html_entities`      | Decode HTML entities in folder names |
//! | `folder.regex`              | Apply user regex to folder names  |
//!
//! These mirror the behaviour of the [`title.*`](super::title) treatments but
//! act on `Field::FolderName` for `Node::Folder` nodes only.  Bookmarks are
//! left alone, keeping the two namespaces independent so a rule set can
//! choose to clean titles, folder names, or both.

use super::text::{decode_html_entities, normalize_whitespace};
use crate::model::document::Field;
use crate::model::node::Node;
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};

// ---------------------------------------------------------------------------
// Normalize whitespace  (folder.whitespace)
// ---------------------------------------------------------------------------

/// Collapses runs of ASCII whitespace and trims edges in folder names.
///
/// Mirrors [`super::title::WhitespaceTreatment`].
pub struct FolderWhitespaceTreatment;

impl Treatment for FolderWhitespaceTreatment {
    fn id(&self) -> &'static str {
        "folder.whitespace"
    }
    fn name(&self) -> &'static str {
        "Normalize folder-name whitespace"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::FolderName
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let folder = match node {
            Node::Folder(f) => f,
            _ => return vec![],
        };
        let before = folder.name.as_str();
        let after = normalize_whitespace(before);
        if after == before {
            return vec![];
        }
        vec![Change::set_field(
            folder.id,
            Field::FolderName,
            before,
            after,
            self.id(),
            "Collapsed whitespace in folder name",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Decode HTML entities  (folder.html_entities)
// ---------------------------------------------------------------------------

/// Decodes the same set of named and numeric HTML entities as
/// [`super::title::HtmlEntitiesTreatment`], but on folder names.
pub struct FolderHtmlEntitiesTreatment;

impl Treatment for FolderHtmlEntitiesTreatment {
    fn id(&self) -> &'static str {
        "folder.html_entities"
    }
    fn name(&self) -> &'static str {
        "Decode HTML entities in folder names"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::FolderName
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let folder = match node {
            Node::Folder(f) => f,
            _ => return vec![],
        };
        let before = folder.name.as_str();
        let after = decode_html_entities(before);
        if after == before {
            return vec![];
        }
        vec![Change::set_field(
            folder.id,
            Field::FolderName,
            before,
            after,
            self.id(),
            "Decoded HTML entities in folder name",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Apply user regex  (folder.regex)
// ---------------------------------------------------------------------------

/// User-supplied regex replace on folder names.  Mirrors
/// [`super::title::RegexTitleTreatment`] with identical semantics
/// (destructive, unapproved by default, empty rule = no-op).
pub struct RegexFolderTreatment {
    rule: Option<RegexRule>,
}

struct RegexRule {
    re: regex::Regex,
    replacement: String,
}

impl RegexFolderTreatment {
    pub fn new(pattern: &str, replacement: impl Into<String>) -> Option<Self> {
        let re = regex::Regex::new(pattern).ok()?;
        Some(Self {
            rule: Some(RegexRule {
                re,
                replacement: replacement.into(),
            }),
        })
    }

    pub fn empty() -> Self {
        Self { rule: None }
    }
}

impl Treatment for RegexFolderTreatment {
    fn id(&self) -> &'static str {
        "folder.regex"
    }

    fn configure(&mut self, config: &toml::Value) -> Result<(), String> {
        let pattern = config
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "folder.regex: config must have a 'pattern' string".to_owned())?;
        let replacement = config
            .get("replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        *self = RegexFolderTreatment::new(pattern, replacement)
            .ok_or_else(|| format!("folder.regex: invalid regex pattern: {pattern}"))?;
        Ok(())
    }

    fn current_config(&self) -> Option<toml::Value> {
        let mut table = toml::map::Map::new();
        let (pattern, replacement) = match &self.rule {
            Some(r) => (r.re.as_str().to_owned(), r.replacement.clone()),
            None => (String::new(), String::new()),
        };
        table.insert("pattern".to_owned(), toml::Value::String(pattern));
        table.insert("replacement".to_owned(), toml::Value::String(replacement));
        Some(toml::Value::Table(table))
    }

    fn name(&self) -> &'static str {
        "Apply regex to folder names"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::FolderName
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let rule = match &self.rule {
            Some(r) => r,
            None => return vec![],
        };
        let folder = match node {
            Node::Folder(f) => f,
            _ => return vec![],
        };
        let before = folder.name.as_str();
        let after = rule.re.replace_all(before, rule.replacement.as_str());
        if after.as_ref() == before {
            return vec![];
        }
        vec![Change::set_field(
            folder.id,
            Field::FolderName,
            before,
            after.into_owned(),
            self.id(),
            "Applied user-configured regex to folder name",
            true,
        )]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder};
    use crate::sanitize::treatment::PassContext;

    fn make_folder(name: &str) -> Node {
        Node::Folder(Folder {
            id: 7,
            name: name.to_owned(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        })
    }

    fn make_bookmark(title: &str) -> Node {
        Node::Bookmark(Bookmark {
            id: 8,
            title: title.to_owned(),
            url: BookmarkUrl::Malformed { raw: "x".into() },
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        })
    }

    fn ctx() -> PassContext {
        PassContext { document_id: 0 }
    }

    // ── FolderWhitespaceTreatment ──────────────────────────────────────────

    #[test]
    fn folder_whitespace_collapses_runs() {
        let n = make_folder("   News   &   Stuff   ");
        let changes = FolderWhitespaceTreatment.propose(&n, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "News & Stuff");
        assert_eq!(changes[0].target_field().unwrap(), Field::FolderName);
    }

    #[test]
    fn folder_whitespace_noop_on_clean_name() {
        let n = make_folder("Recipes");
        assert!(FolderWhitespaceTreatment.propose(&n, &ctx()).is_empty());
    }

    #[test]
    fn folder_whitespace_skips_bookmark() {
        let bm = make_bookmark("  spaced title  ");
        assert!(FolderWhitespaceTreatment.propose(&bm, &ctx()).is_empty());
    }

    // ── FolderHtmlEntitiesTreatment ────────────────────────────────────────

    #[test]
    fn folder_html_entities_decodes_amp() {
        let n = make_folder("Tom &amp; Jerry");
        let changes = FolderHtmlEntitiesTreatment.propose(&n, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Tom & Jerry");
    }

    #[test]
    fn folder_html_entities_decode_dashes_like_titles() {
        let n = make_folder("Rust &mdash; notes &ndash; misc");
        let changes = FolderHtmlEntitiesTreatment.propose(&n, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "Rust — notes – misc");
    }

    #[test]
    fn folder_html_entities_decodes_numeric() {
        let n = make_folder("&#169; Vault");
        let changes = FolderHtmlEntitiesTreatment.propose(&n, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "© Vault");
    }

    #[test]
    fn folder_html_entities_noop_on_clean_name() {
        let n = make_folder("Plain");
        assert!(FolderHtmlEntitiesTreatment.propose(&n, &ctx()).is_empty());
    }

    #[test]
    fn folder_html_entities_skips_bookmark() {
        let bm = make_bookmark("Tom &amp; Jerry");
        assert!(FolderHtmlEntitiesTreatment.propose(&bm, &ctx()).is_empty());
    }

    // ── RegexFolderTreatment ───────────────────────────────────────────────

    #[test]
    fn folder_regex_replaces_match() {
        let t = RegexFolderTreatment::new(r"^TODO[ \t]+", "").unwrap();
        let n = make_folder("TODO Cleanup");
        let changes = t.propose(&n, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Cleanup");
    }

    #[test]
    fn folder_regex_empty_is_noop() {
        let n = make_folder("anything");
        assert!(RegexFolderTreatment::empty().propose(&n, &ctx()).is_empty());
    }

    #[test]
    fn folder_regex_invalid_pattern_returns_none() {
        assert!(RegexFolderTreatment::new(r"(unclosed", "").is_none());
    }

    #[test]
    fn folder_regex_destructive_and_unapproved() {
        let t = RegexFolderTreatment::new(r"x", "y").unwrap();
        let n = make_folder("xyz");
        let changes = t.propose(&n, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    #[test]
    fn folder_regex_skips_bookmarks() {
        let t = RegexFolderTreatment::new(r"x", "y").unwrap();
        let bm = make_bookmark("xyz");
        assert!(t.propose(&bm, &ctx()).is_empty());
    }
}
