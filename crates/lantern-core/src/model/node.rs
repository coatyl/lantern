use chrono::{DateTime, Utc};
use indexmap::IndexMap;

use crate::model::ids::NodeId;

// ---------------------------------------------------------------------------
// AttrMap
// ---------------------------------------------------------------------------

/// An ordered attribute map that preserves insertion order.
///
/// Round-trip fidelity requires that unknown attributes on `<A>` and `<H3>`
/// tags appear in the emitted file in the same order as the original.
/// `IndexMap` provides this with O(1) lookup.
pub type AttrMap = IndexMap<String, String>;

// ---------------------------------------------------------------------------
// BookmarkUrl
// ---------------------------------------------------------------------------

/// A URL that was parsed from a bookmark file.
///
/// Malformed URLs are wrapped in the `Malformed` variant so they can be
/// preserved verbatim on export without crashing the parser. URL treatments
/// only operate on `Valid` variants; `Malformed` URLs pass through unchanged.
#[derive(Debug, Clone)]
pub enum BookmarkUrl {
    Valid(url::Url),
    Malformed { raw: String },
}

impl BookmarkUrl {
    /// Returns the raw URL string regardless of validity.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Valid(u) => u.as_str(),
            Self::Malformed { raw } => raw.as_str(),
        }
    }

    /// Returns `Some(&url::Url)` only when the URL parsed successfully.
    pub fn as_valid(&self) -> Option<&url::Url> {
        match self {
            Self::Valid(u) => Some(u),
            Self::Malformed { .. } => None,
        }
    }

    pub fn is_malformed(&self) -> bool {
        matches!(self, Self::Malformed { .. })
    }
}

impl std::fmt::Display for BookmarkUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// BookmarkFlags
// ---------------------------------------------------------------------------

/// Identifies a single boolean flag on a bookmark node.
///
/// Used by [`ChangeKind::SetFlag`](crate::sanitize::treatment::ChangeKind) and
/// the corresponding [`InverseKind::WriteFlag`](crate::model::document::InverseKind)
/// to record which flag was mutated so the change can be undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookmarkFlag {
    /// Marks the URL as a known link-shortener domain.
    IsShortener,
}

/// Per-item runtime flags. Not serialized to the bookmark file.
#[derive(Debug, Clone, Default)]
pub struct BookmarkFlags {
    /// True once a dead-link check result has been recorded for this item.
    pub is_dead_link_checked: bool,
    /// True when the URL was flagged as a link-shortener by the
    /// `url.host.unshorten.offline` treatment.
    pub is_shortener: bool,
}

// ---------------------------------------------------------------------------
// Folder
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Folder {
    /// Stable within-session identifier.
    pub id: NodeId,
    pub name: String,
    pub add_date: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    /// True when this folder is the browser's bookmark toolbar folder
    /// (`PERSONAL_TOOLBAR_FOLDER="true"` attribute).
    pub is_toolbar: bool,
    /// All attributes on the `<H3>` tag, preserving unknown ones for round-trip.
    pub attrs: AttrMap,
    /// Direct children. `Vec<Node>` is heap-allocated so the recursive type is
    /// fine without explicit boxing.
    pub children: Vec<Node>,
}

// ---------------------------------------------------------------------------
// Bookmark
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: NodeId,
    pub title: String,
    pub url: BookmarkUrl,
    pub add_date: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    /// Original base64 favicon data URL from the `ICON` attribute.
    /// Preserved verbatim; Lantern never decodes or re-encodes favicon blobs.
    pub icon_blob: Option<String>,
    /// Description text from the `<DD>` element that may follow a `<DT><A>`.
    pub description: Option<String>,
    /// All attributes on the `<A>` tag.
    pub attrs: AttrMap,
    pub flags: BookmarkFlags,
}

// ---------------------------------------------------------------------------
// Separator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Separator {
    pub id: NodeId,
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A tree node in a bookmark document.
#[derive(Debug, Clone)]
pub enum Node {
    Folder(Folder),
    Bookmark(Bookmark),
    Separator(Separator),
}

impl Node {
    pub fn id(&self) -> NodeId {
        match self {
            Self::Folder(f) => f.id,
            Self::Bookmark(b) => b.id,
            Self::Separator(s) => s.id,
        }
    }

    pub fn as_folder(&self) -> Option<&Folder> {
        match self {
            Self::Folder(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_bookmark(&self) -> Option<&Bookmark> {
        match self {
            Self::Bookmark(b) => Some(b),
            _ => None,
        }
    }
}
