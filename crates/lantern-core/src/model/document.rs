use std::path::PathBuf;
use std::time::Instant;

use crate::model::ids::{DocumentId, NodeId, NodeIdAllocator};
use crate::model::node::{BookmarkFlag, Folder, Node};

// ---------------------------------------------------------------------------
// DocumentStats
// ---------------------------------------------------------------------------

/// Cached node counts, recomputed after every edit, undo and redo.
#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub bookmark_count: u64,
    pub folder_count: u64,
    pub separator_count: u64,
}

impl DocumentStats {
    /// Count the nodes below `root` (not `root` itself) by type.
    pub fn from_root(root: &Folder) -> Self {
        let mut stats = Self::default();
        stats.count(root);
        stats
    }

    fn count(&mut self, folder: &Folder) {
        for child in &folder.children {
            match child {
                Node::Folder(f) => {
                    self.folder_count += 1;
                    self.count(f);
                }
                Node::Bookmark(_) => self.bookmark_count += 1,
                Node::Separator(_) => self.separator_count += 1,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HeaderMetadata
// ---------------------------------------------------------------------------

/// Line ending convention detected in the original file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

/// Metadata from the Netscape bookmark file's preamble (META charset, TITLE),
/// plus the BOM and line-ending conventions, reproduced on export.
#[derive(Debug, Clone)]
pub struct HeaderMetadata {
    pub charset: Option<String>,
    /// Contents of the `<TITLE>` element (typically `"Bookmarks"`).
    pub title: Option<String>,
    /// True if a UTF-8 BOM was present at the start of the file.
    pub has_bom: bool,
    pub line_ending: LineEnding,
}

impl Default for HeaderMetadata {
    fn default() -> Self {
        Self {
            charset: Some("UTF-8".into()),
            title: Some("Bookmarks".into()),
            has_bom: false,
            line_ending: LineEnding::Lf,
        }
    }
}

// ---------------------------------------------------------------------------
// Edits and undo history
// ---------------------------------------------------------------------------

/// Which text field of a node a change targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Url,
    Title,
    FolderName,
}

/// One edit on one node.
///
/// Undo entries store these as inverses: performing an entry's operations
/// newest-first restores the state before the entry.  Performing an
/// operation yields the operation that reverses it, which is how undo and
/// redo build each other's entries.
#[derive(Debug, Clone)]
pub enum InverseKind {
    /// Set a text field to `value`.
    WriteField { field: Field, value: String },
    /// Insert `snapshot` (with its whole subtree) at `index` in the folder
    /// `parent_id`.
    RestoreNode {
        parent_id: NodeId,
        index: usize,
        snapshot: Box<Node>,
    },
    /// Remove the node and its subtree.
    RemoveNode,
    /// Set a bookmark flag to `value`.
    WriteFlag { flag: BookmarkFlag, value: bool },
    /// Move the node to `to_index` in the folder `to_parent_id`.
    MoveNode {
        to_parent_id: NodeId,
        to_index: usize,
    },
}

/// An [`InverseKind`] and the node it applies to.
#[derive(Debug, Clone)]
pub struct InverseChange {
    pub node_id: NodeId,
    pub kind: InverseKind,
}

/// One item on the undo (or redo) stack: one applied change set or one
/// structural edit.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// Name of the operation that produced this entry (rule-set name,
    /// "rename", "delete", "create", "move").
    pub rule_set_name: String,
    /// In the order they were recorded; replayed newest-first.
    pub inverses: Vec<InverseChange>,
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// An opened bookmark file, held entirely in memory.
///
/// The `Document` is the authoritative state for a tab. The UI holds no
/// canonical data; it caches views of what the Rust side tells it.
///
/// # Invariants
///
/// - Every `NodeId` inside the tree is unique within this document.
/// - `undo_stack` holds at most [`MAX_UNDO_ENTRIES`] entries; the oldest are
///   dropped first.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: DocumentId,
    /// Original file path if this document was loaded from disk.
    /// `None` for documents created by a merge operation.
    pub path: Option<PathBuf>,
    pub root: Folder,
    pub header: HeaderMetadata,
    pub id_gen: NodeIdAllocator,
    pub stats: DocumentStats,
    pub open_timestamp: Instant,
    /// Undo history for this document. Index 0 is the oldest entry.
    pub undo_stack: Vec<UndoEntry>,
    pub redo_stack: Vec<UndoEntry>,
    /// True if any edit has been made (and not fully undone) since open.
    pub dirty: bool,
}

/// Maximum number of undo entries retained per document.
pub const MAX_UNDO_ENTRIES: usize = 100;

impl Document {
    /// Wrap an already-built tree.  The document's id allocator starts after
    /// the highest id in `root`, so nodes created later never collide with
    /// the ones the tree was built with.
    pub fn new(
        id: DocumentId,
        path: Option<PathBuf>,
        root: Folder,
        header: HeaderMetadata,
        stats: DocumentStats,
    ) -> Self {
        Self {
            id,
            path,
            id_gen: NodeIdAllocator::starting_after(max_node_id(&root)),
            root,
            header,
            stats,
            open_timestamp: Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }
}

/// Highest node id in the tree rooted at `folder` (including `folder` itself).
fn max_node_id(folder: &Folder) -> NodeId {
    folder.children.iter().fold(folder.id, |max, child| {
        max.max(match child {
            Node::Folder(f) => max_node_id(f),
            other => other.id(),
        })
    })
}
