//! Exact-URL duplicate detection.
//!
//! | ID                                | Name                              |
//! |-----------------------------------|-----------------------------------|
//! | `structure.duplicates.exact_url`  | Find exact-URL duplicate bookmarks |
//!
//! This is the conservative first-cut: two bookmarks are duplicates only
//! when their URLs match after a light canonicalisation (lowercase host,
//! strip a trailing slash).  Query-parameter and fragment differences are
//! **not** collapsed here — those are near-duplicates for a later slice.
//!
//! The pass only *proposes* [`Change::delete_node`] for the extras in each
//! group.  Nothing is removed until the user (or the CLI, which
//! auto-approves) applies the changeset.

use std::collections::HashMap;

use crate::model::document::Document;
use crate::model::node::{Bookmark, BookmarkUrl, Folder, Node};
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};

/// Treatment ID for the exact-URL duplicate pass.
pub const EXACT_URL_DUPLICATES_ID: &str = "structure.duplicates.exact_url";

/// Finds bookmarks that share an exact URL and proposes deleting the extras.
///
/// Default keeper: the oldest `ADD_DATE` when both sides have one, otherwise
/// the first-seen bookmark in DFS order.  Every proposed deletion is
/// destructive and unapproved so a reviewer can keep any extra.
pub struct ExactUrlDuplicatesTreatment;

impl Treatment for ExactUrlDuplicatesTreatment {
    fn id(&self) -> &'static str {
        EXACT_URL_DUPLICATES_ID
    }
    fn name(&self) -> &'static str {
        "Find exact-URL duplicate bookmarks"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::CrossField
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, _node: &Node, _ctx: &PassContext) -> Vec<Change> {
        vec![] // all work done in propose_document
    }

    fn propose_document(&self, doc: &Document, _ctx: &PassContext) -> Vec<Change> {
        let mut bookmarks = Vec::new();
        collect_bookmarks(&doc.root, &mut bookmarks);
        let keys: Vec<String> = bookmarks.iter().map(|b| exact_url_key(&b.url)).collect();

        // Keeper per URL: the oldest `ADD_DATE` when both sides have one,
        // otherwise the first-seen bookmark.
        let mut keepers: HashMap<&str, &Bookmark> = HashMap::new();
        for (bookmark, key) in bookmarks.iter().zip(&keys) {
            keepers
                .entry(key.as_str())
                .and_modify(|keeper| {
                    if let (Some(kept), Some(this)) = (keeper.add_date, bookmark.add_date) {
                        if this < kept {
                            *keeper = bookmark;
                        }
                    }
                })
                .or_insert(bookmark);
        }

        // Emit the extras in document order so the preview is stable.
        bookmarks
            .iter()
            .zip(&keys)
            .filter_map(|(bookmark, key)| {
                let keeper = keepers[key.as_str()];
                (keeper.id != bookmark.id).then(|| {
                    Change::delete_node(
                        bookmark.id,
                        self.id(),
                        format!(
                            "Exact-URL duplicate of an older (or first-seen) bookmark; \
                             keeper node {} has the same URL",
                            keeper.id
                        ),
                    )
                })
            })
            .collect()
    }
}

/// Light exact-URL key: host is already lowercased by the `url` crate;
/// a trailing slash on a non-root path is stripped.  Query string and
/// fragment are kept verbatim so `?a=1` and `?a=2` stay distinct.
/// Malformed URLs are keyed by their raw text.
fn exact_url_key(url: &BookmarkUrl) -> String {
    match url {
        BookmarkUrl::Valid(u) => {
            let path = u.path();
            if path.len() > 1 && path.ends_with('/') {
                let mut trimmed = u.clone();
                trimmed.set_path(path.trim_end_matches('/'));
                trimmed.into()
            } else {
                u.as_str().to_owned()
            }
        }
        BookmarkUrl::Malformed { raw } => raw.clone(),
    }
}

/// Every bookmark below `folder`, in document (DFS) order.
fn collect_bookmarks<'a>(folder: &'a Folder, out: &mut Vec<&'a Bookmark>) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => out.push(b),
            Node::Folder(f) => collect_bookmarks(f, out),
            Node::Separator(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{Document, DocumentStats, HeaderMetadata};
    use crate::model::ids::next_document_id;
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder};
    use crate::sanitize::pass::{run_pass, PassTarget, RuleSet};
    use crate::sanitize::treatment::{ChangeKind, PassContext};
    use chrono::{DateTime, TimeZone, Utc};

    fn ctx() -> PassContext {
        PassContext { document_id: 0 }
    }

    fn bm(id: u64, href: &str) -> Node {
        bm_dated(id, href, None)
    }

    fn bm_dated(id: u64, href: &str, add_date: Option<DateTime<Utc>>) -> Node {
        let url = url::Url::parse(href).unwrap();
        Node::Bookmark(Bookmark {
            id,
            title: format!("T{id}"),
            url: BookmarkUrl::Valid(url),
            add_date,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        })
    }

    fn make_doc_with_root(children: Vec<Node>) -> Document {
        let root = Folder {
            id: 99,
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
    }

    fn propose(doc: &Document) -> Vec<Change> {
        ExactUrlDuplicatesTreatment.propose_document(doc, &ctx())
    }

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).single().unwrap()
    }

    // ── grouping ──────────────────────────────────────────────────────────

    #[test]
    fn exact_dupes_collapse() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/page"),
            bm(2, "https://a.com/page"),
        ]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 2);
        assert!(matches!(changes[0].kind, ChangeKind::DeleteNode));
    }

    #[test]
    fn different_urls_do_not_collapse() {
        let doc = make_doc_with_root(vec![bm(1, "https://a.com/one"), bm(2, "https://a.com/two")]);
        assert!(propose(&doc).is_empty());
    }

    #[test]
    fn query_param_differences_are_not_collapsed() {
        // Near-dupe work is a later slice; this pass is exact-URL only.
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/page?ref=1"),
            bm(2, "https://a.com/page?ref=2"),
        ]);
        assert!(propose(&doc).is_empty());
    }

    #[test]
    fn query_param_order_is_not_canonicalised() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/?b=2&a=1"),
            bm(2, "https://a.com/?a=1&b=2"),
        ]);
        assert!(propose(&doc).is_empty());
    }

    #[test]
    fn fragment_differences_are_not_collapsed() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/page"),
            bm(2, "https://a.com/page#section"),
        ]);
        assert!(propose(&doc).is_empty());
    }

    #[test]
    fn trailing_slash_is_canonicalised() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/page"),
            bm(2, "https://a.com/page/"),
        ]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 2);
    }

    #[test]
    fn host_case_is_canonicalised() {
        // The url crate lowercases the host on parse.
        let doc = make_doc_with_root(vec![
            bm(1, "https://Example.COM/page"),
            bm(2, "https://example.com/page"),
        ]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 2);
    }

    #[test]
    fn three_copies_keep_first_seen_when_undated() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/"),
            bm(2, "https://a.com/"),
            bm(3, "https://a.com/"),
        ]);
        let changes = propose(&doc);
        let ids: Vec<_> = changes.iter().map(|c| c.node_id).collect();
        assert_eq!(ids, vec![2, 3]);
    }

    #[test]
    fn keeps_oldest_add_date() {
        let doc = make_doc_with_root(vec![
            bm_dated(1, "https://a.com/page", Some(ts(2_000))),
            bm_dated(2, "https://a.com/page", Some(ts(1_000))),
            bm_dated(3, "https://a.com/page", Some(ts(3_000))),
        ]);
        let changes = propose(&doc);
        let ids: Vec<_> = changes.iter().map(|c| c.node_id).collect();
        assert_eq!(ids, vec![1, 3]); // node 2 is oldest
        assert!(changes
            .iter()
            .all(|c| c.rationale.contains("keeper node 2")));
    }

    #[test]
    fn finds_duplicates_across_folders() {
        let subfolder = Node::Folder(Folder {
            id: 50,
            name: "Sub".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![bm(2, "https://a.com/")],
        });
        let doc = make_doc_with_root(vec![bm(1, "https://a.com/"), subfolder]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 2);
    }

    #[test]
    fn deletes_are_destructive_and_unapproved() {
        let doc = make_doc_with_root(vec![bm(1, "https://a.com/"), bm(2, "https://a.com/")]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
        assert_eq!(changes[0].treatment_id, EXACT_URL_DUPLICATES_ID);
    }

    #[test]
    fn unique_and_duplicate_groups_coexist() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://keep-me.com/"),
            bm(2, "https://dup.com/"),
            bm(3, "https://dup.com/"),
            bm(4, "https://also-unique.com/"),
        ]);
        let changes = propose(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 3);
    }

    #[test]
    fn run_pass_emits_unapproved_deletes_until_apply() {
        let mut doc = make_doc_with_root(vec![bm(1, "https://a.com/"), bm(2, "https://a.com/")]);
        let rs = RuleSet {
            id: "find-duplicates".into(),
            name: "Find duplicates".into(),
            treatments: vec![Box::new(ExactUrlDuplicatesTreatment)],
        };
        let mut cs = run_pass(&doc, &rs, PassTarget::AllNodes);
        assert_eq!(cs.changes.len(), 1);
        assert!(!cs.changes[0].approved);
        doc.apply(&cs).unwrap();
        assert_eq!(
            doc.stats.bookmark_count, 2,
            "unapproved delete must not apply"
        );

        cs.changes[0].approved = true;
        doc.apply(&cs).unwrap();
        assert_eq!(doc.stats.bookmark_count, 1);
        assert_eq!(doc.root.children[0].id(), 1);
    }

    #[test]
    fn deletes_are_listed_in_document_order() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/"),
            bm(2, "https://b.com/"),
            bm(3, "https://c.com/"),
            bm(4, "https://c.com/"),
            bm(5, "https://b.com/"),
            bm(6, "https://a.com/"),
        ]);
        let ids: Vec<_> = propose(&doc).iter().map(|c| c.node_id).collect();
        assert_eq!(ids, vec![4, 5, 6]);
    }

    // ── exact_url_key ─────────────────────────────────────────────────────

    #[test]
    fn key_strips_trailing_slash_but_keeps_query() {
        let key = |href: &str| exact_url_key(&BookmarkUrl::Valid(url::Url::parse(href).unwrap()));
        assert_eq!(
            key("https://a.com/page/?q=1"),
            key("https://a.com/page?q=1")
        );
        assert_ne!(key("https://a.com/page?q=1"), key("https://a.com/page?q=2"));
    }

    #[test]
    fn key_root_slash_stays_root() {
        let u = url::Url::parse("https://a.com/").unwrap();
        assert_eq!(exact_url_key(&BookmarkUrl::Valid(u)), "https://a.com/");
    }
}
