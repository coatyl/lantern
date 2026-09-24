//! Read bookmark files and write Netscape bookmark HTML.
//!
//! Reads accept Netscape / Firefox HTML **or** Chrome / Chromium
//! `Bookmarks` JSON (auto-detected from the bytes). Writes always emit
//! Netscape HTML via the existing emitter — Lantern never writes back to
//! a browser profile.
//!
//! # Source-file protection (PRD F-EXP-7)
//!
//! `write_bookmark_file` compares the destination path against the path the
//! document was originally read from (`doc.path`).  If they resolve to the
//! same file, the write is refused and [`IoError::SourceFileOverwrite`] is
//! returned.  The UI never passes a flag to bypass this check; the original
//! file is never modified.

use std::path::Path;

use lantern_core::emit::{emit, EmitOptions};
use lantern_core::model::document::Document;

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Read a bookmark file from `path` into a [`Document`].
///
/// Accepts Netscape / Firefox HTML and Chrome / Chromium `Bookmarks`
/// JSON. Format is sniffed from the bytes (a leading `{` / `[` after a
/// BOM and whitespace selects the JSON parser). Opens the file read-only
/// (PRD F-IMP-5); the original path is recorded so
/// [`write_bookmark_file`] can refuse to overwrite it.
pub fn read_bookmark_file(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;

    let mut doc = lantern_core::parser::parse_auto(&bytes).map_err(|e| IoError::BookmarkParse {
        path: path.to_owned(),
        source: e,
    })?;

    // Record the canonical source path so the write-protection check has
    // something to compare against.
    doc.path = Some(path.to_owned());

    Ok(doc)
}

/// Serialise `doc` to Netscape HTML and write it to `path` atomically.
///
/// Returns [`IoError::SourceFileOverwrite`] if `path` resolves to the same
/// file that `doc` was originally read from (PRD F-EXP-7).
pub fn write_bookmark_file(path: &Path, doc: &Document, opts: &EmitOptions) -> Result<()> {
    // Source-file protection: compare canonical paths.
    if let Some(source) = &doc.path {
        if same_canonical_path(source, path) {
            return Err(IoError::SourceFileOverwrite(path.to_owned()));
        }
    }

    let bytes = emit(doc, opts);
    write_atomic(path, &bytes)
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns `true` if `a` and `b` refer to the same filesystem object.
///
/// Uses `canonicalize` where possible; falls back to a direct comparison for
/// paths that do not yet exist (e.g. a new export destination).
fn same_canonical_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_core::model::node::Node;

    const MINIMAL: &[u8] = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks</H1>
<DL><p>
    <DT><A HREF="https://example.com" ADD_DATE="1700000000">Example</A>
    <DD>A description.
    <DT><H3 ADD_DATE="1700000001">Folder</H3>
    <DL><p>
        <DT><A HREF="https://rust-lang.org">Rust</A>
        <HR>
    </DL><p>
</DL><p>
"#;

    #[test]
    fn round_trip_through_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("bookmarks.html");
        let dst = dir.path().join("export.html");

        // Write the fixture to a temp file so read_bookmark_file has a real path.
        std::fs::write(&src, MINIMAL).unwrap();

        let doc = read_bookmark_file(&src).unwrap();
        assert_eq!(doc.stats.bookmark_count, 2);
        assert_eq!(doc.stats.folder_count, 1);
        assert_eq!(doc.stats.separator_count, 1);

        write_bookmark_file(&dst, &doc, &EmitOptions::default()).unwrap();

        // Re-parse the exported file.
        let doc2 = read_bookmark_file(&dst).unwrap();
        assert_eq!(doc.stats.bookmark_count, doc2.stats.bookmark_count);
        assert_eq!(doc.stats.folder_count, doc2.stats.folder_count);
        assert_eq!(doc.stats.separator_count, doc2.stats.separator_count);

        let Node::Bookmark(bm) = &doc2.root.children[0] else {
            panic!()
        };
        assert_eq!(bm.title, "Example");
    }

    #[test]
    fn reads_chrome_bookmarks_json() {
        let dir = tempfile::tempdir().unwrap();
        // Chromium's on-disk name has no extension; detection is by content.
        let src = dir.path().join("Bookmarks");
        let fixture = include_str!("../tests/fixtures/chrome-bookmarks.json");
        std::fs::write(&src, fixture).unwrap();

        let doc = read_bookmark_file(&src).unwrap();
        assert_eq!(doc.stats.bookmark_count, 3);
        assert_eq!(doc.stats.folder_count, 3);
        assert_eq!(doc.path.as_deref(), Some(src.as_path()));

        let Node::Folder(bar) = &doc.root.children[0] else {
            panic!("expected Bookmarks bar");
        };
        assert!(bar.is_toolbar);
        let Node::Bookmark(bm) = &bar.children[0] else {
            panic!("expected UTM bookmark");
        };
        assert!(bm.url.as_str().contains("utm_source="));
    }

    #[test]
    fn convert_json_write_emits_netscape_html() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Bookmarks");
        let dst = dir.path().join("bookmarks.html");
        let fixture = include_str!("../tests/fixtures/chrome-bookmarks.json");
        std::fs::write(&src, fixture).unwrap();

        let doc = read_bookmark_file(&src).unwrap();
        write_bookmark_file(&dst, &doc, &EmitOptions::default()).unwrap();

        let emitted = std::fs::read_to_string(&dst).unwrap();
        assert!(
            emitted.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"),
            "convert output must be Netscape HTML"
        );
        assert!(emitted.contains("utm_source=newsletter"));

        // The GUI path: re-open the emitted HTML.
        let doc2 = read_bookmark_file(&dst).unwrap();
        assert_eq!(doc2.stats.bookmark_count, doc.stats.bookmark_count);
        assert_eq!(doc2.stats.folder_count, doc.stats.folder_count);
    }

    #[test]
    fn write_refuses_to_overwrite_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookmarks.html");
        std::fs::write(&path, MINIMAL).unwrap();

        let doc = read_bookmark_file(&path).unwrap();
        let result = write_bookmark_file(&path, &doc, &EmitOptions::default());

        assert!(
            matches!(result, Err(IoError::SourceFileOverwrite(_))),
            "expected SourceFileOverwrite, got {:?}",
            result
        );
    }
}
