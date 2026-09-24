//! Pass executor: runs a [`RuleSet`] over a [`Document`] and produces a
//! [`ChangeSet`].
//!
//! The executor is **read-only**; it never mutates the document. The
//! [`ChangeSet`] it returns is what the UI previews; the user reviews and
//! approves individual changes before calling [`Document::apply`].
//!
//! # Parallelism
//!
//! The current implementation is sequential.  Because every [`Treatment`] is
//! pure and `Send + Sync`, switching to `rayon::par_iter` over the node list is
//! a drop-in change once benchmarks show it is needed (NFR-P-5: < 1 s for
//! 10 000 bookmarks with all standard URL treatments).

use std::collections::HashSet;

use crate::model::document::Document;
use crate::model::ids::NodeId;
use crate::model::node::{Folder, Node};

use super::treatment::{Change, ChangeSet, PassContext, Treatment};

// ---------------------------------------------------------------------------
// RuleSet
// ---------------------------------------------------------------------------

/// An ordered collection of treatments applied as a single pass.
///
/// Rule sets are created by `lantern-io` from TOML files (or built in as
/// constants for the default sets).  The treatments are executed in order;
/// the order matters only for conflict detection, not for correctness of
/// individual treatments.
pub struct RuleSet {
    pub id: String,
    pub name: String,
    pub treatments: Vec<Box<dyn Treatment>>,
}

// ---------------------------------------------------------------------------
// PassTarget
// ---------------------------------------------------------------------------

/// Which nodes in the document the pass should examine.
#[derive(Debug, Clone)]
pub enum PassTarget {
    /// Examine every node in the document tree (folders, bookmarks).
    AllNodes,
    /// Examine only the specified nodes (for selection-based passes; v0.0.3+).
    Selection(Vec<NodeId>),
    /// Examine a folder and everything below it.  Passing the document
    /// root's id is the same as [`PassTarget::AllNodes`]; an id that does not
    /// name a folder in the document proposes nothing.
    Subtree(NodeId),
}

// ---------------------------------------------------------------------------
// run_pass
// ---------------------------------------------------------------------------

/// Execute `rule_set` over `doc` according to `target` and return all proposed
/// changes.
///
/// This function is pure: it never modifies `doc`.  Call [`Document::apply`] on
/// the returned [`ChangeSet`] (after the user has reviewed it) to commit the
/// approved changes.
pub fn run_pass(doc: &Document, rule_set: &RuleSet, target: PassTarget) -> ChangeSet {
    let ctx = PassContext {
        document_id: doc.id,
    };
    let mut changes = Vec::new();

    // `None` means every node is in scope.
    let scope: Option<HashSet<NodeId>> = match target {
        PassTarget::AllNodes => {
            collect_changes_in_folder(&doc.root, rule_set, &ctx, &mut changes);
            None
        }
        PassTarget::Selection(ref ids) => {
            collect_changes_for_selection(&doc.root, rule_set, &ctx, ids, &mut changes);
            Some(ids.iter().copied().collect())
        }
        PassTarget::Subtree(root_id) if root_id == doc.root.id => {
            collect_changes_in_folder(&doc.root, rule_set, &ctx, &mut changes);
            None
        }
        PassTarget::Subtree(root_id) => match doc.root.find(root_id) {
            Some(node @ Node::Folder(folder)) => {
                propose_for_node(node, rule_set, &ctx, &mut changes);
                collect_changes_in_folder(folder, rule_set, &ctx, &mut changes);
                let mut ids = HashSet::from([root_id]);
                collect_ids(folder, &mut ids);
                Some(ids)
            }
            _ => Some(HashSet::new()),
        },
    };

    // Document-level (cross-field) treatments see the whole document (a
    // duplicate's keeper may live outside the target), but only changes to
    // in-scope nodes are kept: a folder pass must never propose deleting a
    // bookmark somewhere else.
    for treatment in &rule_set.treatments {
        let proposed = treatment.propose_document(doc, &ctx);
        match &scope {
            None => changes.extend(proposed),
            Some(ids) => changes.extend(proposed.into_iter().filter(|c| ids.contains(&c.node_id))),
        }
    }

    ChangeSet {
        rule_set_name: rule_set.name.clone(),
        changes,
    }
}

// ---------------------------------------------------------------------------
// Tree walkers
// ---------------------------------------------------------------------------

fn collect_changes_in_folder(
    folder: &Folder,
    rule_set: &RuleSet,
    ctx: &PassContext,
    changes: &mut Vec<Change>,
) {
    for child in &folder.children {
        propose_for_node(child, rule_set, ctx, changes);
        if let Node::Folder(f) = child {
            collect_changes_in_folder(f, rule_set, ctx, changes);
        }
    }
}

fn collect_changes_for_selection(
    folder: &Folder,
    rule_set: &RuleSet,
    ctx: &PassContext,
    ids: &[NodeId],
    changes: &mut Vec<Change>,
) {
    for child in &folder.children {
        if ids.contains(&child.id()) {
            propose_for_node(child, rule_set, ctx, changes);
        }
        if let Node::Folder(f) = child {
            collect_changes_for_selection(f, rule_set, ctx, ids, changes);
        }
    }
}

/// Every node id below `folder` (not including `folder` itself).
fn collect_ids(folder: &Folder, ids: &mut HashSet<NodeId>) {
    for child in &folder.children {
        ids.insert(child.id());
        if let Node::Folder(f) = child {
            collect_ids(f, ids);
        }
    }
}

/// Ask every treatment in the rule set to propose changes for `node`, appending
/// to `changes`.
#[inline]
fn propose_for_node(node: &Node, rule_set: &RuleSet, ctx: &PassContext, changes: &mut Vec<Change>) {
    for treatment in &rule_set.treatments {
        changes.extend(treatment.propose(node, ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{DocumentStats, HeaderMetadata};
    use crate::model::ids::next_document_id;
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl};
    use crate::sanitize::treatments::duplicates::ExactUrlDuplicatesTreatment;
    use crate::sanitize::treatments::folder_name::FolderWhitespaceTreatment;
    use crate::sanitize::treatments::title::WhitespaceTreatment;

    fn bm(id: u64, title: &str, href: &str) -> Node {
        Node::Bookmark(Bookmark {
            id,
            title: title.into(),
            url: BookmarkUrl::Valid(url::Url::parse(href).unwrap()),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        })
    }

    fn folder(id: u64, name: &str, children: Vec<Node>) -> Folder {
        Folder {
            id,
            name: name.into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children,
        }
    }

    /// root(99) ─┬─ 1 "  Loose  "  https://a.com/
    ///           └─ folder 10 "  Work  " ─┬─ 11 "  Inside  " https://a.com/
    ///                                   └─ folder 20 ── 21 "  Deep  " https://b.com/
    fn doc() -> Document {
        let deep = folder(20, "Deep", vec![bm(21, "  Deep  ", "https://b.com/")]);
        let work = folder(
            10,
            "  Work  ",
            vec![bm(11, "  Inside  ", "https://a.com/"), Node::Folder(deep)],
        );
        let root = folder(
            99,
            "Bookmarks",
            vec![bm(1, "  Loose  ", "https://a.com/"), Node::Folder(work)],
        );
        let stats = DocumentStats::from_root(&root);
        Document::new(
            next_document_id(),
            None,
            root,
            HeaderMetadata::default(),
            stats,
        )
    }

    fn rules(treatments: Vec<Box<dyn Treatment>>) -> RuleSet {
        RuleSet {
            id: "t".into(),
            name: "T".into(),
            treatments,
        }
    }

    fn touched(cs: &ChangeSet) -> Vec<NodeId> {
        let mut ids: Vec<_> = cs.changes.iter().map(|c| c.node_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    #[test]
    fn subtree_covers_the_folder_and_everything_below_it() {
        let rs = rules(vec![
            Box::new(WhitespaceTreatment),
            Box::new(FolderWhitespaceTreatment),
        ]);
        let cs = run_pass(&doc(), &rs, PassTarget::Subtree(10));
        // 10 is the folder's own name; 1 (outside) is untouched.
        assert_eq!(touched(&cs), vec![10, 11, 21]);
    }

    #[test]
    fn subtree_of_root_matches_all_nodes() {
        let rs = rules(vec![Box::new(WhitespaceTreatment)]);
        let d = doc();
        assert_eq!(
            touched(&run_pass(&d, &rs, PassTarget::Subtree(99))),
            touched(&run_pass(&d, &rs, PassTarget::AllNodes)),
        );
    }

    #[test]
    fn subtree_of_unknown_or_non_folder_id_proposes_nothing() {
        let rs = rules(vec![
            Box::new(WhitespaceTreatment),
            Box::new(ExactUrlDuplicatesTreatment),
        ]);
        assert!(run_pass(&doc(), &rs, PassTarget::Subtree(12345))
            .changes
            .is_empty());
        assert!(run_pass(&doc(), &rs, PassTarget::Subtree(11))
            .changes
            .is_empty());
    }

    #[test]
    fn document_level_changes_are_limited_to_the_target() {
        // 1 (root) and 11 (Work) share a URL; 1 is first-seen, so 11 is the
        // proposed delete.
        let rs = rules(vec![Box::new(ExactUrlDuplicatesTreatment)]);
        let d = doc();
        assert_eq!(touched(&run_pass(&d, &rs, PassTarget::AllNodes)), vec![11]);
        assert_eq!(
            touched(&run_pass(&d, &rs, PassTarget::Subtree(10))),
            vec![11]
        );
        assert!(run_pass(&d, &rs, PassTarget::Subtree(20))
            .changes
            .is_empty());
        assert!(run_pass(&d, &rs, PassTarget::Selection(vec![1]))
            .changes
            .is_empty());
    }
}
