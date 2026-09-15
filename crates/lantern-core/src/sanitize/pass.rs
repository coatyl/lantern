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

    match target {
        PassTarget::AllNodes => {
            collect_changes_in_folder(&doc.root, rule_set, &ctx, &mut changes);
        }
        PassTarget::Selection(ref ids) => {
            collect_changes_for_selection(&doc.root, rule_set, &ctx, ids, &mut changes);
        }
    }

    // Document-level (cross-field) treatments run after the per-node loop.
    for treatment in &rule_set.treatments {
        changes.extend(treatment.propose_document(doc, &ctx));
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

/// Ask every treatment in the rule set to propose changes for `node`, appending
/// to `changes`.
#[inline]
fn propose_for_node(node: &Node, rule_set: &RuleSet, ctx: &PassContext, changes: &mut Vec<Change>) {
    for treatment in &rule_set.treatments {
        changes.extend(treatment.propose(node, ctx));
    }
}
