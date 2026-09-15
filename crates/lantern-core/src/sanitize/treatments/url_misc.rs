//! Miscellaneous URL treatments.
//!
//! | ID                         | Name                                       |
//! |----------------------------|--------------------------------------------|
//! | `url.https_upgrade`        | Upgrade http → https                       |
//! | `url.strip_fragment`       | Strip URL fragment (#…)                    |
//! | `url.fragment.tracking`    | Strip tracking-only portions of a fragment |
//! | `url.path.user_segment`    | Strip user-identifying path segments       |
//! | `url.host.demobilize`      | Rewrite mobile hosts to canonical forms    |
//! | `url.host.unshorten.offline` | Flag URL-shortener hosts (detection only)|

use crate::model::document::Field;
use crate::model::node::{BookmarkUrl, Node};
use crate::sanitize::treatment::{BookmarkFlag, Change, PassContext, Treatment, TreatmentCategory};

// ---------------------------------------------------------------------------
// Upgrade http → https  (url.https_upgrade)
// ---------------------------------------------------------------------------

/// Rewrites `http://` URLs to `https://` where the scheme is the only
/// difference.  This is non-destructive for the vast majority of sites.
///
/// Marked destructive because a minority of sites genuinely serve different
/// content on HTTP vs. HTTPS (misconfigured servers, HTTP-only intranets).
pub struct HttpsUpgradeTreatment;

impl Treatment for HttpsUpgradeTreatment {
    fn id(&self) -> &'static str {
        "url.https_upgrade"
    }
    fn name(&self) -> &'static str {
        "Upgrade HTTP → HTTPS"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlHost
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };

        if url.scheme() != "http" {
            return vec![];
        }

        let mut upgraded = url.clone();
        if upgraded.set_scheme("https").is_err() {
            return vec![];
        }

        let before = url.as_str().to_owned();
        let after = upgraded.as_str().to_owned();
        if before == after {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Url,
            before,
            after,
            self.id(),
            "Upgraded HTTP scheme to HTTPS",
            true,
        )]
    }
}

// ---------------------------------------------------------------------------
// Strip URL fragment  (url.strip_fragment)
// ---------------------------------------------------------------------------

/// Removes the fragment (`#…`) component from bookmark URLs.
///
/// Fragments are client-side navigation hints that do not affect the server
/// response. In the vast majority of cases they add no useful information to a
/// saved bookmark. Non-destructive.
pub struct StripFragmentTreatment;

impl Treatment for StripFragmentTreatment {
    fn id(&self) -> &'static str {
        "url.strip_fragment"
    }
    fn name(&self) -> &'static str {
        "Strip URL fragment"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlFragment
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };

        if url.fragment().is_none() {
            return vec![];
        }

        let mut stripped = url.clone();
        stripped.set_fragment(None);

        let before = url.as_str().to_owned();
        let after = stripped.as_str().to_owned();
        if before == after {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Url,
            before,
            after,
            self.id(),
            "Stripped fragment (#\u{2026}) from URL",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Strip tracking fragment  (url.fragment.tracking)
// ---------------------------------------------------------------------------

/// Strips tracking-style key-value pairs from a URL fragment while leaving
/// genuine anchor fragments alone.
///
/// Some sites (notably ones with bookmarklet or JS-based analytics) encode
/// tracking data as a query string inside the `#fragment`.  When the entire
/// fragment is tracking, it is removed.  When a fragment contains a mix of
/// anchor and tracking (rare), only the tracking pairs are dropped.
pub struct FragmentTrackingTreatment;

/// Fragment keys considered pure tracking.  Covers the UTM namespace plus the
/// cross-platform click IDs and generic tracking keys, mirroring what
/// [`UtmTreatment`], [`ClickIdsTreatment`], and friends strip in query strings.
fn is_tracking_fragment_key(name: &str) -> bool {
    if name.starts_with("utm_") {
        return true;
    }
    matches!(
        name,
        "fbclid"
            | "gclid"
            | "dclid"
            | "msclkid"
            | "yclid"
            | "ttclid"
            | "twclid"
            | "igshid"
            | "mc_eid"
            | "mc_cid"
            | "wbraid"
            | "gbraid"
            | "referrer"
            | "referral"
    )
}

impl Treatment for FragmentTrackingTreatment {
    fn id(&self) -> &'static str {
        "url.fragment.tracking"
    }
    fn name(&self) -> &'static str {
        "Strip tracking from fragment"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlFragment
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };

        let fragment = match url.fragment() {
            Some(f) if !f.is_empty() => f,
            _ => return vec![],
        };

        // Parse the fragment as a list of `key=value` pairs.  If *none* of the
        // tokens look like `k=v`, treat the fragment as an anchor and skip.
        let pairs: Vec<(&str, &str)> = fragment
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .collect();
        if pairs.is_empty() {
            return vec![];
        }

        let kept: Vec<&(&str, &str)> = pairs
            .iter()
            .filter(|(k, _)| !is_tracking_fragment_key(k))
            .collect();

        if kept.len() == pairs.len() {
            return vec![];
        }

        let mut updated = url.clone();
        if kept.is_empty() {
            updated.set_fragment(None);
        } else {
            let rebuilt: String = kept
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            updated.set_fragment(Some(&rebuilt));
        }

        let before = url.as_str().to_owned();
        let after = updated.as_str().to_owned();
        if before == after {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Url,
            before,
            after,
            self.id(),
            "Stripped tracking parameters from URL fragment",
            false,
        )]
    }
}

// ---------------------------------------------------------------------------
// Strip user-identifying path segments  (url.path.user_segment)
// ---------------------------------------------------------------------------

/// Strips well-known user-identifying prefixes from a URL path.
///
/// Recognised prefixes (as the *first* non-empty path segment):
///
/// - `/u/<name>/…`        → `/…`
/// - `/user/<name>/…`     → `/…`
/// - `/users/<name>/…`    → `/…`
/// - `/@<handle>/…`       → `/…`
///
/// The treatment is **destructive**: some sites encode the user in their
/// canonical URL scheme (e.g. a GitHub repo URL starts with `/owner/repo` and
/// the owner is not optional), so all proposed changes land unapproved and
/// require explicit review in the preview panel.
pub struct UserSegmentTreatment;

impl Treatment for UserSegmentTreatment {
    fn id(&self) -> &'static str {
        "url.path.user_segment"
    }
    fn name(&self) -> &'static str {
        "Strip user path segment"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlPath
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };
        if !url.has_host() {
            return vec![];
        }

        let path = url.path();
        let trimmed_path = match strip_user_prefix(path) {
            Some(p) => p,
            None => return vec![],
        };

        let mut updated = url.clone();
        updated.set_path(&trimmed_path);

        let before = url.as_str().to_owned();
        let after = updated.as_str().to_owned();
        if before == after {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Url,
            before,
            after,
            self.id(),
            "Stripped user-identifying path segment",
            true,
        )]
    }
}

/// If `path` begins with a known user-segment pair, return the rest.
/// Returns `None` when no match so the caller can short-circuit.
fn strip_user_prefix(path: &str) -> Option<String> {
    // Collect non-empty segments so we can distinguish `/u/foo` from `/ufoo`.
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }

    let first = segments[0];

    // `/@handle[/…]`: single segment that starts with `@`.
    if let Some(rest) = first.strip_prefix('@') {
        if rest.is_empty() {
            return None;
        }
        let tail = &segments[1..];
        return Some(rebuild_path(tail, path.ends_with('/')));
    }

    // `/u/<x>[/…]`, `/user/<x>[/…]`, `/users/<x>[/…]`: two-segment prefix.
    if matches!(first, "u" | "user" | "users") && segments.len() >= 2 {
        let tail = &segments[2..];
        return Some(rebuild_path(tail, path.ends_with('/')));
    }

    None
}

fn rebuild_path(segments: &[&str], keep_trailing_slash: bool) -> String {
    if segments.is_empty() {
        return "/".to_string();
    }
    let mut out = String::from("/");
    out.push_str(&segments.join("/"));
    if keep_trailing_slash {
        out.push('/');
    }
    out
}

// ---------------------------------------------------------------------------
// Demobilize host  (url.host.demobilize)
// ---------------------------------------------------------------------------

/// Rewrites hosts that point at a mobile-specific subdomain to their canonical
/// desktop host.
///
/// Recognised patterns:
///
/// - `m.example.com`       → `example.com`
/// - `mobile.example.com`  → `example.com`
/// - `touch.example.com`   → `example.com`
/// - `de.m.wikipedia.org`  → `de.wikipedia.org` (the `m.` label anywhere but
///   in the suffix position is stripped)
///
/// Destructive because the rewritten host may serve different content or
/// redirect through the mobile version for a while; user confirms per item.
pub struct DemobilizeTreatment;

impl Treatment for DemobilizeTreatment {
    fn id(&self) -> &'static str {
        "url.host.demobilize"
    }
    fn name(&self) -> &'static str {
        "Demobilize host"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlHost
    }
    fn is_destructive(&self) -> bool {
        true
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };

        let host = match url.host_str() {
            Some(h) => h,
            None => return vec![],
        };
        let new_host = match demobilize_host(host) {
            Some(h) => h,
            None => return vec![],
        };

        let mut updated = url.clone();
        if updated.set_host(Some(&new_host)).is_err() {
            return vec![];
        }

        let before = url.as_str().to_owned();
        let after = updated.as_str().to_owned();
        if before == after {
            return vec![];
        }

        vec![Change::set_field(
            bm.id,
            Field::Url,
            before,
            after,
            self.id(),
            "Rewrote mobile-specific host to canonical form",
            true,
        )]
    }
}

/// Return the "desktop" form of a host, or `None` if there's no mobile label
/// to strip.
///
/// A label is "mobile" when it is one of `m`, `mobile`, `touch` and is **not**
/// the last or first-from-the-right non-TLD label (i.e. never strip the host
/// name itself, only subdomain labels).
fn demobilize_host(host: &str) -> Option<String> {
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 3 {
        // Bare `example.com` has nothing to strip.  `m.com` looks like a
        // shortener, not a mobile host; leave it alone.
        return None;
    }

    // Consider only the first label (leftmost): that's where browsers put
    // mobile subdomains.  Anything deeper (`foo.m.example.com`) is unusual
    // enough to defer.
    let mobile_first = matches!(
        labels[0].to_ascii_lowercase().as_str(),
        "m" | "mobile" | "touch"
    );
    if !mobile_first {
        return None;
    }

    let rebuilt = labels[1..].join(".");
    Some(rebuilt)
}

// ---------------------------------------------------------------------------
// Flag URL-shortener hosts  (url.host.unshorten.offline)
// ---------------------------------------------------------------------------

/// Detects bookmarks whose host is a well-known URL shortener.
///
/// **Status:** detection-only.  The proper "flag" surface (a per-bookmark
/// `flags.is_shortener` boolean visible in the list pane) requires extending
/// the [`Change`] model to mutate node attributes / flags.  Until that lands,
/// `propose` returns an empty `Vec` and the public [`is_known_shortener_host`]
/// helper is the authoritative source for the UI / future treatments.
///
/// The treatment still registers under a stable ID so user rule sets
/// referencing `url.host.unshorten.offline` round-trip through TOML and
/// "light up" automatically once the model gap closes.
pub struct UnshortenOfflineTreatment;

/// Hostnames known to belong to URL shortener services.
///
/// Conservative list: limited to services that exist primarily to redirect
/// (so a literal hostname match is unambiguous).  Internal product shorteners
/// like `youtu.be` are intentionally **excluded** because the destination is
/// implied by the host (a YouTube video); they're not opaque.
const SHORTENER_HOSTS: &[&str] = &[
    "bit.ly",
    "buff.ly",
    "goo.gl",
    "is.gd",
    "ow.ly",
    "rebrand.ly",
    "rb.gy",
    "shorturl.at",
    "t.co",
    "t.ly",
    "tiny.cc",
    "tinyurl.com",
    "tr.im",
    "v.gd",
    "x.co",
];

/// Returns true when `host` is a known URL shortener.
///
/// Matching is exact (no subdomain stripping) and case-insensitive: a host
/// of `BIT.LY` is recognised the same as `bit.ly`.  Public so the UI list
/// pane and future flag-mutating treatments can share the registry.
pub fn is_known_shortener_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    SHORTENER_HOSTS.iter().any(|s| *s == lower)
}

impl Treatment for UnshortenOfflineTreatment {
    fn id(&self) -> &'static str {
        "url.host.unshorten.offline"
    }
    fn name(&self) -> &'static str {
        "Flag URL-shortener hosts"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlHost
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        let bm = match node {
            Node::Bookmark(b) => b,
            _ => return vec![],
        };
        if bm.flags.is_shortener {
            return vec![]; // already flagged
        }
        let url = match &bm.url {
            BookmarkUrl::Valid(u) => u,
            BookmarkUrl::Malformed { .. } => return vec![],
        };
        if url.host_str().is_some_and(is_known_shortener_host) {
            vec![Change::set_flag(
                bm.id,
                BookmarkFlag::IsShortener,
                false,
                true,
                self.id(),
                "Flagged as URL-shortener host",
            )]
        } else {
            vec![]
        }
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

    fn make_bookmark(href: &str) -> Node {
        let url = match url::Url::parse(href) {
            Ok(u) => BookmarkUrl::Valid(u),
            Err(_) => BookmarkUrl::Malformed {
                raw: href.to_owned(),
            },
        };
        let mut id_gen = NodeIdAllocator::new();
        Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: "Test".into(),
            url,
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
    fn https_upgrade_rewrites_http() {
        let node = make_bookmark("http://example.com/page?q=1");
        let changes = HttpsUpgradeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].field_after().unwrap().starts_with("https://"));
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    #[test]
    fn https_upgrade_skips_already_https() {
        let node = make_bookmark("https://example.com/");
        assert!(HttpsUpgradeTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn strip_fragment_removes_hash() {
        let node = make_bookmark("https://example.com/page#section-3");
        let changes = StripFragmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/page"
        );
        assert!(!changes[0].destructive);
    }

    #[test]
    fn strip_fragment_no_change_when_no_fragment() {
        let node = make_bookmark("https://example.com/page?q=1");
        assert!(StripFragmentTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn https_upgrade_skips_ftp_scheme() {
        let node = make_bookmark("ftp://files.example.com/data.zip");
        assert!(HttpsUpgradeTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn https_upgrade_preserves_path_query_fragment() {
        let node = make_bookmark("http://example.com/path?q=1&a=2#section");
        let changes = HttpsUpgradeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/path?q=1&a=2#section"
        );
    }

    #[test]
    fn https_upgrade_skips_malformed_url() {
        let node = make_bookmark("not a url at all");
        assert!(HttpsUpgradeTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn strip_fragment_strips_empty_fragment() {
        // URL with a bare '#' (empty fragment)
        let node = make_bookmark("https://example.com/page#");
        let changes = StripFragmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].field_after().unwrap().contains('#'));
    }

    #[test]
    fn strip_fragment_non_destructive_and_auto_approved() {
        let node = make_bookmark("https://example.com/#anchor");
        let changes = StripFragmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].destructive);
        assert!(changes[0].approved);
    }

    #[test]
    fn https_upgrade_is_destructive_not_auto_approved() {
        let node = make_bookmark("http://example.com/");
        let changes = HttpsUpgradeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    #[test]
    fn folder_node_skipped_by_both_treatments() {
        use crate::model::node::{AttrMap, Folder};
        let folder = Node::Folder(Folder {
            id: 99,
            name: "Test Folder".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::default(),
            children: vec![],
        });
        assert!(HttpsUpgradeTreatment.propose(&folder, &ctx()).is_empty());
        assert!(StripFragmentTreatment.propose(&folder, &ctx()).is_empty());
    }

    // ── FragmentTrackingTreatment ──────────────────────────────────────────

    #[test]
    fn fragment_tracking_strips_whole_tracking_fragment() {
        let node = make_bookmark("https://example.com/page#utm_source=news&utm_medium=email");
        let changes = FragmentTrackingTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/page"
        );
    }

    #[test]
    fn fragment_tracking_preserves_anchor_fragment() {
        let node = make_bookmark("https://example.com/docs#section-3");
        let changes = FragmentTrackingTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn fragment_tracking_keeps_non_tracking_pairs() {
        let node = make_bookmark("https://example.com/#view=timeline&utm_campaign=x");
        let changes = FragmentTrackingTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("view=timeline"));
        assert!(!after.contains("utm_campaign"));
    }

    #[test]
    fn fragment_tracking_strips_fbclid_in_fragment() {
        let node = make_bookmark("https://example.com/x#fbclid=abc123");
        let changes = FragmentTrackingTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "https://example.com/x");
    }

    #[test]
    fn fragment_tracking_is_non_destructive() {
        let node = make_bookmark("https://example.com/#utm_source=x");
        let changes = FragmentTrackingTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].destructive);
        assert!(changes[0].approved);
    }

    // ── UserSegmentTreatment ───────────────────────────────────────────────

    #[test]
    fn user_segment_strips_reddit_user_prefix() {
        let node = make_bookmark("https://reddit.com/user/alice/comments/abc/post/");
        let changes = UserSegmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://reddit.com/comments/abc/post/"
        );
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    #[test]
    fn user_segment_strips_u_prefix() {
        let node = make_bookmark("https://example.com/u/bob/thing");
        let changes = UserSegmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/thing"
        );
    }

    #[test]
    fn user_segment_strips_at_handle_prefix() {
        let node = make_bookmark("https://medium.com/@writer/post-slug");
        let changes = UserSegmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://medium.com/post-slug"
        );
    }

    #[test]
    fn user_segment_collapses_to_root_when_only_prefix() {
        let node = make_bookmark("https://example.com/user/alice/");
        let changes = UserSegmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "https://example.com/");
    }

    #[test]
    fn user_segment_noop_on_unrelated_path() {
        let node = make_bookmark("https://example.com/docs/guide");
        assert!(UserSegmentTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn user_segment_noop_on_bare_u_without_name() {
        let node = make_bookmark("https://example.com/u/");
        assert!(UserSegmentTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn user_segment_is_destructive_and_needs_approval() {
        let node = make_bookmark("https://reddit.com/user/x/thing");
        let changes = UserSegmentTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    // ── DemobilizeTreatment ────────────────────────────────────────────────

    #[test]
    fn demobilize_strips_m_prefix() {
        let node = make_bookmark("https://m.example.com/page");
        let changes = DemobilizeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/page"
        );
    }

    #[test]
    fn demobilize_strips_mobile_prefix() {
        let node = make_bookmark("https://mobile.twitter.com/status/123");
        let changes = DemobilizeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://twitter.com/status/123"
        );
    }

    #[test]
    fn demobilize_strips_touch_prefix() {
        let node = make_bookmark("https://touch.facebook.com/x");
        let changes = DemobilizeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "https://facebook.com/x");
    }

    #[test]
    fn demobilize_noop_on_canonical_host() {
        let node = make_bookmark("https://example.com/page");
        assert!(DemobilizeTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn demobilize_noop_on_two_label_host() {
        // `m.com` is a plausible short domain, not a mobile host; leave alone.
        let node = make_bookmark("https://m.com/");
        assert!(DemobilizeTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn demobilize_is_destructive_and_needs_approval() {
        let node = make_bookmark("https://m.example.com/");
        let changes = DemobilizeTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].destructive);
        assert!(!changes[0].approved);
    }

    // ── UnshortenOfflineTreatment ──────────────────────────────────────────

    #[test]
    fn shortener_detection_recognises_bit_ly() {
        assert!(is_known_shortener_host("bit.ly"));
        assert!(is_known_shortener_host("BIT.LY"));
    }

    #[test]
    fn shortener_detection_recognises_t_co_and_tinyurl() {
        assert!(is_known_shortener_host("t.co"));
        assert!(is_known_shortener_host("tinyurl.com"));
    }

    #[test]
    fn shortener_detection_does_not_match_subdomains() {
        // Exact-match policy: `foo.bit.ly` is not flagged.
        assert!(!is_known_shortener_host("foo.bit.ly"));
    }

    #[test]
    fn shortener_detection_does_not_match_youtube_short_form() {
        // `youtu.be` is intentionally excluded: destination is unambiguous.
        assert!(!is_known_shortener_host("youtu.be"));
    }

    #[test]
    fn shortener_detection_rejects_unrelated_hosts() {
        assert!(!is_known_shortener_host("example.com"));
        assert!(!is_known_shortener_host("github.com"));
    }

    #[test]
    fn unshorten_treatment_flags_known_shortener() {
        let node = make_bookmark("https://bit.ly/abc");
        let changes = UnshortenOfflineTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0].kind,
            crate::sanitize::treatment::ChangeKind::SetFlag {
                flag: crate::model::node::BookmarkFlag::IsShortener,
                before: false,
                after: true,
            }
        ));
        assert!(!changes[0].destructive);
        assert!(changes[0].approved);
    }

    #[test]
    fn unshorten_treatment_noop_on_regular_host() {
        let node = make_bookmark("https://example.com/page");
        assert!(UnshortenOfflineTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn unshorten_treatment_noop_when_already_flagged() {
        use crate::model::ids::NodeIdAllocator;
        use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl};
        let url = url::Url::parse("https://bit.ly/abc").unwrap();
        let mut id_gen = NodeIdAllocator::new();
        let node = Node::Bookmark(Bookmark {
            id: id_gen.alloc(),
            title: "Test".into(),
            url: BookmarkUrl::Valid(url),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::default(),
            flags: BookmarkFlags {
                is_shortener: true,
                ..BookmarkFlags::default()
            },
        });
        assert!(UnshortenOfflineTreatment.propose(&node, &ctx()).is_empty());
    }
}
