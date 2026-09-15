//! Property tests for Lantern's core invariants.
//!
//! Each property is a contract the rest of the system relies on:
//!
//! - **Parser/emitter round-trip**: `parse(emit(doc))` yields a tree with the
//!   same shape, titles, and URLs as `doc`.  Without this, the "export" button
//!   would silently corrupt files.
//! - **Treatment idempotence**: applying a treatment a second time produces no
//!   additional changes.  Without this, users could see the same change
//!   proposal on every pass.
//! - **Diff reassembly**: the `before_spans` / `after_spans` from `char_diff`
//!   reconstruct their source strings when concatenated.  Without this the
//!   UI would render text that does not match what the backend will apply.
//!
//! These tests are cheap to run (10-100 cases each) and catch a whole class of
//! regressions that unit tests tend to miss.

use lantern_core::emit::{emit, EmitOptions};
use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
use lantern_core::model::ids::{next_document_id, NodeIdAllocator};
use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
use lantern_core::parser::parse;
use lantern_core::sanitize::diff::{char_diff, DiffTag};
use lantern_core::sanitize::pass::{run_pass, PassTarget, RuleSet};
use lantern_core::sanitize::treatment::Treatment;
use lantern_core::sanitize::treatments::{
    HtmlEntitiesTreatment, UtmTreatment, WhitespaceTreatment,
};
use proptest::prelude::*;
use proptest::strategy::Just;

// ---------------------------------------------------------------------------
// Strategy: generate small, well-formed documents
// ---------------------------------------------------------------------------

/// Titles that survive a round-trip cleanly.  We avoid characters that
/// Netscape bookmark HTML requires entity-encoding; the emitter handles them
/// correctly but the equality assertion in the round-trip test would have to
/// apply the inverse decoding, which is orthogonal to what we want to test.
fn title_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 _.\\-]{0,40}".prop_map(|s| s.trim().to_owned())
}

fn folder_name_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 _.\\-]{1,30}".prop_map(|s| {
        let t = s.trim();
        if t.is_empty() {
            "folder".into()
        } else {
            t.to_owned()
        }
    })
}

/// Random well-formed URL.  Trailing slash is always present so the `url`
/// crate's canonicalisation (which always adds one on paths with no segment)
/// doesn't cause false-positive diffs.
fn url_strategy() -> impl Strategy<Value = url::Url> {
    ("(https|http)", "[a-z]{3,10}", "[a-z]{2,3}").prop_map(|(scheme, host, tld)| {
        url::Url::parse(&format!("{scheme}://{host}.{tld}/")).unwrap()
    })
}

/// Generate a small flat document (one root with up to 8 bookmark/folder
/// children).  Deep recursion is unnecessary for the round-trip property and
/// would blow up shrinking time.
fn document_strategy() -> impl Strategy<Value = Document> {
    prop::collection::vec(any::<bool>(), 0..8)
        .prop_flat_map(|child_kinds| {
            let n = child_kinds.len();
            (
                Just(child_kinds),
                prop::collection::vec(title_strategy(), n),
                prop::collection::vec(url_strategy(), n),
                prop::collection::vec(folder_name_strategy(), n),
            )
        })
        .prop_map(|(child_kinds, titles, urls, folder_names)| {
            let mut alloc = NodeIdAllocator::new();
            let mut children = Vec::with_capacity(child_kinds.len());
            for (i, is_folder) in child_kinds.iter().enumerate() {
                let id = alloc.alloc();
                if *is_folder {
                    children.push(Node::Folder(Folder {
                        id,
                        name: folder_names[i].clone(),
                        add_date: None,
                        last_modified: None,
                        is_toolbar: false,
                        attrs: AttrMap::default(),
                        children: Vec::new(),
                    }));
                } else {
                    children.push(Node::Bookmark(Bookmark {
                        id,
                        title: titles[i].clone(),
                        url: BookmarkUrl::Valid(urls[i].clone()),
                        add_date: None,
                        last_modified: None,
                        icon_blob: None,
                        description: None,
                        attrs: AttrMap::default(),
                        flags: BookmarkFlags::default(),
                    }));
                }
            }
            let root = Folder {
                id: 0,
                name: "Bookmarks".into(),
                add_date: None,
                last_modified: None,
                is_toolbar: false,
                attrs: AttrMap::default(),
                children,
            };
            let stats = DocumentStats::from_root(&root);
            Document::new(
                next_document_id(),
                None,
                root,
                HeaderMetadata::default(),
                stats,
            )
        })
}

// ---------------------------------------------------------------------------
// Structural comparison helpers
// ---------------------------------------------------------------------------

/// Project a folder tree to the shape we care about for round-trip equality:
/// ordered list of (kind, display_text) tuples.  Ignores node IDs (which are
/// re-allocated on parse) and timestamps (which we leave `None`).
fn project(folder: &Folder) -> Vec<(char, String)> {
    folder
        .children
        .iter()
        .map(|n| match n {
            Node::Bookmark(b) => ('B', format!("{}|{}", b.title, b.url)),
            Node::Folder(f) => ('F', f.name.clone()),
            Node::Separator(_) => ('S', String::new()),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Round-trip property
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// `parse(emit(doc))` yields a document with the same flat structure and
    /// the same titles/URLs as `doc`.
    #[test]
    fn parse_of_emit_preserves_structure(doc in document_strategy()) {
        let bytes = emit(&doc, &EmitOptions::default());
        let reparsed = parse(&bytes).expect("valid emitted document must parse");
        prop_assert_eq!(project(&doc.root), project(&reparsed.root));
    }
}

// ---------------------------------------------------------------------------
// Treatment idempotence property
// ---------------------------------------------------------------------------

/// Convenience: single-treatment rule set.
fn ruleset_with(id: &str, treatments: Vec<Box<dyn Treatment>>) -> RuleSet {
    RuleSet {
        id: id.to_owned(),
        name: id.to_owned(),
        treatments,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// Running `title.whitespace` a second time on an already-cleaned document
    /// yields zero new changes.  This is the core "apply → clean state" contract
    /// users rely on when they re-run a pass to double-check.
    #[test]
    fn title_whitespace_is_idempotent(mut doc in document_strategy()) {
        let rs = ruleset_with("title.whitespace", vec![Box::new(WhitespaceTreatment)]);

        // Pass 1: apply everything.
        let mut cs = run_pass(&doc, &rs, PassTarget::AllNodes);
        for c in &mut cs.changes { c.approved = true; }
        if !cs.changes.is_empty() {
            doc.apply(&cs).expect("first apply must succeed");
        }

        // Pass 2: no new proposals.
        let cs2 = run_pass(&doc, &rs, PassTarget::AllNodes);
        prop_assert!(cs2.changes.is_empty(),
                     "expected no further changes, got {}", cs2.changes.len());
    }

    /// Same property for the HTML-entity title decoder.
    #[test]
    fn title_html_entities_is_idempotent(mut doc in document_strategy()) {
        let rs = ruleset_with("title.html_entities", vec![Box::new(HtmlEntitiesTreatment)]);

        let mut cs = run_pass(&doc, &rs, PassTarget::AllNodes);
        for c in &mut cs.changes { c.approved = true; }
        if !cs.changes.is_empty() {
            doc.apply(&cs).expect("first apply must succeed");
        }

        let cs2 = run_pass(&doc, &rs, PassTarget::AllNodes);
        prop_assert!(cs2.changes.is_empty());
    }

    /// UTM-param stripping is idempotent: once removed the second pass has
    /// nothing left to find.
    #[test]
    fn url_utm_strip_is_idempotent(mut doc in document_strategy()) {
        let rs = ruleset_with("url.qp.utm", vec![Box::new(UtmTreatment)]);

        let mut cs = run_pass(&doc, &rs, PassTarget::AllNodes);
        for c in &mut cs.changes { c.approved = true; }
        if !cs.changes.is_empty() {
            doc.apply(&cs).expect("first apply must succeed");
        }

        let cs2 = run_pass(&doc, &rs, PassTarget::AllNodes);
        prop_assert!(cs2.changes.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Diff-span reassembly property
// ---------------------------------------------------------------------------

fn reassemble(spans: &[lantern_core::sanitize::diff::DiffSpan], side: DiffTag) -> String {
    spans
        .iter()
        .filter(|s| match side {
            DiffTag::Removed => !matches!(s.tag, DiffTag::Added),
            DiffTag::Added => !matches!(s.tag, DiffTag::Removed),
            DiffTag::Equal => true,
        })
        .map(|s| s.text.as_str())
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// For any two strings, concatenating the non-Added spans recovers the
    /// original `before`, and concatenating the non-Removed spans recovers
    /// `after`.  This is what the UI depends on when it renders diffs.
    #[test]
    fn diff_spans_reassemble_to_sources(
        before in "[A-Za-z0-9 ?=&./:_-]{0,120}",
        after  in "[A-Za-z0-9 ?=&./:_-]{0,120}",
    ) {
        let (b_spans, a_spans) = char_diff(&before, &after);
        prop_assert_eq!(reassemble(&b_spans, DiffTag::Removed), before.clone());
        prop_assert_eq!(reassemble(&a_spans, DiffTag::Added), after.clone());
    }
}
