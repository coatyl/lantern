//! Structured filter shared by list browsing and search.

use crate::types::{FilterSpec, FolderItem, ItemKind};

/// Whether `item` survives `filter`.  Active axes compose with AND.
///
/// `depth` is the item's distance from the document root; pass `None` when it
/// is unknown (browsing a single folder) to skip the depth axis.  The date,
/// domain, TLD and scheme axes only test bookmarks and let folders and
/// separators through.
pub(super) fn item_passes_filter(
    item: &FolderItem,
    depth: Option<u32>,
    filter: &FilterSpec,
) -> bool {
    if let (Some(d), Some(df)) = (depth, filter.depth.as_ref()) {
        if df.min.is_some_and(|min| d < min) || df.max.is_some_and(|max| d > max) {
            return false;
        }
    }

    if let Some(kinds) = active(&filter.kinds) {
        if !kinds.contains(&item.kind) {
            return false;
        }
    }

    if item.kind != ItemKind::Bookmark {
        return true;
    }

    // Once a bound is set, bookmarks without a date are excluded.
    if let Some(dr) = filter.date_range.as_ref() {
        if dr.since.is_some() || dr.until.is_some() {
            let Some(ts) = item.add_date else {
                return false;
            };
            if dr.since.is_some_and(|since| ts < since) || dr.until.is_some_and(|until| ts > until)
            {
                return false;
            }
        }
    }

    // `domain` already has `www.` stripped.
    if let Some(domains) = active(&filter.domains) {
        let Some(domain) = item.domain.as_deref() else {
            return false;
        };
        if !domains.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
            return false;
        }
    }

    // TLD = last label of the domain; "com" and ".com" both match.
    if let Some(tlds) = active(&filter.tlds) {
        let Some(tld) = item
            .domain
            .as_deref()
            .and_then(|d| d.rsplit_once('.'))
            .map(|(_, t)| t)
        else {
            return false;
        };
        if !tlds
            .iter()
            .any(|t| t.trim_start_matches('.').eq_ignore_ascii_case(tld))
        {
            return false;
        }
    }

    if let Some(schemes) = active(&filter.schemes) {
        let Some(scheme) = item.url.as_deref().and_then(|u| url::Url::parse(u).ok()) else {
            return false;
        };
        if !schemes
            .iter()
            .any(|s| s.eq_ignore_ascii_case(scheme.scheme()))
        {
            return false;
        }
    }

    true
}

/// An allowlist axis is active when present and non-empty.
fn active<T>(axis: &Option<Vec<T>>) -> Option<&[T]> {
    axis.as_deref().filter(|list| !list.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DateRange, DepthFilter};

    fn bm(domain: &str, scheme: &str, add_date: Option<i64>) -> FolderItem {
        FolderItem {
            id: 1,
            kind: ItemKind::Bookmark,
            title: "t".into(),
            url: Some(format!("{scheme}://{domain}/")),
            domain: Some(domain.into()),
            add_date,
            last_modified: None,
        }
    }

    fn folder() -> FolderItem {
        FolderItem {
            id: 2,
            kind: ItemKind::Folder,
            title: "f".into(),
            url: None,
            domain: None,
            add_date: Some(0),
            last_modified: None,
        }
    }

    fn passes(item: &FolderItem, f: &FilterSpec) -> bool {
        item_passes_filter(item, None, f)
    }

    #[test]
    fn empty_filter_and_empty_lists_pass_everything() {
        let empty = FilterSpec::default();
        let empty_kinds = FilterSpec {
            kinds: Some(vec![]),
            ..Default::default()
        };
        for f in [&empty, &empty_kinds] {
            assert!(passes(&bm("example.com", "https", Some(0)), f));
            assert!(passes(&folder(), f));
        }
    }

    #[test]
    fn kind_filter_excludes_other_kinds() {
        let f = FilterSpec {
            kinds: Some(vec![ItemKind::Bookmark]),
            ..Default::default()
        };
        assert!(passes(&bm("a.com", "https", None), &f));
        assert!(!passes(&folder(), &f));
    }

    #[test]
    fn date_range_is_inclusive_and_excludes_undated_bookmarks() {
        let f = FilterSpec {
            date_range: Some(DateRange {
                since: Some(100),
                until: Some(200),
            }),
            ..Default::default()
        };
        for (date, expected) in [
            (Some(100), true),
            (Some(150), true),
            (Some(200), true),
            (Some(50), false),
            (Some(300), false),
            (None, false),
        ] {
            assert_eq!(
                passes(&bm("a.com", "https", date), &f),
                expected,
                "{date:?}"
            );
        }
        assert!(passes(&folder(), &f), "folders ignore the date axis");

        let open_start = FilterSpec {
            date_range: Some(DateRange {
                since: None,
                until: Some(200),
            }),
            ..Default::default()
        };
        assert!(passes(&bm("a.com", "https", Some(100)), &open_start));
        assert!(!passes(&bm("a.com", "https", Some(300)), &open_start));
    }

    #[test]
    fn domain_tld_and_scheme_allowlists_ignore_case() {
        let domains = FilterSpec {
            domains: Some(vec!["Example.COM".into()]),
            ..Default::default()
        };
        assert!(passes(&bm("example.com", "https", None), &domains));
        assert!(!passes(&bm("other.com", "https", None), &domains));

        let tlds = FilterSpec {
            tlds: Some(vec!["com".into(), ".DEV".into()]),
            ..Default::default()
        };
        assert!(passes(&bm("example.com", "https", None), &tlds));
        assert!(passes(&bm("foo.dev", "https", None), &tlds));
        assert!(!passes(&bm("example.org", "https", None), &tlds));

        let schemes = FilterSpec {
            schemes: Some(vec!["HTTPS".into()]),
            ..Default::default()
        };
        assert!(passes(&bm("a.com", "https", None), &schemes));
        assert!(!passes(&bm("a.com", "http", None), &schemes));
    }

    #[test]
    fn depth_filter_only_applies_when_depth_known() {
        let f = FilterSpec {
            depth: Some(DepthFilter {
                min: Some(2),
                max: Some(4),
            }),
            ..Default::default()
        };
        let item = bm("a.com", "https", None);
        assert!(item_passes_filter(&item, None, &f));
        assert!(!item_passes_filter(&item, Some(1), &f));
        assert!(item_passes_filter(&item, Some(3), &f));
        assert!(!item_passes_filter(&item, Some(5), &f));
    }

    #[test]
    fn axes_compose_with_and() {
        let f = FilterSpec {
            kinds: Some(vec![ItemKind::Bookmark]),
            tlds: Some(vec!["com".into()]),
            schemes: Some(vec!["https".into()]),
            ..Default::default()
        };
        assert!(passes(&bm("ok.com", "https", None), &f));
        assert!(!passes(&bm("ok.com", "http", None), &f));
        assert!(!passes(&bm("ok.org", "https", None), &f));
        assert!(!passes(&folder(), &f));
    }
}
