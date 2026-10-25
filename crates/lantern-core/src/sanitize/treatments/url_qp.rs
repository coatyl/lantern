//! URL query-parameter treatments.
//!
//! | ID                     | Name                              | Version |
//! |------------------------|-----------------------------------|---------|
//! | `url.qp.utm`           | Strip UTM params                  | v0.0.1  |
//! | `url.qp.click_ids`     | Strip click IDs                   | v0.0.1  |
//! | `url.qp.session`       | Strip generic session params      | v0.0.1  |
//! | `url.qp.affiliate`     | Strip affiliate-network params    | v0.0.2  |
//! | `url.qp.search_tokens` | Strip search-engine noise params  | v0.0.2  |
//! | `url.qp.custom`        | Strip user-supplied params        | v0.0.2  |
//!
//! All of these are non-destructive (they only remove tracking and session
//! noise, leaving the underlying resource unchanged) and are therefore
//! auto-approved in the preview panel.

use std::borrow::Cow;

use url::Url;

use crate::model::document::Field;
use crate::model::node::{BookmarkUrl, Node};
use crate::sanitize::treatment::{Change, PassContext, Treatment, TreatmentCategory};

// ---------------------------------------------------------------------------
// Strip UTM params  (url.qp.utm)
// ---------------------------------------------------------------------------

/// Removes every query parameter whose name starts with `utm_`.
///
/// Covers `utm_source`, `utm_medium`, `utm_campaign`, `utm_term`,
/// `utm_content`, `utm_id`, and any future `utm_*` additions.
pub struct UtmTreatment;

impl Treatment for UtmTreatment {
    fn id(&self) -> &'static str {
        "url.qp.utm"
    }
    fn name(&self) -> &'static str {
        "Strip UTM params"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        strip_matching(
            node,
            self.id(),
            Cow::Borrowed("Stripped UTM tracking parameters"),
            |name| name.starts_with("utm_"),
        )
    }
}

// ---------------------------------------------------------------------------
// Strip click IDs  (url.qp.click_ids)
// ---------------------------------------------------------------------------

/// Removes well-known cross-platform click-ID parameters injected by ad
/// networks and social media referral links.
pub struct ClickIdsTreatment;

/// Whether `name` is a tracking parameter (`utm_*` or a known click id):
/// removing it never changes which page a URL points at.  Used by the
/// near-duplicate key.
pub(crate) fn is_tracking_param(name: &str) -> bool {
    name.starts_with("utm_") || CLICK_ID_PARAMS.contains(&name)
}

/// Parameters stripped by [`ClickIdsTreatment`].
const CLICK_ID_PARAMS: &[&str] = &[
    "fbclid", "gclid", "dclid", "msclkid", "yclid", "ttclid", "twclid", "igshid", "mc_eid",
    "mc_cid", "wbraid", "gbraid",
];

impl Treatment for ClickIdsTreatment {
    fn id(&self) -> &'static str {
        "url.qp.click_ids"
    }
    fn name(&self) -> &'static str {
        "Strip click IDs"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        strip_matching(
            node,
            self.id(),
            Cow::Borrowed("Stripped ad-network click-ID parameter"),
            |name| CLICK_ID_PARAMS.contains(&name),
        )
    }
}

// ---------------------------------------------------------------------------
// Strip session params  (url.qp.session)
// ---------------------------------------------------------------------------

/// Removes generic server-side session identifier parameters.
pub struct SessionTreatment;

/// Parameters stripped by [`SessionTreatment`].
const SESSION_PARAMS: &[&str] = &["sid", "session", "sessionid", "phpsessid", "jsessionid"];

impl Treatment for SessionTreatment {
    fn id(&self) -> &'static str {
        "url.qp.session"
    }
    fn name(&self) -> &'static str {
        "Strip generic session params"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        strip_matching(
            node,
            self.id(),
            Cow::Borrowed("Stripped session identifier parameter"),
            |name| {
                // Exact matches for the known list.
                SESSION_PARAMS.contains(&name)
            // asp-style: aspsessionid followed by any suffix
            || name.starts_with("aspsessionid")
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Strip affiliate-network params  (url.qp.affiliate)
// ---------------------------------------------------------------------------

/// Removes affiliate-tracking query parameters from bookmark URLs.
///
/// Host-aware: Amazon and eBay have their own well-known affiliate parameter
/// namespaces; everything else uses the generic list. Parameters are kept as
/// simple string matches (no regex) for predictability.
pub struct AffiliateTreatment;

/// Amazon affiliate/tracking params. Applies to any host ending in `amazon.`
/// plus a TLD (e.g. `amazon.com`, `amazon.co.uk`, `smile.amazon.de`).
const AMAZON_AFFILIATE_PARAMS: &[&str] = &[
    "tag",
    "linkCode",
    "camp",
    "creative",
    "creativeASIN",
    "ascsubtag",
    "asc_campaign",
    "asc_refurl",
    "asc_source",
    "smid",
    "psc",
    "ref_",
    "pd_rd_r",
    "pd_rd_w",
    "pd_rd_wg",
    "pf_rd_i",
    "pf_rd_m",
    "pf_rd_p",
    "pf_rd_r",
    "pf_rd_s",
    "pf_rd_t",
    "_encoding",
];

/// eBay affiliate/tracking params.
const EBAY_AFFILIATE_PARAMS: &[&str] = &[
    "mkcid",
    "mkrid",
    "campid",
    "toolid",
    "customid",
    "mkevt",
    "_trksid",
    "_trkparms",
];

/// Generic affiliate parameters used by smaller networks (ShareASale, CJ,
/// Impact, Skimlinks, etc.) and independent referral programs.
const GENERIC_AFFILIATE_PARAMS: &[&str] = &[
    "affid",
    "affiliate",
    "affiliate_id",
    "affiliateid",
    "aff",
    "aff_id",
    "partner",
    "partner_id",
    "partnerid",
    "referrer",
    "referral",
    "refsrc",
    "ref_src",
    "irclickid",
    "irgwc",
    "impactradius",
    "skimlinks",
    "skim",
    "sscid",
];

impl Treatment for AffiliateTreatment {
    fn id(&self) -> &'static str {
        "url.qp.affiliate"
    }
    fn name(&self) -> &'static str {
        "Strip affiliate params"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        strip_matching_with_url(
            node,
            self.id(),
            Cow::Borrowed("Stripped affiliate-network tracking parameter"),
            |url, name| {
                let host = url.host_str().unwrap_or("");
                if host_is_amazon(host) && AMAZON_AFFILIATE_PARAMS.contains(&name) {
                    return true;
                }
                if host_is_ebay(host) && EBAY_AFFILIATE_PARAMS.contains(&name) {
                    return true;
                }
                GENERIC_AFFILIATE_PARAMS.contains(&name)
            },
        )
    }
}

fn host_is_amazon(host: &str) -> bool {
    // Match `amazon.com`, `amazon.co.uk`, `www.amazon.de`, `smile.amazon.fr`,
    // etc. We look for a label equal to "amazon" in the host.
    host.split('.')
        .any(|label| label.eq_ignore_ascii_case("amazon"))
}

fn host_is_ebay(host: &str) -> bool {
    host.split('.')
        .any(|label| label.eq_ignore_ascii_case("ebay"))
}

// ---------------------------------------------------------------------------
// Strip search-engine noise params  (url.qp.search_tokens)
// ---------------------------------------------------------------------------

/// Removes result-tracking parameters that major search engines append to
/// outbound URLs.
///
/// Only the *noise* tokens are stripped; the actual query parameter (`q` on
/// Google/Bing/DuckDuckGo) is always preserved so the bookmark still describes
/// the original search.
pub struct SearchTokensTreatment;

/// Google: ranking / session / locale / diagnostic tokens appended to result
/// pages. `q` (query) is intentionally *not* in this list.
const GOOGLE_SEARCH_PARAMS: &[&str] = &[
    "ved", "ei", "oq", "gs_lcp", "gs_rn", "sourceid", "rlz", "aqs", "biw", "bih", "bvm", "source",
    "sa", "sca_esv", "sxsrf", "usg", "uact", "cd", "cad", "iflsig", "hl",
];

/// Bing: result-form / click-tracking / session tokens.
const BING_SEARCH_PARAMS: &[&str] = &[
    "qs", "form", "sp", "pq", "sc", "sk", "cvid", "ghsh", "ghacc", "ghpl", "first", "FORM",
];

/// DuckDuckGo: rewrite/advert/session tokens (but not `q`).
const DUCKDUCKGO_SEARCH_PARAMS: &[&str] = &[
    "atb", "t", "ia", "iax", "iar", "iaxm", "ko", "kl", "kd", "kp",
];

impl Treatment for SearchTokensTreatment {
    fn id(&self) -> &'static str {
        "url.qp.search_tokens"
    }
    fn name(&self) -> &'static str {
        "Strip search-engine noise"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        strip_matching_with_url(
            node,
            self.id(),
            Cow::Borrowed("Stripped search-engine result-tracking parameter"),
            |url, name| {
                let host = url.host_str().unwrap_or("");
                if host_is_google(host) && GOOGLE_SEARCH_PARAMS.contains(&name) {
                    return true;
                }
                if host_is_bing(host) && BING_SEARCH_PARAMS.contains(&name) {
                    return true;
                }
                if host_is_duckduckgo(host) && DUCKDUCKGO_SEARCH_PARAMS.contains(&name) {
                    return true;
                }
                false
            },
        )
    }
}

fn host_is_google(host: &str) -> bool {
    // Match `google.com`, `google.co.uk`, `www.google.fr`, etc.  We look for
    // the literal label `google` anywhere in the host.
    host.split('.').any(|l| l.eq_ignore_ascii_case("google"))
}

fn host_is_bing(host: &str) -> bool {
    host.split('.').any(|l| l.eq_ignore_ascii_case("bing"))
}

fn host_is_duckduckgo(host: &str) -> bool {
    host.split('.')
        .any(|l| l.eq_ignore_ascii_case("duckduckgo"))
}

// ---------------------------------------------------------------------------
// Strip user-supplied params  (url.qp.custom)
// ---------------------------------------------------------------------------

/// Removes a user-supplied list of query parameter names.
///
/// Matching is exact and case-sensitive by default.  The empty parameter list
/// is a valid no-op, useful when the treatment is registered as a placeholder
/// and the user has not yet configured it.
///
/// Host scoping (apply only on matching hosts) is a planned follow-up; the
/// initial implementation applies globally.
pub struct CustomQpTreatment {
    names: Vec<String>,
    rationale: Cow<'static, str>,
}

impl CustomQpTreatment {
    /// Create a treatment that strips the listed parameter names on every URL.
    pub fn new(names: Vec<String>) -> Self {
        Self {
            names,
            rationale: Cow::Borrowed("Stripped user-configured query parameter"),
        }
    }

    /// Empty list: registers the treatment with no active rules.  Safe to use
    /// when round-tripping a rule set through TOML before the user has filled
    /// in the parameter list.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

impl Treatment for CustomQpTreatment {
    fn id(&self) -> &'static str {
        "url.qp.custom"
    }

    fn configure(&mut self, config: &toml::Value) -> Result<(), String> {
        let arr = config
            .get("params")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                "url.qp.custom: config must have a 'params' array of strings".to_owned()
            })?;
        self.names = arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_owned())
            .collect();
        self.rationale = if self.names.is_empty() {
            Cow::Borrowed("Stripped user-configured query parameter")
        } else {
            Cow::Owned(format!(
                "Stripped custom query parameters: {}",
                self.names.join(", ")
            ))
        };
        Ok(())
    }

    fn current_config(&self) -> Option<toml::Value> {
        let mut table = toml::map::Map::new();
        let params: Vec<toml::Value> = self
            .names
            .iter()
            .map(|s| toml::Value::String(s.clone()))
            .collect();
        table.insert("params".to_owned(), toml::Value::Array(params));
        Some(toml::Value::Table(table))
    }
    fn name(&self) -> &'static str {
        "Strip custom query params"
    }
    fn category(&self) -> TreatmentCategory {
        TreatmentCategory::UrlQueryParam
    }
    fn is_destructive(&self) -> bool {
        false
    }

    fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
        if self.names.is_empty() {
            return vec![];
        }
        strip_matching(node, self.id(), self.rationale.clone(), |name| {
            self.names.iter().any(|n| n == name)
        })
    }
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

/// Produces a [`Change`] for `node` if any query parameter on its URL matches
/// `should_strip`.  Returns an empty `Vec` for non-bookmark nodes or bookmarks
/// with malformed / parameter-free URLs.
fn strip_matching(
    node: &Node,
    treatment_id: &'static str,
    rationale: Cow<'static, str>,
    should_strip: impl Fn(&str) -> bool,
) -> Vec<Change> {
    let bm = match node {
        Node::Bookmark(b) => b,
        _ => return vec![],
    };
    let url = match &bm.url {
        BookmarkUrl::Valid(u) => u,
        BookmarkUrl::Malformed { .. } => return vec![],
    };

    // Collect all pairs; bail early if none match.
    let original_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let filtered: Vec<&(String, String)> = original_pairs
        .iter()
        .filter(|(k, _)| !should_strip(k.as_str()))
        .collect();

    if filtered.len() == original_pairs.len() {
        return vec![]; // Nothing to strip.
    }

    let after_url = rebuild_url(url, &filtered);
    let before = url.as_str().to_owned();
    let after = after_url.as_str().to_owned();

    if before == after {
        return vec![];
    }

    vec![Change::set_field(
        bm.id,
        Field::Url,
        before,
        after,
        treatment_id,
        rationale,
        false,
    )]
}

/// Same as [`strip_matching`] but the predicate is given the full URL so it
/// can make host-aware decisions (e.g. "strip `tag` only on Amazon hosts").
fn strip_matching_with_url(
    node: &Node,
    treatment_id: &'static str,
    rationale: Cow<'static, str>,
    should_strip: impl Fn(&Url, &str) -> bool,
) -> Vec<Change> {
    let bm = match node {
        Node::Bookmark(b) => b,
        _ => return vec![],
    };
    let url = match &bm.url {
        BookmarkUrl::Valid(u) => u,
        BookmarkUrl::Malformed { .. } => return vec![],
    };

    let original_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let filtered: Vec<&(String, String)> = original_pairs
        .iter()
        .filter(|(k, _)| !should_strip(url, k.as_str()))
        .collect();

    if filtered.len() == original_pairs.len() {
        return vec![];
    }

    let after_url = rebuild_url(url, &filtered);
    let before = url.as_str().to_owned();
    let after = after_url.as_str().to_owned();

    if before == after {
        return vec![];
    }

    vec![Change::set_field(
        bm.id,
        Field::Url,
        before,
        after,
        treatment_id,
        rationale,
        false,
    )]
}

/// Rebuild `base_url` keeping only the `keep` query pairs.
///
/// If `keep` is empty the query component is removed entirely (no trailing
/// `?`).  Otherwise the pairs are re-serialized in their original order.
fn rebuild_url(base_url: &Url, keep: &[&(String, String)]) -> Url {
    let mut modified = base_url.clone();

    if keep.is_empty() {
        modified.set_query(None);
    } else {
        // Write the filtered pairs back.  We must drop the borrow from
        // `query_pairs_mut()` before calling anything else on `modified`,
        // so use a block.
        {
            let mut qs = modified.query_pairs_mut();
            qs.clear();
            for (k, v) in keep {
                qs.append_pair(k, v);
            }
        }
    }

    modified
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
    fn utm_strips_all_utm_variants() {
        let node =
            make_bookmark("https://example.com/page?utm_source=newsletter&utm_medium=email&keep=1");
        let changes = UtmTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/page?keep=1"
        );
    }

    #[test]
    fn utm_no_change_when_no_utm_params() {
        let node = make_bookmark("https://example.com/page?q=rust&lang=en");
        let changes = UtmTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn utm_strips_all_params_removes_query_entirely() {
        let node = make_bookmark("https://example.com/?utm_source=x&utm_campaign=y");
        let changes = UtmTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_after().unwrap(), "https://example.com/");
    }

    #[test]
    fn click_ids_strips_fbclid() {
        let node = make_bookmark("https://example.com/?fbclid=ABC123&q=hello");
        let changes = ClickIdsTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].field_after().unwrap(),
            "https://example.com/?q=hello"
        );
    }

    #[test]
    fn session_strips_phpsessid() {
        // URL query pairs are case-sensitive; param names come through
        // lowercased from some servers but not others.  Test the lowercase form.
        let node_lower = make_bookmark("https://example.com/login?phpsessid=abc&next=%2F");
        let changes = SessionTreatment.propose(&node_lower, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].field_after().unwrap().contains("next="));
        assert!(!changes[0].field_after().unwrap().contains("phpsessid"));
    }

    #[test]
    fn malformed_url_skipped() {
        let node = make_bookmark("not a url");
        assert!(UtmTreatment.propose(&node, &ctx()).is_empty());
        assert!(ClickIdsTreatment.propose(&node, &ctx()).is_empty());
        assert!(SessionTreatment.propose(&node, &ctx()).is_empty());
    }

    // ── AffiliateTreatment ─────────────────────────────────────────────────

    #[test]
    fn affiliate_strips_amazon_tag() {
        let node = make_bookmark("https://www.amazon.com/dp/B0123?tag=acme-20&keywords=book");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].field_after().unwrap().contains("keywords=book"));
        assert!(!changes[0].field_after().unwrap().contains("tag="));
    }

    #[test]
    fn affiliate_strips_amazon_ref_block() {
        let node =
            make_bookmark("https://smile.amazon.co.uk/gp/product/B0ABC?ref_=cm_cr&pd_rd_r=x&y=1");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("y=1"));
        assert!(!after.contains("ref_="));
        assert!(!after.contains("pd_rd_r="));
    }

    #[test]
    fn affiliate_leaves_amazon_params_on_non_amazon_host() {
        // `tag` is Amazon-scoped; on another host it must survive.
        let node = make_bookmark("https://example.com/x?tag=promo&q=hi");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn affiliate_strips_ebay_campid() {
        let node = make_bookmark("https://www.ebay.com/itm/123?campid=5338&mkcid=1&title=wrench");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("title=wrench"));
        assert!(!after.contains("campid="));
        assert!(!after.contains("mkcid="));
    }

    #[test]
    fn affiliate_strips_generic_params_on_any_host() {
        let node = make_bookmark("https://shop.example.com/x?affid=abc&q=t");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(changes[0].field_after().unwrap().contains("q=t"));
        assert!(!changes[0].field_after().unwrap().contains("affid="));
    }

    #[test]
    fn affiliate_no_change_when_nothing_matches() {
        let node = make_bookmark("https://example.com/x?q=1&page=2");
        assert!(AffiliateTreatment.propose(&node, &ctx()).is_empty());
    }

    #[test]
    fn affiliate_non_destructive_and_auto_approved() {
        let node = make_bookmark("https://www.amazon.com/dp/X?tag=y");
        let changes = AffiliateTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].destructive);
        assert!(changes[0].approved);
    }

    // ── SearchTokensTreatment ──────────────────────────────────────────────

    #[test]
    fn search_tokens_strips_google_ved_and_ei_preserving_q() {
        let node =
            make_bookmark("https://www.google.com/search?q=rust+book&ved=abc&ei=xyz&oq=rust");
        let changes = SearchTokensTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("q=rust+book") || after.contains("q=rust%2Bbook"));
        assert!(!after.contains("ved="));
        assert!(!after.contains("ei="));
        assert!(!after.contains("oq="));
    }

    #[test]
    fn search_tokens_strips_bing_form_cvid() {
        let node = make_bookmark("https://www.bing.com/search?q=cats&form=QBLH&cvid=abc&pq=ca");
        let changes = SearchTokensTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("q=cats"));
        assert!(!after.contains("form="));
        assert!(!after.contains("cvid="));
        assert!(!after.contains("pq="));
    }

    #[test]
    fn search_tokens_strips_ddg_atb_t_preserving_q() {
        let node = make_bookmark("https://duckduckgo.com/?q=rust&atb=v123&t=h_&ia=web");
        let changes = SearchTokensTreatment.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("q=rust"));
        assert!(!after.contains("atb="));
        assert!(!after.contains("ia=web"));
    }

    #[test]
    fn search_tokens_preserves_params_on_non_search_host() {
        // ved / ei are Google-scoped; on an unrelated host they stay.
        let node = make_bookmark("https://example.com/x?ved=1&ei=2&q=n");
        let changes = SearchTokensTreatment.propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn search_tokens_leaves_clean_search_url_alone() {
        let node = make_bookmark("https://www.google.com/search?q=rust");
        assert!(SearchTokensTreatment.propose(&node, &ctx()).is_empty());
    }

    // ── CustomQpTreatment ──────────────────────────────────────────────────

    #[test]
    fn custom_empty_list_is_noop() {
        let node = make_bookmark("https://example.com/?a=1&b=2");
        let changes = CustomQpTreatment::empty().propose(&node, &ctx());
        assert!(changes.is_empty());
    }

    #[test]
    fn custom_strips_listed_params() {
        let node = make_bookmark("https://example.com/?drop=1&keep=2&also_drop=3");
        let t = CustomQpTreatment::new(vec!["drop".into(), "also_drop".into()]);
        let changes = t.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("keep=2"));
        assert!(!after.contains("drop="));
        assert!(!after.contains("also_drop="));
    }

    #[test]
    fn custom_case_sensitive_match() {
        let node = make_bookmark("https://example.com/?X=1&x=2");
        // Configured with lowercase 'x'; uppercase 'X' should survive.
        let t = CustomQpTreatment::new(vec!["x".into()]);
        let changes = t.propose(&node, &ctx());
        assert_eq!(changes.len(), 1);
        let after = changes[0].field_after().unwrap();
        assert!(after.contains("X=1"));
        assert!(!after.contains("x=2"));
    }

    #[test]
    fn custom_no_change_when_none_match() {
        let node = make_bookmark("https://example.com/?a=1");
        let t = CustomQpTreatment::new(vec!["b".into()]);
        assert!(t.propose(&node, &ctx()).is_empty());
    }
}
