//! Character-level diff for the change-preview UI.
//!
//! Produces two parallel span sequences (one for the *before* text and one
//! for the *after* text) where each span is tagged [`DiffTag::Equal`],
//! [`DiffTag::Removed`], or [`DiffTag::Added`].  The UI renders equal spans in
//! neutral grey, removed spans with a strike-through, and added spans with a
//! highlight, so the user can see exactly which characters a treatment
//! rewrote instead of scanning two whole strings side-by-side.
//!
//! # Algorithm
//!
//! Classic dynamic-programming longest-common-subsequence table over
//! `char`s, O(n·m) time and memory.  Bookmark titles and URLs are short
//! (typically well under 2 KB) so this cost is trivial in practice.  An input
//! length guard ([`MAX_DIFF_INPUT`]) still falls back to whole-string
//! removed/added spans for pathological inputs.
//!
//! # Choice of granularity
//!
//! Character diff (rather than word or line) is the right fit for the two
//! fields this feeds:
//!
//! - **URLs**: the interesting edits are tiny (one query-param removed, a
//!   scheme letter changed).  Word diff would treat the whole URL as one
//!   token.
//! - **Titles**: common edits are trailing whitespace, an email bracket, a
//!   stripped handle.  Character diff lights exactly the offending run.

use std::borrow::Cow;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Tag on one run of characters in a diff result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTag {
    /// Present in both inputs unchanged: render neutrally.
    Equal,
    /// Present in `before` only: render struck-through.
    Removed,
    /// Present in `after` only: render highlighted.
    Added,
}

/// One contiguous run of same-tag characters in a diff result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSpan {
    pub tag: DiffTag,
    pub text: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Maximum input length (in chars per side) before we fall back to whole-string
/// diff spans.  Beyond this the O(n·m) LCS table becomes wasteful.
pub const MAX_DIFF_INPUT: usize = 4096;

/// Compute character-level diff spans for the `before` and `after` strings.
///
/// Returns `(before_spans, after_spans)` where each vector reconstructs its
/// source string when the `text` fields are concatenated in order, and the
/// `Equal` spans appear in the same relative positions in both vectors.
///
/// # Edge cases
///
/// - If both inputs are equal, both returned vectors are
///   `[DiffSpan { tag: Equal, text: <whole input> }]` (or empty when the input
///   is empty).
/// - If either input exceeds [`MAX_DIFF_INPUT`] characters, the diff is
///   reported coarsely as a single `Removed` + single `Added` span.
pub fn char_diff(before: &str, after: &str) -> (Vec<DiffSpan>, Vec<DiffSpan>) {
    // Fast path: identical strings.
    if before == after {
        if before.is_empty() {
            return (Vec::new(), Vec::new());
        }
        return (
            vec![DiffSpan {
                tag: DiffTag::Equal,
                text: before.to_owned(),
            }],
            vec![DiffSpan {
                tag: DiffTag::Equal,
                text: after.to_owned(),
            }],
        );
    }

    // Guard against pathological sizes.
    let a: Vec<char> = before.chars().collect();
    let b: Vec<char> = after.chars().collect();
    if a.len() > MAX_DIFF_INPUT || b.len() > MAX_DIFF_INPUT {
        let mut before_spans = Vec::new();
        let mut after_spans = Vec::new();
        if !before.is_empty() {
            before_spans.push(DiffSpan {
                tag: DiffTag::Removed,
                text: before.to_owned(),
            });
        }
        if !after.is_empty() {
            after_spans.push(DiffSpan {
                tag: DiffTag::Added,
                text: after.to_owned(),
            });
        }
        return (before_spans, after_spans);
    }

    // Build the classic LCS table: lcs[i][j] = length of the LCS of a[..i] / b[..j].
    let n = a.len();
    let m = b.len();
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            lcs[i + 1][j + 1] = if a[i] == b[j] {
                lcs[i][j] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    // Walk the table in reverse to emit tagged character ops.
    // We collect into two separate per-side sequences first, then merge
    // adjacent same-tag runs into spans.
    let mut ops_before: Vec<(DiffTag, char)> = Vec::new();
    let mut ops_after: Vec<(DiffTag, char)> = Vec::new();
    let mut i = n;
    let mut j = m;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            ops_before.push((DiffTag::Equal, a[i - 1]));
            ops_after.push((DiffTag::Equal, b[j - 1]));
            i -= 1;
            j -= 1;
        } else if lcs[i - 1][j] >= lcs[i][j - 1] {
            ops_before.push((DiffTag::Removed, a[i - 1]));
            i -= 1;
        } else {
            ops_after.push((DiffTag::Added, b[j - 1]));
            j -= 1;
        }
    }
    while i > 0 {
        ops_before.push((DiffTag::Removed, a[i - 1]));
        i -= 1;
    }
    while j > 0 {
        ops_after.push((DiffTag::Added, b[j - 1]));
        j -= 1;
    }
    ops_before.reverse();
    ops_after.reverse();

    (coalesce(&ops_before), coalesce(&ops_after))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Merge adjacent same-tag characters into a single [`DiffSpan`].
fn coalesce(ops: &[(DiffTag, char)]) -> Vec<DiffSpan> {
    let mut out: Vec<DiffSpan> = Vec::new();
    for (tag, ch) in ops {
        match out.last_mut() {
            Some(last) if last.tag == *tag => last.text.push(*ch),
            _ => out.push(DiffSpan {
                tag: *tag,
                text: ch.to_string(),
            }),
        }
    }
    out
}

/// Convenience: reassemble one side's spans into the original string.
/// Handy in tests to assert round-trip fidelity.
#[allow(dead_code)]
pub(crate) fn reassemble(spans: &[DiffSpan], side: DiffTag) -> Cow<'_, str> {
    let mut buf = String::new();
    for s in spans {
        // For the "before" side, skip Added; for the "after" side, skip Removed.
        let skip = match side {
            DiffTag::Removed => matches!(s.tag, DiffTag::Added),
            DiffTag::Added => matches!(s.tag, DiffTag::Removed),
            DiffTag::Equal => false,
        };
        if !skip {
            buf.push_str(&s.text);
        }
    }
    Cow::Owned(buf)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(spans: &[DiffSpan]) -> Vec<DiffTag> {
        spans.iter().map(|s| s.tag).collect()
    }

    fn texts(spans: &[DiffSpan]) -> Vec<&str> {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn identical_strings_produce_single_equal_span() {
        let (b, a) = char_diff("hello", "hello");
        assert_eq!(b.len(), 1);
        assert_eq!(a.len(), 1);
        assert_eq!(b[0].tag, DiffTag::Equal);
        assert_eq!(a[0].tag, DiffTag::Equal);
        assert_eq!(b[0].text, "hello");
    }

    #[test]
    fn both_empty_strings_produce_empty_spans() {
        let (b, a) = char_diff("", "");
        assert!(b.is_empty());
        assert!(a.is_empty());
    }

    #[test]
    fn empty_before_produces_single_added_span() {
        let (b, a) = char_diff("", "new");
        assert!(b.is_empty());
        assert_eq!(tags(&a), vec![DiffTag::Added]);
        assert_eq!(texts(&a), vec!["new"]);
    }

    #[test]
    fn empty_after_produces_single_removed_span() {
        let (b, a) = char_diff("old", "");
        assert_eq!(tags(&b), vec![DiffTag::Removed]);
        assert_eq!(texts(&b), vec!["old"]);
        assert!(a.is_empty());
    }

    #[test]
    fn trailing_whitespace_trimmed() {
        // "hello   " → "hello"
        let (b, a) = char_diff("hello   ", "hello");
        assert_eq!(tags(&b), vec![DiffTag::Equal, DiffTag::Removed]);
        assert_eq!(texts(&b), vec!["hello", "   "]);
        assert_eq!(tags(&a), vec![DiffTag::Equal]);
        assert_eq!(texts(&a), vec!["hello"]);
    }

    #[test]
    fn query_param_removed_from_middle() {
        let before = "https://e.com/?a=1&utm_source=x&b=2";
        let after = "https://e.com/?a=1&b=2";
        let (b, a) = char_diff(before, after);

        // Reassemble should recover both originals.
        assert_eq!(reassemble(&b, DiffTag::Removed), before);
        assert_eq!(reassemble(&a, DiffTag::Added), after);

        // At least one Removed span and no Added on the `before` side.
        assert!(b.iter().any(|s| s.tag == DiffTag::Removed));
        assert!(!b.iter().any(|s| s.tag == DiffTag::Added));
        assert!(!a.iter().any(|s| s.tag == DiffTag::Removed));
    }

    #[test]
    fn scheme_upgrade_is_single_added_letter() {
        let (b, a) = char_diff("http://example.com/", "https://example.com/");
        // `before` has no Added spans.
        assert!(!b.iter().any(|s| s.tag == DiffTag::Added));
        // `after` has exactly one Added span containing "s".
        let added: Vec<_> = a.iter().filter(|s| s.tag == DiffTag::Added).collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].text, "s");
    }

    #[test]
    fn reassembly_round_trips_both_sides() {
        let cases = [
            ("abc", "abd"),
            ("lorem ipsum dolor", "lorem DOLOR"),
            ("a", "b"),
            ("", "full"),
            ("full", ""),
            ("\u{1F680} rocket title", "rocket \u{1F680}"),
        ];
        for (before, after) in cases {
            let (b, a) = char_diff(before, after);
            assert_eq!(reassemble(&b, DiffTag::Removed), before, "before mismatch");
            assert_eq!(reassemble(&a, DiffTag::Added), after, "after mismatch");
        }
    }

    #[test]
    fn pathological_size_falls_back_to_whole_string_spans() {
        let big = "a".repeat(MAX_DIFF_INPUT + 1);
        let (b, a) = char_diff(&big, "short");
        // Whole-string Removed on the before side, whole-string Added on the after side.
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].tag, DiffTag::Removed);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].tag, DiffTag::Added);
    }

    #[test]
    fn unicode_characters_treated_as_single_units() {
        // Diff at the grapheme/char level; rocket emoji must stay whole.
        let (b, a) = char_diff("\u{1F680}a", "\u{1F680}b");
        assert_eq!(reassemble(&b, DiffTag::Removed), "\u{1F680}a");
        assert_eq!(reassemble(&a, DiffTag::Added), "\u{1F680}b");
        // The rocket itself should appear in an Equal span on both sides.
        assert!(b
            .iter()
            .any(|s| s.tag == DiffTag::Equal && s.text.contains('\u{1F680}')));
        assert!(a
            .iter()
            .any(|s| s.tag == DiffTag::Equal && s.text.contains('\u{1F680}')));
    }
}
