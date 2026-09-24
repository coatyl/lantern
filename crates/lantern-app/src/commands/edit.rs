//! Structural edits and undo/redo.  Every edit pushes an undo entry.
//!
//! Folder ids of 0 mean the document root.

use crate::error::CommandResult;
use crate::state::AppState;
use crate::types::{NodeId, TabId};

#[tauri::command]
pub async fn undo(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.edit_doc(tab, |doc| Ok(doc.undo()?))
}

#[tauri::command]
pub async fn redo(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.edit_doc(tab, |doc| Ok(doc.redo()?))
}

/// Rename a bookmark (title) or folder (name).
#[tauri::command]
pub async fn rename_node(
    tab: TabId,
    node_id: NodeId,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state.edit_doc(tab, |doc| Ok(doc.rename_node(node_id, new_name)?))
}

#[tauri::command]
pub async fn delete_node(
    tab: TabId,
    node_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state.edit_doc(tab, |doc| Ok(doc.delete_node(node_id)?))
}

/// Returns the new bookmark's id.
#[tauri::command]
pub async fn create_bookmark(
    tab: TabId,
    folder_id: NodeId,
    title: String,
    url: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    state.edit_doc(tab, |doc| Ok(doc.create_bookmark(folder_id, title, url)?))
}

/// Returns the new folder's id.
#[tauri::command]
pub async fn create_folder(
    tab: TabId,
    folder_id: NodeId,
    name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    state.edit_doc(tab, |doc| Ok(doc.create_folder(folder_id, name)?))
}

/// Returns the new separator's id.
#[tauri::command]
pub async fn create_separator(
    tab: TabId,
    folder_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<NodeId> {
    state.edit_doc(tab, |doc| Ok(doc.create_separator(folder_id)?))
}

/// Move `node_id` to position `new_index` inside `new_parent_id`.
#[tauri::command]
pub async fn move_node(
    tab: TabId,
    node_id: NodeId,
    new_parent_id: NodeId,
    new_index: usize,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    state.edit_doc(tab, |doc| {
        Ok(doc.move_node(node_id, new_parent_id, new_index)?)
    })
}
