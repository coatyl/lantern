//! Application state held for the lifetime of the process.
//!
//! `AppState` is registered with Tauri via `.manage(...)` and injected into
//! every command as `tauri::State<'_, AppState>`.
//!
//! # Locking discipline
//!
//! - Everything mutated during the session sits behind a `parking_lot::RwLock`.
//! - Guards are short-lived and never held across an `.await`.
//! - The only nesting is `documents` -> `search_indexes`: edits invalidate,
//!   and searches cache, an index while still holding the document lock, so
//!   the cache never keeps an index built from an older document.  Nothing
//!   takes them in the other order.
//! - Tab and change-set ids come from atomics and need no lock.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use indexmap::IndexMap;
use parking_lot::RwLock;

use lantern_core::model::document::Document;
use lantern_core::sanitize::treatment::ChangeSet;
use lantern_core::search::SearchIndex;
use lantern_io::Settings;

use crate::error::{CommandResult, UiError};
use crate::types::{ChangeSetId, TabId};

pub struct AppState {
    /// Open documents keyed by tab id.  `IndexMap` keeps the tab-bar order
    /// stable.
    pub documents: RwLock<IndexMap<TabId, Document>>,

    /// Per-tab search indexes.  `None` means "not built yet" or "stale after
    /// a mutation"; [`AppState::ensure_search_index`] rebuilds on demand.
    pub search_indexes: RwLock<HashMap<TabId, Option<Arc<SearchIndex>>>>,

    /// Change sets awaiting review, keyed by the id handed to the UI and
    /// tagged with the tab they were computed for.  Removed when applied or
    /// when that tab closes.
    pub pending_changesets: RwLock<HashMap<ChangeSetId, (TabId, ChangeSet)>>,

    /// User settings, loaded at startup and written back on change.
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    /// Directory holding `*.lantern-rules.toml` files.
    pub rules_dir: PathBuf,

    /// Recently opened file paths, most recent last.
    pub recent_files: RwLock<VecDeque<PathBuf>>,
    /// Documents that were open when the previous session ended uncleanly.
    pub startup_recovery_paths: RwLock<Vec<PathBuf>>,

    next_tab_id: AtomicU64,
    next_changeset_id: AtomicU64,
}

impl AppState {
    pub fn new(
        settings_path: PathBuf,
        rules_dir: PathBuf,
        settings: Settings,
        startup_recovery_paths: Vec<PathBuf>,
    ) -> Self {
        let recent_files = settings.recent_files.iter().cloned().collect();
        Self {
            documents: RwLock::new(IndexMap::new()),
            search_indexes: RwLock::new(HashMap::new()),
            pending_changesets: RwLock::new(HashMap::new()),
            settings: RwLock::new(settings),
            settings_path,
            rules_dir,
            recent_files: RwLock::new(recent_files),
            startup_recovery_paths: RwLock::new(startup_recovery_paths),
            next_tab_id: AtomicU64::new(1),
            next_changeset_id: AtomicU64::new(1),
        }
    }

    pub fn alloc_tab_id(&self) -> TabId {
        self.next_tab_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn alloc_changeset_id(&self) -> ChangeSetId {
        self.next_changeset_id.fetch_add(1, Ordering::Relaxed)
    }

    // -----------------------------------------------------------------------
    // Document access
    // -----------------------------------------------------------------------

    /// Run `f` against the document open in `tab` under a read lock.
    pub fn read_doc<T>(
        &self,
        tab: TabId,
        f: impl FnOnce(&Document) -> CommandResult<T>,
    ) -> CommandResult<T> {
        let docs = self.documents.read();
        f(docs.get(&tab).ok_or(UiError::TabNotFound(tab))?)
    }

    /// Mutate the document open in `tab` and mark its search index stale.
    pub fn edit_doc<T>(
        &self,
        tab: TabId,
        f: impl FnOnce(&mut Document) -> CommandResult<T>,
    ) -> CommandResult<T> {
        let mut docs = self.documents.write();
        let out = f(docs.get_mut(&tab).ok_or(UiError::TabNotFound(tab))?)?;
        self.search_indexes.write().insert(tab, None);
        Ok(out)
    }

    /// Register `doc` as a new tab and return its id.
    pub fn add_document(&self, doc: Document) -> TabId {
        let tab = self.alloc_tab_id();
        self.documents.write().insert(tab, doc);
        tab
    }

    /// Forget everything held for `tab`.  Returns `false` if it was not open.
    pub fn close_document(&self, tab: TabId) -> bool {
        if self.documents.write().shift_remove(&tab).is_none() {
            return false;
        }
        self.pending_changesets
            .write()
            .retain(|_, (owner, _)| *owner != tab);
        self.search_indexes.write().remove(&tab);
        true
    }

    /// Return the search index for `tab`, building it if it is missing or
    /// stale.  `None` when the tab is unknown.
    pub fn ensure_search_index(&self, tab: TabId) -> Option<Arc<SearchIndex>> {
        if let Some(Some(idx)) = self.search_indexes.read().get(&tab) {
            return Some(Arc::clone(idx));
        }
        let docs = self.documents.read();
        let fresh = Arc::new(SearchIndex::build(docs.get(&tab)?));
        self.search_indexes
            .write()
            .insert(tab, Some(Arc::clone(&fresh)));
        Some(fresh)
    }

    // -----------------------------------------------------------------------
    // Recent files and crash recovery
    // -----------------------------------------------------------------------

    /// Move `path` to the front of the recent-files list, capped at
    /// `settings.recent_files_max`.
    pub fn push_recent_file(&self, path: PathBuf) {
        let max = self.settings.read().recent_files_max;
        let mut rf = self.recent_files.write();
        rf.retain(|p| p != &path);
        rf.push_back(path);
        while rf.len() > max {
            rf.pop_front();
        }
    }

    pub fn take_startup_recovery_paths(&self) -> Vec<PathBuf> {
        std::mem::take(&mut *self.startup_recovery_paths.write())
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    /// Record that a session is running so the next launch can offer
    /// recovery if this one ends uncleanly.
    pub fn mark_session_started(&self) {
        let recent = self.recent_files_vec();
        self.save_settings(|s| {
            s.session_was_running = true;
            s.recoverable_documents.clear();
            s.recent_files = recent;
        });
    }

    pub fn mark_session_closed(&self) {
        let recent = self.recent_files_vec();
        self.save_settings(|s| {
            s.session_was_running = false;
            s.recoverable_documents.clear();
            s.recent_files = recent;
        });
    }

    /// Write settings together with the current recent files and open
    /// document paths (the crash-recovery list).
    pub fn persist_runtime_state(&self) {
        let recent = self.recent_files_vec();
        let open: Vec<PathBuf> = self
            .documents
            .read()
            .values()
            .filter_map(|doc| doc.path.clone())
            .collect();
        self.save_settings(|s| {
            s.recent_files = recent;
            s.recoverable_documents = open;
        });
    }

    fn recent_files_vec(&self) -> Vec<PathBuf> {
        self.recent_files.read().iter().cloned().collect()
    }

    /// Apply `update` to the in-memory settings and write a snapshot to disk.
    /// Failures are logged, not fatal: the app keeps working without a
    /// writable settings file.
    fn save_settings(&self, update: impl FnOnce(&mut Settings)) {
        let snapshot = {
            let mut settings = self.settings.write();
            update(&mut settings);
            settings.clone()
        };
        if let Err(err) = lantern_io::write_settings(&self.settings_path, &snapshot) {
            eprintln!("warn: could not persist settings.toml: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_core::sanitize::treatment::ChangeSet;
    use lantern_io::read_settings;

    use crate::test_support::{doc_with, folder};

    fn state_in(dir: &tempfile::TempDir, settings: Settings) -> AppState {
        AppState::new(
            dir.path().join("settings.toml"),
            dir.path().join("rules"),
            settings,
            Vec::new(),
        )
    }

    #[test]
    fn recent_files_are_seeded_from_settings_and_capped() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            recent_files: vec!["one.html".into(), "two.html".into()],
            recent_files_max: 2,
            ..Settings::default()
        };
        let state = state_in(&dir, settings);

        state.push_recent_file("one.html".into());
        state.push_recent_file("three.html".into());

        let recent: Vec<PathBuf> = state.recent_files.read().iter().cloned().collect();
        assert_eq!(recent, vec![PathBuf::from("one.html"), "three.html".into()]);
    }

    #[test]
    fn session_markers_persist_to_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let state = state_in(&dir, Settings::default());

        state.mark_session_started();
        let started = read_settings(&path).unwrap();
        assert!(started.session_was_running);
        assert!(started.recoverable_documents.is_empty());

        state.mark_session_closed();
        let closed = read_settings(&path).unwrap();
        assert!(!closed.session_was_running);
        assert!(closed.recoverable_documents.is_empty());
    }

    #[test]
    fn persist_runtime_state_captures_open_document_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let state = state_in(&dir, Settings::default());

        let mut doc = doc_with(folder(1, "root", vec![]));
        doc.path = Some("recover-me.html".into());
        state.add_document(doc);
        state.persist_runtime_state();

        let persisted = read_settings(&path).unwrap();
        assert_eq!(
            persisted.recoverable_documents,
            vec![PathBuf::from("recover-me.html")]
        );
    }

    #[test]
    fn closing_a_tab_drops_its_pending_changesets_and_index() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir, Settings::default());
        let a = state.add_document(doc_with(folder(1, "a", vec![])));
        let b = state.add_document(doc_with(folder(1, "b", vec![])));
        {
            let mut pending = state.pending_changesets.write();
            let cs = || ChangeSet {
                rule_set_name: "x".into(),
                changes: Vec::new(),
            };
            pending.insert(10, (a, cs()));
            pending.insert(11, (b, cs()));
        }
        state.ensure_search_index(a).unwrap();

        assert!(state.close_document(a));
        assert!(!state.close_document(a), "second close reports not open");

        let pending: Vec<ChangeSetId> = state.pending_changesets.read().keys().copied().collect();
        assert_eq!(pending, vec![11]);
        assert!(!state.search_indexes.read().contains_key(&a));
    }
}
