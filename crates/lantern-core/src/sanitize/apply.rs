//! `Document::apply`, `Document::undo`, and `Document::redo`.
//!
//! These are `impl Document` methods placed in this file to keep the model
//! module free of sanitization concerns while still satisfying the TDD §5.4.2
//! signature (`impl Document { pub fn apply(...) }`).
//!
//! # Apply semantics
//!
//! `apply` walks the approved changes, records the *before* value of each
//! targeted field as an [`InverseChange`], writes the *after* value into the
//! tree, and pushes the resulting [`UndoEntry`] onto the undo stack.  The redo
//! stack is cleared.
//!
//! If a node cannot be found (stale ID from a previous pass), an
//! [`ApplyError::NodeNotFound`] is returned.  In that case **no changes have
//! been applied**; `apply` collects all inverses first and only then performs
//! writes, so the document is never left in a partially-mutated state.
//!
//! # Undo / redo
//!
//! Both stacks hold [`UndoEntry`] values whose `inverses` list is "the field
//! values to write in order to reach the state associated with this entry."
//! Undoing pops from the undo stack, captures the current values as the redo
//! entry, and applies the inverse writes.  Redoing is the mirror operation.

use std::time::Instant;

use thiserror::Error;

use crate::error::{CoreError, Result};
use crate::model::document::{
    Document, DocumentStats, Field, InverseChange, InverseKind, UndoEntry, MAX_UNDO_ENTRIES,
};
use crate::model::ids::NodeId;
use crate::model::node::{
    AttrMap, Bookmark, BookmarkFlag, BookmarkFlags, BookmarkUrl, Folder, Node, Separator,
};

use super::treatment::{ChangeKind, ChangeSet};

// ---------------------------------------------------------------------------
// ApplyError
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error("node {0} not found in document tree")]
    NodeNotFound(NodeId),
}

// ---------------------------------------------------------------------------
// impl Document
// ---------------------------------------------------------------------------

impl Document {
    /// Apply all approved changes in `cs`, record an undo entry, and clear the
    /// redo stack.
    ///
    /// Returns `Err(ApplyError::NodeNotFound)` if any approved change targets a
    /// node ID that no longer exists in the tree.  In that case the document is
    /// **not** modified.
    pub fn apply(&mut self, cs: &ChangeSet) -> std::result::Result<(), ApplyError> {
        let approved: Vec<_> = cs.changes.iter().filter(|c| c.approved).collect();
        if approved.is_empty() {
            return Ok(());
        }

        // Validate every target first so a stale change set leaves the
        // document untouched.
        for change in &approved {
            let id = change.node_id;
            let exists = match &change.kind {
                ChangeKind::SetField { field, .. } => read_field(&self.root, id, *field).is_some(),
                ChangeKind::DeleteNode => find_node_with_parent(&self.root, id).is_some(),
                ChangeKind::SetFlag { flag, .. } => read_flag(&self.root, id, *flag).is_some(),
            };
            if !exists {
                return Err(ApplyError::NodeNotFound(id));
            }
        }

        // Capture each inverse just before its write, so it describes the
        // tree as the earlier writes of this change set left it; undo then
        // replays the inverses in reverse.  A change whose node an earlier
        // change already removed (e.g. a bookmark inside a deleted folder) is
        // skipped.
        let mut inverses: Vec<InverseChange> = Vec::with_capacity(approved.len());
        for change in &approved {
            let id = change.node_id;
            let kind = match &change.kind {
                ChangeKind::SetField { field, after, .. } => read_field(&self.root, id, *field)
                    .map(|value| {
                        write_field(&mut self.root, id, *field, after);
                        InverseKind::WriteField {
                            field: *field,
                            value,
                        }
                    }),
                ChangeKind::DeleteNode => {
                    take_node(&mut self.root, id).map(|(parent_id, index, node)| {
                        InverseKind::RestoreNode {
                            parent_id,
                            index,
                            snapshot: Box::new(node),
                        }
                    })
                }
                ChangeKind::SetFlag { flag, after, .. } => {
                    read_flag(&self.root, id, *flag).map(|value| {
                        write_flag(&mut self.root, id, *flag, *after);
                        InverseKind::WriteFlag { flag: *flag, value }
                    })
                }
            };
            if let Some(kind) = kind {
                inverses.push(InverseChange { node_id: id, kind });
            }
        }

        // Push undo entry; trim if over the limit.
        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: cs.rule_set_name.clone(),
            inverses,
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }

        self.redo_stack.clear();
        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);

        Ok(())
    }

    /// Undo the most recent applied change set.
    ///
    /// Returns `Err(CoreError::InvalidOperation)` if the undo stack is empty.
    pub fn undo(&mut self) -> Result<()> {
        let entry = self
            .undo_stack
            .pop()
            .ok_or_else(|| CoreError::InvalidOperation("nothing to undo".into()))?;

        let redo_entry = self.replay(entry);
        self.redo_stack.push(redo_entry);

        self.dirty = !self.undo_stack.is_empty();
        self.stats = DocumentStats::from_root(&self.root);

        Ok(())
    }

    /// Re-apply the most recently undone change set.
    ///
    /// Returns `Err(CoreError::InvalidOperation)` if the redo stack is empty.
    pub fn redo(&mut self) -> Result<()> {
        let entry = self
            .redo_stack
            .pop()
            .ok_or_else(|| CoreError::InvalidOperation("nothing to redo".into()))?;

        let undo_entry = self.replay(entry);
        self.undo_stack.push(undo_entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }

        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);

        Ok(())
    }

    /// Apply `entry`'s inverses newest-first and return the entry that
    /// reverses this replay (used for both undo and redo).
    ///
    /// Each re-inverse is captured just before its inverse is applied, so the
    /// returned entry is again in "replay newest-first" order.
    fn replay(&mut self, entry: UndoEntry) -> UndoEntry {
        let mut inverses = Vec::with_capacity(entry.inverses.len());
        for inv in entry.inverses.iter().rev() {
            if let Some(kind) = capture_reinverse(&self.root, inv) {
                inverses.push(InverseChange {
                    node_id: inv.node_id,
                    kind,
                });
            }
            apply_inverse(&mut self.root, inv);
        }
        UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: entry.rule_set_name,
            inverses,
        }
    }

    /// Rename a node (bookmark title or folder name) and record an undoable entry.
    ///
    /// Returns `Err` if the node does not exist or is a separator (not renameable).
    pub fn rename_node(&mut self, node_id: NodeId, new_name: String) -> Result<()> {
        // Determine which field applies: Title for bookmarks, FolderName for folders.
        let field = if read_field(&self.root, node_id, Field::Title).is_some() {
            Field::Title
        } else if read_field(&self.root, node_id, Field::FolderName).is_some() {
            Field::FolderName
        } else {
            return Err(CoreError::InvalidOperation(format!(
                "node {node_id} not found or not renameable (separators have no name)"
            )));
        };

        let old_name = read_field(&self.root, node_id, field).unwrap();
        if old_name == new_name {
            return Ok(()); // no-op
        }

        write_field(&mut self.root, node_id, field, &new_name);

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "rename".into(),
            inverses: vec![InverseChange {
                node_id,
                kind: InverseKind::WriteField {
                    field,
                    value: old_name,
                },
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        // Stats don't change on rename.
        Ok(())
    }

    /// Delete a node from the tree and record an undoable entry.
    ///
    /// Returns `Err` if the node is not found.
    pub fn delete_node(&mut self, node_id: NodeId) -> Result<()> {
        let (parent_id, index, snapshot) = take_node(&mut self.root, node_id)
            .ok_or_else(|| CoreError::InvalidOperation(format!("node {node_id} not found")))?;

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "delete".into(),
            inverses: vec![InverseChange {
                node_id,
                kind: InverseKind::RestoreNode {
                    parent_id,
                    index,
                    snapshot: Box::new(snapshot),
                },
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);
        Ok(())
    }

    /// Create a new bookmark inside `parent_id` (0 = document root).
    ///
    /// Returns the new node's ID.
    pub fn create_bookmark(
        &mut self,
        parent_id: NodeId,
        title: String,
        url_str: String,
    ) -> Result<NodeId> {
        let new_id = self.id_gen.alloc();
        let url = match url::Url::parse(&url_str) {
            Ok(u) => BookmarkUrl::Valid(u),
            Err(_) => BookmarkUrl::Malformed { raw: url_str },
        };
        let node = Node::Bookmark(Bookmark {
            id: new_id,
            title,
            url,
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        });
        if parent_id == 0 {
            self.root.children.push(node);
        } else {
            let path = folder_path(&self.root, parent_id).ok_or_else(|| {
                CoreError::InvalidOperation(format!("folder {parent_id} not found"))
            })?;
            folder_at_path(&mut self.root, &path).children.push(node);
        }

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "create".into(),
            inverses: vec![InverseChange {
                node_id: new_id,
                kind: InverseKind::RemoveNode,
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);
        Ok(new_id)
    }

    /// Create a new folder inside `parent_id` (0 = document root).
    ///
    /// Returns the new node's ID.
    pub fn create_folder(&mut self, parent_id: NodeId, name: String) -> Result<NodeId> {
        let new_id = self.id_gen.alloc();
        let node = Node::Folder(Folder {
            id: new_id,
            name,
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: Vec::new(),
        });
        if parent_id == 0 {
            self.root.children.push(node);
        } else {
            let path = folder_path(&self.root, parent_id).ok_or_else(|| {
                CoreError::InvalidOperation(format!("folder {parent_id} not found"))
            })?;
            folder_at_path(&mut self.root, &path).children.push(node);
        }

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "create".into(),
            inverses: vec![InverseChange {
                node_id: new_id,
                kind: InverseKind::RemoveNode,
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);
        Ok(new_id)
    }

    /// Create a new separator inside `parent_id` (0 = document root).
    ///
    /// Returns the new node's ID.
    pub fn create_separator(&mut self, parent_id: NodeId) -> Result<NodeId> {
        let new_id = self.id_gen.alloc();
        let node = Node::Separator(Separator { id: new_id });
        if parent_id == 0 {
            self.root.children.push(node);
        } else {
            let path = folder_path(&self.root, parent_id).ok_or_else(|| {
                CoreError::InvalidOperation(format!("folder {parent_id} not found"))
            })?;
            folder_at_path(&mut self.root, &path).children.push(node);
        }

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "create".into(),
            inverses: vec![InverseChange {
                node_id: new_id,
                kind: InverseKind::RemoveNode,
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        self.stats = DocumentStats::from_root(&self.root);
        Ok(new_id)
    }

    /// Move `node_id` to position `new_index` within `new_parent_id` (0 = root).
    ///
    /// Captures the original position and records an undoable entry.
    /// Clamps `new_index` to the folder length after removal.
    pub fn move_node(
        &mut self,
        node_id: NodeId,
        new_parent_id: NodeId,
        new_index: usize,
    ) -> Result<()> {
        // Capture the original position for the undo inverse.
        let (old_parent_id, old_index, _) = find_node_with_parent(&self.root, node_id)
            .ok_or_else(|| CoreError::InvalidOperation(format!("node {node_id} not found")))?;

        // Verify destination exists before taking the node (prevents losing the node
        // if the destination folder does not exist).
        if new_parent_id != 0 {
            folder_path(&self.root, new_parent_id).ok_or_else(|| {
                CoreError::InvalidOperation(format!("folder {new_parent_id} not found"))
            })?;
            // A folder cannot move into itself or its own subtree: the
            // destination would be taken out of the tree along with it.
            if let Some((_, _, Node::Folder(f))) = find_node_with_parent(&self.root, node_id) {
                if folder_path(&f, new_parent_id).is_some() {
                    return Err(CoreError::InvalidOperation(format!(
                        "cannot move folder {node_id} into its own subtree"
                    )));
                }
            }
        }

        let (_, _, node) = take_node(&mut self.root, node_id).unwrap(); // safe: verified above

        if new_parent_id == 0 {
            let idx = new_index.min(self.root.children.len());
            self.root.children.insert(idx, node);
        } else {
            let path = folder_path(&self.root, new_parent_id).unwrap(); // safe: verified above
            let parent = folder_at_path(&mut self.root, &path);
            let idx = new_index.min(parent.children.len());
            parent.children.insert(idx, node);
        }

        let entry = UndoEntry {
            timestamp: Instant::now(),
            rule_set_name: "move".into(),
            inverses: vec![InverseChange {
                node_id,
                kind: InverseKind::MoveNode {
                    to_parent_id: old_parent_id,
                    to_index: old_index,
                },
            }],
        };
        self.undo_stack.push(entry);
        if self.undo_stack.len() > MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Field read / write helpers
// ---------------------------------------------------------------------------

/// Read the current value of `field` on the node with `node_id`.
///
/// Returns `None` if the node does not exist or the field is not applicable to
/// the node type (e.g. `Field::Url` on a `Folder`).
pub(crate) fn read_field(folder: &Folder, node_id: NodeId, field: Field) -> Option<String> {
    // Check the folder itself.
    if folder.id == node_id {
        return match field {
            Field::FolderName => Some(folder.name.clone()),
            _ => None,
        };
    }

    for child in &folder.children {
        match child {
            Node::Folder(f) => {
                if let Some(v) = read_field(f, node_id, field) {
                    return Some(v);
                }
            }
            Node::Bookmark(b) if b.id == node_id => {
                return match field {
                    Field::Title => Some(b.title.clone()),
                    Field::Url => Some(b.url.as_str().to_owned()),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    None
}

/// Write `value` to `field` on the node with `node_id`.
///
/// Returns `true` if the node was found and written.
/// Silently does nothing if the node is not found (callers in `apply` already
/// verified existence in the read phase).
pub(crate) fn write_field(folder: &mut Folder, node_id: NodeId, field: Field, value: &str) -> bool {
    if folder.id == node_id {
        if field == Field::FolderName {
            folder.name = value.to_owned();
        }
        return true;
    }

    for child in &mut folder.children {
        match child {
            Node::Folder(f) => {
                if f.id == node_id {
                    if field == Field::FolderName {
                        f.name = value.to_owned();
                    }
                    return true;
                }
                if write_field(f, node_id, field, value) {
                    return true;
                }
            }
            Node::Bookmark(b) if b.id == node_id => {
                write_to_bookmark(b, field, value);
                return true;
            }
            _ => {}
        }
    }
    false
}

fn write_to_bookmark(b: &mut Bookmark, field: Field, value: &str) {
    match field {
        Field::Title => b.title = value.to_owned(),
        Field::Url => {
            b.url = match url::Url::parse(value) {
                Ok(u) => BookmarkUrl::Valid(u),
                Err(_) => BookmarkUrl::Malformed {
                    raw: value.to_owned(),
                },
            };
        }
        Field::FolderName => {} // not applicable to bookmarks
    }
}

/// Remove the node with `node_id` from the tree rooted at `folder`, returning
/// its parent folder id, its index within that parent, and the node itself.
///
/// Returns `None` if the node was not found.
fn take_node(folder: &mut Folder, node_id: NodeId) -> Option<(NodeId, usize, Node)> {
    if let Some(pos) = folder.children.iter().position(|n| n.id() == node_id) {
        return Some((folder.id, pos, folder.children.remove(pos)));
    }
    for child in &mut folder.children {
        if let Node::Folder(f) = child {
            if let Some(found) = take_node(f, node_id) {
                return Some(found);
            }
        }
    }
    None
}

/// Find a node by ID and return a clone of it along with its parent folder ID
/// and its index within that parent.  Returns `None` if not found.
fn find_node_with_parent(folder: &Folder, node_id: NodeId) -> Option<(NodeId, usize, Node)> {
    for (i, child) in folder.children.iter().enumerate() {
        if child.id() == node_id {
            return Some((folder.id, i, child.clone()));
        }
        if let Node::Folder(f) = child {
            if let Some(result) = find_node_with_parent(f, node_id) {
                return Some(result);
            }
        }
    }
    None
}

/// Insert `node` at position `index` within the folder whose id is `parent_id`.
///
/// `parent_id` is the actual folder ID (not the sentinel 0).  The index is
/// clamped to the parent's child count.
fn insert_node_at(root: &mut Folder, parent_id: NodeId, index: usize, node: Node) {
    if let Some(path) = folder_path(root, parent_id) {
        let parent = folder_at_path(root, &path);
        let idx = index.min(parent.children.len());
        parent.children.insert(idx, node);
    }
}

/// Read a boolean flag from the bookmark with `node_id`.  Returns `None` if
/// the node is not found or is not a bookmark.
fn read_flag(folder: &Folder, node_id: NodeId, flag: BookmarkFlag) -> Option<bool> {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) if b.id == node_id => {
                return Some(match flag {
                    BookmarkFlag::IsShortener => b.flags.is_shortener,
                });
            }
            Node::Folder(f) => {
                if let Some(v) = read_flag(f, node_id, flag) {
                    return Some(v);
                }
            }
            _ => {}
        }
    }
    None
}

/// Write a boolean flag on the bookmark with `node_id`.  Returns `true` if found.
//
// `clippy::collapsible_match` (Rust 1.95) wants the `if write_flag(...)` block
// folded into the outer arm as a guard.  That doesn't work here: pattern
// guards borrow bound variables immutably (E0596), but `write_flag` needs
// `&mut Folder`.  Suppress the lint locally instead of wrestling the recursion.
#[allow(clippy::collapsible_match)]
fn write_flag(folder: &mut Folder, node_id: NodeId, flag: BookmarkFlag, value: bool) -> bool {
    for child in &mut folder.children {
        match child {
            Node::Bookmark(b) if b.id == node_id => {
                match flag {
                    BookmarkFlag::IsShortener => b.flags.is_shortener = value,
                }
                return true;
            }
            Node::Folder(f) => {
                if write_flag(f, node_id, flag, value) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Compute the "re-inverse" of `inv`: the inverse that would undo the undo.
///
/// Returns `None` when the node no longer exists (stale ID); that entry is
/// silently dropped from the redo / undo list.
fn capture_reinverse(root: &Folder, inv: &InverseChange) -> Option<InverseKind> {
    match &inv.kind {
        InverseKind::WriteField { field, .. } => {
            read_field(root, inv.node_id, *field).map(|value| InverseKind::WriteField {
                field: *field,
                value,
            })
        }
        InverseKind::RestoreNode { .. } => {
            // After re-inserting the snapshot the redo/undo is to remove it again.
            Some(InverseKind::RemoveNode)
        }
        InverseKind::RemoveNode => {
            // After removing, the redo/undo is to restore it with its snapshot.
            find_node_with_parent(root, inv.node_id).map(|(parent_id, index, snapshot)| {
                InverseKind::RestoreNode {
                    parent_id,
                    index,
                    snapshot: Box::new(snapshot),
                }
            })
        }
        InverseKind::WriteFlag { flag, .. } => read_flag(root, inv.node_id, *flag)
            .map(|value| InverseKind::WriteFlag { flag: *flag, value }),
        InverseKind::MoveNode {
            to_parent_id,
            to_index,
        } => {
            find_node_with_parent(root, inv.node_id).map(|(parent_id, index, _)| {
                // Capture the *current* position as the re-inverse destination.
                let _ = (to_parent_id, to_index); // stored position applied below
                InverseKind::MoveNode {
                    to_parent_id: parent_id,
                    to_index: index,
                }
            })
        }
    }
}

/// Apply an inverse change to restore a previous state.
fn apply_inverse(root: &mut Folder, inv: &InverseChange) {
    match &inv.kind {
        InverseKind::WriteField { field, value } => {
            write_field(root, inv.node_id, *field, value);
        }
        InverseKind::RestoreNode {
            parent_id,
            index,
            snapshot,
        } => {
            insert_node_at(root, *parent_id, *index, *snapshot.clone());
        }
        InverseKind::RemoveNode => {
            take_node(root, inv.node_id);
        }
        InverseKind::WriteFlag { flag, value } => {
            write_flag(root, inv.node_id, *flag, *value);
        }
        InverseKind::MoveNode {
            to_parent_id,
            to_index,
        } => {
            if let Some((_, _, node)) = take_node(root, inv.node_id) {
                insert_node_at(root, *to_parent_id, *to_index, node);
            }
        }
    }
}

/// Return the sequence of child indices that lead from `folder` to the
/// sub-folder with `id`.  An empty `Vec` means `folder` itself has that id.
///
/// Uses an immutable borrow so the result can be fed to `folder_at_path`
/// without lifetime conflicts.
fn folder_path(folder: &Folder, id: NodeId) -> Option<Vec<usize>> {
    if folder.id == id {
        return Some(vec![]);
    }
    for (i, child) in folder.children.iter().enumerate() {
        if let Node::Folder(f) = child {
            if let Some(mut path) = folder_path(f, id) {
                path.insert(0, i);
                return Some(path);
            }
        }
    }
    None
}

/// Walk `path` (a sequence of child indices) through `folder` to reach a
/// nested sub-folder, returning a mutable reference to it.
///
/// # Panics
/// Panics if any index in `path` is out-of-bounds or leads to a non-folder
/// node (which cannot happen when the path was produced by `folder_path`).
fn folder_at_path<'a>(folder: &'a mut Folder, path: &[usize]) -> &'a mut Folder {
    if path.is_empty() {
        return folder;
    }
    match &mut folder.children[path[0]] {
        Node::Folder(f) => folder_at_path(f, &path[1..]),
        _ => unreachable!("folder_path produced an index pointing to a non-folder node"),
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

    // ── Test-document factory ─────────────────────────────────────────────
    //
    // Builds a minimal Document without going through the parser.
    // Node IDs start at 100 so that `Document::id_gen` (which starts at 1)
    // never collides with manually assigned IDs when create_* is called.
    //
    // Tree layout:
    //   root (id=99, addressed as folder_id=0)
    //     ├── Bookmark  "Google"  https://google.com/  (id=100)
    //     ├── Folder    "Work"                          (id=101)
    //     │   └── Bookmark  "GitHub"  https://github.com/  (id=102)
    //     └── Separator                                (id=103)

    fn bm(id: NodeId, title: &str, href: &str) -> Node {
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

    fn make_doc() -> Document {
        let subfolder = Node::Folder(Folder {
            id: 101,
            name: "Work".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![bm(102, "GitHub", "https://github.com/")],
        });
        let root = Folder {
            id: 99,
            name: "Bookmarks".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![
                bm(100, "Google", "https://google.com/"),
                subfolder,
                Node::Separator(Separator { id: 103 }),
            ],
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

    fn child_titles(doc: &Document) -> Vec<String> {
        doc.root
            .children
            .iter()
            .map(|n| match n {
                Node::Bookmark(b) => b.title.clone(),
                Node::Folder(f) => f.name.clone(),
                Node::Separator(_) => "<sep>".into(),
            })
            .collect()
    }

    // ── rename_node ───────────────────────────────────────────────────────

    #[test]
    fn rename_bookmark_changes_title() {
        let mut doc = make_doc();
        doc.rename_node(100, "DuckDuckGo".into()).unwrap();
        let bm = doc.root.children[0].as_bookmark().unwrap();
        assert_eq!(bm.title, "DuckDuckGo");
    }

    #[test]
    fn rename_folder_changes_name() {
        let mut doc = make_doc();
        doc.rename_node(101, "Personal".into()).unwrap();
        let folder = doc.root.children[1].as_folder().unwrap();
        assert_eq!(folder.name, "Personal");
    }

    #[test]
    fn rename_separator_returns_err() {
        let mut doc = make_doc();
        assert!(doc.rename_node(103, "whatever".into()).is_err());
    }

    #[test]
    fn rename_nonexistent_returns_err() {
        let mut doc = make_doc();
        assert!(doc.rename_node(9999, "ghost".into()).is_err());
    }

    #[test]
    fn rename_noop_when_same_name() {
        let mut doc = make_doc();
        doc.rename_node(100, "Google".into()).unwrap();
        // No undo entry pushed for a no-op.
        assert!(doc.undo_stack.is_empty());
    }

    #[test]
    fn rename_marks_dirty() {
        let mut doc = make_doc();
        doc.rename_node(100, "Changed".into()).unwrap();
        assert!(doc.dirty);
    }

    #[test]
    fn rename_pushes_undo_entry() {
        use crate::model::document::InverseKind;
        let mut doc = make_doc();
        doc.rename_node(100, "Changed".into()).unwrap();
        assert_eq!(doc.undo_stack.len(), 1);
        let inv = &doc.undo_stack[0].inverses[0];
        assert!(
            matches!(&inv.kind, InverseKind::WriteField { value, .. } if value == "Google"),
            "expected WriteField inverse with value 'Google'"
        );
    }

    #[test]
    fn rename_undo_restores_original_title() {
        let mut doc = make_doc();
        doc.rename_node(100, "Changed".into()).unwrap();
        doc.undo().unwrap();
        let title = &doc.root.children[0].as_bookmark().unwrap().title;
        assert_eq!(title, "Google");
    }

    #[test]
    fn rename_redo_reapplies_new_title() {
        let mut doc = make_doc();
        doc.rename_node(100, "Changed".into()).unwrap();
        doc.undo().unwrap();
        doc.redo().unwrap();
        let title = &doc.root.children[0].as_bookmark().unwrap().title;
        assert_eq!(title, "Changed");
    }

    #[test]
    fn rename_clears_redo_stack() {
        let mut doc = make_doc();
        doc.rename_node(100, "First".into()).unwrap();
        doc.undo().unwrap();
        // Now there's a redo entry.  Renaming again should clear it.
        doc.rename_node(100, "Second".into()).unwrap();
        assert!(doc.redo_stack.is_empty());
    }

    // ── delete_node ───────────────────────────────────────────────────────

    #[test]
    fn delete_removes_root_level_bookmark() {
        let mut doc = make_doc();
        doc.delete_node(100).unwrap();
        assert_eq!(doc.root.children.len(), 2);
        assert!(doc.root.children.iter().all(|n| n.id() != 100));
    }

    #[test]
    fn delete_removes_folder_and_its_subtree() {
        let mut doc = make_doc();
        doc.delete_node(101).unwrap();
        // Work folder gone, its child (GitHub) gone with it.
        assert!(doc.root.children.iter().all(|n| n.id() != 101));
        assert_eq!(doc.stats.folder_count, 0);
        assert_eq!(doc.stats.bookmark_count, 1); // only Google remains
    }

    #[test]
    fn delete_removes_nested_bookmark() {
        let mut doc = make_doc();
        doc.delete_node(102).unwrap(); // GitHub inside Work
        let work = doc.root.children[1].as_folder().unwrap();
        assert!(work.children.is_empty());
    }

    #[test]
    fn delete_removes_separator() {
        let mut doc = make_doc();
        doc.delete_node(103).unwrap();
        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn delete_nonexistent_returns_err() {
        let mut doc = make_doc();
        assert!(doc.delete_node(9999).is_err());
        // Document unchanged.
        assert_eq!(doc.root.children.len(), 3);
    }

    #[test]
    fn delete_marks_dirty_and_updates_stats() {
        let mut doc = make_doc();
        assert_eq!(doc.stats.bookmark_count, 2);
        doc.delete_node(100).unwrap();
        assert!(doc.dirty);
        assert_eq!(doc.stats.bookmark_count, 1);
    }

    // ── create_bookmark ───────────────────────────────────────────────────

    #[test]
    fn create_bookmark_at_root() {
        let mut doc = make_doc();
        let id = doc
            .create_bookmark(0, "Rust".into(), "https://rust-lang.org/".into())
            .unwrap();
        assert!(id > 0);
        let last = doc.root.children.last().unwrap();
        assert_eq!(last.id(), id);
        assert_eq!(last.as_bookmark().unwrap().title, "Rust");
    }

    #[test]
    fn create_bookmark_in_subfolder() {
        let mut doc = make_doc();
        let id = doc
            .create_bookmark(101, "Crates.io".into(), "https://crates.io/".into())
            .unwrap();
        let work = doc.root.children[1].as_folder().unwrap();
        assert_eq!(work.children.len(), 2);
        assert_eq!(work.children.last().unwrap().id(), id);
    }

    #[test]
    fn create_bookmark_with_invalid_url_stored_as_malformed() {
        let mut doc = make_doc();
        doc.create_bookmark(0, "Bad".into(), "not a url".into())
            .unwrap();
        let last = doc.root.children.last().unwrap().as_bookmark().unwrap();
        assert!(last.url.is_malformed());
    }

    #[test]
    fn create_bookmark_marks_dirty_and_updates_stats() {
        let mut doc = make_doc();
        let before = doc.stats.bookmark_count;
        doc.create_bookmark(0, "New".into(), "https://new.example/".into())
            .unwrap();
        assert!(doc.dirty);
        assert_eq!(doc.stats.bookmark_count, before + 1);
    }

    #[test]
    fn create_bookmark_in_nonexistent_folder_returns_err() {
        let mut doc = make_doc();
        assert!(doc
            .create_bookmark(9999, "X".into(), "https://x.com/".into())
            .is_err());
    }

    // ── create_folder ─────────────────────────────────────────────────────

    #[test]
    fn create_folder_at_root() {
        let mut doc = make_doc();
        let id = doc.create_folder(0, "Personal".into()).unwrap();
        let last = doc.root.children.last().unwrap().as_folder().unwrap();
        assert_eq!(last.id, id);
        assert_eq!(last.name, "Personal");
        assert!(last.children.is_empty());
    }

    #[test]
    fn create_folder_updates_folder_count() {
        let mut doc = make_doc();
        let before = doc.stats.folder_count;
        doc.create_folder(0, "Extra".into()).unwrap();
        assert_eq!(doc.stats.folder_count, before + 1);
    }

    #[test]
    fn create_folder_nested_inside_subfolder() {
        let mut doc = make_doc();
        let id = doc.create_folder(101, "Sub-Work".into()).unwrap();
        let work = doc.root.children[1].as_folder().unwrap();
        // Work now has: GitHub bookmark + Sub-Work folder
        assert_eq!(work.children.len(), 2);
        assert_eq!(work.children.last().unwrap().id(), id);
    }

    // ── create_separator ──────────────────────────────────────────────────

    #[test]
    fn create_separator_at_root() {
        let mut doc = make_doc();
        let before = doc.stats.separator_count;
        let id = doc.create_separator(0).unwrap();
        assert_eq!(doc.stats.separator_count, before + 1);
        let last = doc.root.children.last().unwrap();
        assert_eq!(last.id(), id);
        assert!(matches!(last, Node::Separator(_)));
    }

    #[test]
    fn create_separator_marks_dirty() {
        let mut doc = make_doc();
        doc.create_separator(0).unwrap();
        assert!(doc.dirty);
    }

    // ── move_node ─────────────────────────────────────────────────────────

    #[test]
    fn move_node_reorders_within_root() {
        let mut doc = make_doc();
        // Before: [Google(100), Work(101), <sep>(103)]
        // Move Google to index 1 → [Work(101), Google(100), <sep>(103)]
        doc.move_node(100, 0, 1).unwrap();
        assert_eq!(child_titles(&doc), vec!["Work", "Google", "<sep>"]);
    }

    #[test]
    fn move_node_to_end() {
        let mut doc = make_doc();
        // Move Work folder to last position.
        // After take, root has 2 items; inserting at index 2 puts it at end.
        doc.move_node(101, 0, 100).unwrap(); // index 100 clamps to len
        let titles = child_titles(&doc);
        assert_eq!(titles.last().unwrap(), "Work");
    }

    #[test]
    fn move_node_into_subfolder() {
        let mut doc = make_doc();
        // Move separator (103) into Work folder (101) at index 0.
        doc.move_node(103, 101, 0).unwrap();
        // Root should now have: Google, Work (sep gone from root)
        assert_eq!(doc.root.children.len(), 2);
        let work = doc.root.children[1].as_folder().unwrap();
        // Work: sep at 0, GitHub at 1
        assert_eq!(work.children.len(), 2);
        assert!(matches!(work.children[0], Node::Separator(_)));
    }

    #[test]
    fn move_node_nonexistent_returns_err() {
        let mut doc = make_doc();
        assert!(doc.move_node(9999, 0, 0).is_err());
        // Document unchanged.
        assert_eq!(doc.root.children.len(), 3);
    }

    #[test]
    fn move_node_marks_dirty() {
        let mut doc = make_doc();
        doc.move_node(100, 0, 2).unwrap();
        assert!(doc.dirty);
    }

    #[test]
    fn move_node_index_clamped_at_bounds() {
        let mut doc = make_doc();
        // Move Google to a very high index: should end up last.
        doc.move_node(100, 0, usize::MAX).unwrap();
        assert_eq!(doc.root.children.last().unwrap().id(), 100);
    }

    #[test]
    fn move_node_pushes_undo_entry() {
        let mut doc = make_doc();
        doc.move_node(100, 0, 2).unwrap();
        assert_eq!(doc.undo_stack.len(), 1);
        assert_eq!(doc.undo_stack[0].rule_set_name, "move");
    }

    #[test]
    fn move_node_undo_restores_original_position() {
        let mut doc = make_doc();
        // Before: [Google(100), Work(101), <sep>(103)]
        doc.move_node(100, 0, 2).unwrap();
        // After move: [Work(101), <sep>(103), Google(100)]
        doc.undo().unwrap();
        // After undo: [Google(100), Work(101), <sep>(103)]
        assert_eq!(doc.root.children[0].id(), 100);
        assert_eq!(doc.root.children[1].id(), 101);
    }

    // ── delete_node undo ──────────────────────────────────────────────────────

    #[test]
    fn delete_node_pushes_undo_entry() {
        let mut doc = make_doc();
        doc.delete_node(100).unwrap();
        assert_eq!(doc.undo_stack.len(), 1);
        assert_eq!(doc.undo_stack[0].rule_set_name, "delete");
    }

    #[test]
    fn delete_node_undo_restores_bookmark() {
        let mut doc = make_doc();
        doc.delete_node(100).unwrap();
        assert_eq!(doc.root.children.len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.root.children.len(), 3);
        assert_eq!(doc.root.children[0].id(), 100);
    }

    #[test]
    fn delete_node_undo_redo_cycle() {
        let mut doc = make_doc();
        doc.delete_node(100).unwrap();
        doc.undo().unwrap(); // restore
        assert_eq!(doc.root.children.len(), 3);
        doc.redo().unwrap(); // re-delete
        assert_eq!(doc.root.children.len(), 2);
        assert!(doc.root.children.iter().all(|n| n.id() != 100));
    }

    #[test]
    fn delete_folder_undo_restores_subtree() {
        let mut doc = make_doc();
        doc.delete_node(101).unwrap(); // delete Work folder (contains GitHub)
        doc.undo().unwrap();
        let work = doc.root.children[1].as_folder().unwrap();
        assert_eq!(work.id, 101);
        assert_eq!(work.children.len(), 1);
        assert_eq!(work.children[0].id(), 102); // GitHub restored
    }

    // ── create_bookmark undo ──────────────────────────────────────────────────

    #[test]
    fn create_bookmark_pushes_undo_entry() {
        let mut doc = make_doc();
        doc.create_bookmark(0, "New".into(), "https://new.example/".into())
            .unwrap();
        assert_eq!(doc.undo_stack.len(), 1);
        assert_eq!(doc.undo_stack[0].rule_set_name, "create");
    }

    #[test]
    fn create_bookmark_undo_removes_it() {
        let mut doc = make_doc();
        let id = doc
            .create_bookmark(0, "New".into(), "https://new.example/".into())
            .unwrap();
        doc.undo().unwrap();
        assert!(doc.root.children.iter().all(|n| n.id() != id));
        assert_eq!(doc.root.children.len(), 3);
    }

    #[test]
    fn create_folder_undo_removes_it() {
        let mut doc = make_doc();
        let id = doc.create_folder(0, "Temp".into()).unwrap();
        doc.undo().unwrap();
        assert!(doc.root.children.iter().all(|n| n.id() != id));
    }

    #[test]
    fn create_separator_undo_removes_it() {
        let mut doc = make_doc();
        let id = doc.create_separator(0).unwrap();
        doc.undo().unwrap();
        assert!(doc.root.children.iter().all(|n| n.id() != id));
    }

    // ── apply / undo / redo interaction ───────────────────────────────────

    #[test]
    fn undo_empty_stack_returns_err() {
        let mut doc = make_doc();
        assert!(doc.undo().is_err());
    }

    #[test]
    fn redo_empty_stack_returns_err() {
        let mut doc = make_doc();
        assert!(doc.redo().is_err());
    }

    #[test]
    fn multiple_renames_each_push_undo_entry() {
        let mut doc = make_doc();
        doc.rename_node(100, "A".into()).unwrap();
        doc.rename_node(100, "B".into()).unwrap();
        doc.rename_node(100, "C".into()).unwrap();
        assert_eq!(doc.undo_stack.len(), 3);
    }

    #[test]
    fn undo_all_renames_restores_original() {
        let mut doc = make_doc();
        doc.rename_node(100, "A".into()).unwrap();
        doc.rename_node(100, "B".into()).unwrap();
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.root.children[0].as_bookmark().unwrap().title, "Google");
    }

    // ── regressions ───────────────────────────────────────────────────────

    fn all_ids(folder: &Folder, out: &mut Vec<NodeId>) {
        out.push(folder.id);
        for child in &folder.children {
            match child {
                Node::Folder(f) => all_ids(f, out),
                other => out.push(other.id()),
            }
        }
    }

    fn ids_of(folder: &Folder) -> Vec<NodeId> {
        folder.children.iter().map(Node::id).collect()
    }

    fn approved_deletes(ids: &[NodeId]) -> ChangeSet {
        let changes = ids
            .iter()
            .map(|&id| {
                let mut c = crate::sanitize::treatment::Change::delete_node(id, "test", "test");
                c.approved = true;
                c
            })
            .collect();
        ChangeSet {
            rule_set_name: "test".into(),
            changes,
        }
    }

    #[test]
    fn created_ids_never_collide_with_parsed_ids() {
        let mut doc = crate::parser::parse(
            br#"<DL><p>
    <DT><A HREF="https://a.example/">A</A>
    <DT><H3>F</H3>
    <DL><p><DT><A HREF="https://b.example/">B</A></DL><p>
</DL><p>"#,
        )
        .unwrap();
        let mut existing = Vec::new();
        all_ids(&doc.root, &mut existing);

        let folder = doc.create_folder(0, "New".into()).unwrap();
        let bookmark = doc
            .create_bookmark(folder, "N".into(), "https://n.example/".into())
            .unwrap();
        let separator = doc.create_separator(0).unwrap();
        for id in [folder, bookmark, separator] {
            assert!(
                !existing.contains(&id),
                "id {id} reused from the parsed tree"
            );
        }
    }

    #[test]
    fn undo_restores_positions_whatever_the_change_order() {
        // root: [Google(100), Work(101), <sep>(103)]; delete Work before
        // Google, i.e. not in document order.
        let mut doc = make_doc();
        let original = ids_of(&doc.root);
        doc.apply(&approved_deletes(&[101, 100])).unwrap();
        assert_eq!(ids_of(&doc.root), vec![103]);

        doc.undo().unwrap();
        assert_eq!(ids_of(&doc.root), original);
        doc.redo().unwrap();
        assert_eq!(ids_of(&doc.root), vec![103]);
        doc.undo().unwrap();
        assert_eq!(ids_of(&doc.root), original);
    }

    #[test]
    fn undo_of_nested_deletes_does_not_duplicate_children() {
        // Work(101) and its only child GitHub(102) deleted in one change set.
        let mut doc = make_doc();
        doc.apply(&approved_deletes(&[101, 102])).unwrap();
        doc.undo().unwrap();
        let work = doc.root.children[1].as_folder().unwrap();
        assert_eq!(ids_of(work), vec![102]);
        assert_eq!(doc.stats.bookmark_count, 2);
    }

    #[test]
    fn move_folder_into_its_own_subtree_is_rejected() {
        let mut doc = make_doc();
        let sub = doc.create_folder(101, "Sub".into()).unwrap();
        assert!(doc.move_node(101, 101, 0).is_err());
        assert!(doc.move_node(101, sub, 0).is_err());
        assert_eq!(child_titles(&doc), vec!["Google", "Work", "<sep>"]);
        assert_eq!(doc.undo_stack.len(), 1); // only the create
    }

    // ── folder_path / folder_at_path helpers ──────────────────────────────

    #[test]
    fn folder_path_finds_root_itself() {
        let doc = make_doc();
        let path = folder_path(&doc.root, 99).unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn folder_path_finds_direct_child_folder() {
        let doc = make_doc();
        let path = folder_path(&doc.root, 101).unwrap();
        assert_eq!(path, vec![1]); // Work is at index 1 of root
    }

    #[test]
    fn folder_path_returns_none_for_missing_id() {
        let doc = make_doc();
        assert!(folder_path(&doc.root, 9999).is_none());
    }

    #[test]
    fn folder_path_returns_none_for_bookmark_id() {
        // Bookmarks are not folders: shouldn't be found by folder_path.
        let doc = make_doc();
        assert!(folder_path(&doc.root, 100).is_none());
    }
}
