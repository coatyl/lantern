//! Open documents (tabs) and reopening the previous session after a crash.

use std::collections::HashSet;
use std::path::PathBuf;

use lantern_core::model::document::Document;

use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Open a bookmark file in a new tab and return the tab id.
#[tauri::command]
pub async fn open_file(path: PathBuf, state: tauri::State<'_, AppState>) -> CommandResult<TabId> {
    let tab = open_document(&state, path)?;
    state.persist_runtime_state();
    Ok(tab)
}

#[tauri::command]
pub async fn close_tab(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    if !state.close_document(tab) {
        return Err(UiError::TabNotFound(tab));
    }
    state.persist_runtime_state();
    Ok(())
}

#[tauri::command]
pub async fn list_tabs(state: tauri::State<'_, AppState>) -> CommandResult<Vec<TabInfo>> {
    Ok(state
        .documents
        .read()
        .iter()
        .map(|(id, doc)| tab_info(*id, doc))
        .collect())
}

/// Files that were open when the previous session ended uncleanly.
#[tauri::command]
pub async fn get_recovery_state(state: tauri::State<'_, AppState>) -> CommandResult<RecoveryState> {
    let paths = state
        .startup_recovery_paths
        .read()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(RecoveryState { paths })
}

/// Reopen the previous session's files, skipping any that are already open.
/// Files that fail to open are reported, not fatal.
#[tauri::command]
pub async fn restore_recovery_session(
    state: tauri::State<'_, AppState>,
) -> CommandResult<RecoveryRestoreReport> {
    let mut seen: HashSet<PathBuf> = state
        .documents
        .read()
        .values()
        .filter_map(|doc| doc.path.clone())
        .collect();
    let mut report = RecoveryRestoreReport {
        restored_tab_ids: Vec::new(),
        restored_paths: Vec::new(),
        failed_paths: Vec::new(),
    };

    for path in state.take_startup_recovery_paths() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let display = path.to_string_lossy().into_owned();
        match open_document(&state, path) {
            Ok(tab) => {
                report.restored_tab_ids.push(tab);
                report.restored_paths.push(display);
            }
            Err(err) => {
                eprintln!("warn: could not restore {display:?}: {err}");
                report.failed_paths.push(display);
            }
        }
    }

    state.persist_runtime_state();
    Ok(report)
}

#[tauri::command]
pub async fn dismiss_recovery_session(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.startup_recovery_paths.write().clear();
    Ok(())
}

/// Read `path` into a new tab and record it as recently opened.
fn open_document(state: &AppState, path: PathBuf) -> Result<TabId, lantern_io::IoError> {
    let doc = lantern_io::read_bookmark_file(&path)?;
    let tab = state.add_document(doc);
    state.push_recent_file(path);
    Ok(tab)
}

fn tab_info(id: TabId, doc: &Document) -> TabInfo {
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
