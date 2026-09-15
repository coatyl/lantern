//! Application state held for the lifetime of the process.
//!
//! `AppState` is registered with Tauri via `.manage(AppState::new(...))` at
//! startup and injected into every command handler via
//! `tauri::State<'_, AppState>`.
//!
//! # Locking discipline
//!
//! - All fields that are mutated during the session are behind
//!   `parking_lot::RwLock`.
//! - Commands that only read (e.g. `get_tree`, `search`) take a read lock and
//!   release it before returning.
//! - Commands that write (e.g. `apply_changeset`, `open_file`) take a write
//!   lock, perform the mutation, and release it immediately; never across an
//!   `.await` boundary.
//! - `next_tab_id` and `next_changeset_id` use `AtomicU64` so ID allocation
//!   never needs a lock.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::IndexMap;
use parking_lot::RwLock;

use lantern_core::model::document::Document;
use lantern_core::sanitize::treatment::ChangeSet;
use lantern_core::search::SearchIndex;
use lantern_io::Settings;

use crate::types::{ChangeSetId, TabId};

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

/// The single long-lived application state (TDD §8.1).
pub struct AppState {
    /// Open documents keyed by tab ID.  `IndexMap` preserves insertion order
    /// so the tab bar order is stable.
    pub documents: RwLock<IndexMap<TabId, Document>>,

    /// Per-tab search indexes (v0.0.8, NFR-P-4).
    ///
    /// `None` means "not yet built" or "invalidated by a recent mutation".
    /// The `search` command lazily rebuilds via [`AppState::ensure_search_index`]
    /// on the first query after invalidation.  Mutating commands call
    /// [`AppState::invalidate_search_index`] so the next query sees a fresh
    /// build.
    pub search_indexes: RwLock<HashMap<TabId, Option<SearchIndex>>>,

    /// Pending change sets waiting for user approval.  Keyed by the ID
    /// returned to the UI in `ChangeSetPreview`.  Entries are removed when
    /// `apply_changeset` is called or when the associated tab is closed.
    pub pending_changesets: RwLock<HashMap<ChangeSetId, ChangeSet>>,

    /// User settings (loaded from disk at startup, written back on change).
    pub settings: RwLock<Settings>,
    /// Resolved settings.toml path for this build flavor.
    pub settings_path: PathBuf,
    /// Directory holding on-disk rule-set files (`*.lantern-rules.toml`).
    pub rules_dir: PathBuf,

    /// Recently opened file paths (most-recent-last).
    pub recent_files: RwLock<VecDeque<PathBuf>>,
    /// Recoverable paths discovered at startup from an unclean prior session.
    pub startup_recovery_paths: RwLock<Vec<PathBuf>>,

    /// Session-scoped flags (e.g. dead-link checker opted in this session).
    pub session_flags: RwLock<SessionFlags>,

    pub next_tab_id: AtomicU64,
    pub next_changeset_id: AtomicU64,
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
            session_flags: RwLock::new(SessionFlags::default()),
            next_tab_id: AtomicU64::new(1),
            next_changeset_id: AtomicU64::new(1),
        }
    }

    // -----------------------------------------------------------------------
    // Search-index helpers (v0.0.8)
    // -----------------------------------------------------------------------

    /// Mark the search index for `tab` stale so the next `search` call
    /// rebuilds it.  Cheap (one hash-map insert); call this everywhere a
    /// document mutation lands (apply / undo / redo / structural edits).
    pub fn invalidate_search_index(&self, tab: TabId) {
        self.search_indexes.write().insert(tab, None);
    }

    /// Drop any search index associated with `tab`.  Used by `close_tab` so
    /// the index doesn't outlive its document.
    pub fn drop_search_index(&self, tab: TabId) {
        self.search_indexes.write().remove(&tab);
    }

    /// Ensure a fresh `SearchIndex` exists for `tab` and return a clone of
    /// it.  Lazily builds on the first query after a mutation; subsequent
    /// queries reuse the cached index until the next mutation invalidates
    /// it.
    ///
    /// Returns `None` when the tab is unknown.
    ///
    /// Cloning the index is intentional: the `search` command needs to
    /// release the index lock before iterating the document, otherwise a
    /// concurrent mutation could deadlock waiting on the same lock.  The
    /// index itself is two `HashMap`s of small `Vec<NodeId>` so the clone
    /// is cheap relative to the verification pass that follows.
    pub fn ensure_search_index(&self, tab: TabId) -> Option<SearchIndex> {
        // Fast path: already built.
        if let Some(Some(idx)) = self.search_indexes.read().get(&tab) {
            return Some(idx.clone());
        }

        // Slow path: build under the documents read lock.
        let docs = self.documents.read();
        let doc = docs.get(&tab)?;
        let fresh = SearchIndex::build(doc);
        drop(docs);

        let mut indexes = self.search_indexes.write();
        indexes.insert(tab, Some(fresh.clone()));
        Some(fresh)
    }

    /// Allocate a fresh `TabId`.
    pub fn alloc_tab_id(&self) -> TabId {
        self.next_tab_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate a fresh `ChangeSetId`.
    pub fn alloc_changeset_id(&self) -> ChangeSetId {
        self.next_changeset_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Add a path to the recent files list (capped at `settings.recent_files_max`).
    pub fn push_recent_file(&self, path: PathBuf) {
        let max = self.settings.read().recent_files_max;
        let mut rf = self.recent_files.write();
        // Remove duplicate if already present.
        rf.retain(|p| p != &path);
        rf.push_back(path);
        while rf.len() > max {
            rf.pop_front();
        }
    }

    pub fn clear_recent_files(&self) {
        self.recent_files.write().clear();
    }

    pub fn startup_recovery_paths(&self) -> Vec<PathBuf> {
        self.startup_recovery_paths.read().clone()
    }

    pub fn take_startup_recovery_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.startup_recovery_paths.write();
        std::mem::take(&mut *paths)
    }

    pub fn clear_startup_recovery_paths(&self) {
        self.startup_recovery_paths.write().clear();
    }

    pub fn mark_session_started(&self) {
        let snapshot = {
            let mut settings = self.settings.write();
            settings.session_was_running = true;
            settings.recoverable_documents.clear();
            settings.recent_files = self.recent_files.read().iter().cloned().collect();
            settings.clone()
        };
        self.write_settings_snapshot(&snapshot);
    }

    pub fn mark_session_closed(&self) {
        let current_recent_files: Vec<PathBuf> = self.recent_files.read().iter().cloned().collect();
        let snapshot = {
            let mut settings = self.settings.write();
            settings.session_was_running = false;
            settings.recent_files = current_recent_files;
            settings.recoverable_documents.clear();
            settings.clone()
        };
        self.write_settings_snapshot(&snapshot);
    }

    pub fn persist_runtime_state(&self) {
        let current_documents = self.current_document_paths();
        let current_recent_files: Vec<PathBuf> = self.recent_files.read().iter().cloned().collect();
        let snapshot = {
            let mut settings = self.settings.write();
            settings.recent_files = current_recent_files;
            settings.recoverable_documents = current_documents;
            settings.clone()
        };
        self.write_settings_snapshot(&snapshot);
    }

    /// Remove all pending change sets for the given tab.
    pub fn clear_pending_for_tab(&self, tab: TabId) {
        // We don't track which change set belongs to which tab in v0.0.1
        // (there's only one tab), so this is a no-op placeholder.
        let _ = tab;
    }

    fn current_document_paths(&self) -> Vec<PathBuf> {
        self.documents
            .read()
            .values()
            .filter_map(|doc| doc.path.clone())
            .collect()
    }

    fn write_settings_snapshot(&self, snapshot: &Settings) {
        if let Err(err) = lantern_io::write_settings(&self.settings_path, snapshot) {
            eprintln!("warn: could not persist settings.toml: {err}");
        }
    }
}

// ---------------------------------------------------------------------------
// SessionFlags
// ---------------------------------------------------------------------------

/// Per-session boolean flags.
#[derive(Debug, Default)]
pub struct SessionFlags {
    /// True if the user has explicitly opted into the dead-link checker for
    /// this session (PRD NFR-PR-1).
    pub dead_link_checker_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
    use lantern_core::model::node::Folder;
    use lantern_io::read_settings;

    #[test]
    fn app_state_seeds_recent_files_from_settings() {
        let settings = Settings {
            recent_files: vec![PathBuf::from("one.html"), PathBuf::from("two.html")],
            ..Settings::default()
        };

        let state = AppState::new(
            PathBuf::from("settings.toml"),
            PathBuf::from("rules"),
            settings,
            Vec::new(),
        );
        let recent: Vec<PathBuf> = state.recent_files.read().iter().cloned().collect();

        assert_eq!(
            recent,
            vec![PathBuf::from("one.html"), PathBuf::from("two.html")]
        );
    }

    #[test]
    fn session_markers_persist_to_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let state = AppState::new(
            path.clone(),
            dir.path().join("rules"),
            Settings::default(),
            Vec::new(),
        );

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
        let state = AppState::new(
            path.clone(),
            dir.path().join("rules"),
            Settings::default(),
            Vec::new(),
        );

        let doc_path = PathBuf::from("recover-me.html");
        let doc = Document::new(
            1,
            Some(doc_path.clone()),
            Folder {
                id: 1,
                name: "root".into(),
                add_date: None,
                last_modified: None,
                is_toolbar: false,
                attrs: IndexMap::new(),
                children: Vec::new(),
            },
            HeaderMetadata::default(),
            DocumentStats::default(),
        );
        state.documents.write().insert(1, doc);

        state.persist_runtime_state();

        let persisted = read_settings(&path).unwrap();
        assert!(!persisted.session_was_running);
        assert_eq!(persisted.recoverable_documents, vec![doc_path]);
    }
}
