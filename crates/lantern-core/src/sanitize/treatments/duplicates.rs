//! Duplicate-bookmark detection.
//!
//! | ID                                | Name                                  |
//! |-----------------------------------|---------------------------------------|
//! | `structure.duplicates.exact_url`  | Find exact-URL duplicate bookmarks    |
//! | `structure.duplicates.near_url`   | Find near-duplicate bookmarks         |
//!
//! Both group bookmarks by a URL key and propose [`Change::delete_node`] for
//! every bookmark in a group except the keeper: the oldest `ADD_DATE` when
//! both sides have one, otherwise the first in document order.  Proposals
//! are destructive and unapproved, so nothing is removed until the user
//! selects it in the review (the CLI's non-dry-run `sanitize` auto-approves).
//!
//! - **exact**: the URL with only a trailing slash on a non-root path
//!   removed.  `?a=1` and `?a=2`, `#x` and `#y` stay distinct.
//! - **near**: additionally treats `http` and `https`, a leading `www.`,
//!   the fragment, tracking parameters (`utm_*`, click ids) and query
//!   parameter order as the same page.  Other query differences still
//!   count: `?page=1` and `?page=2` are different bookmarks.

use std::collections::HashMap;

use url::Url;

use crate::model::document::Document;
use crate::model::node::{Bookmark, BookmarkUrl, Folder, Node};
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};
use crate::sanitize::treatments::url_qp::is_tracking_param;

/// Treatment ID for the exact-URL duplicate pass.
pub const EXACT_URL_DUPLICATES_ID: &str = "structure.duplicates.exact_url";
/// Treatment ID for the near-duplicate pass.
pub const NEAR_URL_DUPLICATES_ID: &str = "structure.duplicates.near_url";

/// Finds bookmarks whose URLs are identical up to a trailing slash.
pub struct ExactUrlDuplicatesTreatment;

/// Finds bookmarks that point at the same page: [`ExactUrlDuplicatesTreatment`]
/// plus scheme, `www.`, fragment, tracking-parameter and parameter-order
/// differences.  Its groups include every exact-duplicate group.
pub struct NearUrlDuplicatesTreatment;

macro_rules! duplicate_treatment {
    ($ty:ty, $id:expr, $name:expr, $key:expr) => {
        impl Treatment for $ty {
            fn id(&self) -> &'static str {
                $id
            }
            fn name(&self) -> &'static str {
                $name
            }
            fn category(&self) -> TreatmentCategory {
                TreatmentCategory::CrossField
            }
            fn is_destructive(&self) -> bool {
                true
            }
            fn propose(&self, _node: &Node, _ctx: &PassContext) -> Vec<Change> {
                vec![] // all work happens in propose_document
            }
            fn propose_document(&self, doc: &Document, _ctx: &PassContext) -> Vec<Change> {
                propose_duplicates(doc, $id, $key)
            }
        }
    };
}

duplicate_treatment!(
    ExactUrlDuplicatesTreatment,
    EXACT_URL_DUPLICATES_ID,
    "Find exact-URL duplicate bookmarks",
    exact_url_key
);
duplicate_treatment!(
    NearUrlDuplicatesTreatment,
    NEAR_URL_DUPLICATES_ID,
    "Find near-duplicate bookmarks",
    near_url_key
);

/// Group bookmarks by `key` and propose deleting every non-keeper, in
/// document order so the review is stable.
fn propose_duplicates(
    doc: &Document,
    treatment_id: &'static str,
    key: fn(&BookmarkUrl) -> String,
) -> Vec<Change> {
    let mut bookmarks = Vec::new();
    collect_bookmarks(&doc.root, &mut bookmarks);
    let keys: Vec<String> = bookmarks.iter().map(|b| key(&b.url)).collect();

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

    bookmarks
        .iter()
        .zip(&keys)
        .filter_map(|(bookmark, key)| {
            let keeper = keepers[key.as_str()];
            (keeper.id != bookmark.id).then(|| {
                Change::delete_node(bookmark.id, treatment_id, rationale(bookmark, keeper))
            })
        })
        .collect()
}

/// "Same page as “Keeper”, which is older; differs only in …".
fn rationale(bookmark: &Bookmark, keeper: &Bookmark) -> String {
    let title = if keeper.title.trim().is_empty() {
        keeper.url.as_str()
    } else {
        keeper.title.trim()
    };
    let why = match (bookmark.add_date, keeper.add_date) {
        (Some(_), Some(_)) => "older",
        _ => "first in the document",
    };
    let differences = match (&bookmark.url, &keeper.url) {
        (BookmarkUrl::Valid(a), BookmarkUrl::Valid(b)) => describe_differences(a, b),
        _ => Vec::new(),
    };
    if differences.is_empty() {
        format!("Same URL as “{title}”, which is kept ({why})")
    } else {
        format!(
            "Same page as “{title}”, which is kept ({why}); differs only in {}",
            differences.join(", ")
        )
    }
}

/// The ways two near-duplicate URLs differ, for the review.
fn describe_differences(a: &Url, b: &Url) -> Vec<&'static str> {
    let mut out = Vec::new();
    if a.scheme() != b.scheme() {
        out.push("http vs https");
    }
    let (ha, hb) = (a.host_str().unwrap_or(""), b.host_str().unwrap_or(""));
    if ha != hb && ha.trim_start_matches("www.") == hb.trim_start_matches("www.") {
        out.push("www.");
    }
    if a.path() != b.path() && a.path().trim_end_matches('/') == b.path().trim_end_matches('/') {
        out.push("trailing slash");
    }
    if a.query() != b.query() {
        let has_tracking = |u: &Url| u.query_pairs().any(|(k, _)| is_tracking_param(&k));
        out.push(if has_tracking(a) || has_tracking(b) {
            "tracking parameters"
        } else {
            "parameter order"
        });
    }
    if a.fragment() != b.fragment() {
        out.push("#fragment");
    }
    out
}

/// Exact key: a trailing slash on a non-root path is ignored; everything
/// else, query and fragment included, must match.  The `url` crate has
/// already lowercased the scheme and host.  Malformed URLs key on their
/// raw text.
fn exact_url_key(url: &BookmarkUrl) -> String {
    match url {
        BookmarkUrl::Valid(u) => {
            let mut u = u.clone();
            trim_trailing_slash(&mut u);
            u.into()
        }
        BookmarkUrl::Malformed { raw } => raw.clone(),
    }
}

/// Near key: the exact key after forcing `https`, dropping a leading
/// `www.`, the fragment and tracking parameters, and sorting the remaining
/// query parameters.
fn near_url_key(url: &BookmarkUrl) -> String {
    let BookmarkUrl::Valid(u) = url else {
        return exact_url_key(url);
    };
    let mut u = u.clone();
    if u.scheme() == "http" {
        let _ = u.set_scheme("https");
    }
    if let Some(host) = u
        .host_str()
        .and_then(|h| h.strip_prefix("www."))
        .map(str::to_owned)
    {
        let _ = u.set_host(Some(&host));
    }
    u.set_fragment(None);
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !is_tracking_param(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    if pairs.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut().clear().extend_pairs(&pairs);
    }
    trim_trailing_slash(&mut u);
    u.into()
}

fn trim_trailing_slash(u: &mut Url) {
    let path = u.path();
    if path.len() > 1 && path.ends_with('/') {
        let trimmed = path.trim_end_matches('/').to_owned();
        u.set_path(&trimmed);
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
        assert!(changes.iter().all(|c| c
            .rationale
            .starts_with("Same URL as “T2”, which is kept (older)")));
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

    // ── near duplicates ───────────────────────────────────────────────────

    fn near(doc: &Document) -> Vec<Change> {
        NearUrlDuplicatesTreatment.propose_document(doc, &ctx())
    }

    #[test]
    fn near_key_equates_the_same_page_and_nothing_else() {
        let key = |href: &str| near_url_key(&BookmarkUrl::Valid(url::Url::parse(href).unwrap()));
        let base = key("https://example.com/docs?page=2&lang=en");
        for same in [
            "http://example.com/docs?page=2&lang=en",
            "https://www.example.com/docs?page=2&lang=en",
            "https://example.com/docs/?page=2&lang=en",
            "https://example.com/docs?lang=en&page=2",
            "https://example.com/docs?page=2&lang=en#intro",
            "https://example.com/docs?utm_source=x&page=2&fbclid=y&lang=en",
        ] {
            assert_eq!(key(same), base, "{same}");
        }
        for different in [
            "https://example.com/docs?page=3&lang=en",
            "https://example.com/docs?page=2",
            "https://example.com/Docs?page=2&lang=en",
            "https://docs.example.com/docs?page=2&lang=en",
        ] {
            assert_ne!(key(different), base, "{different}");
        }
    }

    #[test]
    fn near_duplicates_name_the_keeper_and_what_differs() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://example.com/page"),
            bm(2, "http://www.example.com/page/?utm_source=mail#top"),
            bm(3, "https://example.com/page?id=7"),
        ]);
        let changes = near(&doc);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 2);
        assert_eq!(
            changes[0].rationale,
            "Same page as “T1”, which is kept (first in the document); differs only in \
             http vs https, www., trailing slash, tracking parameters, #fragment"
        );
        assert!(changes[0].destructive && !changes[0].approved);
    }

    #[test]
    fn near_groups_include_every_exact_group() {
        let doc = make_doc_with_root(vec![
            bm(1, "https://a.com/x"),
            bm(2, "https://a.com/x/"),
            bm(3, "http://a.com/x#y"),
            bm(4, "https://b.com/"),
        ]);
        let exact: Vec<_> = propose(&doc).iter().map(|c| c.node_id).collect();
        let near: Vec<_> = near(&doc).iter().map(|c| c.node_id).collect();
        assert_eq!(exact, vec![2]);
        assert_eq!(near, vec![2, 3]);
        assert!(exact.iter().all(|id| near.contains(id)));
    }
}
