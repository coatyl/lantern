//! Cross-field treatments that require access to the whole document.
//!
//! (`cross.dedupe` was folded into `structure.duplicates.near_url`; see
//! `duplicates.rs`.  Saved rule sets naming it load the near-duplicate pass.)
//!
//! | ID                    | Name                                      |
//! |-----------------------|-------------------------------------------|
//! | `cross.empty_folders` | Remove folders with no bookmark content   |

use crate::model::document::Document;
use crate::model::node::{Folder, Node};
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};

// ---------------------------------------------------------------------------
// cross.empty_folders
// ---------------------------------------------------------------------------

/// Removes folders whose subtree contains no bookmarks.
///
/// Only the outermost empty ancestor is proposed for deletion; inner empty
/// folders are implicitly removed as part of their parent's subtree.
///
/// Separators inside an otherwise-empty folder are not treated as content;
/// a folder containing only separators is considered empty.
pub struct EmptyFoldersTreatment;

impl Treatment for EmptyFoldersTreatment {
    fn id(&self) -> &'static str {
        "cross.empty_folders"
    }
    fn name(&self) -> &'static str {
        "Remove empty folders"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::CrossField
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, _node: &crate::model::node::Node, _ctx: &PassContext) -> Vec<Change> {
        vec![]
    }

    fn propose_document(&self, doc: &Document, _ctx: &PassContext) -> Vec<Change> {
        let mut changes = Vec::new();
        collect_empty_folder_deletes(&doc.root, self.id(), &mut changes);
        changes
    }
}

/// Returns `true` when `folder` contains no bookmark nodes anywhere in its
/// subtree (recursive).  Separators do not count as content.
fn has_no_bookmarks(folder: &Folder) -> bool {
    for child in &folder.children {
        match child {
            Node::Bookmark(_) => return false,
            Node::Folder(f) => {
                if !has_no_bookmarks(f) {
                    return false;
                }
            }
            Node::Separator(_) => {}
        }
    }
    true
}

/// DFS walk that emits `DeleteNode` for the outermost empty folders only.
///
/// When an empty folder is found we emit a delete and stop recursing into it
/// (its children will be removed as part of the parent's subtree deletion).
fn collect_empty_folder_deletes(
    folder: &Folder,
    treatment_id: &'static str,
    changes: &mut Vec<Change>,
) {
    for child in &folder.children {
        if let Node::Folder(f) = child {
            if has_no_bookmarks(f) {
                changes.push(Change::delete_node(
                    f.id,
                    treatment_id,
                    "Folder contains no bookmarks",
                ));
                // Don't recurse: the whole subtree is already covered.
            } else {
                collect_empty_folder_deletes(f, treatment_id, changes);
            }
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
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Separator};
    use crate::sanitize::treatment::PassContext;

    fn ctx() -> PassContext {
        PassContext { document_id: 0 }
    }

    fn bm(id: u64, href: &str) -> Node {
        let url = url::Url::parse(href).unwrap();
        Node::Bookmark(Bookmark {
            id,
            title: "T".into(),
            url: BookmarkUrl::Valid(url),
            add_date: None,
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

    #[test]
    fn empty_folders_no_changes_when_all_have_content() {
        let doc = make_doc_with_root(vec![bm(1, "https://a.com/")]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn empty_folders_flags_empty_leaf_folder() {
        let empty = Node::Folder(Folder {
            id: 50,
            name: "Empty".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        let doc = make_doc_with_root(vec![bm(1, "https://a.com/"), empty]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 50);
    }

    #[test]
    fn empty_folders_only_outermost_empty_ancestor() {
        // Outer empty folder containing inner empty folder: only outer is proposed.
        let inner = Node::Folder(Folder {
            id: 51,
            name: "Inner".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        let outer = Node::Folder(Folder {
            id: 50,
            name: "Outer".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![inner],
        });
        let doc = make_doc_with_root(vec![outer]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 50); // only outer proposed
    }

    #[test]
    fn empty_folders_separator_only_is_empty() {
        let sep_only = Node::Folder(Folder {
            id: 50,
            name: "Sep".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![Node::Separator(Separator { id: 98 })],
        });
        let doc = make_doc_with_root(vec![sep_only]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn empty_folders_recurses_into_non_empty_folders() {
        // Parent has a bookmark; child folder is empty.  Only child proposed.
        let empty_child = Node::Folder(Folder {
            id: 51,
            name: "EmptyChild".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        let parent = Node::Folder(Folder {
            id: 50,
            name: "Parent".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![bm(1, "https://a.com/"), empty_child],
        });
        let doc = make_doc_with_root(vec![parent]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].node_id, 51);
    }

    #[test]
    fn empty_folders_changes_are_destructive_and_unapproved() {
        let empty = Node::Folder(Folder {
            id: 50,
            name: "Empty".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        let doc = make_doc_with_root(vec![empty]);
        let changes = EmptyFoldersTreatment.propose_document(&doc, &ctx());
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }
}
