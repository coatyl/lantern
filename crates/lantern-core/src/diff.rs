//! Document-vs-document diff for the v0.0.4 "compare snapshots" feature.
//!
//! The diff is **URL-keyed**: two bookmarks compare equal if and only if their
//! URL strings match byte-for-byte after parsing.  Folder structure is ignored
//! (treating moves as no-ops); titles that change between two snapshots are
//! reported as `Modified`.  Folders themselves are not currently diffed
//! (only the bookmarks they contain) because folders rarely carry user-
//! observable identity in a typical "did this scrub change anything" workflow.
//!
//! # Output shape
//!
//! [`DocDiff`] groups the result into three buckets:
//!
//! - `added`:      present in `right` only.
//! - `removed`:    present in `left` only.
//! - `modified`:   present in both, but the title (or another tracked field)
//!   differs.  Each entry carries both the `before` and `after` view.
//!
//! Within a bucket, entries are sorted by URL so the output is deterministic
//! and friendly to text-based testing.
//!
//! # Performance
//!
//! O(N + M) in the total bookmark count of the two documents.  Both sides are
//! flattened into a `BTreeMap<url, BookmarkSnapshot>`; iterating both maps in
//! lockstep gives the diff with no extra allocations beyond the result itself.

use std::collections::BTreeMap;

use crate::model::document::Document;
use crate::model::node::Node;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Compact view of a bookmark used for diff reporting.
///
/// Carries everything the UI needs to render a "this side" row without holding
/// references back into the original `Document`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkSnapshot {
    pub url: String,
    pub title: String,
}

/// One bookmark that exists on both sides but differs in some way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedBookmark {
    pub before: BookmarkSnapshot,
    pub after: BookmarkSnapshot,
}

/// Three-way diff between two documents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocDiff {
    /// Bookmarks present in the right document but not in the left.
    pub added: Vec<BookmarkSnapshot>,
    /// Bookmarks present in the left document but not in the right.
    pub removed: Vec<BookmarkSnapshot>,
    /// Bookmarks whose URL appears on both sides but with a different title.
    pub modified: Vec<ModifiedBookmark>,
}

impl DocDiff {
    /// True when both sides are bookmark-equivalent.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Total number of differing bookmarks (added + removed + modified).
    pub fn total(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Compute a [`DocDiff`] between `left` and `right`.
///
/// Bookmarks with identical URL and identical title are considered equal and
/// do not appear in the result.  Duplicate URLs within a single document are
/// collapsed to their last occurrence; this matches the on-disk emitter's
/// dedup behaviour.
pub fn diff_documents(left: &Document, right: &Document) -> DocDiff {
    let l = collect_bookmarks(left);
    let r = collect_bookmarks(right);

    let mut diff = DocDiff::default();

    // Walk the left side: anything missing from the right is `removed`,
    // anything present on both with a different title is `modified`.
    for (url, before) in &l {
        match r.get(url) {
            None => diff.removed.push(before.clone()),
            Some(after) if after.title != before.title => {
                diff.modified.push(ModifiedBookmark {
                    before: before.clone(),
                    after: after.clone(),
                });
            }
            Some(_) => {} // identical
        }
    }

    // Walk the right side: anything missing from the left is `added`.
    for (url, after) in &r {
        if !l.contains_key(url) {
            diff.added.push(after.clone());
        }
    }

    diff
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_bookmarks(doc: &Document) -> BTreeMap<String, BookmarkSnapshot> {
    let mut out = BTreeMap::new();
    walk(&doc.root, &mut out);
    out
}

fn walk(folder: &crate::model::node::Folder, out: &mut BTreeMap<String, BookmarkSnapshot>) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                out.insert(
                    b.url.as_str().to_owned(),
                    BookmarkSnapshot {
                        url: b.url.as_str().to_owned(),
                        title: b.title.clone(),
                    },
                );
            }
            Node::Folder(f) => walk(f, out),
            _ => {}
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
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};

    fn bm(id: u64, url: &str, title: &str) -> Node {
        Node::Bookmark(Bookmark {
            id,
            title: title.into(),
            url: url::Url::parse(url)
                .map(BookmarkUrl::Valid)
                .unwrap_or_else(|_| BookmarkUrl::Malformed { raw: url.into() }),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        })
    }

    fn doc_with(children: Vec<Node>) -> Document {
        let root = Folder {
            id: 1,
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

    #[test]
    fn identical_documents_produce_empty_diff() {
        let a = doc_with(vec![bm(1, "https://a.example/", "A")]);
        let b = doc_with(vec![bm(2, "https://a.example/", "A")]);
        let diff = diff_documents(&a, &b);
        assert!(diff.is_empty());
        assert_eq!(diff.total(), 0);
    }

    #[test]
    fn detects_added_bookmark() {
        let a = doc_with(vec![bm(1, "https://a.example/", "A")]);
        let b = doc_with(vec![
            bm(2, "https://a.example/", "A"),
            bm(3, "https://b.example/", "B"),
        ]);
        let diff = diff_documents(&a, &b);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].url, "https://b.example/");
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn detects_removed_bookmark() {
        let a = doc_with(vec![
            bm(1, "https://a.example/", "A"),
            bm(2, "https://b.example/", "B"),
        ]);
        let b = doc_with(vec![bm(3, "https://a.example/", "A")]);
        let diff = diff_documents(&a, &b);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].url, "https://b.example/");
        assert!(diff.added.is_empty());
    }

    #[test]
    fn detects_title_change_as_modified() {
        let a = doc_with(vec![bm(1, "https://a.example/", "Old title")]);
        let b = doc_with(vec![bm(2, "https://a.example/", "New title")]);
        let diff = diff_documents(&a, &b);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].before.title, "Old title");
        assert_eq!(diff.modified[0].after.title, "New title");
    }

    fn folder(id: u64, name: &str, children: Vec<Node>) -> Node {
        Node::Folder(Folder {
            id,
            name: name.into(),
            children,
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
        })
    }

    #[test]
    fn diff_walks_into_folders() {
        let nested_a = folder(
            10,
            "Inbox",
            vec![bm(11, "https://nested.example/", "Nested")],
        );
        let a = doc_with(vec![nested_a]);
        let b = doc_with(vec![]);
        let diff = diff_documents(&a, &b);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].url, "https://nested.example/");
    }

    #[test]
    fn diff_ignores_folder_moves() {
        // Same bookmark, but reparented under a folder.  The diff should be
        // empty since URL identity is what we key on.
        let nested = folder(10, "Inbox", vec![bm(11, "https://x.example/", "X")]);
        let a = doc_with(vec![bm(1, "https://x.example/", "X")]);
        let b = doc_with(vec![nested]);
        let diff = diff_documents(&a, &b);
        assert!(diff.is_empty());
    }

    #[test]
    fn diff_total_counts_each_bucket() {
        let a = doc_with(vec![
            bm(1, "https://a.example/", "A"),
            bm(2, "https://b.example/", "B"),
        ]);
        let b = doc_with(vec![
            bm(3, "https://a.example/", "A renamed"),
            bm(4, "https://c.example/", "C"),
        ]);
        let diff = diff_documents(&a, &b);
        assert_eq!(diff.added.len(), 1); // c
        assert_eq!(diff.removed.len(), 1); // b
        assert_eq!(diff.modified.len(), 1); // a (title)
        assert_eq!(diff.total(), 3);
        assert!(!diff.is_empty());
    }
}
