//! Read bookmark files and write Netscape bookmark HTML.
//!
//! Reads accept Netscape / Firefox HTML or Chrome / Chromium `Bookmarks`
//! JSON (sniffed from the bytes).  Writes always emit Netscape HTML: Lantern
//! never writes back to a browser profile.
//!
//! # Source-file protection
//!
//! [`read_bookmark_file`] records the path it read in `doc.path`, and
//! [`write_bookmark_file`] refuses with [`IoError::SourceFileOverwrite`] when
//! the destination resolves to that same file.  There is no bypass: the
//! original file is never modified.

use std::path::Path;

use lantern_core::emit::{emit, EmitOptions};
use lantern_core::model::document::Document;

use crate::atomic::write_atomic;
use crate::error::{IoError, Result};

/// Read a bookmark file from `path` into a [`Document`].
///
/// The format is sniffed from the bytes: a leading `{` / `[` (after a BOM
/// and whitespace) selects the Chrome JSON parser, anything else the HTML
/// parser.  The file is opened read-only.
pub fn read_bookmark_file(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path).map_err(|e| IoError::Read {
        path: path.to_owned(),
        source: e,
    })?;

    let mut doc = lantern_core::parser::parse_auto(&bytes).map_err(|e| IoError::BookmarkParse {
        path: path.to_owned(),
        source: e,
    })?;
    doc.path = Some(path.to_owned());
    Ok(doc)
}

/// Serialise `doc` to Netscape HTML and write it to `path` atomically.
///
/// Returns [`IoError::SourceFileOverwrite`] if `path` resolves to the file
/// `doc` was read from.
pub fn write_bookmark_file(path: &Path, doc: &Document, opts: &EmitOptions) -> Result<()> {
    if let Some(source) = &doc.path {
        if same_file(source, path) {
            return Err(IoError::SourceFileOverwrite(path.to_owned()));
        }
    }
    write_atomic(path, &emit(doc, opts))
}

/// True if `a` and `b` refer to the same filesystem object.
///
/// Compares canonical paths; falls back to a literal comparison when either
/// path does not exist yet (e.g. a new export destination).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

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
    fn html_round_trips_through_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("bookmarks.html");
        let dst = dir.path().join("export.html");
        std::fs::write(&src, MINIMAL).unwrap();

        let doc = read_bookmark_file(&src).unwrap();
        assert_eq!(doc.stats.bookmark_count, 2);
        assert_eq!(doc.stats.folder_count, 1);
        assert_eq!(doc.stats.separator_count, 1);

        write_bookmark_file(&dst, &doc, &EmitOptions::default()).unwrap();

        let doc2 = read_bookmark_file(&dst).unwrap();
        assert_eq!(doc.stats.bookmark_count, doc2.stats.bookmark_count);
        assert_eq!(doc.stats.folder_count, doc2.stats.folder_count);
        assert_eq!(doc.stats.separator_count, doc2.stats.separator_count);
        let Node::Bookmark(bm) = &doc2.root.children[0] else {
            panic!("expected a bookmark first");
        };
        assert_eq!(bm.title, "Example");
    }

    #[test]
    fn chrome_json_is_read_and_written_as_netscape_html() {
        let dir = tempfile::tempdir().unwrap();
        // Chromium's on-disk name has no extension; detection is by content.
        let src = dir.path().join("Bookmarks");
        let dst = dir.path().join("bookmarks.html");
        std::fs::write(
            &src,
            include_str!("../tests/fixtures/chrome-bookmarks.json"),
        )
        .unwrap();

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

        write_bookmark_file(&dst, &doc, &EmitOptions::default()).unwrap();
        let emitted = std::fs::read_to_string(&dst).unwrap();
        assert!(emitted.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(emitted.contains("utm_source=newsletter"));

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
            "expected SourceFileOverwrite, got {result:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), MINIMAL);
    }
}
