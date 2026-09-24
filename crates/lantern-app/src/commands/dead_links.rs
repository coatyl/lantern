//! Dead-link checker: the app's only network access.
//!
//! Compiled only with the `checker` feature so the offline build links no
//! networking code at all.

use lantern_core::model::node::{Folder, Node};

use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Probe every bookmark in `tab` and report one entry per bookmark.
///
/// Refused unless the user has opted in under Settings.
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

    // Copy what we need so no lock is held across the network awaits.
    let bookmarks = state.read_doc(tab, |doc| {
        let mut out = Vec::new();
        collect_bookmarks(&doc.root, &mut out);
        Ok(out)
    })?;
    let urls: Vec<String> = bookmarks.iter().map(|(_, url, _)| url.clone()).collect();
    let results = lantern_net::check_links(&urls, &lantern_net::CheckOptions::default()).await;

    let total_bookmarks = bookmarks.len();
    let mut probed = 0;
    let entries = bookmarks
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

/// `(id, url, title)` of every bookmark below `folder`, in document order.
fn collect_bookmarks(folder: &Folder, out: &mut Vec<(NodeId, String, String)>) {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) => out.push((b.id, b.url.as_str().to_owned(), b.title.clone())),
            Node::Folder(f) => collect_bookmarks(f, out),
            Node::Separator(_) => {}
        }
    }
}
