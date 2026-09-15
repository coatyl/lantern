//! Thin wrapper around html5ever's tolerant HTML5 parser.
//!
//! Responsibilities:
//! - Detect and strip a UTF-8 BOM if present (recording `has_bom` for the
//!   round-trip header).
//! - Detect the dominant line-ending convention in the raw bytes.
//! - Run html5ever, which is infallible: it tolerates any byte sequence and
//!   never returns an error.

use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::RcDom;

use crate::model::document::LineEnding;

/// Everything the `build` step needs from the tokenizer.
pub struct TokenizeResult {
    pub dom: RcDom,
    pub has_bom: bool,
    pub line_ending: LineEnding,
}

/// Parse `bytes` into an RcDom, stripping any leading BOM first.
pub fn tokenize(bytes: &[u8]) -> TokenizeResult {
    let has_bom = bytes.starts_with(b"\xef\xbb\xbf");
    let content: &[u8] = if has_bom { &bytes[3..] } else { bytes };

    let line_ending = detect_line_ending(content);

    // `&[u8]` implements `std::io::Read`; html5ever's `read_from` advances the
    // slice reference as it reads, leaving us with an infallible `io::Result`.
    let mut input: &[u8] = content;
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut input)
        .expect("html5ever read from &[u8] is infallible");

    TokenizeResult {
        dom,
        has_bom,
        line_ending,
    }
}

fn detect_line_ending(bytes: &[u8]) -> LineEnding {
    if bytes.contains(&b'\r') {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}
