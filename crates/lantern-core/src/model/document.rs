use std::path::PathBuf;
use std::time::Instant;

use crate::model::ids::{DocumentId, NodeId, NodeIdAllocator};
use crate::model::node::{BookmarkFlag, Folder, Node};

// ---------------------------------------------------------------------------
// DocumentStats
// ---------------------------------------------------------------------------

/// Cached counts for a document; invalidated and recomputed whenever a pass
/// is applied or undone.
#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub bookmark_count: u64,
    pub folder_count: u64,
    pub separator_count: u64,
}

impl DocumentStats {
    /// Walk `root` and count all nodes by type.
    pub fn from_root(root: &Folder) -> Self {
        let mut stats = Self::default();
        Self::count_recursive(root, &mut stats);
        stats
    }

    fn count_recursive(folder: &Folder, stats: &mut Self) {
        for child in &folder.children {
            match child {
                Node::Folder(f) => {
                    stats.folder_count += 1;
                    Self::count_recursive(f, stats);
                }
                Node::Bookmark(_) => stats.bookmark_count += 1,
                Node::Separator(_) => stats.separator_count += 1,
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

/// Metadata from the Netscape bookmark file's preamble (DOCTYPE, META, TITLE,
/// H1). Preserved byte-for-byte on round-trip when none of the header fields
/// were touched.
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
// Undo model
// ---------------------------------------------------------------------------

/// Which text field of a node a change targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Url,
    Title,
    FolderName,
}

/// The kind of a single undo/redo inverse operation.
///
/// Applying all inverses in an [`UndoEntry`] exactly restores the prior state.
#[derive(Debug, Clone)]
pub enum InverseKind {
    /// Restore a single text field to its previous value.
    WriteField { field: Field, value: String },
    /// Re-insert a previously deleted node at its original position.
    ///
    /// The snapshot contains the full subtree so children are restored too.
    RestoreNode {
        /// Parent folder ID (0 means document root).
        parent_id: NodeId,
        /// Child index within the parent at the time of deletion.
        index: usize,
        snapshot: Box<Node>,
    },
    /// Remove a node that was created (inverse of create_*).
    RemoveNode,
    /// Restore a bookmark flag to its previous value.
    WriteFlag { flag: BookmarkFlag, value: bool },
    /// Move a node back to a previous `(parent_id, index)` position.
    MoveNode {
        to_parent_id: NodeId,
        to_index: usize,
    },
}

/// The inverse of a single applied operation.
#[derive(Debug, Clone)]
pub struct InverseChange {
    pub node_id: NodeId,
    pub kind: InverseKind,
}

/// One item on the undo stack, corresponding to one apply / structural edit.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub timestamp: Instant,
    /// Name of the operation that produced this entry (rule-set name, "rename",
    /// "delete", "create", "move", …).
    pub rule_set_name: String,
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
/// - `root` always contains a valid folder tree.
/// - Every `NodeId` inside the tree is unique within this document.
/// - `undo_stack` is bounded to `MAX_UNDO_ENTRIES` entries; older entries are
///   dropped FIFO (PRD F-SAN-6).
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
    /// True if any pass has been applied (and not fully undone) since open.
    pub dirty: bool,
}

/// Maximum number of undo entries retained per document (PRD F-SAN-6).
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

    /// Returns `true` if there is at least one entry to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there is at least one entry to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
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
