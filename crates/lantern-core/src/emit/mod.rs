//! Document → bytes, in every format Lantern writes.
//!
//! - [`html`] — Netscape bookmark HTML, the format every browser imports and
//!   the only one that round-trips everything Lantern reads.
//! - [`json`] — Chrome / Chromium `Bookmarks` JSON.
//! - [`markdown`] — a nested Markdown link list, for notes and wikis.
//! - [`csv`] — one row per bookmark, for spreadsheets.
//!
//! [`export`] picks the writer from an [`ExportFormat`]. Every writer returns
//! raw bytes, so the caller decides where they go and `lantern-core` stays
//! I/O-free.

pub mod csv;
pub mod html;
pub mod json;
pub mod markdown;

pub use html::{emit, EmitOptions};

use crate::model::document::Document;
use crate::model::node::{Folder, Node};

/// A file format Lantern can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    /// Netscape bookmark HTML.
    #[default]
    Html,
    /// Chrome / Chromium `Bookmarks` JSON.
    Json,
    /// Nested Markdown link list.
    Markdown,
    /// Comma-separated values, one row per bookmark.
    Csv,
}

impl ExportFormat {
    /// Every format, in the order the UI lists them.
    pub const ALL: [ExportFormat; 4] = [Self::Html, Self::Json, Self::Markdown, Self::Csv];

    /// The format a file extension or CLI name stands for (case-insensitive,
    /// without the dot): `html`/`htm`, `json`, `md`/`markdown`, `csv`.
    /// `None` for anything Lantern does not write.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "html" | "htm" => Some(Self::Html),
            "json" => Some(Self::Json),
            "md" | "markdown" => Some(Self::Markdown),
            "csv" => Some(Self::Csv),
            _ => None,
        }
    }

    /// The short name used by the CLI and in messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Csv => "csv",
        }
    }

    /// The conventional file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Csv => "csv",
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Serialise `doc` in `format`. `opts` only affects HTML.
pub fn export(doc: &Document, format: ExportFormat, opts: &EmitOptions) -> Vec<u8> {
    match format {
        ExportFormat::Html => emit(doc, opts),
        ExportFormat::Json => json::emit_json(doc),
        ExportFormat::Markdown => markdown::emit_markdown(doc),
        ExportFormat::Csv => csv::emit_csv(doc),
    }
}

/// Visit the folders and bookmarks under `folder` depth-first, passing each
/// with the names of the folders above it (the unnamed document root is not
/// included). Separators are skipped: the flat formats have no place for
/// them.
pub(crate) fn walk<'a>(
    folder: &'a Folder,
    path: &mut Vec<&'a str>,
    f: &mut impl FnMut(&[&'a str], &'a Node),
) {
    for child in &folder.children {
        match child {
            Node::Folder(sub) => {
                f(path, child);
                path.push(&sub.name);
                walk(sub, path, f);
                path.pop();
            }
            Node::Bookmark(_) => f(path, child),
            Node::Separator(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_resolve_from_extensions_and_names() {
        assert_eq!(
            ExportFormat::from_extension("HTM"),
            Some(ExportFormat::Html)
        );
        assert_eq!(
            ExportFormat::from_extension("json"),
            Some(ExportFormat::Json)
        );
        assert_eq!(
            ExportFormat::from_extension("Markdown"),
            Some(ExportFormat::Markdown)
        );
        assert_eq!(ExportFormat::from_extension("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_extension("txt"), None);
        for f in ExportFormat::ALL {
            assert_eq!(ExportFormat::from_extension(f.name()), Some(f));
            assert_eq!(ExportFormat::from_extension(f.extension()), Some(f));
        }
    }
}
