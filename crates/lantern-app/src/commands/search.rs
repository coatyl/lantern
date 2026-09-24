//! Recursive search over titles and URLs.

use std::collections::HashSet;

use lantern_core::model::node::{Folder, Node};

use super::browse::node_to_item;
use super::filter::item_passes_filter;
use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Search the whole document in substring, glob or regex mode.
///
/// Substring and glob queries first ask the tab's search index for candidate
/// bookmarks, then verify each candidate with the matcher.  Regex queries
/// always scan every bookmark.
#[tauri::command]
pub async fn search(
    tab: TabId,
    query: SearchSpec,
    state: tauri::State<'_, AppState>,
) -> CommandResult<SearchResults> {
    if query.query.trim().is_empty() {
        return Ok(SearchResults {
            items: vec![],
            total: 0,
        });
    }

    let matcher = build_matcher(&query)?;
    let candidates: Option<HashSet<NodeId>> = match query.mode {
        SearchMode::Substring | SearchMode::Glob => state
            .ensure_search_index(tab)
            .and_then(|idx| idx.candidates(&query.query))
            .map(HashSet::from_iter),
        SearchMode::Regex => None,
    };

    let items = state.read_doc(tab, |doc| {
        let mut items = Vec::new();
        search_folder(
            &doc.root,
            0,
            &matcher,
            &query,
            candidates.as_ref(),
            &mut items,
        );
        Ok(items)
    })?;
    let total = items.len();
    Ok(SearchResults { items, total })
}

/// Collect matching bookmarks below `folder`.  When `candidates` is set, only
/// those bookmark ids are tested.
fn search_folder(
    folder: &Folder,
    depth: u32,
    matcher: &dyn Fn(&str) -> bool,
    spec: &SearchSpec,
    candidates: Option<&HashSet<NodeId>>,
    results: &mut Vec<FolderItem>,
) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                if candidates.is_some_and(|set| !set.contains(&b.id)) {
                    continue;
                }
                let hit = (spec.search_titles && matcher(&b.title))
                    || (spec.search_urls && matcher(b.url.as_str()));
                if hit {
                    let item = node_to_item(child);
                    if item_passes_filter(&item, Some(depth), &spec.filter) {
                        results.push(item);
                    }
                }
            }
            Node::Folder(f) => search_folder(f, depth + 1, matcher, spec, candidates, results),
            Node::Separator(_) => {}
        }
    }
}

type Matcher = Box<dyn Fn(&str) -> bool>;

/// Case-insensitive matcher for the query in `spec`.
fn build_matcher(spec: &SearchSpec) -> CommandResult<Matcher> {
    match spec.mode {
        SearchMode::Substring => {
            let q = spec.query.to_lowercase();
            Ok(Box::new(move |s: &str| s.to_lowercase().contains(&q)))
        }
        SearchMode::Glob => {
            let tokens = parse_glob(&spec.query);
            Ok(Box::new(move |s: &str| glob_matches(&tokens, s)))
        }
        SearchMode::Regex => {
            let re = regex::RegexBuilder::new(&spec.query)
                .case_insensitive(true)
                .build()
                .map_err(|e| UiError::InvalidOperation(format!("invalid regex: {e}")))?;
            Ok(Box::new(move |s: &str| re.is_match(s)))
        }
    }
}

enum GlobToken {
    AnySeq,
    AnyChar,
    Literal(char),
}

/// Parse `*` (any run), `?` (one char) and `\` escapes; everything is
/// lowercased so matching ignores case.
fn parse_glob(pattern: &str) -> Vec<GlobToken> {
    let mut tokens = Vec::new();
    let mut escaped = false;

    for ch in pattern.to_lowercase().chars() {
        if escaped {
            tokens.push(GlobToken::Literal(ch));
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '*' => {
                if !matches!(tokens.last(), Some(GlobToken::AnySeq)) {
                    tokens.push(GlobToken::AnySeq);
                }
            }
            '?' => tokens.push(GlobToken::AnyChar),
            _ => tokens.push(GlobToken::Literal(ch)),
        }
    }
    if escaped {
        tokens.push(GlobToken::Literal('\\'));
    }
    tokens
}

/// Whole-string glob match with single-star backtracking.
fn glob_matches(tokens: &[GlobToken], candidate: &str) -> bool {
    let chars: Vec<char> = candidate.to_lowercase().chars().collect();
    let mut token_index = 0usize;
    let mut char_index = 0usize;
    let mut last_star = None;
    let mut retry_char_index = 0usize;

    while char_index < chars.len() {
        match tokens.get(token_index) {
            Some(GlobToken::Literal(expected)) if *expected == chars[char_index] => {
                token_index += 1;
                char_index += 1;
            }
            Some(GlobToken::AnyChar) => {
                token_index += 1;
                char_index += 1;
            }
            Some(GlobToken::AnySeq) => {
                last_star = Some(token_index);
                token_index += 1;
                retry_char_index = char_index;
            }
            _ => match last_star {
                Some(star_index) => {
                    retry_char_index += 1;
                    char_index = retry_char_index;
                    token_index = star_index + 1;
                }
                None => return false,
            },
        }
    }

    tokens[token_index..]
        .iter()
        .all(|t| matches!(t, GlobToken::AnySeq))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, bookmark, doc_with, folder, Harness};

    fn spec(query: &str, mode: SearchMode) -> SearchSpec {
        SearchSpec {
            query: query.into(),
            search_titles: true,
            search_urls: true,
            mode,
            filter: FilterSpec::default(),
        }
    }

    fn ids(results: &SearchResults) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = results.items.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn glob_star_question_mark_and_escapes() {
        let star = parse_glob("git*hub");
        assert!(glob_matches(&star, "GitHub"));
        assert!(glob_matches(&star, "git-awesome-hub"));
        assert!(!glob_matches(&star, "gitlab"));

        let one = parse_glob("v?.json");
        assert!(glob_matches(&one, "v1.json"));
        assert!(!glob_matches(&one, "v10.json"));

        let escaped = parse_glob(r"file\?.txt");
        assert!(glob_matches(&escaped, "file?.txt"));
        assert!(!glob_matches(&escaped, "file1.txt"));
    }

    #[test]
    fn matchers_ignore_case_and_reject_bad_regex() {
        let glob = build_matcher(&spec("*mozilla*", SearchMode::Glob)).unwrap();
        assert!(glob("MOZILLA Developer Network"));
        let regex = build_matcher(&spec("^rust", SearchMode::Regex)).unwrap();
        assert!(regex("Rust book"));
        assert!(matches!(
            build_matcher(&spec("(unterminated", SearchMode::Regex)),
            Err(UiError::InvalidOperation(_))
        ));
    }

    #[test]
    fn indexed_search_agrees_with_a_full_scan() {
        let h = Harness::new();
        let tab = h.open(doc_with(folder(
            1,
            "Bookmarks",
            vec![
                bookmark(10, "Rust async runtime", "https://example.com/a"),
                bookmark(11, "Rust types", "https://example.com/t"),
                bookmark(12, "Python async", "https://example.com/p"),
                bookmark(13, "Unrelated", "https://other.example/u"),
            ],
        )));

        let indexed =
            block_on(search(tab, spec("rust", SearchMode::Substring), h.state())).unwrap();
        let scanned = block_on(search(tab, spec("rust", SearchMode::Regex), h.state())).unwrap();
        assert_eq!(ids(&indexed), vec![10, 11]);
        assert_eq!(ids(&indexed), ids(&scanned));
    }

    #[test]
    fn search_after_an_edit_rebuilds_the_index() {
        let h = Harness::new();
        let tab = h.open(doc_with(folder(
            1,
            "Bookmarks",
            vec![
                bookmark(20, "Rust types", "https://example.com/a"),
                bookmark(21, "Python async", "https://example.com/p"),
            ],
        )));
        let rust =
            || block_on(search(tab, spec("rust", SearchMode::Substring), h.state())).unwrap();
        let cached = || matches!(h.state().search_indexes.read().get(&tab), Some(Some(_)));

        assert_eq!(ids(&rust()), vec![20]);
        assert!(cached());

        h.state()
            .edit_doc(tab, |doc| {
                doc.rename_node(20, "Renamed away".into())?;
                doc.rename_node(21, "Now rust".into())?;
                Ok(())
            })
            .unwrap();
        assert!(!cached(), "an edit marks the index stale");

        assert_eq!(ids(&rust()), vec![21]);
        assert!(cached());
    }
}
