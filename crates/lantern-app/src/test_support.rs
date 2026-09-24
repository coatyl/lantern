//! Fixtures shared by the unit tests: document builders and a mock Tauri app
//! whose managed state the real command functions can be called against.

use std::future::Future;

use lantern_core::model::document::{Document, DocumentStats, HeaderMetadata};
use lantern_core::model::ids::next_document_id;
use lantern_core::model::node::{AttrMap, Bookmark, BookmarkFlags, BookmarkUrl, Folder, Node};
use lantern_io::Settings;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::Manager;

use crate::state::AppState;
use crate::types::{NodeId, TabId};

pub fn folder(id: NodeId, name: &str, children: Vec<Node>) -> Folder {
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

pub fn bookmark(id: NodeId, title: &str, url: &str) -> Node {
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

pub fn doc_with(root: Folder) -> Document {
    let stats = DocumentStats::from_root(&root);
    Document::new(
        next_document_id(),
        None,
        root,
        HeaderMetadata::default(),
        stats,
    )
}

/// A mock-runtime app managing a fresh [`AppState`] rooted in a temp dir.
pub struct Harness {
    pub app: tauri::App<MockRuntime>,
    _dir: tempfile::TempDir,
}

impl Harness {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(
            dir.path().join("settings.toml"),
            dir.path().join("rules"),
            Settings::default(),
            Vec::new(),
        );
        let app = mock_builder()
            .manage(state)
            .invoke_handler(crate::commands::handler())
            .build(mock_context(noop_assets()))
            .unwrap();
        Self { app, _dir: dir }
    }

    pub fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state()
    }

    pub fn open(&self, doc: Document) -> TabId {
        self.state().add_document(doc)
    }
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    tauri::async_runtime::block_on(future)
}
