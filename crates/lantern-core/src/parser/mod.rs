//! Netscape bookmark HTML parser.
//!
//! Public entry point is [`parse`]. Everything else is an implementation detail.
//!
//! Behaviour on malformed input (per TDD §5.3):
//! - Unclosed tags, mixed case, BOM, CRLF/LF mixing: tolerated.
//! - Unknown tags inside `<DL>`: silently skipped (stray-node preservation
//!   is a future concern; for v0.0.1 correctness is the priority).
//! - Truly unparseable (not a bookmark file at all): `CoreError::ParseFailed`.

mod build;
mod tokenize;

use crate::error::Result;
use crate::model::Document;

/// Parse a raw byte slice (the full contents of a `.html` bookmark export) into
/// an in-memory [`Document`].
///
/// The caller is responsible for reading the file bytes; this function never
/// touches the filesystem (zero I/O, per the `lantern-core` contract).
pub fn parse(bytes: &[u8]) -> Result<Document> {
    let result = tokenize::tokenize(bytes);
    build::build_document(result)
}
