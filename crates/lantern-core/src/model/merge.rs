//! Cross-document merge: build a brand-new `Document` from picked subtrees of
//! existing documents.
//!
//! Implements the spec ratified in ADR-0009 (cross-document merge / `NodeId`
//! scope).  The merge produces a self-contained `Document` whose tree is
//! deep-cloned from the source(s) with **all** `NodeId`s reassigned by a fresh
//! `NodeIdAllocator`.  Source documents are read-only inputs and never touched.
//!
//! # Conflict resolution
//!
//! When picks land sibling subtrees under the same merged-tree parent, the
//! [`ConflictStrategy`] decides who wins for **bookmark URL collisions**:
//!
//! - [`ConflictStrategy::KeepFirst`]: drop a bookmark if a sibling with the
//!   same URL was already added.
//! - [`ConflictStrategy::KeepNewest`]: keep the bookmark with the most-recent
//!   `add_date`.  Ties resolve to the first one seen (i.e. KeepFirst).
//! - [`ConflictStrategy::KeepBoth`]: keep both; the second occurrence's title
//!   gets a `" (2)"` suffix to disambiguate (and `" (3)"`, `" (4)"`, …
//!   thereafter).
//!
//! Folders with the same name landing under the same parent are **merged**:
//! their children are unioned and the conflict strategy applies recursively
//! inside.  Separators always pass through.
//!
//! # Invariants
//!
//! - Every `NodeId` reachable from the merged root was issued by the new
//!   document's `id_gen`; no source `NodeId` is reused.
//! - The merged document has empty `undo_stack` / `redo_stack` (the merge
//!   itself is a creation event, not a recordable edit; see ADR-0009).
//! - The merged document has `path: None` (no on-disk identity until the user
//!   exports it).

use thiserror::Error;

use crate::model::document::{Document, DocumentStats, HeaderMetadata};
use crate::model::ids::{next_document_id, NodeId, NodeIdAllocator};
use crate::model::node::{AttrMap, Bookmark, Folder, Node, Separator};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// How to resolve URL collisions among bookmarks landing under the same parent
/// in the merged tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Drop a bookmark if a sibling with the same URL was already added.
    KeepFirst,
    /// Keep the bookmark with the most-recent `add_date`.  Ties resolve to
    /// the first one seen (i.e. equivalent to `KeepFirst` on a tie).
    KeepNewest,
    /// Keep both; later duplicates have `" (n)"` appended to their title.
    KeepBoth,
}

/// One subtree to pull into the merged document.
///
/// `source_doc_id` is an index into the `sources` slice passed to
/// [`build_merged_document`].  `root_node_id` is the picked subtree root;
/// any node kind (folder, bookmark, separator) is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergePick {
    pub source_doc_id: usize,
    pub root_node_id: NodeId,
}

/// Plan handed to [`build_merged_document`].
#[derive(Debug, Clone)]
pub struct MergePlan {
    pub picks: Vec<MergePick>,
    pub strategy: ConflictStrategy,
    pub merged_root_name: String,
}

/// Errors returned by [`build_merged_document`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MergeError {
    /// `plan.picks` was empty: nothing to merge.
    #[error("merge plan has no picks")]
    EmptyPlan,
    /// A pick referenced a `source_doc_id` that is out of range for the
    /// `sources` slice.
    #[error("source index {idx} out of range")]
    SourceIndexOutOfRange { idx: usize },
    /// A pick referenced a `root_node_id` that does not exist inside the
    /// named source document.
    #[error("node id {node_id} not found in source document {source_idx}")]
    NodeIdNotFound { source_idx: usize, node_id: NodeId },
    /// `plan.merged_root_name` was empty (or whitespace only).
    #[error("merged root name must not be empty")]
    EmptyMergedRootName,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Build a brand-new `Document` from the picked subtrees in `sources`.
///
/// See module docs for the full contract.  Validates the plan up-front, then
/// folds each picked subtree into a fresh root folder named
/// `plan.merged_root_name`, applying same-name folder merging and the
/// configured bookmark-URL conflict strategy.
pub fn build_merged_document(
    sources: &[&Document],
    plan: &MergePlan,
) -> Result<Document, MergeError> {
    if plan.picks.is_empty() {
        return Err(MergeError::EmptyPlan);
    }
    if plan.merged_root_name.trim().is_empty() {
        return Err(MergeError::EmptyMergedRootName);
    }

    // Validate every pick before allocating anything so a bad plan does not
    // leave the merged-doc allocator with one or two ids burned.
    for pick in &plan.picks {
        if pick.source_doc_id >= sources.len() {
            return Err(MergeError::SourceIndexOutOfRange {
                idx: pick.source_doc_id,
            });
        }
        let src = sources[pick.source_doc_id];
        if find_node(&src.root, pick.root_node_id).is_none() {
            return Err(MergeError::NodeIdNotFound {
                source_idx: pick.source_doc_id,
                node_id: pick.root_node_id,
            });
        }
    }

    let mut id_gen = NodeIdAllocator::new();

    // Build the fresh root folder.
    let merged_root_id = id_gen.alloc();
    let mut root = Folder {
        id: merged_root_id,
        name: plan.merged_root_name.clone(),
        add_date: None,
        last_modified: None,
        is_toolbar: false,
        attrs: AttrMap::new(),
        children: Vec::new(),
    };

    // Fold each pick into the merged root.  Folders with matching names get
    // merged in place; everything else is appended (and bookmarks deduped by
    // URL according to `plan.strategy`).
    for pick in &plan.picks {
        let src = sources[pick.source_doc_id];
        let original = find_node(&src.root, pick.root_node_id).expect("validated above");
        let cloned = clone_node_with_fresh_ids(original, &mut id_gen);
        merge_node_into_folder(&mut root, cloned, plan.strategy);
    }

    let stats = DocumentStats::from_root(&root);

    Ok(Document {
        id: next_document_id(),
        path: None,
        root,
        header: HeaderMetadata::default(),
        id_gen,
        stats,
        open_timestamp: std::time::Instant::now(),
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        dirty: false,
    })
}

// ---------------------------------------------------------------------------
// Subtree lookup + deep clone with id reassignment
// ---------------------------------------------------------------------------

/// Locate a node by id anywhere in the subtree rooted at `folder`.
///
/// Only descendants of `folder` are searched; the document's own root
/// folder is intentionally not pickable through this helper.  In practice
/// the root is never pick-worthy (the user picks subtrees, not the whole
/// document; they'd just use "open in new tab" for that).
fn find_node(folder: &Folder, id: NodeId) -> Option<&Node> {
    for child in &folder.children {
        match child {
            Node::Folder(f) => {
                if f.id == id {
                    return Some(child);
                }
                if let Some(found) = find_node(f, id) {
                    return Some(found);
                }
            }
            Node::Bookmark(b) if b.id == id => return Some(child),
            Node::Separator(s) if s.id == id => return Some(child),
            _ => {}
        }
    }
    None
}

/// Deep-clone a node, reassigning every `NodeId` (including descendants) from
/// `id_gen`.
fn clone_node_with_fresh_ids(node: &Node, id_gen: &mut NodeIdAllocator) -> Node {
    match node {
        Node::Folder(f) => Node::Folder(clone_folder_with_fresh_ids(f, id_gen)),
        Node::Bookmark(b) => Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: b.title.clone(),
            url: b.url.clone(),
            add_date: b.add_date,
            last_modified: b.last_modified,
            icon_blob: b.icon_blob.clone(),
            description: b.description.clone(),
            attrs: b.attrs.clone(),
            flags: b.flags.clone(),
        }),
        Node::Separator(_) => Node::Separator(Separator { id: id_gen.alloc() }),
    }
}

fn clone_folder_with_fresh_ids(folder: &Folder, id_gen: &mut NodeIdAllocator) -> Folder {
    Folder {
        id: id_gen.alloc(),
        name: folder.name.clone(),
        add_date: folder.add_date,
        last_modified: folder.last_modified,
        is_toolbar: folder.is_toolbar,
        attrs: folder.attrs.clone(),
        children: folder
            .children
            .iter()
            .map(|c| clone_node_with_fresh_ids(c, id_gen))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Folding picks into a parent folder (with same-name folder merging and
// conflict resolution).
// ---------------------------------------------------------------------------

/// Fold `incoming` into `parent`, applying same-name folder merging and
/// bookmark-URL conflict resolution.
///
/// - Folders with a name matching an existing child folder are merged in
///   place: their children are unioned and the strategy is recursively
///   applied inside the merged folder.
/// - Bookmarks that collide with an existing sibling URL are resolved by
///   `strategy`.
/// - Separators always pass through.
///
/// All `NodeId`s on `incoming` are assumed to already have been reassigned by
/// [`clone_node_with_fresh_ids`] before this is called.
fn merge_node_into_folder(parent: &mut Folder, incoming: Node, strategy: ConflictStrategy) {
    match incoming {
        Node::Folder(incoming_folder) => {
            // Look for an existing same-named folder among the parent's
            // children.  If present, fold the incoming folder's children into
            // it; otherwise append as a new sibling.
            let existing_idx = parent.children.iter().position(|c| match c {
                Node::Folder(f) => f.name == incoming_folder.name,
                _ => false,
            });
            match existing_idx {
                Some(idx) => {
                    let Node::Folder(existing) = &mut parent.children[idx] else {
                        unreachable!("position predicate guarantees Folder");
                    };
                    // Recursively fold each child of the incoming folder into
                    // the existing one.
                    for child in incoming_folder.children {
                        merge_node_into_folder(existing, child, strategy);
                    }
                }
                None => parent.children.push(Node::Folder(incoming_folder)),
            }
        }
        Node::Bookmark(incoming_bm) => {
            // Find an existing sibling with the same URL; if any, apply the
            // configured strategy.
            let url_key = incoming_bm.url.as_str().to_owned();
            let collision_idx = parent.children.iter().position(|c| match c {
                Node::Bookmark(b) => b.url.as_str() == url_key,
                _ => false,
            });
            match (strategy, collision_idx) {
                (_, None) => parent.children.push(Node::Bookmark(incoming_bm)),
                (ConflictStrategy::KeepFirst, Some(_)) => {
                    // Drop the incoming bookmark.
                }
                (ConflictStrategy::KeepNewest, Some(idx)) => {
                    let Node::Bookmark(existing) = &parent.children[idx] else {
                        unreachable!("collision predicate guarantees Bookmark");
                    };
                    let existing_dt = existing.add_date;
                    let incoming_dt = incoming_bm.add_date;
                    let replace = match (existing_dt, incoming_dt) {
                        (Some(e), Some(i)) => i > e,
                        (None, Some(_)) => true,
                        _ => false,
                    };
                    if replace {
                        parent.children[idx] = Node::Bookmark(incoming_bm);
                    }
                    // Otherwise keep the existing.
                }
                (ConflictStrategy::KeepBoth, Some(_)) => {
                    // Disambiguate the incoming bookmark's title with a
                    // " (n)" suffix.  Count how many existing siblings share
                    // the URL so the suffix index is monotonic.
                    let occurrences = parent
                        .children
                        .iter()
                        .filter(|c| matches!(c, Node::Bookmark(b) if b.url.as_str() == url_key))
                        .count();
                    let mut renamed = incoming_bm;
                    // First duplicate gets " (2)", second " (3)", …
                    let suffix_n = occurrences + 1;
                    renamed.title = format!("{} ({})", renamed.title, suffix_n);
                    parent.children.push(Node::Bookmark(renamed));
                }
            }
        }
        Node::Separator(_) => parent.children.push(incoming),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{Document, DocumentStats, HeaderMetadata};
    use crate::model::ids::{next_document_id, NodeIdAllocator};
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
    use chrono::{TimeZone, Utc};

    /// Build a tiny document inline with a fresh allocator.  Helper for tests.
    fn make_doc<F: FnOnce(&mut NodeIdAllocator) -> Folder>(builder: F) -> Document {
        let mut id_gen = NodeIdAllocator::new();
        let root = builder(&mut id_gen);
        let stats = DocumentStats::from_root(&root);
        Document {
            id: next_document_id(),
            path: None,
            root,
            header: HeaderMetadata::default(),
            id_gen,
            stats,
            open_timestamp: std::time::Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    fn folder(id_gen: &mut NodeIdAllocator, name: &str, children: Vec<Node>) -> Folder {
        Folder {
            id: id_gen.alloc(),
            name: name.into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::new(),
            children,
        }
    }

    fn bookmark(id_gen: &mut NodeIdAllocator, title: &str, url: &str) -> Node {
        Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: title.into(),
            url: BookmarkUrl::Valid(url::Url::parse(url).unwrap()),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::new(),
            flags: BookmarkFlags::default(),
        })
    }

    fn bookmark_dated(id_gen: &mut NodeIdAllocator, title: &str, url: &str, ts: i64) -> Node {
        Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: title.into(),
            url: BookmarkUrl::Valid(url::Url::parse(url).unwrap()),
            add_date: Some(Utc.timestamp_opt(ts, 0).unwrap()),
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::new(),
            flags: BookmarkFlags::default(),
        })
    }

    /// Walk a folder and collect every node id reachable from it.
    fn collect_all_ids(folder: &Folder, out: &mut Vec<NodeId>) {
        out.push(folder.id);
        for child in &folder.children {
            match child {
                Node::Folder(f) => collect_all_ids(f, out),
                Node::Bookmark(b) => out.push(b.id),
                Node::Separator(s) => out.push(s.id),
            }
        }
    }

    #[test]
    fn empty_plan_errors() {
        let doc = make_doc(|g| folder(g, "Bookmarks", vec![]));
        let plan = MergePlan {
            picks: vec![],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let err = build_merged_document(&[&doc], &plan).unwrap_err();
        assert_eq!(err, MergeError::EmptyPlan);
    }

    #[test]
    fn empty_merged_root_name_errors() {
        let doc = make_doc(|g| {
            let bm = bookmark(g, "x", "https://x.example/");
            folder(g, "Bookmarks", vec![bm])
        });
        let pick_id = doc.root.children[0].id();
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: pick_id,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "   ".into(),
        };
        let err = build_merged_document(&[&doc], &plan).unwrap_err();
        assert_eq!(err, MergeError::EmptyMergedRootName);
    }

    #[test]
    fn source_index_out_of_range_errors() {
        let doc = make_doc(|g| folder(g, "Bookmarks", vec![]));
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 5,
                root_node_id: 1,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let err = build_merged_document(&[&doc], &plan).unwrap_err();
        assert_eq!(err, MergeError::SourceIndexOutOfRange { idx: 5 });
    }

    #[test]
    fn unknown_node_id_errors() {
        let doc = make_doc(|g| folder(g, "Bookmarks", vec![]));
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: 999,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let err = build_merged_document(&[&doc], &plan).unwrap_err();
        assert!(matches!(err, MergeError::NodeIdNotFound { .. }));
    }

    #[test]
    fn one_source_one_pick_identity_modulo_ids() {
        let doc = make_doc(|g| {
            let bm = bookmark(g, "x", "https://x.example/");
            folder(g, "Bookmarks", vec![bm])
        });
        let pick_id = doc.root.children[0].id();
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: pick_id,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc], &plan).unwrap();

        assert_eq!(merged.root.name, "Merged");
        assert_eq!(merged.root.children.len(), 1);
        let cloned = merged.root.children[0].as_bookmark().expect("bookmark");
        assert_eq!(cloned.title, "x");
        assert_eq!(cloned.url.as_str(), "https://x.example/");
        // Every id is fresh (allocator started at 1; root got 1, bookmark got 2).
        assert_eq!(merged.root.id, 1);
        assert_eq!(cloned.id, 2);
        // Source untouched.
        assert!(doc.undo_stack.is_empty());
        // Merged starts clean.
        assert!(merged.undo_stack.is_empty());
        assert!(merged.redo_stack.is_empty());
        assert!(merged.path.is_none());
    }

    #[test]
    fn deep_subtree_is_fully_reassigned() {
        let doc = make_doc(|g| {
            let inner_bm = bookmark(g, "inner", "https://inner.example/");
            let inner = folder(g, "inner-folder", vec![inner_bm]);
            let outer_bm = bookmark(g, "outer", "https://outer.example/");
            folder(g, "Bookmarks", vec![Node::Folder(inner), outer_bm])
        });
        let inner_id = doc.root.children[0].id();
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: inner_id,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "M".into(),
        };
        let merged = build_merged_document(&[&doc], &plan).unwrap();

        let mut merged_ids = Vec::new();
        collect_all_ids(&merged.root, &mut merged_ids);

        let unique: std::collections::HashSet<NodeId> = merged_ids.iter().copied().collect();
        assert_eq!(unique.len(), merged_ids.len(), "merged ids must be unique");
    }

    #[test]
    fn keep_first_drops_duplicate_url_under_same_parent() {
        let doc_a = make_doc(|g| {
            let bm = bookmark(g, "from-A", "https://shared.example/");
            folder(g, "A", vec![bm])
        });
        let doc_b = make_doc(|g| {
            let bm = bookmark(g, "from-B", "https://shared.example/");
            folder(g, "B", vec![bm])
        });
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();

        let plan = MergePlan {
            picks: vec![
                MergePick {
                    source_doc_id: 0,
                    root_node_id: pick_a,
                },
                MergePick {
                    source_doc_id: 1,
                    root_node_id: pick_b,
                },
            ],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc_a, &doc_b], &plan).unwrap();
        // Both picks are bookmarks with the same URL; only the first wins.
        assert_eq!(merged.root.children.len(), 1);
        let surviving = merged.root.children[0].as_bookmark().unwrap();
        assert_eq!(surviving.title, "from-A");
    }

    #[test]
    fn keep_newest_picks_most_recent_add_date() {
        let doc_a = make_doc(|g| {
            let bm = bookmark_dated(g, "older", "https://shared.example/", 100);
            folder(g, "A", vec![bm])
        });
        let doc_b = make_doc(|g| {
            let bm = bookmark_dated(g, "newer", "https://shared.example/", 200);
            folder(g, "B", vec![bm])
        });
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();

        let plan = MergePlan {
            picks: vec![
                MergePick {
                    source_doc_id: 0,
                    root_node_id: pick_a,
                },
                MergePick {
                    source_doc_id: 1,
                    root_node_id: pick_b,
                },
            ],
            strategy: ConflictStrategy::KeepNewest,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc_a, &doc_b], &plan).unwrap();
        assert_eq!(merged.root.children.len(), 1);
        let winner = merged.root.children[0].as_bookmark().unwrap();
        assert_eq!(winner.title, "newer");
    }

    #[test]
    fn keep_both_appends_suffix_to_duplicate() {
        let doc_a = make_doc(|g| {
            let bm = bookmark(g, "from-A", "https://shared.example/");
            folder(g, "A", vec![bm])
        });
        let doc_b = make_doc(|g| {
            let bm = bookmark(g, "from-B", "https://shared.example/");
            folder(g, "B", vec![bm])
        });
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();

        let plan = MergePlan {
            picks: vec![
                MergePick {
                    source_doc_id: 0,
                    root_node_id: pick_a,
                },
                MergePick {
                    source_doc_id: 1,
                    root_node_id: pick_b,
                },
            ],
            strategy: ConflictStrategy::KeepBoth,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc_a, &doc_b], &plan).unwrap();
        assert_eq!(merged.root.children.len(), 2);
        let titles: Vec<&str> = merged
            .root
            .children
            .iter()
            .filter_map(|c| c.as_bookmark().map(|b| b.title.as_str()))
            .collect();
        // First wins its title; second gets the " (2)" suffix.
        assert_eq!(titles, vec!["from-A", "from-B (2)"]);
    }

    #[test]
    fn merged_doc_starts_with_empty_undo_redo_and_no_path() {
        let doc = make_doc(|g| {
            let bm = bookmark(g, "x", "https://x.example/");
            folder(g, "Bookmarks", vec![bm])
        });
        let pick_id = doc.root.children[0].id();
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: pick_id,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc], &plan).unwrap();
        assert!(merged.undo_stack.is_empty());
        assert!(merged.redo_stack.is_empty());
        assert!(!merged.dirty);
        assert!(merged.path.is_none());
    }

    #[test]
    fn source_documents_are_unmodified() {
        let doc = make_doc(|g| {
            let bm = bookmark(g, "x", "https://x.example/");
            folder(g, "Bookmarks", vec![bm])
        });
        let original_id = doc.root.id;
        let original_child_id = doc.root.children[0].id();
        let original_child_count = doc.root.children.len();

        let pick_id = doc.root.children[0].id();
        let plan = MergePlan {
            picks: vec![MergePick {
                source_doc_id: 0,
                root_node_id: pick_id,
            }],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let _merged = build_merged_document(&[&doc], &plan).unwrap();

        // Source is untouched.
        assert_eq!(doc.root.id, original_id);
        assert_eq!(doc.root.children[0].id(), original_child_id);
        assert_eq!(doc.root.children.len(), original_child_count);
        assert!(doc.undo_stack.is_empty());
    }

    #[test]
    fn same_name_folders_under_merged_root_are_unioned() {
        // Two picks land same-named "Inbox" folders under the merged root.
        // Their children should be unioned into a single "Inbox" folder.
        let doc_a = make_doc(|g| {
            let bm = bookmark(g, "a", "https://a.example/");
            let inbox = folder(g, "Inbox", vec![bm]);
            folder(g, "A", vec![Node::Folder(inbox)])
        });
        let doc_b = make_doc(|g| {
            let bm = bookmark(g, "b", "https://b.example/");
            let inbox = folder(g, "Inbox", vec![bm]);
            folder(g, "B", vec![Node::Folder(inbox)])
        });
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();

        let plan = MergePlan {
            picks: vec![
                MergePick {
                    source_doc_id: 0,
                    root_node_id: pick_a,
                },
                MergePick {
                    source_doc_id: 1,
                    root_node_id: pick_b,
                },
            ],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc_a, &doc_b], &plan).unwrap();
        assert_eq!(merged.root.children.len(), 1);
        let unioned = merged.root.children[0].as_folder().expect("folder");
        assert_eq!(unioned.name, "Inbox");
        assert_eq!(unioned.children.len(), 2);
    }

    #[test]
    fn same_name_folders_recursively_resolve_url_conflicts() {
        // Same-named folders union; URL conflicts inside resolve via
        // strategy.  KeepFirst means the duplicate URL drops.
        let doc_a = make_doc(|g| {
            let bm = bookmark(g, "from-A", "https://shared.example/");
            let inbox = folder(g, "Inbox", vec![bm]);
            folder(g, "A", vec![Node::Folder(inbox)])
        });
        let doc_b = make_doc(|g| {
            let bm = bookmark(g, "from-B", "https://shared.example/");
            let inbox = folder(g, "Inbox", vec![bm]);
            folder(g, "B", vec![Node::Folder(inbox)])
        });
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();

        let plan = MergePlan {
            picks: vec![
                MergePick {
                    source_doc_id: 0,
                    root_node_id: pick_a,
                },
                MergePick {
                    source_doc_id: 1,
                    root_node_id: pick_b,
                },
            ],
            strategy: ConflictStrategy::KeepFirst,
            merged_root_name: "Merged".into(),
        };
        let merged = build_merged_document(&[&doc_a, &doc_b], &plan).unwrap();
        let unioned = merged.root.children[0].as_folder().unwrap();
        assert_eq!(unioned.children.len(), 1);
        assert_eq!(unioned.children[0].as_bookmark().unwrap().title, "from-A");
    }
}
