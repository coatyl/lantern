//! Chrome / Chromium `Bookmarks` JSON parser.
//!
//! Chromium (Chrome, Edge, Brave, …) stores the profile bookmark database as
//! a JSON file named `Bookmarks` (no extension). The on-disk shape is:
//!
//! ```json
//! {
//!   "checksum": "…",
//!   "roots": {
//!     "bookmark_bar": { "type": "folder", "name": "Bookmarks bar", "children": [ … ] },
//!     "other":        { "type": "folder", "name": "Other bookmarks", "children": [ … ] },
//!     "synced":       { "type": "folder", "name": "Mobile bookmarks", "children": [ … ] }
//!   },
//!   "version": 1
//! }
//! ```
//!
//! Each node is either `type: "url"` (a bookmark: `name`, `url`, `date_added`)
//! or `type: "folder"` (`name`, `children`, `date_added`, `date_modified`).
//! Timestamps are Chrome's epoch: **microseconds since 1601-01-01 UTC**,
//! encoded as a decimal string or (less commonly) a JSON number.
//!
//! This module never touches the filesystem. The caller supplies bytes;
//! [`parse_chrome_json`] returns the same [`Document`] model as the Netscape
//! HTML parser. Extra Chromium fields (`guid`, `id`, `meta_info`, …) are
//! ignored — we are read-only and do not write the profile file back.

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{CoreError, Result};
use crate::model::{
    document::{Document, DocumentStats, HeaderMetadata},
    ids::{next_document_id, NodeIdAllocator},
    node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node},
};

use super::strip_utf8_bom;

/// Microseconds between the Windows/Chrome epoch (1601-01-01) and Unix
/// (1970-01-01). `11_644_473_600` seconds × 1_000_000.
const CHROME_EPOCH_DELTA_MICROS: i64 = 11_644_473_600_000_000;

/// Well-known Chromium root keys, in the order Chrome's UI presents them.
const KNOWN_ROOTS: &[(&str, bool)] = &[("bookmark_bar", true), ("other", false), ("synced", false)];

// ---------------------------------------------------------------------------
// Serde shape
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ChromeBookmarksFile {
    /// Kept in file order so extra roots import in the order they appear.
    roots: IndexMap<String, ChromeNode>,
}

#[derive(Debug, Deserialize)]
struct ChromeNode {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    date_added: Option<ChromeTimestamp>,
    #[serde(default)]
    date_modified: Option<ChromeTimestamp>,
    #[serde(default)]
    children: Vec<ChromeNode>,
}

/// Chrome timestamps arrive as a decimal string or a JSON integer.
#[derive(Debug, Clone, Copy)]
struct ChromeTimestamp(i64);

impl<'de> Deserialize<'de> for ChromeTimestamp {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ChromeTimestamp;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a Chrome timestamp (decimal string or integer)")
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(ChromeTimestamp(v))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                i64::try_from(v)
                    .map(ChromeTimestamp)
                    .map_err(|_| E::custom("timestamp exceeds i64"))
            }

            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                v.trim()
                    .parse::<i64>()
                    .map(ChromeTimestamp)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl ChromeTimestamp {
    fn to_utc(self) -> Option<DateTime<Utc>> {
        chrome_time_to_utc(self.0)
    }
}

/// Convert a Chrome-epoch microsecond count to UTC.
///
/// Returns `None` if the value is out of `DateTime` range after subtracting
/// the 1601→1970 offset.
pub(crate) fn chrome_time_to_utc(chrome_micros: i64) -> Option<DateTime<Utc>> {
    let unix_micros = chrome_micros.checked_sub(CHROME_EPOCH_DELTA_MICROS)?;
    DateTime::from_timestamp_micros(unix_micros)
}

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

/// Parse a Chrome / Chromium `Bookmarks` JSON byte slice into a [`Document`].
///
/// Tolerates a leading UTF-8 BOM. Unknown node types and extra object fields
/// are skipped. The file must contain a `roots` object or the call returns
/// [`CoreError::ParseFailed`].
pub fn parse_chrome_json(bytes: &[u8]) -> Result<Document> {
    let (has_bom, body) = strip_utf8_bom(bytes);

    let file: ChromeBookmarksFile =
        serde_json::from_slice(body).map_err(|e| CoreError::ParseFailed {
            line: e.line() as u32,
            column: e.column() as u32,
            reason: format!("not a Chrome Bookmarks JSON file: {e}"),
        })?;

    if file.roots.is_empty() {
        return Err(CoreError::ParseFailed {
            line: 0,
            column: 0,
            reason: "Chrome Bookmarks JSON has an empty `roots` object".into(),
        });
    }

    let mut id_gen = NodeIdAllocator::new();
    let children = build_roots(file.roots, &mut id_gen);

    let root = Folder {
        id: id_gen.alloc(),
        name: String::new(),
        add_date: None,
        last_modified: None,
        is_toolbar: false,
        attrs: AttrMap::default(),
        children,
    };

    let stats = DocumentStats::from_root(&root);
    let header = HeaderMetadata {
        charset: Some("UTF-8".into()),
        title: Some("Bookmarks".into()),
        has_bom,
        line_ending: crate::model::document::LineEnding::Lf,
    };

    Ok(Document::new(next_document_id(), None, root, header, stats))
}

// ---------------------------------------------------------------------------
// Tree construction
// ---------------------------------------------------------------------------

fn build_roots(mut roots: IndexMap<String, ChromeNode>, id_gen: &mut NodeIdAllocator) -> Vec<Node> {
    let mut children = Vec::new();

    for (key, is_toolbar) in KNOWN_ROOTS {
        if let Some(node) = roots.shift_remove(*key) {
            if let Some(n) = chrome_node_to_model(node, *is_toolbar, id_gen) {
                children.push(n);
            }
        }
    }

    // Any additional roots (e.g. enterprise `managed`) follow in JSON order.
    for (_key, node) in roots {
        if let Some(n) = chrome_node_to_model(node, false, id_gen) {
            children.push(n);
        }
    }

    children
}

fn chrome_node_to_model(
    node: ChromeNode,
    is_toolbar: bool,
    id_gen: &mut NodeIdAllocator,
) -> Option<Node> {
    match classify(&node) {
        NodeKind::Url => Some(Node::Bookmark(parse_url_node(node, id_gen))),
        NodeKind::Folder => Some(Node::Folder(parse_folder_node(node, is_toolbar, id_gen))),
        NodeKind::Unknown => None,
    }
}

enum NodeKind {
    Url,
    Folder,
    Unknown,
}

fn classify(node: &ChromeNode) -> NodeKind {
    match node
        .kind
        .as_deref()
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("url") => NodeKind::Url,
        Some("folder") => NodeKind::Folder,
        Some(_) => NodeKind::Unknown,
        None if node.url.is_some() => NodeKind::Url,
        None => NodeKind::Folder,
    }
}

fn parse_folder_node(node: ChromeNode, is_toolbar: bool, id_gen: &mut NodeIdAllocator) -> Folder {
    let children = node
        .children
        .into_iter()
        .filter_map(|child| chrome_node_to_model(child, false, id_gen))
        .collect();

    Folder {
        id: id_gen.alloc(),
        name: node.name.unwrap_or_default(),
        add_date: node.date_added.and_then(ChromeTimestamp::to_utc),
        last_modified: node.date_modified.and_then(ChromeTimestamp::to_utc),
        is_toolbar,
        attrs: AttrMap::default(),
        children,
    }
}

fn parse_url_node(node: ChromeNode, id_gen: &mut NodeIdAllocator) -> Bookmark {
    let href = node.url.unwrap_or_default();
    let url = match url::Url::parse(&href) {
        Ok(u) => BookmarkUrl::Valid(u),
        Err(_) => BookmarkUrl::Malformed { raw: href },
    };

    Bookmark {
        id: id_gen.alloc(),
        title: node.name.unwrap_or_default(),
        url,
        add_date: node.date_added.and_then(ChromeTimestamp::to_utc),
        last_modified: node.date_modified.and_then(ChromeTimestamp::to_utc),
        icon_blob: None,
        description: None,
        attrs: AttrMap::default(),
        flags: BookmarkFlags::default(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{looks_like_json, parse_auto};

    const FIXTURE: &str = include_str!("../../tests/fixtures/chrome-bookmarks.json");

    /// Unix 1_700_000_000 as a Chrome-epoch microsecond count.
    const CHROME_1700000000: i64 = 13_344_473_600_000_000;

    #[test]
    fn chrome_epoch_converts_known_unix_second() {
        let dt = chrome_time_to_utc(CHROME_1700000000).expect("in range");
        assert_eq!(dt.timestamp(), 1_700_000_000);
    }

    #[test]
    fn chrome_epoch_rejects_underflow() {
        assert!(chrome_time_to_utc(0).is_some()); // 1601-01-01 is representable
        assert!(chrome_time_to_utc(i64::MIN).is_none());
    }

    #[test]
    fn parses_fixture_folder_and_bookmarks() {
        let doc = parse_chrome_json(FIXTURE.as_bytes()).expect("fixture should parse");

        assert_eq!(doc.stats.bookmark_count, 3);
        assert_eq!(doc.stats.folder_count, 3); // bar + Reading + other
        assert_eq!(doc.header.title.as_deref(), Some("Bookmarks"));
        assert!(!doc.header.has_bom);

        // roots: bookmark_bar, other (synced omitted from this fixture)
        assert_eq!(doc.root.children.len(), 2);

        let Node::Folder(bar) = &doc.root.children[0] else {
            panic!("expected Bookmarks bar folder");
        };
        assert_eq!(bar.name, "Bookmarks bar");
        assert!(bar.is_toolbar);
        assert_eq!(bar.add_date.map(|d| d.timestamp()), Some(1_700_000_000));
        assert_eq!(bar.children.len(), 2);

        let Node::Bookmark(utm) = &bar.children[0] else {
            panic!("expected first child to be the UTM bookmark");
        };
        assert_eq!(utm.title, "Example Site");
        assert!(
            utm.url.as_str().contains("utm_source=newsletter"),
            "UTM params must survive import so sanitize can strip them later: {}",
            utm.url.as_str()
        );
        assert_eq!(utm.add_date.map(|d| d.timestamp()), Some(1_700_000_010));

        let Node::Folder(reading) = &bar.children[1] else {
            panic!("expected Reading folder");
        };
        assert_eq!(reading.name, "Reading");
        assert!(!reading.is_toolbar);
        assert_eq!(reading.children.len(), 2);

        let Node::Bookmark(docs) = &reading.children[0] else {
            panic!("expected Rust docs bookmark");
        };
        assert_eq!(docs.title, "Rust docs");
        assert_eq!(docs.url.as_str(), "https://docs.rust-lang.org/");

        let Node::Folder(other) = &doc.root.children[1] else {
            panic!("expected Other bookmarks folder");
        };
        assert_eq!(other.name, "Other bookmarks");
        assert!(other.children.is_empty());
    }

    #[test]
    fn accepts_numeric_timestamps_and_bom() {
        let json = br#"{
            "roots": {
                "bookmark_bar": {
                    "type": "folder",
                    "name": "Bookmarks bar",
                    "date_added": 13344473600000000,
                    "children": [
                        {
                            "type": "url",
                            "name": "Plain",
                            "url": "https://example.org/",
                            "date_added": 13344473600000000
                        }
                    ]
                }
            }
        }"#;
        let mut with_bom = b"\xef\xbb\xbf".to_vec();
        with_bom.extend_from_slice(json);

        let doc = parse_chrome_json(&with_bom).expect("numeric timestamps + BOM");
        assert!(doc.header.has_bom);
        assert_eq!(doc.stats.bookmark_count, 1);

        let Node::Folder(bar) = &doc.root.children[0] else {
            panic!("expected bar");
        };
        assert_eq!(bar.add_date.map(|d| d.timestamp()), Some(1_700_000_000));
    }

    #[test]
    fn malformed_url_is_preserved() {
        let json = br#"{
            "roots": {
                "other": {
                    "type": "folder",
                    "name": "Other bookmarks",
                    "children": [
                        { "type": "url", "name": "Broken", "url": "not a url at all" }
                    ]
                }
            }
        }"#;
        let doc = parse_chrome_json(json).expect("malformed URL must not fail the parse");
        let Node::Folder(other) = &doc.root.children[0] else {
            panic!("expected folder");
        };
        let Node::Bookmark(bm) = &other.children[0] else {
            panic!("expected bookmark");
        };
        assert!(bm.url.is_malformed());
        assert_eq!(bm.url.as_str(), "not a url at all");
    }

    #[test]
    fn skips_unknown_node_types() {
        let json = br#"{
            "roots": {
                "other": {
                    "type": "folder",
                    "name": "Other bookmarks",
                    "children": [
                        { "type": "workspace", "name": "ignored" },
                        { "type": "url", "name": "Kept", "url": "https://example.com/" }
                    ]
                }
            }
        }"#;
        let doc = parse_chrome_json(json).unwrap();
        assert_eq!(doc.stats.bookmark_count, 1);
        assert_eq!(doc.stats.folder_count, 1);
    }

    #[test]
    fn missing_roots_is_an_error() {
        let err = parse_chrome_json(br#"{"version":1}"#).unwrap_err();
        match err {
            CoreError::ParseFailed { reason, .. } => {
                assert!(
                    reason.contains("roots") || reason.contains("Chrome"),
                    "unexpected reason: {reason}"
                );
            }
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    #[test]
    fn empty_roots_is_an_error() {
        let err = parse_chrome_json(br#"{"roots":{}}"#).unwrap_err();
        match err {
            CoreError::ParseFailed { reason, .. } => {
                assert!(reason.contains("empty"), "unexpected reason: {reason}");
            }
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    #[test]
    fn looks_like_json_tolerates_bom_and_whitespace() {
        assert!(looks_like_json(br#"{"roots":{}}"#));
        assert!(looks_like_json(b"\xef\xbb\xbf\n  {\"a\":1}"));
        assert!(looks_like_json(b"  \n\t[1,2]"));
        assert!(!looks_like_json(b"<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(!looks_like_json(b""));
        assert!(!looks_like_json(b"   "));
    }

    #[test]
    fn parse_auto_dispatches_json_and_html() {
        let json_doc = parse_auto(FIXTURE.as_bytes()).expect("JSON via parse_auto");
        assert_eq!(json_doc.stats.bookmark_count, 3);

        let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<TITLE>Bookmarks</TITLE>
<DL><p>
    <DT><A HREF="https://example.com">Example</A>
</DL><p>
"#;
        let html_doc = parse_auto(html).expect("HTML via parse_auto");
        assert_eq!(html_doc.stats.bookmark_count, 1);
    }

    #[test]
    fn extra_managed_root_is_imported() {
        let json = br#"{
            "roots": {
                "bookmark_bar": {
                    "type": "folder",
                    "name": "Bookmarks bar",
                    "children": []
                },
                "managed": {
                    "type": "folder",
                    "name": "Managed bookmarks",
                    "children": [
                        { "type": "url", "name": "Policy", "url": "https://example.com/policy" }
                    ]
                }
            }
        }"#;
        let doc = parse_chrome_json(json).unwrap();
        assert_eq!(doc.root.children.len(), 2);
        let Node::Folder(managed) = &doc.root.children[1] else {
            panic!("expected managed folder");
        };
        assert_eq!(managed.name, "Managed bookmarks");
        assert_eq!(doc.stats.bookmark_count, 1);
    }

    #[test]
    fn extra_roots_keep_file_order() {
        let json = br#"{
            "roots": {
                "zeta":  { "type": "folder", "name": "Z", "children": [] },
                "other": { "type": "folder", "name": "Other", "children": [] },
                "alpha": { "type": "folder", "name": "A", "children": [] },
                "mid":   { "type": "folder", "name": "M", "children": [] },
                "beta":  { "type": "folder", "name": "B", "children": [] }
            }
        }"#;
        let doc = parse_chrome_json(json).unwrap();
        let names: Vec<_> = doc
            .root
            .children
            .iter()
            .map(|n| n.as_folder().unwrap().name.as_str())
            .collect();
        // Known roots first, then the rest exactly as they appear.
        assert_eq!(names, ["Other", "Z", "A", "M", "B"]);
    }
}
