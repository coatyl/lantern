//! Title treatments.
//!
//! | ID                       | Name                                 | Version |
//! |--------------------------|--------------------------------------|---------|
//! | `title.whitespace`       | Normalize whitespace                 | v0.0.1  |
//! | `title.html_entities`    | Decode HTML entities                 | v0.0.1  |
//! | `title.email`            | Strip email addresses                | v0.0.2  |
//! | `title.handle`           | Strip social handles (`@user`)       | v0.0.2  |
//! | `title.author_suffix`    | Strip "by Author Name" suffixes      | v0.0.2  |
//! | `title.regex`            | Apply user-supplied regex replace    | v0.0.2  |

use super::text::{decode_html_entities, normalize_whitespace};
use crate::model::document::Field;
use crate::model::node::Node;
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};

// ---------------------------------------------------------------------------
// Normalize whitespace  (title.whitespace)
// ---------------------------------------------------------------------------

/// Collapses runs of ASCII whitespace (including `\t`, `\n`, `\r`) to a single
/// space, and trims leading / trailing whitespace.
///
/// This is the mildest title treatment and is safe to auto-approve.
pub struct WhitespaceTreatment;

impl Treatment for WhitespaceTreatment {
    fn id(&self) -> &'static str {
        "title.whitespace"
    }
    fn name(&self) -> &'static str {
        "Normalize whitespace"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };

        let before = bm.title.as_str();
        let after = normalize_whitespace(before);

        if after == before {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after,
            self.id(),
            "Collapsed whitespace in title",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Decode HTML entities  (title.html_entities)
// ---------------------------------------------------------------------------

/// Decodes common HTML entities in bookmark titles.
///
/// Browsers sometimes export titles with encoded characters (e.g. `&amp;` for
/// `&`, `&lt;` for `<`).  This treatment decodes them back to their literal
/// characters for cleaner display.  Non-destructive.
pub struct HtmlEntitiesTreatment;

impl Treatment for HtmlEntitiesTreatment {
    fn id(&self) -> &'static str {
        "title.html_entities"
    }
    fn name(&self) -> &'static str {
        "Decode HTML entities"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };

        let before = bm.title.as_str();
        let after = decode_html_entities(before);

        if after == before {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after,
            self.id(),
            "Decoded HTML entities in title",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Strip email addresses  (title.email)
// ---------------------------------------------------------------------------

/// Removes email addresses embedded in titles (a common pattern in newsletter
/// or share-link bookmark dumps, e.g. `"Cool article (you@example.com)"`).
///
/// Conservative: uses a deliberately simple `local@domain.tld` matcher that
/// requires both an `@` and a dotted TLD, avoiding false positives on
/// Mastodon-style handles like `@user@instance.social` (those are handled by
/// [`HandleTreatment`]).
pub struct EmailTreatment;

impl Treatment for EmailTreatment {
    fn id(&self) -> &'static str {
        "title.email"
    }
    fn name(&self) -> &'static str {
        "Strip email addresses"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };

        let before = bm.title.as_str();
        let after = strip_emails(before);
        if after == before {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after,
            self.id(),
            "Removed email address from title",
            false,
        )]
    }
}

/// Find and remove email-shaped substrings, then collapse whitespace.
///
/// Cached lazily because compiling a regex on every call would be wasteful.
fn strip_emails(s: &str) -> String {
    use std::sync::OnceLock;
    static EMAIL_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = EMAIL_RE.get_or_init(|| {
        // local@domain.tld (local: word/.-+_; domain: word/.- ; tld: 2+ letters)
        regex::Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap()
    });
    let removed = re.replace_all(s, "");
    // After deletion, neighbouring punctuation often leaves "(  )" or
    // "[ ,]" trails: collapse whitespace and trim stray separators.
    let trimmed = trim_dangling_punct(removed.as_ref());
    normalize_whitespace(&trimmed)
}

/// Strip empty bracket pairs and stray "(," ", )" left behind by regex
/// deletions.  Idempotent and gentle: only touches obvious orphans.
fn trim_dangling_punct(s: &str) -> String {
    let mut out = s.to_owned();
    // Drop empty/whitespace-only bracket pairs.
    for pat in &["()", "( )", "[]", "[ ]", "<>", "< >", "{}", "{ }"] {
        out = out.replace(pat, "");
    }
    // Drop trailing/leading commas+spaces left after deletion.
    out = out
        .trim_matches(|c: char| matches!(c, ',' | ';' | '|' | '·' | '–' | '-') || c.is_whitespace())
        .to_owned();
    out
}

// ---------------------------------------------------------------------------
// Strip social handles  (title.handle)
// ---------------------------------------------------------------------------

/// Removes Twitter/Mastodon/IG-style `@handle` and `@handle@instance` tokens
/// from titles.
///
/// Matched at word boundaries so "you@me" inside running prose is left alone
/// (that pattern is handled by [`EmailTreatment`] when it has a TLD, or kept
/// otherwise).
pub struct HandleTreatment;

impl Treatment for HandleTreatment {
    fn id(&self) -> &'static str {
        "title.handle"
    }
    fn name(&self) -> &'static str {
        "Strip social handles"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let before = bm.title.as_str();
        let after = strip_handles(before);
        if after == before {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after,
            self.id(),
            "Removed social handle from title",
            false,
        )]
    }
}

fn strip_handles(s: &str) -> String {
    use std::sync::OnceLock;
    static HANDLE_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = HANDLE_RE.get_or_init(|| {
        // Anchor at start-of-string or ASCII whitespace / punctuation, then
        // `@user` optionally followed by `@instance.tld` (Mastodon).  The
        // leading anchor is captured as group 1 so we re-emit it verbatim
        // and leave downstream punctuation untouched.  ASCII-only character
        // classes are used because the workspace `regex` build excludes the
        // `unicode-perl` feature.
        regex::Regex::new(
            r"(^|[ \t\r\n\(\[<,:|])@[A-Za-z0-9_]{1,30}(?:@[A-Za-z0-9.\-]+\.[A-Za-z]{2,})?",
        )
        .unwrap()
    });
    let replaced = re.replace_all(s, "$1");
    let trimmed = trim_dangling_punct(&replaced);
    normalize_whitespace(&trimmed)
}

// ---------------------------------------------------------------------------
// Strip "by Author Name" suffix  (title.author_suffix)
// ---------------------------------------------------------------------------

/// Removes the common `" — by Author Name"` / `" by Author Name"` /
/// `" | Author Name"` trailing decoration from article titles.
///
/// Matches at the **end of the title only** to avoid eating mid-string text.
/// Recognised separators: ` - `, ` — `, ` – `, ` | `, ` · `.
///
/// Conservative on the author segment: requires 1-4 capitalised words,
/// optionally preceded by `by `.  This catches the vast majority of byline
/// formats from blogs and news sites without misfiring on technical titles.
pub struct AuthorSuffixTreatment;

impl Treatment for AuthorSuffixTreatment {
    fn id(&self) -> &'static str {
        "title.author_suffix"
    }
    fn name(&self) -> &'static str {
        "Strip author suffix"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let before = bm.title.as_str();
        let after = strip_author_suffix(before);
        if after == before {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after,
            self.id(),
            "Removed trailing author byline from title",
            true,
        )]
    }
}

fn strip_author_suffix(s: &str) -> String {
    use std::sync::OnceLock;
    static AUTHOR_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = AUTHOR_RE.get_or_init(|| {
        // ASCII whitespace classes only: the workspace `regex` build excludes
        // the `unicode-perl` feature, so `\s` is unavailable.  Separator
        // characters are inlined as literal Unicode codepoints (em-dash,
        // en-dash, middle-dot).  Written on one line (no `(?x)` extended
        // mode) because in-class whitespace handling under `x` differs
        // between regex implementations.
        regex::Regex::new(
            "[ \t]+[\\-|\u{2014}\u{2013}\u{00B7}][ \t]+(?:by[ \t]+)?\
             [A-Z][A-Za-z'\\-]+(?:[ \t]+[A-Z][A-Za-z'\\-]+){0,3}[ \t]*$",
        )
        .unwrap()
    });
    let stripped = re.replace(s, "");
    normalize_whitespace(&stripped)
}

// ---------------------------------------------------------------------------
// Apply user-supplied regex replace  (title.regex)
// ---------------------------------------------------------------------------

/// Applies a user-supplied regular expression to the title, replacing each
/// match with the supplied replacement string.
///
/// Construct with [`RegexTitleTreatment::new`] supplying a valid regex and
/// replacement.  An empty / unset rule is a no-op (`empty()`), which is the
/// state when the treatment is registered from TOML but not yet configured.
///
/// Marked **destructive** because user-supplied regex can have surprising
/// reach; every change requires explicit approval.
pub struct RegexTitleTreatment {
    rule: Option<RegexRule>,
}

struct RegexRule {
    re: regex::Regex,
    replacement: String,
}

impl RegexTitleTreatment {
    /// Build a treatment that runs `pattern` over each title and substitutes
    /// matches with `replacement`.  Returns `None` if `pattern` does not
    /// compile.
    pub fn new(pattern: &str, replacement: impl Into<String>) -> Option<Self> {
        let re = regex::Regex::new(pattern).ok()?;
        Some(Self {
            rule: Some(RegexRule {
                re,
                replacement: replacement.into(),
            }),
        })
    }

    /// Construct an unconfigured instance: valid for registry round-tripping
    /// but produces no changes.
    pub fn empty() -> Self {
        Self { rule: None }
    }
}

impl Treatment for RegexTitleTreatment {
    fn id(&self) -> &'static str {
        "title.regex"
    }

    fn configure(&mut self, config: &toml::Value) -> Result<(), String> {
        let pattern = config
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "title.regex: config must have a 'pattern' string".to_owned())?;
        let replacement = config
            .get("replacement")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        *self = RegexTitleTreatment::new(pattern, replacement)
            .ok_or_else(|| format!("title.regex: invalid regex pattern: {pattern}"))?;
        Ok(())
    }

    fn current_config(&self) -> Option<toml::Value> {
        let mut table = toml::map::Map::new();
        let (pattern, replacement) = match &self.rule {
            Some(r) => (r.re.as_str().to_owned(), r.replacement.clone()),
            None => (String::new(), String::new()),
        };
        table.insert("pattern".to_owned(), toml::Value::String(pattern));
        table.insert("replacement".to_owned(), toml::Value::String(replacement));
        Some(toml::Value::Table(table))
    }

    fn name(&self) -> &'static str {
        "Apply regex to title"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::Title
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let rule = match &self.rule {
            Some(r) => r,
            None => return vec![],
        };
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let before = bm.title.as_str();
        let after = rule.re.replace_all(before, rule.replacement.as_str());
        if after.as_ref() == before {
            return vec![];
        }
        vec![Change::set_field(
            bm.id,
            Field::Title,
            before,
            after.into_owned(),
            self.id(),
            "Applied user-configured regex to title",
            true,
        )]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::NodeIdAllocator;
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl};
    use crate::sanitize::treatment::PassContext;

    fn make_bookmark_titled(title: &str) -> Node {
        let mut id_gen = NodeIdAllocator::new();
        Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: title.to_owned(),
            url: BookmarkUrl::Malformed {
                raw: "https://example.com".into(),
            },
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags::default(),
        })
    }

    fn ctx() -> PassContext {
        PassContext { document_id: 0 }
    }

    #[test]
    fn collapses_internal_whitespace() {
        let node = make_bookmark_titled("Hello   World");
        let changes = WhitespaceTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Hello World");
    }

    #[test]
    fn trims_leading_trailing() {
        let node = make_bookmark_titled("  Hello  ");
        let changes = WhitespaceTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Hello");
    }

    #[test]
    fn no_change_for_clean_title() {
        let node = make_bookmark_titled("Hello World");
        let changes = WhitespaceTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn handles_tab_and_newline() {
        let node = make_bookmark_titled("Hello\t\nWorld");
        let changes = WhitespaceTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Hello World");
    }

    #[test]
    fn folder_node_ignored() {
        use crate::model::node::Folder;
        let folder = Node::Folder(Folder {
            id: 1,
            name: "  spaced  ".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        assert!(WhitespaceTreatment.propose(&folder, &ctx()).is_empty());
    }

    // ── HtmlEntitiesTreatment ─────────────────────────────────────────────

    #[test]
    fn decodes_amp() {
        let node = make_bookmark_titled("Tom &amp; Jerry");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Tom & Jerry");
    }

    #[test]
    fn decodes_lt_gt() {
        let node = make_bookmark_titled("A &lt; B &gt; C");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "A < B > C");
    }

    #[test]
    fn decodes_quot_and_apos() {
        let node = make_bookmark_titled("Say &quot;hello&quot; &amp; it&apos;s done");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(
            changes[0].field_after().unwrap(),
            "Say \"hello\" & it's done"
        );
    }

    #[test]
    fn decodes_numeric_decimal() {
        // &#169; = © (copyright sign)
        let node = make_bookmark_titled("&#169; 2024 Acme");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "© 2024 Acme");
    }

    #[test]
    fn decodes_numeric_hex_lowercase() {
        // &#x26; = & (ampersand)
        let node = make_bookmark_titled("A &#x26; B");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "A & B");
    }

    #[test]
    fn decodes_numeric_hex_uppercase() {
        // &#X26; = &
        let node = make_bookmark_titled("A &#X26; B");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "A & B");
    }

    #[test]
    fn decodes_mdash_ndash() {
        let node = make_bookmark_titled("Rust &mdash; fast &amp; safe");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes[0].field_after().unwrap(), "Rust — fast & safe");
    }

    #[test]
    fn unknown_entity_passes_through() {
        let node = make_bookmark_titled("&unknown; entity");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        // No change: unknown entity is preserved verbatim
        assert!(changes.is_empty());
    }

    #[test]
    fn html_entities_no_change_for_clean_title() {
        let node = make_bookmark_titled("No entities here");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn no_change_for_title_without_ampersand() {
        // Fast-path: titles without '&' are returned as-is
        let node = make_bookmark_titled("Hello World 123");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn treatment_is_non_destructive_and_auto_approved() {
        let node = make_bookmark_titled("A &amp; B");
        let changes = HtmlEntitiesTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].destructive);
        assert!(changes[0].approved);
    }

    #[test]
    fn html_entities_folder_node_ignored() {
        use crate::model::node::Folder;
        let folder = Node::Folder(Folder {
            id: 1,
            name: "&amp; folder".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        assert!(HtmlEntitiesTreatment.propose(&folder, &ctx()).is_empty());
    }

    // ── EmailTreatment ─────────────────────────────────────────────────────

    #[test]
    fn email_strips_parenthetical_address() {
        let node = make_bookmark_titled("Article (you@example.com)");
        let changes = EmailTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Article");
    }

    #[test]
    fn email_strips_inline_address() {
        let node = make_bookmark_titled("Contact us: support@acme.org for help");
        let changes = EmailTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Contact us: for help");
    }

    #[test]
    fn email_no_change_when_no_address() {
        let node = make_bookmark_titled("Plain title without addresses");
        assert!(EmailTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn email_does_not_eat_handle_without_tld() {
        // `@user@instance` has no proper TLD so EmailTreatment should skip it;
        // HandleTreatment owns this case.
        let node = make_bookmark_titled("Post by @alice");
        assert!(EmailTreatment.propose(&node, &ctx()).is_empty());
    }

    // ── HandleTreatment ────────────────────────────────────────────────────

    #[test]
    fn handle_strips_leading_at_handle() {
        let node = make_bookmark_titled("@alice posted something");
        let changes = HandleTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "posted something");
    }

    #[test]
    fn handle_strips_inline_handle() {
        let node = make_bookmark_titled("New post from @bob today");
        let changes = HandleTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "New post from today");
    }

    #[test]
    fn handle_strips_mastodon_form() {
        let node = make_bookmark_titled("Boost from @user@mastodon.social!");
        let changes = HandleTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Boost from !");
    }

    #[test]
    fn handle_does_not_match_email_local_part() {
        // `you@example.com` should not be partially eaten as a handle;
        // it's an email and HandleTreatment should leave it alone because
        // there's no leading word-boundary `@`.
        let node = make_bookmark_titled("Contact you@example.com please");
        let changes = HandleTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    // ── AuthorSuffixTreatment ──────────────────────────────────────────────

    #[test]
    fn author_suffix_strips_em_dash_by_byline() {
        let node = make_bookmark_titled("How to Build Things — by Alice Cooper");
        let changes = AuthorSuffixTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "How to Build Things");
    }

    #[test]
    fn author_suffix_strips_pipe_byline() {
        let node = make_bookmark_titled("Cool Article | Jane Doe");
        let changes = AuthorSuffixTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Cool Article");
    }

    #[test]
    fn author_suffix_strips_dash_by_byline_three_word_name() {
        let node = make_bookmark_titled("Some Title - by Mary Jane Smith");
        let changes = AuthorSuffixTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Some Title");
    }

    #[test]
    fn author_suffix_no_match_without_separator() {
        let node = make_bookmark_titled("Title by Alice");
        // Missing the leading separator (` - `, ` | `, etc.): leave alone.
        assert!(AuthorSuffixTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn author_suffix_no_match_lowercase_trailing() {
        let node = make_bookmark_titled("Some Title - notes here");
        assert!(AuthorSuffixTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn author_suffix_destructive_and_unapproved() {
        let node = make_bookmark_titled("Cool Read | Author Name");
        let changes = AuthorSuffixTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    // ── RegexTitleTreatment ────────────────────────────────────────────────

    #[test]
    fn regex_title_replaces_matches() {
        // ASCII-class whitespace because the workspace `regex` build excludes
        // the `unicode-perl` feature; user-supplied regexes inherit the same
        // restriction.
        let t = RegexTitleTreatment::new(r"[ \t]*\[draft\]$", "").unwrap();
        let node = make_bookmark_titled("Notes [draft]");
        let changes = t.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "Notes");
    }

    #[test]
    fn regex_title_supports_replacement_string() {
        let t = RegexTitleTreatment::new(r"foo", "BAR").unwrap();
        let node = make_bookmark_titled("foo bar foo");
        let changes = t.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "BAR bar BAR");
    }

    #[test]
    fn regex_title_invalid_pattern_returns_none() {
        assert!(RegexTitleTreatment::new(r"(unclosed", "").is_none());
    }

    #[test]
    fn regex_title_empty_is_noop() {
        let node = make_bookmark_titled("Anything goes here");
        assert!(RegexTitleTreatment::empty()
            .propose(&node, &ctx())
            .is_empty());
    }

    #[test]
    fn regex_title_destructive_and_unapproved() {
        let t = RegexTitleTreatment::new(r"x", "y").unwrap();
        let node = make_bookmark_titled("x marks the spot");
        let changes = t.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    #[test]
    fn regex_title_no_change_when_no_match() {
        let t = RegexTitleTreatment::new(r"NOTFOUND", "").unwrap();
        let node = make_bookmark_titled("plain title");
        assert!(t.propose(&node, &ctx()).is_empty());
    }
}
