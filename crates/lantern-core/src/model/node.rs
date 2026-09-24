use chrono::{DateTime, Utc};
use indexmap::IndexMap;

use crate::model::ids::NodeId;

/// Attribute map that preserves insertion order, so unknown attributes on
/// `<A>` and `<H3>` tags are emitted in their original order.
pub type AttrMap = IndexMap<String, String>;

// ---------------------------------------------------------------------------
// BookmarkUrl
// ---------------------------------------------------------------------------

/// A URL that was parsed from a bookmark file.
///
/// Malformed URLs are kept verbatim so they survive export. URL treatments
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

/// Identifies one boolean flag on a bookmark, so a flag change can be
/// recorded and undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookmarkFlag {
    /// Marks the URL as a known link-shortener domain.
    IsShortener,
}

/// Per-item runtime flags. Not serialized to the bookmark file.
#[derive(Debug, Clone, Default)]
pub struct BookmarkFlags {
    /// True when the URL was flagged as a link-shortener by the
    /// `url.host.unshorten.offline` treatment.
    pub is_shortener: bool,
}

// ---------------------------------------------------------------------------
// Tree nodes
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
    pub children: Vec<Node>,
}

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

#[derive(Debug, Clone)]
pub struct Separator {
    pub id: NodeId,
}

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

// ---------------------------------------------------------------------------
// Tree lookups
// ---------------------------------------------------------------------------

impl Folder {
    /// Depth-first lookup of the descendant with `id` (never `self`).
    pub(crate) fn find(&self, id: NodeId) -> Option<&Node> {
        self.children.iter().find_map(|child| match child {
            _ if child.id() == id => Some(child),
            Node::Folder(f) => f.find(id),
            _ => None,
        })
    }
}
