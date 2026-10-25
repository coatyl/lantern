//! Document → a nested Markdown link list.
//!
//! ```markdown
//! # Bookmarks
//!
//! - **Folder**
//!   - [Example](<https://example.com/>)
//! - [Rust](<https://rust-lang.org/>)
//! ```
//!
//! Folders are bold list items with their contents nested under them.
//! Destinations are wrapped in `<…>` so URLs with parentheses or spaces stay
//! intact. Separators, timestamps and favicons are dropped; a bookmark's
//! description follows its link after an em dash.

use crate::model::document::Document;
use crate::model::node::{Folder, Node};

/// Serialise `doc` as a Markdown document (UTF-8, LF line endings).
pub fn emit_markdown(doc: &Document) -> Vec<u8> {
    let title = doc
        .header
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or("Bookmarks");
    let mut out = format!("# {}\n\n", escape_text(title));
    write_folder(&mut out, &doc.root, 0);
    out.into_bytes()
}

fn write_folder(out: &mut String, folder: &Folder, depth: usize) {
    for child in &folder.children {
        match child {
            Node::Folder(f) => {
                indent(out, depth);
                let name = if f.name.trim().is_empty() {
                    "Untitled folder"
                } else {
                    &f.name
                };
                out.push_str(&format!("- **{}**\n", escape_text(name)));
                write_folder(out, f, depth + 1);
            }
            Node::Bookmark(b) => {
                indent(out, depth);
                let url = b.url.as_str();
                let text = if b.title.trim().is_empty() {
                    url
                } else {
                    &b.title
                };
                out.push_str(&format!(
                    "- [{}](<{}>)",
                    escape_text(text),
                    escape_destination(url)
                ));
                if let Some(desc) = b
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|d| !d.is_empty())
                {
                    out.push_str(" — ");
                    out.push_str(&escape_text(desc));
                }
                out.push('\n');
            }
            Node::Separator(_) => {}
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// Backslash-escape the characters that would start Markdown or HTML syntax
/// in inline text, and fold line breaks into spaces so an item stays on one
/// line.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' | '[' | ']' | '*' | '_' | '`' | '<' | '>' | '#' | '|' | '~' | '!' => {
                out.push('\\');
                out.push(ch);
            }
            '\r' | '\n' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Escape a link destination written between `<` and `>`.
fn escape_destination(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' | '<' | '>' => {
                out.push('\\');
                out.push(ch);
            }
            '\r' | '\n' => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn render(html: &[u8]) -> String {
        String::from_utf8(emit_markdown(&parse(html).unwrap())).unwrap()
    }

    #[test]
    fn nests_folders_and_links() {
        let md = render(
            br#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<TITLE>Bookmarks</TITLE>
<DL><p>
    <DT><H3>Reading</H3>
    <DL><p>
        <DT><A HREF="https://example.com/a_(b)">Example</A>
        <DD>Worth a look.
        <HR>
        <DT><H3>Deeper</H3>
        <DL><p>
            <DT><A HREF="https://rust-lang.org/">Rust</A>
        </DL><p>
    </DL><p>
    <DT><A HREF="https://top.example/"></A>
</DL><p>
"#,
        );
        assert_eq!(
            md,
            "# Bookmarks\n\n\
             - **Reading**\n\
             \x20 - [Example](<https://example.com/a_(b)>) — Worth a look.\n\
             \x20 - **Deeper**\n\
             \x20   - [Rust](<https://rust-lang.org/>)\n\
             - [https://top.example/](<https://top.example/>)\n"
        );
    }

    #[test]
    fn escapes_markdown_syntax_in_titles() {
        let md = render(
            br#"<DL><p><DT><A HREF="https://x.example/">[a] *b* _c_ `d` &lt;e&gt;</A></DL><p>"#,
        );
        assert!(
            md.contains(r"- [\[a\] \*b\* \_c\_ \`d\` \<e\>](<https://x.example/>)"),
            "{md}"
        );
    }
}
