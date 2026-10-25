//! Document → Chrome / Chromium `Bookmarks` JSON.
//!
//! The output reads back through [`parse_chrome_json`](crate::parser::parse_chrome_json)
//! into the same tree, and has the shape Chromium browsers load:
//!
//! ```json
//! {
//!   "roots": {
//!     "bookmark_bar": { "type": "folder", "name": "Bookmarks bar", "children": [ … ] },
//!     "other":        { "type": "folder", "name": "Other bookmarks", "children": [ … ] },
//!     "synced":       { "type": "folder", "name": "Mobile bookmarks", "children": [ … ] }
//!   },
//!   "version": 1
//! }
//! ```
//!
//! # Mapping the tree onto Chrome's three roots
//!
//! - `bookmark_bar` holds the children of the first top-level toolbar folder
//!   (`PERSONAL_TOOLBAR_FOLDER`).
//! - `synced` holds the children of a top-level folder named
//!   "Mobile bookmarks".
//! - `other` holds everything else at the top level, in document order. A
//!   top-level folder named "Other bookmarks" (or Firefox's
//!   `UNFILED_BOOKMARKS_FOLDER`) is unwrapped in place rather than nested.
//!
//! All three roots are always written, since Chrome refuses a file without
//! them. Separators are dropped (Chrome has none), as are favicons,
//! descriptions and unknown attributes. There is no `checksum`: Chrome
//! accepts a file without one. Timestamps are Chrome-epoch microseconds as
//! decimal strings; a missing timestamp is omitted.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::model::document::Document;
use crate::model::node::{Folder, Node};
use crate::parser::utc_to_chrome_time;

const OTHER_NAME: &str = "Other bookmarks";
const MOBILE_NAME: &str = "Mobile bookmarks";

#[derive(Serialize)]
struct File<'a> {
    roots: Roots<'a>,
    version: u32,
}

#[derive(Serialize)]
struct Roots<'a> {
    bookmark_bar: JsonNode<'a>,
    other: JsonNode<'a>,
    synced: JsonNode<'a>,
}

#[derive(Serialize)]
struct JsonNode<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<JsonNode<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_added: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_modified: Option<String>,
    id: String,
    name: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
}

/// Serialise `doc` as Chrome `Bookmarks` JSON (pretty-printed, UTF-8, no BOM).
pub fn emit_json(doc: &Document) -> Vec<u8> {
    let top = &doc.root.children;
    let bar = top
        .iter()
        .find_map(|n| n.as_folder().filter(|f| f.is_toolbar));
    let mobile = top.iter().find_map(|n| {
        n.as_folder()
            .filter(|f| !f.is_toolbar && f.name.eq_ignore_ascii_case(MOBILE_NAME))
    });
    let other = top
        .iter()
        .find_map(|n| n.as_folder().filter(|f| is_other_folder(f)));
    let is = |candidate: Option<&Folder>, f: &Folder| candidate.is_some_and(|c| std::ptr::eq(c, f));

    let mut ids = 3u64; // 1–3 are the roots
    let mut next_id = move || {
        ids += 1;
        ids.to_string()
    };

    let bar_children = bar.map_or_else(Vec::new, |f| convert_all(&f.children, &mut next_id));
    let mut other_children = Vec::new();
    for node in top {
        match node {
            Node::Folder(f) if is(bar, f) || is(mobile, f) => {}
            Node::Folder(f) if is(other, f) => {
                other_children.extend(convert_all(&f.children, &mut next_id));
            }
            _ => other_children.extend(convert(node, &mut next_id)),
        }
    }
    let mobile_children = mobile.map_or_else(Vec::new, |f| convert_all(&f.children, &mut next_id));

    let file = File {
        roots: Roots {
            bookmark_bar: root(
                "1",
                bar.map_or("Bookmarks bar", |f| f.name.as_str()),
                bar,
                bar_children,
            ),
            other: root("2", OTHER_NAME, other, other_children),
            synced: root("3", MOBILE_NAME, mobile, mobile_children),
        },
        version: 1,
    };

    let mut out = serde_json::to_vec_pretty(&file).expect("serialising plain structs cannot fail");
    out.push(b'\n');
    out
}

/// One of the three root folders. Chrome shows its own localised root names,
/// so `name` matters only for the round trip back into Lantern.
fn root<'a>(
    id: &str,
    name: &'a str,
    source: Option<&Folder>,
    children: Vec<JsonNode<'a>>,
) -> JsonNode<'a> {
    JsonNode {
        children: Some(children),
        date_added: source.and_then(|f| f.add_date).map(chrome_time),
        date_modified: source.and_then(|f| f.last_modified).map(chrome_time),
        id: id.to_owned(),
        name,
        kind: "folder",
        url: None,
    }
}

/// True for the folder that stands for Chrome's "Other bookmarks" root.
fn is_other_folder(f: &Folder) -> bool {
    !f.is_toolbar
        && (f.name.eq_ignore_ascii_case(OTHER_NAME)
            || f.attrs.contains_key("unfiled_bookmarks_folder"))
}

fn convert_all<'a>(nodes: &'a [Node], next_id: &mut impl FnMut() -> String) -> Vec<JsonNode<'a>> {
    nodes.iter().filter_map(|n| convert(n, next_id)).collect()
}

fn convert<'a>(node: &'a Node, next_id: &mut impl FnMut() -> String) -> Option<JsonNode<'a>> {
    match node {
        Node::Folder(f) => Some(JsonNode {
            id: next_id(),
            children: Some(convert_all(&f.children, next_id)),
            date_added: f.add_date.map(chrome_time),
            date_modified: f.last_modified.map(chrome_time),
            name: &f.name,
            kind: "folder",
            url: None,
        }),
        Node::Bookmark(b) => Some(JsonNode {
            id: next_id(),
            children: None,
            date_added: b.add_date.map(chrome_time),
            date_modified: b.last_modified.map(chrome_time),
            name: &b.title,
            kind: "url",
            url: Some(b.url.as_str()),
        }),
        Node::Separator(_) => None,
    }
}

/// UTC → Chrome-epoch microseconds, as the decimal string Chrome writes.
fn chrome_time(t: DateTime<Utc>) -> String {
    utc_to_chrome_time(t).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, parse_chrome_json};

    const FIXTURE: &str = include_str!("../../tests/fixtures/chrome-bookmarks.json");

    fn value(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn chrome_file_round_trips() {
        let doc = parse_chrome_json(FIXTURE.as_bytes()).unwrap();
        let again = parse_chrome_json(&emit_json(&doc)).unwrap();

        // The re-import gains an empty "Mobile bookmarks" root, since every
        // written file carries all three.
        assert_eq!(again.root.children.len(), doc.root.children.len() + 1);
        for (a, b) in doc.root.children.iter().zip(&again.root.children) {
            assert_same_tree(a, b);
        }
    }

    fn assert_same_tree(a: &Node, b: &Node) {
        match (a, b) {
            (Node::Folder(a), Node::Folder(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.is_toolbar, b.is_toolbar);
                assert_eq!(a.add_date, b.add_date);
                assert_eq!(a.children.len(), b.children.len(), "{}", a.name);
                for (x, y) in a.children.iter().zip(&b.children) {
                    assert_same_tree(x, y);
                }
            }
            (Node::Bookmark(a), Node::Bookmark(b)) => {
                assert_eq!(a.title, b.title);
                assert_eq!(a.url.as_str(), b.url.as_str());
                assert_eq!(a.add_date, b.add_date);
                assert_eq!(a.last_modified, b.last_modified);
            }
            (a, b) => panic!("node kinds differ: {a:?} vs {b:?}"),
        }
    }

    #[test]
    fn html_tree_maps_onto_chrome_roots() {
        let html = br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
    <DT><H3 PERSONAL_TOOLBAR_FOLDER="true">Toolbar</H3>
    <DL><p>
        <DT><A HREF="https://bar.example/" ADD_DATE="1700000000">Bar</A>
        <HR>
    </DL><p>
    <DT><A HREF="https://loose.example/">Loose</A>
    <DT><H3 UNFILED_BOOKMARKS_FOLDER="true">Other Bookmarks</H3>
    <DL><p>
        <DT><A HREF="https://unfiled.example/">Unfiled</A>
    </DL><p>
    <DT><H3>Menu folder</H3>
    <DL><p></DL><p>
    <DT><H3>Mobile bookmarks</H3>
    <DL><p>
        <DT><A HREF="https://phone.example/">Phone</A>
    </DL><p>
</DL><p>
"#;
        let json = value(&emit_json(&parse(html).unwrap()));
        let roots = &json["roots"];

        let bar = roots["bookmark_bar"]["children"].as_array().unwrap();
        assert_eq!(roots["bookmark_bar"]["name"], "Toolbar");
        assert_eq!(bar.len(), 1, "separator dropped");
        assert_eq!(bar[0]["url"], "https://bar.example/");
        assert_eq!(bar[0]["date_added"], "13344473600000000");

        let other: Vec<_> = roots["other"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["name"].as_str().unwrap())
            .collect();
        assert_eq!(other, ["Loose", "Unfiled", "Menu folder"]);

        assert_eq!(roots["synced"]["children"][0]["name"], "Phone");
        assert_eq!(json["version"], 1);
    }

    #[test]
    fn empty_document_still_has_all_roots() {
        let doc = parse(b"<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<DL><p>\n</DL><p>\n").unwrap();
        let json = value(&emit_json(&doc));
        for key in ["bookmark_bar", "other", "synced"] {
            assert_eq!(json["roots"][key]["type"], "folder", "{key}");
            assert!(json["roots"][key]["children"]
                .as_array()
                .unwrap()
                .is_empty());
        }
    }

    #[test]
    fn ids_are_unique() {
        fn collect(v: &serde_json::Value, ids: &mut Vec<String>) {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                ids.push(id.to_owned());
            }
            for child in v
                .get("children")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
            {
                collect(child, ids);
            }
        }
        let html = br#"<DL><p><DT><H3>A</H3><DL><p><DT><A HREF="https://a/">a</A></DL><p><DT><A HREF="https://b/">b</A></DL><p>"#;
        let json = value(&emit_json(&parse(html).unwrap()));
        let mut ids = Vec::new();
        for key in ["bookmark_bar", "other", "synced"] {
            collect(&json["roots"][key], &mut ids);
        }
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), 6, "3 roots + a folder + 2 bookmarks: {ids:?}");
        assert_eq!(unique.len(), ids.len());
    }
}
