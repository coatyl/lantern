//! Per-tab search index.
//!
//! [`SearchIndex`] is an inverted index from lowercase character trigrams to
//! the bookmarks whose title or URL contains them.  The application layer
//! asks it for [`SearchIndex::candidates`] before running its substring or
//! glob matcher, which then verifies each candidate.
//!
//! # Correctness contract
//!
//! The candidate set is always a **superset** of the bookmarks the matcher
//! would accept, so narrowing never hides a hit:
//!
//! - Text is lowercased and split into runs of ASCII alphanumerics; every
//!   3-character window of every run is indexed.
//! - A query is split the same way.  Any substring match of the query places
//!   each of its runs inside a run of the bookmark's text, so every trigram
//!   of every query run must be indexed for that bookmark.  Candidates are
//!   the intersection of those trigrams' postings.
//! - Query runs shorter than three characters (`"go"`) can match anywhere
//!   inside a longer word and are ignored.  When no run is long enough the
//!   index returns `None` and the caller scans the whole document.
//!
//! Regex queries do not use the index.

use std::collections::HashMap;

use crate::model::document::Document;
use crate::model::ids::NodeId;
use crate::model::node::{Folder, Node};

/// Width of the character n-grams the index is keyed on.
const NGRAM: usize = 3;

type Trigram = [u8; NGRAM];

/// Inverted index over bookmark titles and URLs; see the module docs.
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    /// Trigram → sorted, deduplicated ids of the bookmarks containing it.
    postings: HashMap<Trigram, Vec<NodeId>>,
}

impl SearchIndex {
    /// Index every bookmark in `doc`.
    pub fn build(doc: &Document) -> Self {
        let mut postings = HashMap::new();
        index_folder(&doc.root, &mut postings);
        for ids in postings.values_mut() {
            ids.sort_unstable();
            ids.dedup();
        }
        Self { postings }
    }

    /// A sorted superset of the bookmarks whose title or URL can match
    /// `query` (see the module docs).
    ///
    /// Returns `None` when the index cannot narrow the search (no
    /// alphanumeric run of three or more characters in the query), and
    /// `Some(empty)` when some query trigram occurs in no bookmark at all.
    pub fn candidates(&self, query: &str) -> Option<Vec<NodeId>> {
        let lower = query.to_lowercase();
        let mut lists: Vec<&[NodeId]> = Vec::new();
        for gram in tokens_of(&lower).flat_map(trigrams) {
            match self.postings.get(&gram) {
                Some(ids) => lists.push(ids),
                None => return Some(Vec::new()),
            }
        }

        // Intersect starting from the shortest list.
        lists.sort_unstable_by_key(|ids| ids.len());
        let (first, rest) = lists.split_first()?;
        let mut acc = first.to_vec();
        for ids in rest {
            if acc.is_empty() {
                break;
            }
            acc = intersect_sorted(&acc, ids);
        }
        Some(acc)
    }
}

// ---------------------------------------------------------------------------
// Indexing helpers
// ---------------------------------------------------------------------------

fn index_folder(folder: &Folder, postings: &mut HashMap<Trigram, Vec<NodeId>>) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                for text in [b.title.as_str(), b.url.as_str()] {
                    let lower = text.to_lowercase();
                    for gram in tokens_of(&lower).flat_map(trigrams) {
                        let ids = postings.entry(gram).or_default();
                        // Skip repeats of the same bookmark; `build` dedups
                        // anything else.
                        if ids.last() != Some(&b.id) {
                            ids.push(b.id);
                        }
                    }
                }
            }
            Node::Folder(f) => index_folder(f, postings),
            Node::Separator(_) => {}
        }
    }
}

/// Runs of ASCII alphanumerics in `text` (which must already be lowercase).
fn tokens_of(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
}

/// Every [`NGRAM`]-byte window of an ASCII `token`; none if it is shorter.
fn trigrams(token: &str) -> impl Iterator<Item = Trigram> + '_ {
    token.as_bytes().windows(NGRAM).map(|w| {
        let mut gram = [0; NGRAM];
        gram.copy_from_slice(w);
        gram
    })
}

/// Intersection of two sorted id lists.
fn intersect_sorted(a: &[NodeId], b: &[NodeId]) -> Vec<NodeId> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{DocumentStats, HeaderMetadata};
    use crate::model::ids::{next_document_id, NodeIdAllocator};
    use crate::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
    use std::time::Instant;

    fn make_bookmark(id: NodeId, title: &str, url: &str) -> Node {
        Node::Bookmark(Bookmark {
            id,
            title: title.into(),
            url: url::Url::parse(url)
                .map(BookmarkUrl::Valid)
                .unwrap_or_else(|_| BookmarkUrl::Malformed { raw: url.into() }),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::new(),
            flags: BookmarkFlags::default(),
        })
    }

    fn make_doc(children: Vec<Node>) -> Document {
        let mut id_gen = NodeIdAllocator::new();
        let root_id = id_gen.alloc();
        // Drain the allocator forward so it's past the bookmark IDs the
        // caller used (tests assign IDs by hand for deterministic asserts).
        let root = Folder {
            id: root_id,
            name: "Bookmarks".into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::new(),
            children,
        };
        let stats = DocumentStats::from_root(&root);
        Document {
            id: next_document_id(),
            path: None,
            root,
            header: HeaderMetadata::default(),
            id_gen,
            stats,
            open_timestamp: Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    #[test]
    fn empty_document_returns_some_empty() {
        let doc = make_doc(Vec::new());
        let idx = SearchIndex::build(&doc);
        // The query produces trigrams but no postings exist; Some(empty)
        // tells the caller "no hits, skip the linear scan".
        assert_eq!(idx.candidates("foo"), Some(Vec::new()));
    }

    #[test]
    fn title_token_is_indexed_and_candidate_returns_node_id() {
        let doc = make_doc(vec![make_bookmark(
            42,
            "The Rust Programming Language",
            "https://example.com/",
        )]);
        let idx = SearchIndex::build(&doc);
        let candidates = idx.candidates("rust").expect("query produces tokens");
        assert!(
            candidates.contains(&42),
            "expected 42 in candidates, got {candidates:?}"
        );
    }

    #[test]
    fn url_token_is_indexed_and_candidate_returns_node_id() {
        let doc = make_doc(vec![make_bookmark(
            7,
            "An example",
            "https://github.com/coatyl/lantern",
        )]);
        let idx = SearchIndex::build(&doc);
        let candidates = idx.candidates("github").expect("tokens");
        assert!(
            candidates.contains(&7),
            "expected 7 in candidates, got {candidates:?}"
        );
    }

    #[test]
    fn two_token_query_requires_both_tokens_and_semantics() {
        let doc = make_doc(vec![
            make_bookmark(1, "Rust async runtime", "https://example.com/a"),
            make_bookmark(2, "Rust types", "https://example.com/t"),
            make_bookmark(3, "Python async", "https://example.com/p"),
        ]);
        let idx = SearchIndex::build(&doc);
        let candidates = idx.candidates("rust async").expect("tokens");
        // Only bookmark 1 contains both "rust" and "async"; bookmarks 2 and
        // 3 each carry only one of the two tokens.
        assert_eq!(candidates, vec![1], "AND semantics across query tokens");
    }

    #[test]
    fn empty_query_returns_none() {
        let doc = make_doc(vec![make_bookmark(1, "anything", "https://example.com/")]);
        let idx = SearchIndex::build(&doc);
        assert!(idx.candidates("").is_none());
        assert!(idx.candidates("   ").is_none());
    }

    #[test]
    fn nontokenisable_query_returns_none() {
        let doc = make_doc(vec![make_bookmark(1, "anything", "https://example.com/")]);
        let idx = SearchIndex::build(&doc);
        // Only punctuation: produces no tokens, so the index can't help.
        assert!(idx.candidates("!!!").is_none());
        assert!(idx.candidates("...---...").is_none());
    }

    #[test]
    fn prefix_query_hits_longer_token() {
        // Substring searches like "lib" should still hit "library" because
        // we explicitly index the 3- and 4-char prefixes.
        let doc = make_doc(vec![make_bookmark(
            99,
            "Local library catalogue",
            "https://example.org/",
        )]);
        let idx = SearchIndex::build(&doc);
        let candidates = idx.candidates("lib").expect("tokens");
        assert!(candidates.contains(&99));
    }

    #[test]
    fn build_for_25k_completes_in_under_100_ms() {
        // Build a 25 000-bookmark document with deterministic, varied
        // titles/URLs.  Asserts well under the < 100 ms gate the brief
        // calls out (the v0.0.8 reference budget is 50 ms).
        //
        // The timing assertion is gated to release builds; dev profile
        // is unoptimised in this workspace's `[profile.dev]` block, and
        // would routinely exceed 100 ms there.  Run `cargo test --release
        // -p lantern-core search::` to verify the perf budget.
        const N: u64 = 25_000;
        let mut children = Vec::with_capacity(N as usize);
        for i in 0..N {
            children.push(make_bookmark(
                i + 1,
                &format!("Bookmark number {i}: sample title rust async lib"),
                &format!("https://example.com/host{}/page-{i}", i % 64),
            ));
        }
        let doc = make_doc(children);

        let start = Instant::now();
        let idx = SearchIndex::build(&doc);
        let elapsed = start.elapsed();

        assert_eq!(idx.candidates("sample").map(|c| c.len()), Some(N as usize));
        eprintln!(
            "SearchIndex::build over 25k bookmarks: {} ms",
            elapsed.as_millis()
        );
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_millis() < 100,
            "build over 25k bookmarks took {} ms (target < 100 ms, soft target < 50 ms)",
            elapsed.as_millis()
        );
    }

    #[test]
    fn query_for_25k_returns_correct_count_and_runs_under_50_ms() {
        // See the note on `build_for_25k_completes_in_under_100_ms` for
        // why the timing assertion is release-profile-only.
        const N: u64 = 25_000;
        let mut children = Vec::with_capacity(N as usize);
        for i in 0..N {
            // Every 100th bookmark gets the unique token "needle" so we
            // can assert candidate count exactly.
            let title = if i % 100 == 0 {
                format!("Bookmark {i} needle")
            } else {
                format!("Bookmark {i}")
            };
            children.push(make_bookmark(
                i + 1,
                &title,
                &format!("https://example.com/page-{i}"),
            ));
        }
        let doc = make_doc(children);
        let idx = SearchIndex::build(&doc);

        let start = Instant::now();
        let candidates = idx.candidates("needle").expect("tokens");
        let elapsed = start.elapsed();

        assert_eq!(candidates.len(), 250, "every 100th of 25k = 250");
        eprintln!(
            "SearchIndex::candidates over 25k bookmarks: {} ms",
            elapsed.as_millis()
        );
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_millis() < 50,
            "query over 25k bookmarks took {} ms (target < 50 ms)",
            elapsed.as_millis()
        );
    }

    #[test]
    fn short_and_mid_word_queries_still_find_their_bookmarks() {
        // Linear substring search finds "go" in "Google" / "Algorithms" and
        // "brar" in "library"; the index must never narrow those away.
        let doc = make_doc(vec![
            make_bookmark(1, "Google", "https://www.google.com/"),
            make_bookmark(2, "Algorithms", "https://example.com/algo"),
            make_bookmark(3, "Local library", "https://example.org/"),
        ]);
        let idx = SearchIndex::build(&doc);
        assert_eq!(idx.candidates("go"), None, "too short to narrow");
        assert_eq!(idx.candidates("brar"), Some(vec![3]));
        assert_eq!(idx.candidates("ogl"), Some(vec![1]));
        assert_eq!(idx.candidates("orith"), Some(vec![2]));
        // Mixed: the short run is ignored, the long one still narrows.
        assert_eq!(idx.candidates("go library"), Some(vec![3]));
    }

    #[test]
    fn unicode_query_falls_through_to_none_or_empty() {
        // Non-ASCII characters split tokens (everything outside [a-z0-9]
        // is a separator), so a pure-Unicode query produces no tokens.
        let doc = make_doc(vec![make_bookmark(
            1,
            "Hello world",
            "https://example.com/",
        )]);
        let idx = SearchIndex::build(&doc);
        // "中文" tokenises to nothing → None (index can't help; fall back
        // to linear scan).
        assert!(idx.candidates("中文").is_none());
    }
}
