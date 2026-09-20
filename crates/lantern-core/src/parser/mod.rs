//! Bookmark file parsers.
//!
//! Public entry points:
//! - [`parse`] — Netscape / Firefox bookmark HTML (the historical format).
//! - [`parse_chrome_json`] — Chrome / Chromium / Edge / Brave `Bookmarks` JSON.
//! - [`parse_auto`] — sniff JSON vs HTML and dispatch.
//! - [`looks_like_json`] — BOM- and whitespace-tolerant byte-level detector.
//!
//! Everything else is an implementation detail. The caller supplies bytes;
//! these functions never touch the filesystem (zero I/O, per the
//! `lantern-core` contract).
//!
//! Behaviour on malformed Netscape HTML (per TDD §5.3):
//! - Unclosed tags, mixed case, BOM, CRLF/LF mixing: tolerated.
//! - Unknown tags inside `<DL>`: silently skipped (stray-node preservation
//!   is a future concern; for v0.0.1 correctness is the priority).
//! - Truly unparseable (not a bookmark file at all): `CoreError::ParseFailed`.

mod build;
mod chrome;
mod tokenize;

use crate::error::Result;
use crate::model::Document;

pub use chrome::parse_chrome_json;

/// Parse a raw byte slice (the full contents of a `.html` bookmark export)
/// into an in-memory [`Document`].
///
/// This is the Netscape HTML path. Firefox's HTML export uses the same
/// format, so it flows through here unchanged. Prefer [`parse_auto`] when
/// the caller does not already know the format.
pub fn parse(bytes: &[u8]) -> Result<Document> {
    let result = tokenize::tokenize(bytes);
    build::build_document(result)
}

/// Auto-detect the bookmark format and parse it.
///
/// If the bytes look like JSON (after a BOM and leading whitespace), they
/// are handed to [`parse_chrome_json`]; otherwise they go to the Netscape
/// HTML parser. Detection never falls back: a file that looks like JSON
/// but is not a Chrome `Bookmarks` file returns that parser's error.
pub fn parse_auto(bytes: &[u8]) -> Result<Document> {
    if looks_like_json(bytes) {
        parse_chrome_json(bytes)
    } else {
        parse(bytes)
    }
}

/// True if `bytes` look like a JSON value (`{` or `[`) after a UTF-8 BOM
/// and leading ASCII whitespace.
pub fn looks_like_json(bytes: &[u8]) -> bool {
    let (_, rest) = strip_utf8_bom(bytes);
    let rest = skip_ascii_whitespace(rest);
    matches!(rest.first(), Some(b'{' | b'['))
}

/// Strip a leading UTF-8 BOM if present. Returns `(had_bom, remainder)`.
pub(crate) fn strip_utf8_bom(bytes: &[u8]) -> (bool, &[u8]) {
    if bytes.starts_with(b"\xef\xbb\xbf") {
        (true, &bytes[3..])
    } else {
        (false, bytes)
    }
}

fn skip_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let i = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[i..]
}
