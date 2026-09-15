pub mod document;
pub mod ids;
pub mod merge;
pub mod node;

pub use document::{
    Document, DocumentStats, Field, HeaderMetadata, InverseChange, LineEnding, UndoEntry,
    MAX_UNDO_ENTRIES,
};
pub use ids::{next_document_id, DocumentId, NodeId, NodeIdAllocator};
pub use merge::{build_merged_document, ConflictStrategy, MergeError, MergePick, MergePlan};
pub use node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node, Separator};
