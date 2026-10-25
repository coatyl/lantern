//! Document → CSV, one row per bookmark.
//!
//! ```text
//! title,url,folder,added,modified
//! Example,https://example.com/,Reading / Deeper,2023-11-14T22:13:20Z,
//! ```
//!
//! `folder` is the path of folder names from the top level, joined with
//! ` / `; it is empty for top-level bookmarks. Timestamps are ISO 8601 in
//! UTC. The file follows RFC 4180 (CRLF rows, fields with commas, quotes or
//! line breaks quoted) and starts with a UTF-8 BOM so spreadsheet apps pick
//! the right encoding.
//!
//! A field that starts with `=`, `+`, `-`, `@`, a tab or a carriage return
//! is prefixed with `'` so a spreadsheet shows it as text instead of running
//! it as a formula.

use chrono::{DateTime, SecondsFormat, Utc};

use super::walk;
use crate::model::document::Document;
use crate::model::node::Node;

const HEADER: &str = "title,url,folder,added,modified\r\n";

/// Serialise the bookmarks in `doc` as CSV.
pub fn emit_csv(doc: &Document) -> Vec<u8> {
    let mut out = String::from("\u{feff}");
    out.push_str(HEADER);
    walk(&doc.root, &mut Vec::new(), &mut |path, node| {
        let Node::Bookmark(b) = node else { return };
        let fields = [
            field(&b.title),
            field(b.url.as_str()),
            field(&path.join(" / ")),
            timestamp(b.add_date),
            timestamp(b.last_modified),
        ];
        out.push_str(&fields.join(","));
        out.push_str("\r\n");
    });
    out.into_bytes()
}

fn timestamp(t: Option<DateTime<Utc>>) -> String {
    t.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Neutralise formula triggers, then quote the field if RFC 4180 needs it.
fn field(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn writes_one_row_per_bookmark_with_folder_paths() {
        let doc = parse(
            br#"<DL><p>
    <DT><A HREF="https://top.example/" ADD_DATE="1700000000">Top</A>
    <DT><H3>Reading</H3>
    <DL><p>
        <HR>
        <DT><H3>Deeper</H3>
        <DL><p>
            <DT><A HREF="https://rust-lang.org/" LAST_MODIFIED="1700000000">Rust, "the book"</A>
        </DL><p>
    </DL><p>
</DL><p>
"#,
        )
        .unwrap();
        let csv = String::from_utf8(emit_csv(&doc)).unwrap();
        assert_eq!(
            csv,
            "\u{feff}title,url,folder,added,modified\r\n\
             Top,https://top.example/,,2023-11-14T22:13:20Z,\r\n\
             \"Rust, \"\"the book\"\"\",https://rust-lang.org/,Reading / Deeper,,2023-11-14T22:13:20Z\r\n"
        );
    }

    #[test]
    fn formula_triggers_are_neutralised() {
        assert_eq!(field("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(field("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(field("-1"), "'-1");
        assert_eq!(field("plain"), "plain");
        assert_eq!(field("line\nbreak"), "\"line\nbreak\"");
    }
}
