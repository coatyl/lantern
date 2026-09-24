//! Document → Netscape bookmark HTML emitter.
//!
//! Entry point is [`emit`].  The output is a `Vec<u8>` (raw bytes) so the
//! caller controls where they go; this keeps `lantern-core` I/O-free.
//!
//! # Output format
//!
//! ```html
//! <!DOCTYPE NETSCAPE-Bookmark-file-1>
//! <META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
//! <TITLE>Bookmarks</TITLE>
//! <H1>Bookmarks</H1>
//! <DL><p>
//!     <DT><H3 ADD_DATE="1700000001">Folder</H3>
//!     <DL><p>
//!         <DT><A HREF="https://example.com/" ADD_DATE="1700000000">Example</A>
//!         <DD>Optional description.
//!         <HR>
//!     </DL><p>
//! </DL><p>
//! ```
//!
//! # Attribute handling
//!
//! Known structural attributes (HREF, ADD_DATE, LAST_MODIFIED,
//! PERSONAL_TOOLBAR_FOLDER, ICON) are emitted from the typed model fields so
//! that any sanitization changes are reflected.  Unknown attributes are emitted
//! verbatim from the node's [`AttrMap`] in their original insertion order
//! (F-EXP-3).
//!
//! # Round-trip invariant (TDD §5.7)
//!
//! For every well-formed export from the supported browser set,
//! `parse(bytes) → emit → parse` yields a semantically equivalent `Document`
//! (same tree, same attributes, same metadata).  Byte-identity is not
//! guaranteed but targeted where practical.

use std::fmt::Write as _;

use crate::model::document::{Document, LineEnding};
use crate::model::node::{Folder, Node};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Options controlling how the output is serialised.
#[derive(Debug, Clone, Default)]
pub struct EmitOptions {
    /// When `true` emit without indentation (useful for diffs / line counts).
    /// Default: `false`.
    pub compact: bool,
}

/// Serialise `doc` to Netscape bookmark HTML bytes.
///
/// Respects `doc.header.line_ending` and `doc.header.has_bom`.
pub fn emit(doc: &Document, opts: &EmitOptions) -> Vec<u8> {
    let le = match doc.header.line_ending {
        LineEnding::Crlf => "\r\n",
        LineEnding::Lf => "\n",
    };

    let mut buf = String::with_capacity(4096);
    write_header(&mut buf, doc, le);
    emit_folder_contents(&mut buf, &doc.root, 1, opts, le);
    buf.push_str("</DL><p>");
    buf.push_str(le);

    let bytes = buf.into_bytes();
    if doc.header.has_bom {
        let mut out = b"\xef\xbb\xbf".to_vec();
        out.extend(bytes);
        out
    } else {
        bytes
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn write_header(buf: &mut String, doc: &Document, le: &str) {
    let charset = doc.header.charset.as_deref().unwrap_or("UTF-8");

    let title = doc.header.title.as_deref().unwrap_or("Bookmarks");

    buf.push_str("<!DOCTYPE NETSCAPE-Bookmark-file-1>");
    buf.push_str(le);
    write!(
        buf,
        r#"<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset={}">"#,
        escape_attr(charset)
    )
    .unwrap();
    buf.push_str(le);
    write!(buf, "<TITLE>{}</TITLE>", escape_text(title)).unwrap();
    buf.push_str(le);
    write!(buf, "<H1>{}</H1>", escape_text(title)).unwrap();
    buf.push_str(le);
    buf.push_str("<DL><p>");
    buf.push_str(le);
}

// ---------------------------------------------------------------------------
// Tree emission
// ---------------------------------------------------------------------------

fn emit_folder_contents(
    buf: &mut String,
    folder: &Folder,
    depth: usize,
    opts: &EmitOptions,
    le: &str,
) {
    for child in &folder.children {
        match child {
            Node::Folder(f) => emit_folder_node(buf, f, depth, opts, le),
            Node::Bookmark(b) => emit_bookmark_node(buf, b, depth, opts, le),
            Node::Separator(_) => {
                if !opts.compact {
                    push_indent(buf, depth);
                }
                buf.push_str("<HR>");
                buf.push_str(le);
            }
        }
    }
}

fn emit_folder_node(
    buf: &mut String,
    f: &crate::model::node::Folder,
    depth: usize,
    opts: &EmitOptions,
    le: &str,
) {
    if !opts.compact {
        push_indent(buf, depth);
    }
    buf.push_str("<DT><H3");
    write_h3_attrs(buf, f);
    buf.push('>');
    buf.push_str(&escape_text(&f.name));
    buf.push_str("</H3>");
    buf.push_str(le);

    if !opts.compact {
        push_indent(buf, depth);
    }
    buf.push_str("<DL><p>");
    buf.push_str(le);

    emit_folder_contents(buf, f, depth + 1, opts, le);

    if !opts.compact {
        push_indent(buf, depth);
    }
    buf.push_str("</DL><p>");
    buf.push_str(le);
}

fn emit_bookmark_node(
    buf: &mut String,
    b: &crate::model::node::Bookmark,
    depth: usize,
    opts: &EmitOptions,
    le: &str,
) {
    if !opts.compact {
        push_indent(buf, depth);
    }
    buf.push_str("<DT><A");
    write_a_attrs(buf, b);
    buf.push('>');
    buf.push_str(&escape_text(&b.title));
    buf.push_str("</A>");
    buf.push_str(le);

    if let Some(desc) = &b.description {
        let desc = desc.trim();
        if !desc.is_empty() {
            if !opts.compact {
                push_indent(buf, depth);
            }
            buf.push_str("<DD>");
            buf.push_str(&escape_text(desc));
            buf.push_str(le);
        }
    }
}

// ---------------------------------------------------------------------------
// Attribute writers
// ---------------------------------------------------------------------------

/// Emit attributes for a `<H3>` element.
///
/// Canonical order: ADD_DATE, LAST_MODIFIED, PERSONAL_TOOLBAR_FOLDER, then
/// any unknown attrs from the preserved map (skipping the known names).
fn write_h3_attrs(buf: &mut String, f: &crate::model::node::Folder) {
    const KNOWN: &[&str] = &["add_date", "last_modified", "personal_toolbar_folder"];

    if let Some(ts) = f.add_date {
        write!(buf, r#" ADD_DATE="{}""#, ts.timestamp()).unwrap();
    }
    if let Some(ts) = f.last_modified {
        write!(buf, r#" LAST_MODIFIED="{}""#, ts.timestamp()).unwrap();
    }
    if f.is_toolbar {
        buf.push_str(r#" PERSONAL_TOOLBAR_FOLDER="true""#);
    }
    for (k, v) in &f.attrs {
        if !KNOWN.contains(&k.as_str()) {
            write!(buf, r#" {}="{}""#, k.to_ascii_uppercase(), escape_attr(v)).unwrap();
        }
    }
}

/// Emit attributes for a `<A>` element.
///
/// Canonical order: HREF, ADD_DATE, LAST_MODIFIED, ICON, then unknown attrs.
fn write_a_attrs(buf: &mut String, b: &crate::model::node::Bookmark) {
    const KNOWN: &[&str] = &["href", "add_date", "last_modified", "icon"];

    write!(buf, r#" HREF="{}""#, escape_attr(b.url.as_str())).unwrap();
    if let Some(ts) = b.add_date {
        write!(buf, r#" ADD_DATE="{}""#, ts.timestamp()).unwrap();
    }
    if let Some(ts) = b.last_modified {
        write!(buf, r#" LAST_MODIFIED="{}""#, ts.timestamp()).unwrap();
    }
    if let Some(icon) = &b.icon_blob {
        // Icon data URLs can be very long; emit verbatim (no length limit).
        write!(buf, r#" ICON="{}""#, escape_attr(icon)).unwrap();
    }
    for (k, v) in &b.attrs {
        if !KNOWN.contains(&k.as_str()) {
            write!(buf, r#" {}="{}""#, k.to_ascii_uppercase(), escape_attr(v)).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// HTML escaping helpers
// ---------------------------------------------------------------------------

/// Escape a string for use in an attribute value (double-quoted).
fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// Escape a string for use as element text content.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Indentation
// ---------------------------------------------------------------------------

fn push_indent(buf: &mut String, depth: usize) {
    for _ in 0..depth {
        buf.push_str("    ");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// The same minimal fixture used by the parser tests.
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
    fn emits_without_panic() {
        let doc = parse(MINIMAL).unwrap();
        let bytes = emit(&doc, &EmitOptions::default());
        assert!(!bytes.is_empty());
        let text = std::str::from_utf8(&bytes).expect("output must be valid UTF-8");
        assert!(text.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
    }

    #[test]
    fn round_trip_preserves_structure() {
        let doc1 = parse(MINIMAL).unwrap();
        let emitted = emit(&doc1, &EmitOptions::default());
        let doc2 = parse(&emitted).unwrap();

        // Same top-level item count.
        assert_eq!(doc1.root.children.len(), doc2.root.children.len());

        // Same stats.
        assert_eq!(doc1.stats.bookmark_count, doc2.stats.bookmark_count);
        assert_eq!(doc1.stats.folder_count, doc2.stats.folder_count);
        assert_eq!(doc1.stats.separator_count, doc2.stats.separator_count);

        // First child is still the bookmark with the right URL.
        let Node::Bookmark(bm1) = &doc1.root.children[0] else {
            panic!()
        };
        let Node::Bookmark(bm2) = &doc2.root.children[0] else {
            panic!()
        };
        assert_eq!(bm1.url.as_str(), bm2.url.as_str());
        assert_eq!(bm1.title, bm2.title);

        // Description round-trips.
        assert_eq!(bm1.description, bm2.description);

        // Second child is still a folder with the same name.
        let Node::Folder(f1) = &doc1.root.children[1] else {
            panic!()
        };
        let Node::Folder(f2) = &doc2.root.children[1] else {
            panic!()
        };
        assert_eq!(f1.name, f2.name);
        assert_eq!(f1.children.len(), f2.children.len());
    }

    #[test]
    fn bom_round_trips() {
        let mut with_bom = b"\xef\xbb\xbf".to_vec();
        with_bom.extend_from_slice(MINIMAL);
        let doc = parse(&with_bom).unwrap();
        let emitted = emit(&doc, &EmitOptions::default());
        assert!(
            emitted.starts_with(b"\xef\xbb\xbf"),
            "BOM must be preserved"
        );
    }

    #[test]
    fn compact_mode_produces_no_leading_spaces() {
        let doc = parse(MINIMAL).unwrap();
        let bytes = emit(&doc, &EmitOptions { compact: true });
        let text = std::str::from_utf8(&bytes).unwrap();
        // No line should start with spaces in compact mode.
        for line in text.lines() {
            assert!(
                !line.starts_with(' '),
                "unexpected indent in compact mode: {:?}",
                line
            );
        }
    }

    #[test]
    fn unknown_attributes_round_trip_including_icon_uri() {
        let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
    <DT><A HREF="https://a.example/" ICON_URI="https://a.example/favicon.ico" TAGS="x,y">A</A>
</DL><p>
"#;
        let emitted = emit(&parse(html).unwrap(), &EmitOptions::default());
        let text = std::str::from_utf8(&emitted).unwrap();
        assert!(
            text.contains(r#"ICON_URI="https://a.example/favicon.ico" TAGS="x,y""#),
            "{text}"
        );
    }

    #[test]
    fn header_charset_is_escaped() {
        let mut doc = parse(MINIMAL).unwrap();
        doc.header.charset = Some(r#"x"><b>"#.into());
        let emitted = emit(&doc, &EmitOptions::default());
        let text = std::str::from_utf8(&emitted).unwrap();
        assert!(text.contains("charset=x&quot;&gt;&lt;b&gt;\">"), "{text}");
    }

    #[test]
    fn special_chars_escaped_in_title() {
        use crate::model::document::{DocumentStats, HeaderMetadata};
        use crate::model::ids::next_document_id;
        use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder};

        let bm = Bookmark {
            id: 1,
            title: "A & B < C > D".into(),
            url: BookmarkUrl::Valid(url::Url::parse("https://example.com/").unwrap()),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        };

        let root = Folder {
            id: 0,
            name: String::new(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![Node::Bookmark(bm)],
        };

        let doc = Document::new(
            next_document_id(),
            None,
            root,
            HeaderMetadata::default(),
            DocumentStats::default(),
        );
        let bytes = emit(&doc, &EmitOptions::default());
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("A &amp; B &lt; C &gt; D"));
        assert!(!text.contains("A & B"));
    }
}
