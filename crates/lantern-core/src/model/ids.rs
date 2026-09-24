use std::sync::atomic::{AtomicU64, Ordering};

/// Stable, session-scoped identifier for a single node inside a `Document`.
///
/// NodeIds are assigned by `NodeIdAllocator` at parse time. They are **not**
/// serialized to disk and **not** stable across sessions: a file re-opened in
/// a new session will have different NodeIds. The in-session stability guarantee
/// is what allows the undo stack to reference nodes by ID.
pub type NodeId = u64;

/// Stable identifier for an open document (tab). Unique within a session.
pub type DocumentId = u64;

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

/// Returns a fresh `DocumentId` unique within the process lifetime.
pub fn next_document_id() -> DocumentId {
    NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Allocates `NodeId`s for a single document.
///
/// Kept inside `Document` so that every node created during a merge or edit
/// gets a new, never-reused id within that document's lifetime.
#[derive(Debug, Clone)]
pub struct NodeIdAllocator {
    next: u64,
}

impl NodeIdAllocator {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// An allocator whose first id is `last + 1`, so it never hands out an id
    /// that is already present in a tree whose highest id is `last`.
    pub(crate) fn starting_after(last: NodeId) -> Self {
        Self {
            next: last.checked_add(1).expect("NodeId overflow (> 2^64 nodes)"),
        }
    }

    pub fn alloc(&mut self) -> NodeId {
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .expect("NodeId overflow (> 2^64 nodes)");
        id
    }
}

impl Default for NodeIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}
