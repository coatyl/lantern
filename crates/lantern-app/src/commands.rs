//! Tauri command handlers (TDD §8.2).
//!
//! Commands are thin wrappers: validation and view-model conversion happen
//! here; all logic lives in `lantern-core` and `lantern-io`.
//!
//! # v0.0.1 commands
//!
//! - `open_file` / `close_tab` / `list_tabs`
//! - `get_tree` / `get_folder_items`
//! - `search`
//! - `run_pass` / `apply_changeset`
//! - `undo` / `redo`
//! - `export`

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use lantern_core::emit::EmitOptions;
use lantern_core::model::document::{Document, DocumentStats};
use lantern_core::model::node::{Folder, Node};
use lantern_core::sanitize::diff::char_diff;
use lantern_core::sanitize::pass::PassTarget;
use lantern_core::sanitize::treatment::{Change, TreatmentCategory};
use lantern_core::sanitize::treatments::{
    AffiliateTreatment, AuthorSuffixTreatment, ClickIdsTreatment, CustomQpTreatment,
    DeduplicateTreatment, DemobilizeTreatment, EmailTreatment, EmptyFoldersTreatment,
    ExactUrlDuplicatesTreatment, FolderHtmlEntitiesTreatment, FolderWhitespaceTreatment,
    FragmentTrackingTreatment, HandleTreatment, HtmlEntitiesTreatment, HttpsUpgradeTreatment,
    RegexFolderTreatment, RegexTitleTreatment, SearchTokensTreatment, SessionTreatment,
    StripFragmentTreatment, UnshortenOfflineTreatment, UserSegmentTreatment, UtmTreatment,
    WhitespaceTreatment,
};
use lantern_io::rulestore;

use crate::types::SearchMode;

use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

// ---------------------------------------------------------------------------
// Tab management
// ---------------------------------------------------------------------------

/// Open a bookmark file and register it as a new tab.
///
/// Returns the new tab's ID.
#[tauri::command]
pub async fn open_file(path: PathBuf, state: tauri::State<'_, AppState>) -> CommandResult<TabId> {
    let doc = lantern_io::read_bookmark_file(&path)?;

    let tab_id = state.alloc_tab_id();
    state.documents.write().insert(tab_id, doc);
    state.push_recent_file(path);
    state.persist_runtime_state();

    Ok(tab_id)
}

/// Close a tab and discard its document.
#[tauri::command]
pub async fn close_tab(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    let removed = state.documents.write().shift_remove(&tab).is_some();
    if !removed {
        return Err(UiError::TabNotFound(tab));
    }
    state.clear_pending_for_tab(tab);
    state.drop_search_index(tab);
    state.persist_runtime_state();
    Ok(())
}

/// Return summary info for all open tabs.
#[tauri::command]
pub async fn list_tabs(state: tauri::State<'_, AppState>) -> CommandResult<Vec<TabInfo>> {
    let docs = state.documents.read();
    let infos = docs
        .iter()
        .map(|(id, doc)| doc_to_tab_info(*id, doc))
        .collect();
    Ok(infos)
}

// ---------------------------------------------------------------------------
// Tree / list panes
// ---------------------------------------------------------------------------

/// Return the complete folder tree for a tab (tree pane data).
#[tauri::command]
pub async fn get_tree(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<TreeView> {
    let docs = state.documents.read();
    let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;
    Ok(TreeView {
        root: folder_to_tree_node(&doc.root),
    })
}

/// Return the immediate folder children of the document root.
///
/// v0.0.8 progressive tree expansion (perf hardening, slice 2): the
/// initial tree-pane mount only fetches one level; subsequent levels
/// are loaded on demand by [`get_tree_children`].  Each entry's
/// [`TreeNodeLazy::has_children`] flag tells the UI whether to render an
/// expand chevron for that row.
#[tauri::command]
pub async fn get_tree_root(
    tab_id: TabId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<TreeNodeLazy>> {
    get_tree_root_impl(tab_id, state.inner())
}

/// State-coupled implementation, factored out so unit tests can call it
/// without spinning up a Tauri runtime.
pub fn get_tree_root_impl(tab_id: TabId, state: &AppState) -> CommandResult<Vec<TreeNodeLazy>> {
    let docs = state.documents.read();
    let doc = docs.get(&tab_id).ok_or(UiError::TabNotFound(tab_id))?;
    Ok(folder_to_lazy_children(&doc.root))
}

/// Return the immediate folder children of `parent_id` for lazy expansion.
///
/// Errors with [`UiError::TabNotFound`] when the tab is unknown and with
/// [`UiError::InvalidOperation`] when `parent_id` doesn't resolve to a
/// folder inside the tab's document.
#[tauri::command]
pub async fn get_tree_children(
    tab_id: TabId,
    parent_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<TreeNodeLazy>> {
    get_tree_children_impl(tab_id, parent_id, state.inner())
}

/// State-coupled implementation, factored out so unit tests can call it
/// without spinning up a Tauri runtime.
pub fn get_tree_children_impl(
    tab_id: TabId,
    parent_id: NodeId,
    state: &AppState,
) -> CommandResult<Vec<TreeNodeLazy>> {
    let docs = state.documents.read();
    let doc = docs.get(&tab_id).ok_or(UiError::TabNotFound(tab_id))?;
    let folder = find_folder(&doc.root, parent_id)
        .ok_or_else(|| UiError::InvalidOperation(format!("folder {parent_id} not found")))?;
    Ok(folder_to_lazy_children(folder))
}

/// Return the items inside `folder_id` for the list pane.
///
/// If `folder_id` is 0, returns items from the document root.
/// Pass `offset` and `limit` for pagination; omit both to get all items.
///
/// `filter`, when set, applies the v0.0.5 structured filter (kind / date /
/// domain / TLD / scheme).  The depth axis is intentionally ignored in
/// browse mode; depth filtering only kicks in during recursive search.
#[tauri::command]
pub async fn get_folder_items(
    tab: TabId,
    folder_id: NodeId,
    sort: Option<SortSpec>,
    offset: Option<usize>,
    limit: Option<usize>,
    filter: Option<FilterSpec>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ItemPage> {
    let docs = state.documents.read();
    let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;

    let folder = if folder_id == 0 {
        &doc.root
    } else {
        find_folder(&doc.root, folder_id)
            .ok_or_else(|| UiError::InvalidOperation(format!("folder {folder_id} not found")))?
    };

    let mut items: Vec<FolderItem> = folder.children.iter().map(node_to_item).collect();

    if let Some(f) = filter.as_ref() {
        items.retain(|item| item_passes_filter(item, None, f));
    }

    if let Some(spec) = sort {
        sort_items(&mut items, &spec);
    }

    let total = items.len();

    // Apply pagination window.
    let items = match (offset, limit) {
        (Some(off), Some(lim)) => {
            let start = off.min(total);
            let end = (start + lim).min(total);
            items[start..end].to_vec()
        }
        (Some(off), None) => {
            let start = off.min(total);
            items[start..].to_vec()
        }
        _ => items,
    };

    Ok(ItemPage { items, total })
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Search across titles and/or URLs.
///
/// Supports substring, glob, and regex modes.
///
/// v0.0.8: closes NFR-P-4.  Substring and glob queries first consult a
/// per-tab `SearchIndex` to narrow the candidate set; the existing matcher
/// then verifies each candidate.  Regex queries skip the index entirely
/// in this baseline (the linear scan keeps regex correct; a future slice
/// can extract literal hints via `regex_syntax::hir::literal::Extractor`).
#[tauri::command]
pub async fn search(
    tab: TabId,
    query: SearchSpec,
    state: tauri::State<'_, AppState>,
) -> CommandResult<SearchResults> {
    search_impl(tab, query, state.inner())
}

/// State-coupled implementation, factored out so unit / integration tests
/// can drive the search path without spinning up a Tauri runtime.
pub fn search_impl(
    tab: TabId,
    query: SearchSpec,
    state: &AppState,
) -> CommandResult<SearchResults> {
    if query.query.trim().is_empty() {
        return Ok(SearchResults {
            items: vec![],
            total: 0,
        });
    }

    let matcher = build_search_matcher(&query)?;

    // Consult the per-tab inverted index for substring/glob queries.
    // Regex falls through to the linear scan path; see the doc comment
    // on `search` for why.
    let candidates: Option<HashSet<NodeId>> = match query.mode {
        SearchMode::Substring | SearchMode::Glob => state
            .ensure_search_index(tab)
            .and_then(|idx| idx.candidates(&query.query))
            .map(|ids| ids.into_iter().collect()),
        SearchMode::Regex => None,
    };

    let docs = state.documents.read();
    let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;

    let mut items = Vec::new();
    match candidates {
        Some(set) => {
            search_folder_with_candidates(&doc.root, 0, &matcher, &query, &set, &mut items);
        }
        None => {
            search_folder_with(&doc.root, 0, &matcher, &query, &mut items);
        }
    }

    let total = items.len();
    Ok(SearchResults { items, total })
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

/// Run a rule set over the document and return a change set for preview.
///
/// The change set is stored in `AppState::pending_changesets` until the user
/// calls `apply_changeset` (or discards it by closing the preview panel; the
/// ID simply expires when the tab is closed).
///
/// v0.0.1: runs synchronously on the Tauri async executor.  For large
/// documents (> 5 000 nodes) this should be moved to `spawn_blocking`.
#[tauri::command]
pub async fn run_pass(
    tab: TabId,
    rule_set_name: String,
    scope: Option<NodeId>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ChangeSetPreview> {
    // Prefer the on-disk rule set; falls back to the in-memory built-in
    // catalogue if the file is missing (so a fresh install without a seeded
    // rules dir still works).
    let rule_set = rulestore::load_rule_set(&state.rules_dir, &rule_set_name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))?;

    // `scope` limits the pass to one folder and everything below it; `None`
    // (or the root folder's id) covers the whole document.
    let target = scope.map_or(PassTarget::AllNodes, PassTarget::Subtree);
    let cs_id = state.alloc_changeset_id();
    let (changeset, preview) = {
        let docs = state.documents.read();
        let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;
        let changeset = lantern_core::sanitize::run_pass(doc, &rule_set, target);
        let preview = changeset_to_preview(cs_id, &changeset, doc);
        (changeset, preview)
    };
    state.pending_changesets.write().insert(cs_id, changeset);

    Ok(preview)
}

/// Apply approved changes from a pending change set.
///
/// `approvals` is a parallel array of booleans matching `ChangeSetPreview.changes`.
/// `true` = apply this change; `false` = skip it.
#[tauri::command]
pub async fn apply_changeset(
    tab: TabId,
    changeset_id: ChangeSetId,
    approvals: Vec<bool>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ApplyReport> {
    let mut cs = state
        .pending_changesets
        .write()
        .remove(&changeset_id)
        .ok_or(UiError::ChangeSetNotFound(changeset_id))?;

    // Apply the user's approval decisions.
    for (change, &approved) in cs.changes.iter_mut().zip(approvals.iter()) {
        change.approved = approved;
    }
    // Any change beyond the approvals array keeps its existing approval flag.

    let approved_count = cs.changes.iter().filter(|c| c.approved).count();
    let skipped_count = cs.changes.len() - approved_count;

    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .apply(&cs)
        .map_err(|e: lantern_core::sanitize::apply::ApplyError| UiError::Internal(e.to_string()))?;

    // The document mutated; drop the cached search index so the next
    // query rebuilds against the new tree (v0.0.8).
    state.invalidate_search_index(tab);

    Ok(ApplyReport {
        applied_count: approved_count,
        skipped_count,
    })
}

// ---------------------------------------------------------------------------
// Undo / redo
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn undo(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .undo()
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(())
}

#[tauri::command]
pub async fn redo(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .redo()
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(())
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Document diff (v0.0.4)
// ---------------------------------------------------------------------------

/// Compare two open tabs by URL identity and report the differences.
///
/// The result groups bookmarks into added / removed / modified buckets.
/// Folder structure is intentionally ignored; see [`lantern_core::diff`] for
/// the exact equality model.
#[tauri::command]
pub async fn compare_tabs(
    left: TabId,
    right: TabId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<DocDiffReport> {
    let docs = state.documents.read();
    let l = docs.get(&left).ok_or(UiError::TabNotFound(left))?;
    let r = docs.get(&right).ok_or(UiError::TabNotFound(right))?;

    let diff = lantern_core::diff::diff_documents(l, r);

    let left_title = l.header.title.clone().unwrap_or_else(|| "Bookmarks".into());
    let right_title = r.header.title.clone().unwrap_or_else(|| "Bookmarks".into());

    Ok(DocDiffReport {
        left_title,
        right_title,
        added: diff.added.into_iter().map(Into::into).collect(),
        removed: diff.removed.into_iter().map(Into::into).collect(),
        modified: diff.modified.into_iter().map(Into::into).collect(),
    })
}

// ---------------------------------------------------------------------------
// Dead-link checker (v0.0.4)
// ---------------------------------------------------------------------------
//
// The dead-link checker is the sole network-touching feature in the desktop
// app.  In v0.0.6 it was moved behind the `checker` feature flag so the
// `--no-default-features` build is provably offline (no reqwest, tokio, or
// rustls symbols).  Everything below this banner (the command, its helper,
// and the `lantern_net` import in `From<LinkStatus>`) is gated.

/// Probe every bookmark in `tab` for reachability and return one
/// [`LinkCheckEntry`] per bookmark.
///
/// Refuses to run if the user has not opted in
/// (`AppSettings::dead_link_checker_opt_in`); this honours the privacy
/// guarantee in PRD NFR-PRIV-2.
#[cfg(feature = "checker")]
#[tauri::command]
pub async fn check_dead_links(
    tab: TabId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<LinkCheckReport> {
    if !state.settings.read().dead_link_checker_opt_in {
        return Err(UiError::InvalidOperation(
            "Dead-link checker is disabled in Settings".into(),
        ));
    }

    // Collect (node_id, url, title) for every bookmark, then drop the lock
    // before we await on network I/O.
    let bookmarks: Vec<(NodeId, String, String)> = {
        let docs = state.documents.read();
        let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;
        let mut out = Vec::new();
        collect_bookmark_links(&doc.root, &mut out);
        out
    };

    let total_bookmarks = bookmarks.len();
    let urls: Vec<String> = bookmarks.iter().map(|(_, u, _)| u.clone()).collect();

    let opts = lantern_net::CheckOptions::default();
    let results = lantern_net::check_links(&urls, &opts).await;

    let mut probed = 0usize;
    let entries: Vec<LinkCheckEntry> = bookmarks
        .into_iter()
        .zip(results)
        .map(|((node_id, url, title), r)| {
            if !matches!(r.status, lantern_net::LinkStatus::Skipped { .. }) {
                probed += 1;
            }
            LinkCheckEntry {
                node_id,
                url,
                title,
                status: r.status.into(),
                elapsed_ms: r.elapsed_ms,
            }
        })
        .collect();

    Ok(LinkCheckReport {
        entries,
        total_bookmarks,
        probed,
    })
}

#[cfg(feature = "checker")]
fn collect_bookmark_links(folder: &Folder, out: &mut Vec<(NodeId, String, String)>) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                out.push((b.id, b.url.as_str().to_owned(), b.title.clone()));
            }
            Node::Folder(f) => collect_bookmark_links(f, out),
            _ => {}
        }
    }
}

/// Export the document (or a subtree of it) to a Netscape bookmark HTML file.
///
/// Refuses to overwrite the source file the document was loaded from
/// (PRD F-EXP-7); that protection is enforced in `lantern_io`.
///
/// `scope` selects the unit of export:
/// - `WholeDocument` writes the document as-is.
/// - `Subtree { root_id }` (v0.0.5) writes a self-contained file rooted at
///   the named folder.  The synthetic export-time document carries no
///   undo/redo state and is detached from the original on-disk path.
#[tauri::command]
pub async fn export(
    tab: TabId,
    scope: ExportScope,
    path: PathBuf,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ExportReport> {
    let docs = state.documents.read();
    let doc = docs.get(&tab).ok_or(UiError::TabNotFound(tab))?;

    let opts = EmitOptions::default();

    // Branching on scope without holding two clones of the world.
    let (bytes_written, write_result) = match scope {
        ExportScope::WholeDocument => {
            let bytes = lantern_core::emit::emit(doc, &opts);
            let n = bytes.len();
            let r = lantern_io::write_bookmark_file(&path, doc, &opts);
            (n, r)
        }
        ExportScope::Subtree { root_id } => {
            let folder = find_folder(&doc.root, root_id)
                .ok_or_else(|| UiError::InvalidOperation(format!("folder {root_id} not found")))?;
            let synthetic = build_subtree_document(doc, folder);
            let bytes = lantern_core::emit::emit(&synthetic, &opts);
            let n = bytes.len();
            let r = lantern_io::write_bookmark_file(&path, &synthetic, &opts);
            (n, r)
        }
    };
    write_result?;

    Ok(ExportReport {
        path: path.to_string_lossy().into_owned(),
        bytes_written,
    })
}

// ---------------------------------------------------------------------------
// Structural edits
// ---------------------------------------------------------------------------

/// Rename a node (bookmark title or folder name) and push an undo entry.
#[tauri::command]
pub async fn rename_node(
    tab: TabId,
    node_id: NodeId,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .rename_node(node_id, new_name)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(())
}

/// Delete a node from the document tree.
///
/// This operation is not undoable in v0.0.2 (structural undo planned for v0.0.3).
#[tauri::command]
pub async fn delete_node(
    tab: TabId,
    node_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .delete_node(node_id)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(())
}

/// Create a new bookmark inside `folder_id` (0 = document root).
///
/// Returns the new node's ID.
#[tauri::command]
pub async fn create_bookmark(
    tab: TabId,
    folder_id: NodeId,
    title: String,
    url: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    let new_id = state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .create_bookmark(folder_id, title, url)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(new_id)
}

/// Create a new folder inside `folder_id` (0 = document root).
///
/// Returns the new node's ID.
#[tauri::command]
pub async fn create_folder(
    tab: TabId,
    folder_id: NodeId,
    name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    let new_id = state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .create_folder(folder_id, name)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(new_id)
}

/// Create a new separator inside `folder_id` (0 = document root).
///
/// Returns the new node's ID.
#[tauri::command]
pub async fn create_separator(
    tab: TabId,
    folder_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    let new_id = state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .create_separator(folder_id)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(new_id)
}

/// Move `node_id` to `new_index` within `new_parent_id` (0 = root).
///
/// Not undoable in v0.0.2 (structural undo planned for v0.0.3).
#[tauri::command]
pub async fn move_node(
    tab: TabId,
    node_id: NodeId,
    new_parent_id: NodeId,
    new_index: usize,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state
        .documents
        .write()
        .get_mut(&tab)
        .ok_or(UiError::TabNotFound(tab))?
        .move_node(node_id, new_parent_id, new_index)
        .map_err(UiError::from)?;
    state.invalidate_search_index(tab);
    Ok(())
}

// ---------------------------------------------------------------------------
// Recent files
// ---------------------------------------------------------------------------

/// Return the list of recently opened file paths (most-recent-first).
#[tauri::command]
pub async fn list_recent_files(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    let rf = state.recent_files.read();
    Ok(rf
        .iter()
        .rev()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

/// Clear the persisted recent-files list.
#[tauri::command]
pub async fn clear_recent_files(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.clear_recent_files();
    state.persist_runtime_state();
    Ok(())
}

/// Return recoverable document paths from the previous unclean session.
#[tauri::command]
pub async fn get_recovery_state(state: tauri::State<'_, AppState>) -> CommandResult<RecoveryState> {
    let paths = state
        .startup_recovery_paths()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(RecoveryState { paths })
}

/// Restore recoverable documents from the previous unclean session.
#[tauri::command]
pub async fn restore_recovery_session(
    state: tauri::State<'_, AppState>,
) -> CommandResult<RecoveryRestoreReport> {
    let recovery_paths = state.take_startup_recovery_paths();
    let existing_paths: HashSet<PathBuf> = state
        .documents
        .read()
        .values()
        .filter_map(|doc| doc.path.clone())
        .collect();

    let mut seen = existing_paths;
    let mut restored_tab_ids = Vec::new();
    let mut restored_paths = Vec::new();
    let mut failed_paths = Vec::new();

    for path in recovery_paths {
        if !seen.insert(path.clone()) {
            continue;
        }

        match lantern_io::read_bookmark_file(&path) {
            Ok(doc) => {
                let tab_id = state.alloc_tab_id();
                state.documents.write().insert(tab_id, doc);
                state.push_recent_file(path.clone());
                restored_tab_ids.push(tab_id);
                restored_paths.push(path.to_string_lossy().into_owned());
            }
            Err(err) => {
                eprintln!("warn: could not restore {:?}: {err}", path);
                failed_paths.push(path.to_string_lossy().into_owned());
            }
        }
    }

    state.persist_runtime_state();

    Ok(RecoveryRestoreReport {
        restored_tab_ids,
        restored_paths,
        failed_paths,
    })
}

/// Dismiss the current recovery prompt.
#[tauri::command]
pub async fn dismiss_recovery_session(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.clear_startup_recovery_paths();
    Ok(())
}

// ---------------------------------------------------------------------------
// View-model conversions
// ---------------------------------------------------------------------------

fn doc_to_tab_info(id: TabId, doc: &Document) -> TabInfo {
    TabInfo {
        id,
        title: doc
            .header
            .title
            .clone()
            .unwrap_or_else(|| "Bookmarks".into()),
        path: doc.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
        dirty: doc.dirty,
        stats: DocStats {
            bookmark_count: doc.stats.bookmark_count,
            folder_count: doc.stats.folder_count,
            separator_count: doc.stats.separator_count,
        },
    }
}

fn folder_to_tree_node(folder: &Folder) -> TreeNode {
    let children = folder
        .children
        .iter()
        .filter_map(|child| {
            if let Node::Folder(f) = child {
                Some(folder_to_tree_node(f))
            } else {
                None
            }
        })
        .collect();

    TreeNode {
        id: folder.id,
        name: folder.name.clone(),
        children,
    }
}

/// Build the immediate folder children of `parent` as `TreeNodeLazy` rows.
///
/// Used by [`get_tree_root`] and [`get_tree_children`].  Skips bookmarks and
/// separators; the tree pane only renders folders.  `has_children` is
/// `true` when the child folder itself contains at least one folder
/// descendant (the expand chevron should appear).
fn folder_to_lazy_children(parent: &Folder) -> Vec<TreeNodeLazy> {
    parent
        .children
        .iter()
        .filter_map(|child| {
            if let Node::Folder(f) = child {
                Some(TreeNodeLazy {
                    id: f.id,
                    name: f.name.clone(),
                    has_children: f.children.iter().any(|c| matches!(c, Node::Folder(_))),
                })
            } else {
                None
            }
        })
        .collect()
}

fn node_to_item(node: &Node) -> FolderItem {
    match node {
        Node::Bookmark(b) => FolderItem {
            id: b.id,
            kind: ItemKind::Bookmark,
            title: b.title.clone(),
            url: Some(b.url.as_str().to_owned()),
            domain: extract_domain(b.url.as_str()),
            add_date: b.add_date.map(|dt| dt.timestamp()),
            last_modified: b.last_modified.map(|dt| dt.timestamp()),
        },
        Node::Folder(f) => FolderItem {
            id: f.id,
            kind: ItemKind::Folder,
            title: f.name.clone(),
            url: None,
            domain: None,
            add_date: f.add_date.map(|dt| dt.timestamp()),
            last_modified: f.last_modified.map(|dt| dt.timestamp()),
        },
        Node::Separator(s) => FolderItem {
            id: s.id,
            kind: ItemKind::Separator,
            title: String::new(),
            url: None,
            domain: None,
            add_date: None,
            last_modified: None,
        },
    }
}

/// What the review surface shows about the node a change touches.
struct NodeContext {
    title: String,
    url: Option<String>,
    location: Vec<String>,
}

/// One walk of the document, collecting context for the nodes in `wanted`.
fn node_contexts(doc: &Document, wanted: &HashSet<NodeId>) -> HashMap<NodeId, NodeContext> {
    fn walk(
        folder: &Folder,
        path: &mut Vec<String>,
        wanted: &HashSet<NodeId>,
        out: &mut HashMap<NodeId, NodeContext>,
    ) {
        for child in &folder.children {
            if wanted.contains(&child.id()) {
                let (title, url) = match child {
                    Node::Bookmark(b) => (b.title.clone(), Some(b.url.as_str().to_owned())),
                    Node::Folder(f) => (f.name.clone(), None),
                    Node::Separator(_) => (String::new(), None),
                };
                out.insert(
                    child.id(),
                    NodeContext {
                        title,
                        url,
                        location: path.clone(),
                    },
                );
            }
            if let Node::Folder(f) = child {
                path.push(f.name.clone());
                walk(f, path, wanted, out);
                path.pop();
            }
        }
    }
    let mut out = HashMap::with_capacity(wanted.len());
    walk(&doc.root, &mut Vec::new(), wanted, &mut out);
    out
}

fn change_to_entry(index: usize, c: &Change, ctx: Option<&NodeContext>) -> ChangeEntry {
    use lantern_core::sanitize::treatment::ChangeKind;
    let (field, before, after) = match &c.kind {
        ChangeKind::SetField {
            field,
            before,
            after,
        } => (
            format!("{field:?}").to_ascii_lowercase(),
            before.clone(),
            after.clone(),
        ),
        ChangeKind::DeleteNode => ("node".into(), String::new(), "(deleted)".into()),
        ChangeKind::SetFlag {
            flag,
            before,
            after,
        } => (
            format!("flag:{flag:?}").to_ascii_lowercase(),
            before.to_string(),
            after.to_string(),
        ),
    };
    let (before_spans, after_spans) = char_diff(&before, &after);
    ChangeEntry {
        index,
        node_id: c.node_id,
        field,
        before,
        after,
        before_spans: before_spans.into_iter().map(DiffSpan::from).collect(),
        after_spans: after_spans.into_iter().map(DiffSpan::from).collect(),
        treatment_id: c.treatment_id.to_owned(),
        rationale: c.rationale.to_string(),
        destructive: c.destructive,
        approved: c.approved,
        node_title: ctx.map(|n| n.title.clone()).unwrap_or_default(),
        node_url: ctx.and_then(|n| n.url.clone()),
        location: ctx.map(|n| n.location.clone()).unwrap_or_default(),
    }
}

fn changeset_to_preview(
    cs_id: ChangeSetId,
    cs: &lantern_core::sanitize::treatment::ChangeSet,
    doc: &Document,
) -> ChangeSetPreview {
    let wanted: HashSet<NodeId> = cs.changes.iter().map(|c| c.node_id).collect();
    let contexts = node_contexts(doc, &wanted);
    ChangeSetPreview {
        changeset_id: cs_id,
        rule_set_name: cs.rule_set_name.clone(),
        changes: cs
            .changes
            .iter()
            .enumerate()
            .map(|(i, c)| change_to_entry(i, c, contexts.get(&c.node_id)))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Tree / search helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlobToken {
    AnySeq,
    AnyChar,
    Literal(char),
}

type SearchMatcher = Box<dyn Fn(&str) -> bool + Send>;

fn build_search_matcher(spec: &SearchSpec) -> CommandResult<SearchMatcher> {
    match spec.mode {
        SearchMode::Substring => {
            let q = spec.query.to_lowercase();
            Ok(Box::new(move |s: &str| {
                s.to_lowercase().contains(q.as_str())
            }))
        }
        SearchMode::Glob => {
            let tokens = parse_glob_pattern(&spec.query);
            Ok(Box::new(move |s: &str| glob_matches_tokens(&tokens, s)))
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

fn parse_glob_pattern(pattern: &str) -> Vec<GlobToken> {
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

fn glob_matches_tokens(tokens: &[GlobToken], candidate: &str) -> bool {
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
            _ => {
                if let Some(star_index) = last_star {
                    retry_char_index += 1;
                    char_index = retry_char_index;
                    token_index = star_index + 1;
                } else {
                    return false;
                }
            }
        }
    }

    while matches!(tokens.get(token_index), Some(GlobToken::AnySeq)) {
        token_index += 1;
    }

    token_index == tokens.len()
}

fn find_folder(folder: &Folder, id: NodeId) -> Option<&Folder> {
    if folder.id == id {
        return Some(folder);
    }
    for child in &folder.children {
        if let Node::Folder(f) = child {
            if let Some(found) = find_folder(f, id) {
                return Some(found);
            }
        }
    }
    None
}

fn search_folder_with(
    folder: &Folder,
    depth: u32,
    matcher: &dyn Fn(&str) -> bool,
    spec: &SearchSpec,
    results: &mut Vec<FolderItem>,
) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                let title_hit = spec.search_titles && matcher(&b.title);
                let url_hit = spec.search_urls && matcher(b.url.as_str());
                if title_hit || url_hit {
                    let item = node_to_item(child);
                    if item_passes_filter(&item, Some(depth), &spec.filter) {
                        results.push(item);
                    }
                }
            }
            Node::Folder(f) => search_folder_with(f, depth + 1, matcher, spec, results),
            _ => {}
        }
    }
}

/// Walk the tree like [`search_folder_with`] but only run the matcher on
/// bookmark IDs present in `candidates`.  Used by the v0.0.8 indexed
/// search path (substring / glob); the linear scan is still authoritative
/// for membership; the index merely narrows the candidate set so we
/// don't run the matcher over every bookmark in the document.
fn search_folder_with_candidates(
    folder: &Folder,
    depth: u32,
    matcher: &dyn Fn(&str) -> bool,
    spec: &SearchSpec,
    candidates: &HashSet<NodeId>,
    results: &mut Vec<FolderItem>,
) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => {
                if !candidates.contains(&b.id) {
                    continue;
                }
                let title_hit = spec.search_titles && matcher(&b.title);
                let url_hit = spec.search_urls && matcher(b.url.as_str());
                if title_hit || url_hit {
                    let item = node_to_item(child);
                    if item_passes_filter(&item, Some(depth), &spec.filter) {
                        results.push(item);
                    }
                }
            }
            Node::Folder(f) => {
                search_folder_with_candidates(f, depth + 1, matcher, spec, candidates, results);
            }
            _ => {}
        }
    }
}

fn sort_items(items: &mut [FolderItem], spec: &SortSpec) {
    items.sort_by(|a, b| {
        let ord = match spec.column {
            SortColumn::Title => a.title.cmp(&b.title),
            SortColumn::Url => a.url.cmp(&b.url),
            SortColumn::Domain => a.domain.cmp(&b.domain),
            SortColumn::AddDate => a.add_date.cmp(&b.add_date),
            SortColumn::LastModified => a.last_modified.cmp(&b.last_modified),
        };
        if spec.descending {
            ord.reverse()
        } else {
            ord
        }
    });
}

fn extract_domain(raw: &str) -> Option<String> {
    url::Url::parse(raw).ok().and_then(|u: url::Url| {
        u.host_str()
            .map(|h: &str| h.trim_start_matches("www.").to_owned())
    })
}

/// Build a self-contained `Document` whose root is a clone of `folder`.
///
/// The synthetic document inherits header metadata (charset, line endings,
/// BOM presence, title) from the source so the exported file feels at home
/// alongside its sibling.  It carries no undo/redo state.
///
/// The source path is intentionally preserved so `lantern_io`'s
/// "no overwrite of source" guard (PRD F-EXP-7) still fires if the user
/// accidentally targets the original file.
fn build_subtree_document(source: &Document, folder: &Folder) -> Document {
    let root = folder.clone();
    let stats = DocumentStats::from_root(&root);
    Document {
        id: source.id,
        path: source.path.clone(),
        root,
        header: source.header.clone(),
        id_gen: source.id_gen.clone(),
        stats,
        open_timestamp: source.open_timestamp,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        dirty: false,
    }
}

fn extract_scheme(raw: &str) -> Option<String> {
    url::Url::parse(raw).ok().map(|u| u.scheme().to_owned())
}

/// Last DNS label of a registered domain (e.g. "com" for "example.com",
/// "co.uk" → "uk").  Used by the TLD filter axis.
fn extract_tld(domain: &str) -> Option<String> {
    domain.rsplit_once('.').map(|(_, tld)| tld.to_owned())
}

/// Test whether `item` survives `filter`.
///
/// `depth` is the item's distance from the document root and is only
/// consulted by the depth axis.  Pass `None` to skip depth checking
/// (browse mode); pass `Some(d)` during recursive walks.
///
/// Bookmark-specific axes (date, domain, TLD, scheme) silently pass
/// non-bookmark items.
fn item_passes_filter(item: &FolderItem, depth: Option<u32>, filter: &FilterSpec) -> bool {
    // Depth: only meaningful when caller knows depth.
    if let (Some(d), Some(df)) = (depth, filter.depth.as_ref()) {
        if let Some(min) = df.min {
            if d < min {
                return false;
            }
        }
        if let Some(max) = df.max {
            if d > max {
                return false;
            }
        }
    }

    // Kind: applies to all items.
    if let Some(kinds) = filter.kinds.as_ref() {
        if !kinds.is_empty() && !kinds.contains(&item.kind) {
            return false;
        }
    }

    // The remaining axes only test bookmarks; folders and separators
    // pass through unaffected.
    if !matches!(item.kind, ItemKind::Bookmark) {
        return true;
    }

    // Date range: items without a date are excluded once the axis is set.
    if let Some(dr) = filter.date_range.as_ref() {
        match item.add_date {
            None => {
                if dr.since.is_some() || dr.until.is_some() {
                    return false;
                }
            }
            Some(ts) => {
                if let Some(since) = dr.since {
                    if ts < since {
                        return false;
                    }
                }
                if let Some(until) = dr.until {
                    if ts > until {
                        return false;
                    }
                }
            }
        }
    }

    // Domain allowlist: case-insensitive equality against the registered
    // domain (eTLD+1, with `www.` already stripped by `extract_domain`).
    if let Some(domains) = filter.domains.as_ref() {
        if !domains.is_empty() {
            let item_domain = match item.domain.as_deref() {
                Some(d) => d.to_lowercase(),
                None => return false,
            };
            if !domains.iter().any(|d| d.to_lowercase() == item_domain) {
                return false;
            }
        }
    }

    // TLD allowlist: last label of the domain.  Leading dots tolerated
    // ("com" and ".com" both match `example.com`).
    if let Some(tlds) = filter.tlds.as_ref() {
        if !tlds.is_empty() {
            let item_tld = match item.domain.as_deref().and_then(extract_tld) {
                Some(t) => t.to_lowercase(),
                None => return false,
            };
            if !tlds
                .iter()
                .any(|t| t.trim_start_matches('.').to_lowercase() == item_tld)
            {
                return false;
            }
        }
    }

    // Scheme allowlist: case-insensitive against the URL's scheme.
    if let Some(schemes) = filter.schemes.as_ref() {
        if !schemes.is_empty() {
            let item_scheme = match item.url.as_deref().and_then(extract_scheme) {
                Some(s) => s.to_lowercase(),
                None => return false,
            };
            if !schemes.iter().any(|s| s.to_lowercase() == item_scheme) {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{
        build_search_matcher, glob_matches_tokens, item_passes_filter, parse_glob_pattern,
    };
    use crate::error::UiError;
    use crate::types::{
        DateRange, DepthFilter, FilterSpec, FolderItem, ItemKind, SearchMode, SearchSpec,
    };

    fn make_spec(query: &str, mode: SearchMode) -> SearchSpec {
        SearchSpec {
            query: query.into(),
            search_titles: true,
            search_urls: true,
            mode,
            filter: FilterSpec::default(),
        }
    }

    // ─── Filter helpers ──────────────────────────────────────────────────

    fn bookmark(domain: &str, scheme: &str, add_date: Option<i64>) -> FolderItem {
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

    #[test]
    fn empty_filter_passes_everything() {
        let f = FilterSpec::default();
        assert!(item_passes_filter(
            &bookmark("example.com", "https", Some(0)),
            None,
            &f
        ));
        assert!(item_passes_filter(&folder(), None, &f));
    }

    #[test]
    fn kind_filter_excludes_other_kinds() {
        let f = FilterSpec {
            kinds: Some(vec![ItemKind::Bookmark]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", None),
            None,
            &f
        ));
        assert!(!item_passes_filter(&folder(), None, &f));
    }

    #[test]
    fn empty_kinds_list_treated_as_no_constraint() {
        let f = FilterSpec {
            kinds: Some(vec![]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", None),
            None,
            &f
        ));
        assert!(item_passes_filter(&folder(), None, &f));
    }

    #[test]
    fn date_range_excludes_outside_window() {
        let f = FilterSpec {
            date_range: Some(DateRange {
                since: Some(100),
                until: Some(200),
            }),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", Some(150)),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", Some(50)),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", Some(300)),
            None,
            &f
        ));
        // Bookmarks with no date are excluded once an axis is set.
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", None),
            None,
            &f
        ));
    }

    #[test]
    fn date_range_open_bounds() {
        let f = FilterSpec {
            date_range: Some(DateRange {
                since: None,
                until: Some(200),
            }),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", Some(100)),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", Some(300)),
            None,
            &f
        ));
    }

    #[test]
    fn date_range_inclusive_endpoints() {
        let f = FilterSpec {
            date_range: Some(DateRange {
                since: Some(100),
                until: Some(200),
            }),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", Some(100)),
            None,
            &f
        ));
        assert!(item_passes_filter(
            &bookmark("a.com", "https", Some(200)),
            None,
            &f
        ));
    }

    #[test]
    fn date_range_does_not_apply_to_folders() {
        let f = FilterSpec {
            date_range: Some(DateRange {
                since: Some(100),
                until: Some(200),
            }),
            ..Default::default()
        };
        assert!(item_passes_filter(&folder(), None, &f));
    }

    #[test]
    fn domain_allowlist_case_insensitive() {
        let f = FilterSpec {
            domains: Some(vec!["Example.COM".into()]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("example.com", "https", None),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("other.com", "https", None),
            None,
            &f
        ));
    }

    #[test]
    fn tld_allowlist_matches_last_label() {
        let f = FilterSpec {
            tlds: Some(vec!["com".into()]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("example.com", "https", None),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("example.org", "https", None),
            None,
            &f
        ));
    }

    #[test]
    fn tld_allowlist_tolerates_leading_dot() {
        let f = FilterSpec {
            tlds: Some(vec![".dev".into()]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("foo.dev", "https", None),
            None,
            &f
        ));
    }

    #[test]
    fn scheme_allowlist_filters_by_url_scheme() {
        let f = FilterSpec {
            schemes: Some(vec!["https".into()]),
            ..Default::default()
        };
        assert!(item_passes_filter(
            &bookmark("a.com", "https", None),
            None,
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("a.com", "http", None),
            None,
            &f
        ));
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
        // Browse mode (no depth supplied): never filters by depth.
        assert!(item_passes_filter(
            &bookmark("a.com", "https", None),
            None,
            &f
        ));
        // Search mode: depth is enforced.
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", None),
            Some(1),
            &f
        ));
        assert!(item_passes_filter(
            &bookmark("a.com", "https", None),
            Some(3),
            &f
        ));
        assert!(!item_passes_filter(
            &bookmark("a.com", "https", None),
            Some(5),
            &f
        ));
    }

    #[test]
    fn axes_compose_with_and() {
        let f = FilterSpec {
            kinds: Some(vec![ItemKind::Bookmark]),
            tlds: Some(vec!["com".into()]),
            schemes: Some(vec!["https".into()]),
            ..Default::default()
        };

        // All three pass
        assert!(item_passes_filter(
            &bookmark("ok.com", "https", None),
            None,
            &f
        ));
        // Wrong scheme
        assert!(!item_passes_filter(
            &bookmark("ok.com", "http", None),
            None,
            &f
        ));
        // Wrong TLD
        assert!(!item_passes_filter(
            &bookmark("ok.org", "https", None),
            None,
            &f
        ));
        // Wrong kind
        assert!(!item_passes_filter(&folder(), None, &f));
    }

    #[test]
    fn glob_matches_star_spans_any_length() {
        let tokens = parse_glob_pattern("git*hub");
        assert!(glob_matches_tokens(&tokens, "GitHub"));
        assert!(glob_matches_tokens(&tokens, "git-awesome-hub"));
        assert!(!glob_matches_tokens(&tokens, "gitlab"));
    }

    #[test]
    fn glob_matches_question_as_single_character() {
        let tokens = parse_glob_pattern("v?.json");
        assert!(glob_matches_tokens(&tokens, "v1.json"));
        assert!(glob_matches_tokens(&tokens, "vA.json"));
        assert!(!glob_matches_tokens(&tokens, "v10.json"));
    }

    #[test]
    fn glob_honors_escape_sequences() {
        let tokens = parse_glob_pattern(r"file\?.txt");
        assert!(glob_matches_tokens(&tokens, "file?.txt"));
        assert!(!glob_matches_tokens(&tokens, "file1.txt"));
    }

    #[test]
    fn glob_matcher_is_case_insensitive() {
        let matcher = build_search_matcher(&make_spec("*mozilla*", SearchMode::Glob)).unwrap();
        assert!(matcher("MOZILLA Developer Network"));
    }

    #[test]
    fn regex_matcher_rejects_invalid_patterns() {
        match build_search_matcher(&make_spec("(unterminated", SearchMode::Regex)) {
            Err(UiError::InvalidOperation(_)) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
            Ok(_) => panic!("expected invalid regex to fail"),
        }
    }
}

// ---------------------------------------------------------------------------
// Settings (v0.0.2)
// ---------------------------------------------------------------------------

/// Return the current user-facing settings subset.
#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> CommandResult<AppSettings> {
    let s = state.settings.read();
    Ok(AppSettings {
        theme: s.theme.clone().into(),
        dead_link_checker_opt_in: s.dead_link_checker_opt_in,
        recent_files_max: s.recent_files_max as u32,
        crash_recovery_enabled: s.crash_recovery_enabled,
        list_density: s.list_density.into(),
        settings_path: state.settings_path.to_string_lossy().into_owned(),
        rules_dir: state.rules_dir.to_string_lossy().into_owned(),
    })
}

/// Replace the user-facing settings with `patch` and persist to disk.
///
/// Runtime fields (recent files, recoverable documents, session flag) are
/// preserved; only the preferences surfaced in [`AppSettings`] are updated.
#[tauri::command]
pub async fn update_settings(
    patch: AppSettings,
    state: tauri::State<'_, AppState>,
) -> CommandResult<AppSettings> {
    // Apply to the in-memory copy while holding the lock.
    {
        let mut s = state.settings.write();
        s.theme = patch.theme.into();
        s.dead_link_checker_opt_in = patch.dead_link_checker_opt_in;
        s.recent_files_max = patch.recent_files_max as usize;
        s.crash_recovery_enabled = patch.crash_recovery_enabled;
        s.list_density = patch.list_density.into();
    }
    // Drop the lock, then persist (write_atomic does disk I/O).
    state.persist_runtime_state();

    // Return the freshly-read canonical shape so the UI stays in sync with
    // whatever normalisation happened on save.
    get_settings(state).await
}

// ---------------------------------------------------------------------------
// Rule-set management (v0.0.2)
// ---------------------------------------------------------------------------

/// Enumerate all rule sets: built-ins plus any user-saved sets.
#[tauri::command]
pub async fn list_rule_sets(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<RuleSetSummary>> {
    let raw =
        rulestore::list_rule_sets(&state.rules_dir).map_err(|e| UiError::Io(e.to_string()))?;
    Ok(raw
        .into_iter()
        .map(|s| RuleSetSummary {
            name: s.name,
            treatment_count: s.treatment_count as u32,
            is_builtin: s.is_builtin,
            path: s.path.to_string_lossy().into_owned(),
        })
        .collect())
}

/// Load the full treatment list for one named rule set.
#[tauri::command]
pub async fn get_rule_set(
    name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    let rs = rulestore::load_rule_set(&state.rules_dir, &name)
        .map_err(|e| UiError::Io(e.to_string()))?;
    let path = rulestore::file_path_for(&state.rules_dir, &rs.name);
    Ok(ruleset_to_detail(&rs, &path))
}

/// Create or overwrite a rule set with the given ordered treatment IDs.
///
/// Called by the Settings → Rule sets editor when there are no per-treatment
/// configs to persist (the simple path).
#[tauri::command]
pub async fn save_rule_set(
    name: String,
    treatment_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    let refs: Vec<&str> = treatment_ids.iter().map(String::as_str).collect();
    let path = rulestore::save_rule_set(&state.rules_dir, &name, &refs)
        .map_err(|e| UiError::Io(e.to_string()))?;
    let rs = rulestore::load_rule_set(&state.rules_dir, &name)
        .map_err(|e| UiError::Io(e.to_string()))?;
    Ok(ruleset_to_detail(&rs, &path))
}

/// Save a rule set with per-treatment configs (custom QP list, regex
/// pattern/replacement, …).
///
/// The companion to [`save_rule_set`] for parameterised treatments.
#[tauri::command]
pub async fn save_rule_set_with_configs(
    name: String,
    treatments: Vec<RuleSetTreatment>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    let entries: Vec<(String, Option<toml::Value>)> = treatments
        .into_iter()
        .map(|t| {
            let cfg = match t.config {
                Some(json) => Some(json_to_toml(&json).map_err(UiError::InvalidOperation)?),
                None => None,
            };
            Ok::<_, UiError>((t.id, cfg))
        })
        .collect::<Result<_, _>>()?;
    let path = rulestore::save_rule_set_with_configs(&state.rules_dir, &name, &entries)
        .map_err(|e| UiError::Io(e.to_string()))?;
    let rs = rulestore::load_rule_set(&state.rules_dir, &name)
        .map_err(|e| UiError::Io(e.to_string()))?;
    Ok(ruleset_to_detail(&rs, &path))
}

fn ruleset_to_detail(
    rs: &lantern_core::sanitize::pass::RuleSet,
    path: &std::path::Path,
) -> RuleSetDetail {
    let treatment_ids: Vec<String> = rs.treatments.iter().map(|t| t.id().to_owned()).collect();
    let treatments: Vec<RuleSetTreatment> = rs
        .treatments
        .iter()
        .map(|t| RuleSetTreatment {
            id: t.id().to_owned(),
            config: t.current_config().map(|tv| toml_to_json(&tv)),
        })
        .collect();
    RuleSetDetail {
        name: rs.name.clone(),
        treatment_ids,
        treatments,
        is_builtin: rulestore::is_builtin(&rs.name),
        path: path.to_string_lossy().into_owned(),
    }
}

fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(tbl) => {
            let map: serde_json::Map<String, serde_json::Value> = tbl
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

fn json_to_toml(v: &serde_json::Value) -> Result<toml::Value, String> {
    match v {
        serde_json::Value::Null => Ok(toml::Value::String(String::new())),
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                Err("number out of range".to_owned())
            }
        }
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_to_toml).collect();
            Ok(toml::Value::Array(items?))
        }
        serde_json::Value::Object(obj) => {
            let mut tbl = toml::map::Map::new();
            for (k, val) in obj {
                tbl.insert(k.clone(), json_to_toml(val)?);
            }
            Ok(toml::Value::Table(tbl))
        }
    }
}

/// Delete a user-created rule set.  Refuses built-ins (the UI should show a
/// reset-to-default affordance instead of a delete button for built-ins).
#[tauri::command]
pub async fn delete_rule_set(name: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    rulestore::delete_rule_set(&state.rules_dir, &name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))
}

/// Copy an existing rule set under a new name.
#[tauri::command]
pub async fn duplicate_rule_set(
    src_name: String,
    dst_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RuleSetDetail> {
    rulestore::duplicate_rule_set(&state.rules_dir, &src_name, &dst_name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))?;
    get_rule_set(dst_name, state).await
}

// ---------------------------------------------------------------------------
// Treatment registry (v0.0.2)
// ---------------------------------------------------------------------------

/// Return the catalogue of every built-in treatment, used by the rule-set
/// editor's "Add treatment" picker so the UI does not have to hardcode the
/// list.
#[tauri::command]
pub async fn list_treatments() -> CommandResult<Vec<TreatmentInfo>> {
    Ok(builtin_treatment_catalogue())
}

// ---------------------------------------------------------------------------
// Cross-document merge (v0.0.7, ADR-0009)
// ---------------------------------------------------------------------------

/// Build a brand-new `Document` from picked subtrees of one or more open
/// tabs and register it as a new tab.  Source documents are read-only inputs
/// (no fields are touched, no undo entries are pushed).  See ADR-0009 for
/// the full contract.
///
/// Returns the new tab ID for the merged document.
#[tauri::command]
pub async fn merge_documents(
    picks: Vec<MergePickRequest>,
    strategy: ConflictStrategyView,
    merged_root_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<TabId> {
    merge_documents_impl(picks, strategy, merged_root_name, state.inner())
}

/// State-coupled implementation, factored out so unit tests can drive the
/// merge without spinning up a Tauri runtime.
pub fn merge_documents_impl(
    picks: Vec<MergePickRequest>,
    strategy: ConflictStrategyView,
    merged_root_name: String,
    state: &AppState,
) -> CommandResult<TabId> {
    use lantern_core::model::merge::{build_merged_document, MergePick, MergePlan};

    if picks.is_empty() {
        return Err(UiError::InvalidOperation("merge plan has no picks".into()));
    }

    // Resolve each pick's tab id to an index into a `sources` slice we
    // hand the core builder.  We deduplicate tabs so identical source-tab
    // ids share a slot; the core does not care, but it keeps the slice
    // small and makes the indices stable.
    let merged_doc = {
        let docs = state.documents.read();

        // Build the ordered list of unique source tab ids.
        let mut order: Vec<TabId> = Vec::new();
        for p in &picks {
            if !order.contains(&p.source_tab_id) {
                order.push(p.source_tab_id);
            }
        }

        // Resolve each tab id to a `&Document`.
        let mut sources: Vec<&Document> = Vec::with_capacity(order.len());
        for tab_id in &order {
            let doc = docs.get(tab_id).ok_or(UiError::TabNotFound(*tab_id))?;
            sources.push(doc);
        }

        // Translate `MergePickRequest` (tab-id-based) to `MergePick`
        // (slice-index-based).
        let core_picks: Vec<MergePick> = picks
            .iter()
            .map(|p| MergePick {
                source_doc_id: order
                    .iter()
                    .position(|id| *id == p.source_tab_id)
                    .expect("just inserted above"),
                root_node_id: p.root_node_id,
            })
            .collect();

        let plan = MergePlan {
            picks: core_picks,
            strategy: strategy.into(),
            merged_root_name,
        };

        build_merged_document(&sources, &plan)
            .map_err(|e| UiError::InvalidOperation(e.to_string()))?
    };

    // Register the new tab.  Take the write lock only after we've dropped
    // the read lock above (block above ends at semicolon).
    let tab_id = state.alloc_tab_id();
    state.documents.write().insert(tab_id, merged_doc);
    state.persist_runtime_state();
    Ok(tab_id)
}

fn category_id(c: TreatmentCategory) -> &'static str {
    match c {
        TreatmentCategory::UrlQueryParam => "url_query_param",
        TreatmentCategory::UrlPath => "url_path",
        TreatmentCategory::UrlFragment => "url_fragment",
        TreatmentCategory::UrlHost => "url_host",
        TreatmentCategory::Title => "title",
        TreatmentCategory::FolderName => "folder_name",
        TreatmentCategory::CrossField => "cross_field",
    }
}

/// Assemble the treatment catalogue by instantiating one of each built-in.
/// Kept as a function (rather than a static) because parameterised treatments
/// use `::empty()` constructors.
fn builtin_treatment_catalogue() -> Vec<TreatmentInfo> {
    use lantern_core::sanitize::treatment::Treatment;

    let instances: Vec<Box<dyn Treatment>> = vec![
        Box::new(UtmTreatment),
        Box::new(ClickIdsTreatment),
        Box::new(SessionTreatment),
        Box::new(AffiliateTreatment),
        Box::new(SearchTokensTreatment),
        Box::new(CustomQpTreatment::empty()),
        Box::new(UserSegmentTreatment),
        Box::new(StripFragmentTreatment),
        Box::new(FragmentTrackingTreatment),
        Box::new(HttpsUpgradeTreatment),
        Box::new(DemobilizeTreatment),
        Box::new(UnshortenOfflineTreatment),
        Box::new(WhitespaceTreatment),
        Box::new(HtmlEntitiesTreatment),
        Box::new(EmailTreatment),
        Box::new(HandleTreatment),
        Box::new(AuthorSuffixTreatment),
        Box::new(RegexTitleTreatment::empty()),
        Box::new(FolderWhitespaceTreatment),
        Box::new(FolderHtmlEntitiesTreatment),
        Box::new(RegexFolderTreatment::empty()),
        Box::new(DeduplicateTreatment),
        Box::new(EmptyFoldersTreatment),
        Box::new(ExactUrlDuplicatesTreatment),
    ];

    instances
        .into_iter()
        .map(|t| TreatmentInfo {
            id: t.id().to_owned(),
            name: t.name().to_owned(),
            category: category_id(t.category()).to_owned(),
            destructive: t.is_destructive(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Settings UI completeness (v0.0.7): Keyboard / Logs / About panes
// ---------------------------------------------------------------------------

/// Static list of keyboard shortcuts surfaced in the Settings → Keyboard pane.
///
/// Mirrors PRD §8.9.  Hardcoded for v0.0.7; user-rebinding is deferred to a
/// later milestone but the `action_id` field is laid out for it.
#[tauri::command]
pub fn list_shortcuts() -> CommandResult<Vec<ShortcutBinding>> {
    Ok(default_shortcuts())
}

fn default_shortcuts() -> Vec<ShortcutBinding> {
    fn s(action_id: &str, label: &str, key_combo: &str, category: &str) -> ShortcutBinding {
        ShortcutBinding {
            action_id: action_id.to_owned(),
            label: label.to_owned(),
            key_combo: key_combo.to_owned(),
            category: category.to_owned(),
        }
    }

    vec![
        // File
        s("open_file", "Open file", "Ctrl+O", "File"),
        s("close_tab", "Close tab", "Ctrl+W", "File"),
        s("close_all_tabs", "Close all tabs", "Ctrl+Shift+W", "File"),
        s("export", "Export", "Ctrl+E", "File"),
        // Edit
        s("undo", "Undo", "Ctrl+Z", "Edit"),
        s("redo", "Redo", "Ctrl+Shift+Z", "Edit"),
        // View
        s("search", "Search", "Ctrl+F", "View"),
        s("filter_drawer", "Filter drawer", "Ctrl+Shift+F", "View"),
        // Tools
        s("command_palette", "Command palette", "Ctrl+K / ⌘K", "Tools"),
        s("settings", "Open settings", "Ctrl+,", "Tools"),
        s(
            "compare_tabs",
            "Compare tabs (diff)",
            "Ctrl+Shift+D",
            "Tools",
        ),
        s("merge_documents", "Merge documents", "Ctrl+M", "Tools"),
        s(
            "run_pass",
            "Run pass with default rule set",
            "Ctrl+R",
            "Tools",
        ),
        // App
        s("help", "Open help", "F1", "App"),
    ]
}

/// Read the most recent log lines from `<settings_dir>/logs/lantern.log`.
///
/// Returns an empty `Vec` when the file does not exist (a fresh install has
/// not yet written anything).  Other I/O errors propagate as `UiError::Io`.
#[tauri::command]
pub async fn get_logs(_state: tauri::State<'_, AppState>) -> CommandResult<Vec<LogEntry>> {
    get_logs_from(&lantern_io::log_path())
}

/// Implementation split out so tests can drive a tempfile path without
/// Tauri runtime.
pub fn get_logs_from(path: &std::path::Path) -> CommandResult<Vec<LogEntry>> {
    match lantern_io::read_recent(path, 200) {
        Ok(raw) => Ok(raw
            .into_iter()
            .map(|r| LogEntry {
                timestamp: r.timestamp,
                level: LogLevel::from_token(&r.level),
                message: r.message,
            })
            .collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(UiError::Io(e.to_string())),
    }
}

/// Surface compile-time facts about this build for the About pane.
#[tauri::command]
pub fn get_build_info() -> CommandResult<BuildInfo> {
    Ok(build_info())
}

fn build_info() -> BuildInfo {
    let build_flavor = if cfg!(feature = "checker") {
        "default"
    } else {
        "offline-only"
    };

    // The build script promotes the `LANTERN_SIGNED` env var (set by the
    // CI sign-windows job) into `option_env!`-readable form.  An unset or
    // empty value resolves to `false`, exactly the right behavior for
    // local dev builds.
    let signed = matches!(option_env!("LANTERN_SIGNED"), Some("1") | Some("true"));

    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_flavor: build_flavor.to_string(),
        rust_version: option_env!("LANTERN_RUSTC_VERSION")
            .unwrap_or("unknown")
            .to_string(),
        git_commit: None,
        license: "Apache-2.0 OR MIT".to_string(),
        adr_index_path: "private/adrs/".to_string(),
        signed,
    }
}

// ---------------------------------------------------------------------------
// Tests for settings-UI completeness (v0.0.7)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod settings_ui_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn list_shortcuts_returns_at_least_ten_entries() {
        let shortcuts = list_shortcuts().unwrap();
        assert!(
            shortcuts.len() >= 10,
            "expected >= 10 shortcuts, got {}",
            shortcuts.len()
        );
    }

    #[test]
    fn list_shortcuts_contains_canonical_combos() {
        let shortcuts = list_shortcuts().unwrap();
        let combos: Vec<&str> = shortcuts.iter().map(|s| s.key_combo.as_str()).collect();
        assert!(combos.contains(&"Ctrl+O"), "missing Ctrl+O");
        assert!(combos.contains(&"Ctrl+,"), "missing Ctrl+,");
        assert!(combos.contains(&"Ctrl+M"), "missing Ctrl+M");
        assert!(
            combos.contains(&"Ctrl+K / ⌘K"),
            "missing command-palette Ctrl+K"
        );
    }

    #[test]
    fn list_shortcuts_groups_by_known_categories() {
        let shortcuts = list_shortcuts().unwrap();
        let cats: std::collections::HashSet<&str> =
            shortcuts.iter().map(|s| s.category.as_str()).collect();
        for expected in &["File", "Edit", "View", "Tools", "App"] {
            assert!(cats.contains(*expected), "missing category {}", expected);
        }
    }

    #[test]
    fn get_logs_missing_file_returns_empty_vec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.log");
        let entries = get_logs_from(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn get_logs_parses_three_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lantern.log");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "2026-05-05T17:23:45Z INFO startup complete").unwrap();
        writeln!(f, "2026-05-05T17:23:46Z WARN dead-link checker disabled").unwrap();
        writeln!(f, "2026-05-05T17:23:47Z ERROR could not parse rule set").unwrap();
        drop(f);

        let entries = get_logs_from(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0].level, LogLevel::Info));
        assert!(matches!(entries[1].level, LogLevel::Warn));
        assert!(matches!(entries[2].level, LogLevel::Error));
        assert_eq!(entries[0].message, "startup complete");
        assert_eq!(entries[2].timestamp, "2026-05-05T17:23:47Z");
    }

    #[test]
    fn get_build_info_reports_workspace_version() {
        let info = get_build_info().unwrap();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(
            !info.rust_version.is_empty(),
            "rust_version should not be empty"
        );
        assert!(
            info.build_flavor == "default" || info.build_flavor == "offline-only",
            "unexpected build_flavor {}",
            info.build_flavor
        );
        assert_eq!(info.license, "Apache-2.0 OR MIT");
    }

    #[test]
    fn get_build_info_signed_field_defaults_false_in_dev() {
        // Local dev builds don't set LANTERN_SIGNED, so the binary reports
        // unsigned.  CI signing job sets the env var; that path is exercised
        // there, not here.
        let info = get_build_info().unwrap();
        assert!(
            !info.signed,
            "dev build should report signed=false (LANTERN_SIGNED env not set during cargo test)"
        );
    }
}

// ---------------------------------------------------------------------------
// Tests for cross-document merge (v0.0.7)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod merge_tests {
    use super::*;
    use crate::types::{ConflictStrategyView, MergePickRequest};
    use indexmap::IndexMap;
    use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
    use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
    use lantern_io::Settings;
    use std::path::PathBuf;

    fn make_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        // Leak the tempdir; it lives for the test, and AppState writes to
        // disk on every mutation so the file must exist.
        std::mem::forget(dir);
        AppState::new(
            path,
            PathBuf::from("rules"),
            Settings::default(),
            Vec::new(),
        )
    }

    fn make_doc_with(name: &str, bookmarks: &[(&str, &str)]) -> Document {
        let mut id_gen = lantern_core::model::ids::NodeIdAllocator::new();
        let root_id = id_gen.alloc();
        let mut children = Vec::new();
        for (title, url) in bookmarks {
            children.push(Node::Bookmark(Bookmark {
                id: id_gen.alloc(),
                title: (*title).into(),
                url: BookmarkUrl::Valid(url::Url::parse(url).unwrap()),
                add_date: None,
                last_modified: None,
                icon_blob: None,
                description: None,
                attrs: AttrMap::new(),
                flags: BookmarkFlags::default(),
            }));
        }
        let root = Folder {
            id: root_id,
            name: name.into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::new(),
            children,
        };
        let stats = DocumentStats::from_root(&root);
        Document {
            id: lantern_core::model::ids::next_document_id(),
            path: None,
            root,
            header: HeaderMetadata::default(),
            id_gen,
            stats,
            open_timestamp: std::time::Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    fn insert_doc(state: &AppState, doc: Document) -> TabId {
        let tab_id = state.alloc_tab_id();
        let mut docs: parking_lot::lock_api::RwLockWriteGuard<
            '_,
            parking_lot::RawRwLock,
            IndexMap<TabId, Document>,
        > = state.documents.write();
        docs.insert(tab_id, doc);
        tab_id
    }

    #[test]
    fn empty_picks_returns_invalid_operation() {
        let state = make_state();
        let err = merge_documents_impl(
            vec![],
            ConflictStrategyView::KeepFirst,
            "Merged".into(),
            &state,
        )
        .unwrap_err();
        assert!(matches!(err, UiError::InvalidOperation(_)));
    }

    #[test]
    fn single_source_pass_through_makes_a_new_tab() {
        let state = make_state();
        let doc = make_doc_with("Bookmarks", &[("a", "https://a.example/")]);
        let pick_id = doc.root.children[0].id();
        let src_tab = insert_doc(&state, doc);

        let new_tab = merge_documents_impl(
            vec![MergePickRequest {
                source_tab_id: src_tab,
                root_node_id: pick_id,
            }],
            ConflictStrategyView::KeepFirst,
            "Merged".into(),
            &state,
        )
        .unwrap();

        assert_ne!(new_tab, src_tab);
        let docs = state.documents.read();
        assert_eq!(docs.len(), 2);
        let merged = docs.get(&new_tab).unwrap();
        assert_eq!(merged.root.name, "Merged");
        assert_eq!(merged.root.children.len(), 1);
        assert!(merged.path.is_none());
        assert!(merged.undo_stack.is_empty());
    }

    #[test]
    fn multi_source_merge_combines_picks_from_each_tab() {
        let state = make_state();
        let doc_a = make_doc_with("A", &[("a", "https://a.example/")]);
        let doc_b = make_doc_with("B", &[("b", "https://b.example/")]);
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();
        let tab_a = insert_doc(&state, doc_a);
        let tab_b = insert_doc(&state, doc_b);

        let new_tab = merge_documents_impl(
            vec![
                MergePickRequest {
                    source_tab_id: tab_a,
                    root_node_id: pick_a,
                },
                MergePickRequest {
                    source_tab_id: tab_b,
                    root_node_id: pick_b,
                },
            ],
            ConflictStrategyView::KeepBoth,
            "Combined".into(),
            &state,
        )
        .unwrap();

        let docs = state.documents.read();
        let merged = docs.get(&new_tab).unwrap();
        assert_eq!(merged.root.name, "Combined");
        assert_eq!(merged.root.children.len(), 2);
        // Source documents are unchanged.
        assert_eq!(docs.get(&tab_a).unwrap().root.children.len(), 1);
        assert_eq!(docs.get(&tab_b).unwrap().root.children.len(), 1);
    }

    #[test]
    fn invalid_tab_id_returns_tab_not_found() {
        let state = make_state();
        let err = merge_documents_impl(
            vec![MergePickRequest {
                source_tab_id: 999,
                root_node_id: 1,
            }],
            ConflictStrategyView::KeepFirst,
            "Merged".into(),
            &state,
        )
        .unwrap_err();
        assert!(matches!(err, UiError::TabNotFound(999)));
    }

    #[test]
    fn keep_first_dedups_overlapping_urls() {
        let state = make_state();
        let doc_a = make_doc_with("A", &[("from-A", "https://shared.example/")]);
        let doc_b = make_doc_with("B", &[("from-B", "https://shared.example/")]);
        let pick_a = doc_a.root.children[0].id();
        let pick_b = doc_b.root.children[0].id();
        let tab_a = insert_doc(&state, doc_a);
        let tab_b = insert_doc(&state, doc_b);

        let new_tab = merge_documents_impl(
            vec![
                MergePickRequest {
                    source_tab_id: tab_a,
                    root_node_id: pick_a,
                },
                MergePickRequest {
                    source_tab_id: tab_b,
                    root_node_id: pick_b,
                },
            ],
            ConflictStrategyView::KeepFirst,
            "Merged".into(),
            &state,
        )
        .unwrap();

        let docs = state.documents.read();
        let merged = docs.get(&new_tab).unwrap();
        assert_eq!(merged.root.children.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Tests for progressive tree expansion (v0.0.8)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tree_lazy_tests {
    use super::*;
    use indexmap::IndexMap;
    use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
    use lantern_core::model::ids::NodeIdAllocator;
    use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
    use lantern_io::Settings;
    use std::path::PathBuf;

    fn make_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        // Leak the tempdir so the file persists for the duration of the test.
        std::mem::forget(dir);
        AppState::new(
            path,
            PathBuf::from("rules"),
            Settings::default(),
            Vec::new(),
        )
    }

    fn folder(id: NodeId, name: &str, children: Vec<Node>) -> Folder {
        Folder {
            id,
            name: name.into(),
            add_date: None,
            last_modified: None,
            is_toolbar: false,
            attrs: AttrMap::new(),
            children,
        }
    }

    fn bookmark(id: NodeId, title: &str) -> Node {
        Node::Bookmark(Bookmark {
            id,
            title: title.into(),
            url: BookmarkUrl::Valid(url::Url::parse("https://example.com/").unwrap()),
            add_date: None,
            last_modified: None,
            icon_blob: None,
            description: None,
            attrs: AttrMap::new(),
            flags: BookmarkFlags::default(),
        })
    }

    /// Build a small document with two top-level folders.
    /// - "Top A" has one folder child ("Nested A1") with one bookmark
    ///   (so Top A has folder children → has_children=true; Nested A1 is
    ///   a leaf folder → has_children=false).
    /// - "Top B" has only a bookmark child (so has_children=false).
    /// - The root also has a bookmark, which should be filtered out.
    ///
    /// Returns the document and a tuple of node ids in the order
    /// `(top_a, nested_a1, top_b)`.
    fn make_doc() -> (Document, (NodeId, NodeId, NodeId)) {
        let mut id_gen = NodeIdAllocator::new();
        let root_id = id_gen.alloc();
        let top_a_id = id_gen.alloc();
        let nested_a1_id = id_gen.alloc();
        let nested_a1_bookmark_id = id_gen.alloc();
        let top_b_id = id_gen.alloc();
        let top_b_bookmark_id = id_gen.alloc();
        let root_bookmark_id = id_gen.alloc();

        let nested_a1 = folder(
            nested_a1_id,
            "Nested A1",
            vec![bookmark(nested_a1_bookmark_id, "Inner")],
        );
        let top_a = folder(top_a_id, "Top A", vec![Node::Folder(nested_a1)]);
        let top_b = folder(top_b_id, "Top B", vec![bookmark(top_b_bookmark_id, "Beep")]);

        let root = folder(
            root_id,
            "Bookmarks",
            vec![
                Node::Folder(top_a),
                Node::Folder(top_b),
                bookmark(root_bookmark_id, "Loose"),
            ],
        );

        let stats = DocumentStats::from_root(&root);
        let doc = Document {
            id: lantern_core::model::ids::next_document_id(),
            path: None,
            root,
            header: HeaderMetadata::default(),
            id_gen,
            stats,
            open_timestamp: std::time::Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        };

        (doc, (top_a_id, nested_a1_id, top_b_id))
    }

    fn insert_doc(state: &AppState, doc: Document) -> TabId {
        let tab_id = state.alloc_tab_id();
        let mut docs: parking_lot::lock_api::RwLockWriteGuard<
            '_,
            parking_lot::RawRwLock,
            IndexMap<TabId, Document>,
        > = state.documents.write();
        docs.insert(tab_id, doc);
        tab_id
    }

    #[test]
    fn get_tree_root_returns_top_level_folders_only() {
        let state = make_state();
        let (doc, (top_a, _nested_a1, top_b)) = make_doc();
        let tab_id = insert_doc(&state, doc);

        let rows = get_tree_root_impl(tab_id, &state).unwrap();

        // Two folders at the top level; the loose bookmark is filtered out.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, top_a);
        assert_eq!(rows[0].name, "Top A");
        assert!(
            rows[0].has_children,
            "Top A nests a folder so it should have a chevron"
        );
        assert_eq!(rows[1].id, top_b);
        assert_eq!(rows[1].name, "Top B");
        assert!(
            !rows[1].has_children,
            "Top B has only a bookmark child so no chevron"
        );
    }

    #[test]
    fn get_tree_children_returns_immediate_folder_children() {
        let state = make_state();
        let (doc, (top_a, nested_a1, _top_b)) = make_doc();
        let tab_id = insert_doc(&state, doc);

        let rows = get_tree_children_impl(tab_id, top_a, &state).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, nested_a1);
        assert_eq!(rows[0].name, "Nested A1");
        assert!(
            !rows[0].has_children,
            "Nested A1 holds only a bookmark so it has no folder children"
        );
    }

    #[test]
    fn get_tree_children_on_leaf_folder_returns_empty_vec() {
        let state = make_state();
        let (doc, (_top_a, _nested_a1, top_b)) = make_doc();
        let tab_id = insert_doc(&state, doc);

        let rows = get_tree_children_impl(tab_id, top_b, &state).unwrap();
        assert!(
            rows.is_empty(),
            "Top B has only a bookmark child, so no folder children"
        );
    }

    #[test]
    fn get_tree_children_with_missing_tab_id_errors() {
        let state = make_state();
        let err = get_tree_children_impl(999, 1, &state).unwrap_err();
        assert!(matches!(err, UiError::TabNotFound(999)));
    }

    #[test]
    fn get_tree_children_with_missing_node_id_errors() {
        let state = make_state();
        let (doc, _) = make_doc();
        let tab_id = insert_doc(&state, doc);

        let err = get_tree_children_impl(tab_id, 99_999, &state).unwrap_err();
        assert!(matches!(err, UiError::InvalidOperation(_)));
    }

    #[test]
    fn get_tree_root_with_missing_tab_id_errors() {
        let state = make_state();
        let err = get_tree_root_impl(999, &state).unwrap_err();
        assert!(matches!(err, UiError::TabNotFound(999)));
    }
}

// ---------------------------------------------------------------------------
// Tests for the v0.0.8 background search index integration
// ---------------------------------------------------------------------------

#[cfg(test)]
mod search_index_integration_tests {
    use super::*;
    use crate::types::{FilterSpec, SearchMode, SearchSpec};
    use indexmap::IndexMap;
    use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
    use lantern_core::model::ids::NodeIdAllocator;
    use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
    use lantern_io::Settings;
    use std::path::PathBuf;

    fn make_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        // Leak the tempdir so the file persists for the duration of the test.
        std::mem::forget(dir);
        AppState::new(
            path,
            PathBuf::from("rules"),
            Settings::default(),
            Vec::new(),
        )
    }

    fn bookmark(id: NodeId, title: &str, url: &str) -> Node {
        Node::Bookmark(Bookmark {
            id,
            title: title.into(),
            url: BookmarkUrl::Valid(url::Url::parse(url).unwrap()),
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
            id: lantern_core::model::ids::next_document_id(),
            path: None,
            root,
            header: HeaderMetadata::default(),
            id_gen,
            stats,
            open_timestamp: std::time::Instant::now(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    fn insert_doc(state: &AppState, doc: Document) -> TabId {
        let tab_id = state.alloc_tab_id();
        let mut docs: parking_lot::lock_api::RwLockWriteGuard<
            '_,
            parking_lot::RawRwLock,
            IndexMap<TabId, Document>,
        > = state.documents.write();
        docs.insert(tab_id, doc);
        tab_id
    }

    fn make_spec(query: &str, mode: SearchMode) -> SearchSpec {
        SearchSpec {
            query: query.into(),
            search_titles: true,
            search_urls: true,
            mode,
            filter: FilterSpec::default(),
        }
    }

    /// Assert that the indexed search path returns the same hits as a
    /// fresh linear scan over the same document.
    #[test]
    fn search_returns_same_results_with_and_without_index() {
        let state = make_state();
        let doc = make_doc(vec![
            bookmark(10, "Rust async runtime", "https://example.com/a"),
            bookmark(11, "Rust types", "https://example.com/t"),
            bookmark(12, "Python async", "https://example.com/p"),
            bookmark(13, "Unrelated bookmark", "https://other.example/u"),
        ]);
        let tab_id = insert_doc(&state, doc);

        // First call: index is built lazily, candidates path runs.
        let spec = make_spec("rust", SearchMode::Substring);
        let indexed = search_impl(tab_id, spec, &state).unwrap();

        // Force the index off and rerun; the linear-scan path is the
        // ground truth for this assertion.
        state.invalidate_search_index(tab_id);
        // Sentinel: manually replace the index with `None` and skip the
        // ensure step by reaching for the matcher directly.
        let docs = state.documents.read();
        let doc = docs.get(&tab_id).unwrap();
        let raw_spec = make_spec("rust", SearchMode::Substring);
        let matcher = build_search_matcher(&raw_spec).unwrap();
        let mut linear = Vec::new();
        search_folder_with(&doc.root, 0, &matcher, &raw_spec, &mut linear);
        drop(docs);

        let mut indexed_ids: Vec<NodeId> = indexed.items.iter().map(|i| i.id).collect();
        let mut linear_ids: Vec<NodeId> = linear.iter().map(|i| i.id).collect();
        indexed_ids.sort_unstable();
        linear_ids.sort_unstable();
        assert_eq!(indexed_ids, linear_ids);
        // Concretely: both should hit only the two "Rust …" bookmarks.
        assert_eq!(linear_ids, vec![10, 11]);
    }

    /// Assert that a document mutation invalidates the cached index so the
    /// next query rebuilds against the post-mutation tree.
    #[test]
    fn search_after_mutation_rebuilds_the_index() {
        let state = make_state();
        let doc = make_doc(vec![
            bookmark(20, "Rust types", "https://example.com/a"),
            bookmark(21, "Python async", "https://example.com/p"),
        ]);
        let tab_id = insert_doc(&state, doc);

        // Warm the index by running a query: produces a hit on bookmark 20.
        let r1 = search_impl(tab_id, make_spec("rust", SearchMode::Substring), &state).unwrap();
        assert_eq!(r1.items.len(), 1);
        assert_eq!(r1.items[0].id, 20);

        // The index should be cached now.
        assert!(
            state
                .search_indexes
                .read()
                .get(&tab_id)
                .map(|opt| opt.is_some())
                .unwrap_or(false),
            "index should be cached after the first search"
        );

        // Mutate the document: rename bookmark 20 so it no longer matches
        // "rust", and rename bookmark 21 to add the token "rust".  This
        // path goes through `Document::rename_node` directly so the test
        // stays free of the Tauri runtime.
        {
            let mut docs = state.documents.write();
            let d = docs.get_mut(&tab_id).unwrap();
            d.rename_node(20, "Renamed away".into()).unwrap();
            d.rename_node(21, "Now rust".into()).unwrap();
        }
        state.invalidate_search_index(tab_id);

        // Cached index slot should now hold `None` (i.e. invalidated).
        assert!(
            matches!(state.search_indexes.read().get(&tab_id), Some(None)),
            "invalidate_search_index should null the slot"
        );

        // Rerun the query; the index rebuilds against the new tree.
        let r2 = search_impl(tab_id, make_spec("rust", SearchMode::Substring), &state).unwrap();
        assert_eq!(r2.items.len(), 1, "exactly one bookmark now matches");
        assert_eq!(r2.items[0].id, 21, "the rebuilt index reflects the rename");

        // And the slot is now populated again.
        assert!(
            state
                .search_indexes
                .read()
                .get(&tab_id)
                .map(|opt| opt.is_some())
                .unwrap_or(false),
            "ensure_search_index repopulates after invalidation"
        );
    }
}
