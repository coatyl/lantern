//! DOM → Document tree builder.
//!
//! Takes the `RcDom` produced by html5ever and walks it to extract the Netscape
//! bookmark structure, producing a fully-populated [`Document`].
//!
//! # Netscape format recap
//!
//! ```html
//! <!DOCTYPE NETSCAPE-Bookmark-file-1>
//! <META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
//! <TITLE>Bookmarks</TITLE>
//! <H1>Bookmarks</H1>
//! <DL><p>
//!   <DT><H3 ADD_DATE="…" PERSONAL_TOOLBAR_FOLDER="true">Toolbar</H3>
//!   <DL><p>
//!     <DT><A HREF="…" ADD_DATE="…" ICON="…">Title</A>
//!     <DD>Optional description
//!     <HR>
//!   </DL><p>
//! </DL><p>
//! ```
//!
//! After html5ever's HTML5 tree construction, this becomes a normal DOM with
//! `<body>` containing `<h1>` and `<dl>`. The stray `<p>` inside `<dl>` is
//! emitted as a sibling element and is simply skipped by our walker.

use chrono::{DateTime, Utc};
use markup5ever_rcdom::{Handle, NodeData};

use super::tokenize::TokenizeResult;
use crate::error::{CoreError, Result};
use crate::model::{
    document::{Document, DocumentStats, HeaderMetadata, LineEnding},
    ids::{next_document_id, NodeIdAllocator},
    node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node, Separator},
};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn build_document(result: TokenizeResult) -> Result<Document> {
    let TokenizeResult {
        dom,
        has_bom,
        line_ending,
    } = result;
    let mut id_gen = NodeIdAllocator::new();

    let header = extract_header(&dom.document, has_bom, line_ending);

    // The root <DL> is the top-level bookmark list. If we cannot find it, the
    // file is not a bookmark HTML at all.
    let root_dl = find_element_dfs(&dom.document, "dl").ok_or_else(|| CoreError::ParseFailed {
        line: 0,
        column: 0,
        reason: "no root <DL> element found; is this a Netscape bookmark HTML file?".into(),
    })?;

    let children = parse_dl_contents(&root_dl, &mut id_gen);

    // The document root is an unnamed synthetic folder that wraps all top-level
    // items, consistent with how every browser renders a bookmark file.
    let root = Folder {
        id: id_gen.alloc(),
        name: String::new(),
        add_date: None,
        last_modified: None,
        is_toolbar: false,
        attrs: AttrMap::default(),
        children,
    };

    let stats = DocumentStats::from_root(&root);

    Ok(Document::new(next_document_id(), None, root, header, stats))
}

// ---------------------------------------------------------------------------
// Header extraction
// ---------------------------------------------------------------------------

fn extract_header(doc: &Handle, has_bom: bool, line_ending: LineEnding) -> HeaderMetadata {
    let mut charset = None;
    let mut title = None;

    visit_all(doc, &mut |node| match tag_name(node).as_deref() {
        Some("meta") => {
            let attrs = element_attrs(node);
            let is_content_type = attr_val(&attrs, "http-equiv")
                .is_some_and(|v| v.eq_ignore_ascii_case("content-type"));
            if is_content_type {
                if let Some(content) = attr_val(&attrs, "content") {
                    // "text/html; charset=UTF-8"
                    if let Some(cs) = content.split("charset=").nth(1) {
                        charset = Some(cs.trim().to_owned());
                    }
                }
            }
        }
        Some("title") => {
            let t = text_content(node);
            let t = t.trim();
            if !t.is_empty() {
                title = Some(t.to_owned());
            }
        }
        _ => {}
    });

    HeaderMetadata {
        charset,
        title,
        has_bom,
        line_ending,
    }
}

// ---------------------------------------------------------------------------
// Bookmark structure parsing
// ---------------------------------------------------------------------------

/// Parse the contents of a `<DL>` into a `Vec<Node>`.
///
/// html5ever's HTML5 parser produces slightly different trees depending on the
/// exact whitespace and quoting in the source file:
///
/// - Sometimes `<DT>` elements are **direct children** of `<DL>`.
/// - Sometimes they are **inside a `<P>`** that is a direct child of `<DL>`
///   (the Netscape `<DL><p>` idiom triggers this path).
/// - Sometimes the nested `<DL>` for a folder's contents lands **inside the
///   `<DT>`** rather than as a sibling of it.
///
/// [`flatten_dl`] and [`find_sub_dl`] handle all three cases.
fn parse_dl_contents(dl: &Handle, id_gen: &mut NodeIdAllocator) -> Vec<Node> {
    // Flatten: treat any immediate <p> children as transparent wrappers so we
    // get a single uniform sibling list regardless of the <DL><p> variant.
    let siblings = flatten_dl(dl);
    let mut nodes = Vec::new();

    for (i, child) in siblings.iter().enumerate() {
        match tag_name(child).as_deref() {
            Some("dt") => {
                let dt_kids: Vec<Handle> = child.children.borrow().iter().cloned().collect();

                if let Some(inner) = dt_kids
                    .iter()
                    .find(|n| matches!(tag_name(n).as_deref(), Some("h3") | Some("a")))
                {
                    match tag_name(inner).as_deref() {
                        Some("h3") => {
                            // The subfolder <DL> might be inside this <DT> or
                            // the next sibling. Check both.
                            let sub_dl = find_sub_dl(child, &siblings, i);
                            nodes.push(Node::Folder(parse_folder(inner, sub_dl, id_gen)));
                        }
                        Some("a") => {
                            let desc = next_sibling_dd_text(&siblings, i);
                            nodes.push(Node::Bookmark(parse_bookmark(inner, desc, id_gen)));
                            // html5ever quirk: <HR> following a bare <DT><A>…
                            // lands *inside* the still-open <DT> rather than as
                            // its sibling.  Emit a Separator for each such <HR>.
                            for sib in dt_kids.iter() {
                                if tag_name(sib).as_deref() == Some("hr") {
                                    nodes.push(Node::Separator(Separator { id: id_gen.alloc() }));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some("hr") => {
                nodes.push(Node::Separator(Separator { id: id_gen.alloc() }));
            }
            // <p>, stray <dl>, <dd>, text nodes, comments: all skipped.
            _ => {}
        }
    }

    nodes
}

/// Flatten the immediate children of a `<DL>` element, treating any direct
/// `<p>` children as transparent containers.
///
/// This normalises the two html5ever output shapes for `<DL><p><DT>…`:
/// - Shape A: `<DL> → [<DT>, …]`  (DTs are direct DL children)
/// - Shape B: `<DL> → [<P> → [<DT>, …]]`  (DTs are inside the <p>)
fn flatten_dl(dl: &Handle) -> Vec<Handle> {
    let mut items = Vec::new();
    for child in dl.children.borrow().iter() {
        if tag_name(child).as_deref() == Some("p") {
            items.extend(child.children.borrow().iter().cloned());
        } else {
            items.push(child.clone());
        }
    }
    items
}

/// Find the `<DL>` that holds a folder's contents.
///
/// html5ever can place it either:
/// 1. **Inside the `<DT>`**: when it sees `<H3>…</H3><DL>` without an
///    explicit `</DT>` first (the `<DL>` is block-level and gets reparented).
/// 2. **As a sibling** of the `<DT>` in the parent `<DL>`.
///
/// We check location 1 first; if not found, fall back to location 2.
fn find_sub_dl(dt: &Handle, siblings: &[Handle], dt_index: usize) -> Option<Handle> {
    dt.children
        .borrow()
        .iter()
        .find(|n| tag_name(n).as_deref() == Some("dl"))
        .cloned()
        .or_else(|| next_sibling_with_tag(siblings, dt_index, "dl"))
}

fn parse_folder(h3: &Handle, sub_dl: Option<Handle>, id_gen: &mut NodeIdAllocator) -> Folder {
    let raw = element_attrs(h3);

    let add_date = attr_val(&raw, "add_date").and_then(|s| parse_timestamp(&s));
    let last_modified = attr_val(&raw, "last_modified").and_then(|s| parse_timestamp(&s));
    let is_toolbar =
        attr_val(&raw, "personal_toolbar_folder").is_some_and(|v| v.eq_ignore_ascii_case("true"));

    let mut attrs = AttrMap::default();
    for (k, v) in raw {
        attrs.insert(k, v);
    }

    let children = sub_dl
        .map(|dl| parse_dl_contents(&dl, id_gen))
        .unwrap_or_default();

    Folder {
        id: id_gen.alloc(),
        name: text_content(h3).trim().to_owned(),
        add_date,
        last_modified,
        is_toolbar,
        attrs,
        children,
    }
}

fn parse_bookmark(a: &Handle, desc: Option<String>, id_gen: &mut NodeIdAllocator) -> Bookmark {
    let raw = element_attrs(a);

    let href = attr_val(&raw, "href").unwrap_or_default();
    let url = match url::Url::parse(&href) {
        Ok(u) => BookmarkUrl::Valid(u),
        Err(_) => BookmarkUrl::Malformed { raw: href },
    };

    let add_date = attr_val(&raw, "add_date").and_then(|s| parse_timestamp(&s));
    let last_modified = attr_val(&raw, "last_modified").and_then(|s| parse_timestamp(&s));
    let icon_blob = attr_val(&raw, "icon").filter(|s| !s.is_empty());

    let mut attrs = AttrMap::default();
    for (k, v) in raw {
        attrs.insert(k, v);
    }

    Bookmark {
        id: id_gen.alloc(),
        title: text_content(a).trim().to_owned(),
        url,
        add_date,
        last_modified,
        icon_blob,
        description: desc,
        attrs,
        flags: BookmarkFlags::default(),
    }
}

// ---------------------------------------------------------------------------
// Sibling scanning helpers
// ---------------------------------------------------------------------------

/// Returns the first sibling after `from_index` whose tag matches `tag`.
fn next_sibling_with_tag(siblings: &[Handle], from_index: usize, tag: &str) -> Option<Handle> {
    siblings[from_index + 1..]
        .iter()
        .find(|n| tag_name(n).as_deref() == Some(tag))
        .cloned()
}

/// Returns the trimmed text of the first `<DD>` sibling immediately after
/// `from_index`, if any.
fn next_sibling_dd_text(siblings: &[Handle], from_index: usize) -> Option<String> {
    // A description can only appear as the very next meaningful sibling;
    // we check a small window to skip over intervening whitespace-only text nodes.
    siblings[from_index + 1..]
        .iter()
        .take(3)
        .find(|n| tag_name(n).as_deref() == Some("dd"))
        .map(|dd| text_content(dd).trim().to_owned())
        .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------

/// Depth-first search for the first element whose tag matches `tag`.
fn find_element_dfs(node: &Handle, tag: &str) -> Option<Handle> {
    for child in node.children.borrow().iter() {
        if tag_name(child).as_deref() == Some(tag) {
            return Some(child.clone());
        }
        if let Some(found) = find_element_dfs(child, tag) {
            return Some(found);
        }
    }
    None
}

/// Visit every node depth-first, calling `f` on each.
fn visit_all(node: &Handle, f: &mut impl FnMut(&Handle)) {
    f(node);
    for child in node.children.borrow().iter() {
        visit_all(child, f);
    }
}

/// Returns the lowercase tag name of an element, or `None` for non-elements.
fn tag_name(node: &Handle) -> Option<String> {
    match &node.data {
        NodeData::Element { name, .. } => Some(name.local.as_ref().to_ascii_lowercase()),
        _ => None,
    }
}

/// Returns all `(name, value)` pairs for an element's attributes, names lowercased.
fn element_attrs(node: &Handle) -> Vec<(String, String)> {
    match &node.data {
        NodeData::Element { attrs, .. } => attrs
            .borrow()
            .iter()
            .map(|a| {
                (
                    a.name.local.as_ref().to_ascii_lowercase(),
                    a.value.as_ref().to_owned(),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Case-insensitive attribute lookup. Returns a clone of the value if found.
fn attr_val(attrs: &[(String, String)], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k.as_str() == name)
        .map(|(_, v)| v.clone())
}

/// Recursively collects all text node content under `node`.
fn text_content(node: &Handle) -> String {
    let mut buf = String::new();
    collect_text(node, &mut buf);
    buf
}

fn collect_text(node: &Handle, buf: &mut String) {
    match &node.data {
        NodeData::Text { contents } => {
            buf.push_str(contents.borrow().as_ref());
        }
        _ => {
            for child in node.children.borrow().iter() {
                collect_text(child, buf);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Timestamp parsing
// ---------------------------------------------------------------------------

/// Parses a Netscape `ADD_DATE` / `LAST_MODIFIED` value (Unix seconds) into a
/// `DateTime<Utc>`. Returns `None` for missing, empty, or out-of-range values.
fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    s.trim()
        .parse::<i64>()
        .ok()
        .and_then(DateTime::from_timestamp_secs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

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
    fn parses_minimal_fixture() {
        let doc = parse(MINIMAL).expect("should parse without error");

        // Root folder wraps all top-level items.
        let children = &doc.root.children;
        assert_eq!(children.len(), 2, "expected bookmark + folder at root");

        // First child: bookmark
        let Node::Bookmark(bm) = &children[0] else {
            panic!("expected first child to be a Bookmark");
        };
        assert_eq!(bm.title, "Example");
        assert_eq!(bm.url.as_str(), "https://example.com/");
        assert_eq!(bm.description.as_deref(), Some("A description."));
        assert!(bm.add_date.is_some());

        // Second child: folder containing one bookmark and one separator
        let Node::Folder(folder) = &children[1] else {
            panic!("expected second child to be a Folder");
        };
        assert_eq!(folder.name, "Folder");
        assert_eq!(folder.children.len(), 2);
        assert!(matches!(folder.children[0], Node::Bookmark(_)));
        assert!(matches!(folder.children[1], Node::Separator(_)));
    }

    #[test]
    fn stats_are_correct() {
        let doc = parse(MINIMAL).expect("should parse");
        assert_eq!(doc.stats.bookmark_count, 2);
        assert_eq!(doc.stats.folder_count, 1);
        assert_eq!(doc.stats.separator_count, 1);
    }

    #[test]
    fn header_charset_extracted() {
        let doc = parse(MINIMAL).expect("should parse");
        assert_eq!(doc.header.charset.as_deref(), Some("UTF-8"));
        assert_eq!(doc.header.title.as_deref(), Some("Bookmarks"));
        assert!(!doc.header.has_bom);
    }

    #[test]
    fn bom_detected() {
        let mut with_bom = b"\xef\xbb\xbf".to_vec();
        with_bom.extend_from_slice(MINIMAL);
        let doc = parse(&with_bom).expect("should parse with BOM");
        assert!(doc.header.has_bom);
    }

    #[test]
    fn malformed_url_preserved() {
        let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<TITLE>Bookmarks</TITLE>
<DL><p>
    <DT><A HREF="not a url at all">Bad URL</A>
</DL><p>
"#;
        let doc = parse(html).expect("should parse even with malformed URL");
        let Node::Bookmark(bm) = &doc.root.children[0] else {
            panic!("expected a bookmark");
        };
        assert!(bm.url.is_malformed());
        assert_eq!(bm.url.as_str(), "not a url at all");
    }

    #[test]
    fn empty_file_returns_error() {
        let result = parse(b"<html><body></body></html>");
        assert!(result.is_err(), "should fail without a root <DL>");
    }
}
