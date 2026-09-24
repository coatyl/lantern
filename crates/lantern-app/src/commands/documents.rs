//! Whole-document operations: export, compare two tabs, merge tabs.

use std::path::PathBuf;

use lantern_core::emit::EmitOptions;
use lantern_core::model::document::{Document, DocumentStats};
use lantern_core::model::merge::{build_merged_document, MergePick, MergePlan};
use lantern_core::model::node::Folder;

use super::browse::folder_or_err;
use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Write the document, or one folder of it, as a Netscape bookmark file.
///
/// `lantern_io` refuses to overwrite the file the document was opened from;
/// a subtree export keeps the source path so that guard still applies.
#[tauri::command]
pub async fn export(
    tab: TabId,
    scope: ExportScope,
    path: PathBuf,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ExportReport> {
    let opts = EmitOptions::default();
    state.read_doc(tab, |doc| match scope {
        ExportScope::WholeDocument => Ok(lantern_io::write_bookmark_file(&path, doc, &opts)?),
        ExportScope::Subtree { root_id } => {
            let subtree = subtree_document(doc, folder_or_err(&doc.root, root_id)?);
            Ok(lantern_io::write_bookmark_file(&path, &subtree, &opts)?)
        }
    })?;
    let bytes_written = std::fs::metadata(&path)
        .map_err(|e| UiError::Io(e.to_string()))?
        .len() as usize;

    Ok(ExportReport {
        path: path.to_string_lossy().into_owned(),
        bytes_written,
    })
}

/// Compare two tabs by URL: bookmarks added, removed and modified from
/// `left` to `right`.  Folder structure is ignored.
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
    let title = |d: &Document| d.header.title.clone().unwrap_or_else(|| "Bookmarks".into());

    Ok(DocDiffReport {
        left_title: title(l),
        right_title: title(r),
        added: diff.added.into_iter().map(Into::into).collect(),
        removed: diff.removed.into_iter().map(Into::into).collect(),
        modified: diff.modified.into_iter().map(Into::into).collect(),
    })
}

/// Build a new document from subtrees picked across open tabs, open it in a
/// new tab and return that tab's id.  The source documents are not touched.
#[tauri::command]
pub async fn merge_documents(
    picks: Vec<MergePickRequest>,
    strategy: ConflictStrategyView,
    merged_root_name: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<TabId> {
    let merged = {
        let docs = state.documents.read();
        // The core addresses sources by index; each distinct tab gets one.
        let mut tabs: Vec<TabId> = Vec::new();
        let mut sources: Vec<&Document> = Vec::new();
        let mut core_picks = Vec::with_capacity(picks.len());
        for pick in &picks {
            let tab = pick.source_tab_id;
            let index = match tabs.iter().position(|t| *t == tab) {
                Some(index) => index,
                None => {
                    sources.push(docs.get(&tab).ok_or(UiError::TabNotFound(tab))?);
                    tabs.push(tab);
                    tabs.len() - 1
                }
            };
            core_picks.push(MergePick {
                source_doc_id: index,
                root_node_id: pick.root_node_id,
            });
        }
        let plan = MergePlan {
            picks: core_picks,
            strategy: strategy.into(),
            merged_root_name,
        };
        build_merged_document(&sources, &plan)
            .map_err(|e| UiError::InvalidOperation(e.to_string()))?
    };

    let tab = state.add_document(merged);
    state.persist_runtime_state();
    Ok(tab)
}

/// A standalone document whose root is a copy of `folder`, with the source's
/// header and path and no undo history.
fn subtree_document(source: &Document, folder: &Folder) -> Document {
    let root = folder.clone();
    Document {
        id: source.id,
        path: source.path.clone(),
        stats: DocumentStats::from_root(&root),
        root,
        header: source.header.clone(),
        id_gen: source.id_gen.clone(),
        open_timestamp: source.open_timestamp,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        dirty: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, bookmark, doc_with, folder, Harness};

    fn pick(source_tab_id: TabId, root_node_id: NodeId) -> MergePickRequest {
        MergePickRequest {
            source_tab_id,
            root_node_id,
        }
    }

    fn merge(
        h: &Harness,
        picks: Vec<MergePickRequest>,
        strategy: ConflictStrategyView,
    ) -> CommandResult<TabId> {
        block_on(merge_documents(picks, strategy, "Merged".into(), h.state()))
    }

    #[test]
    fn merge_combines_picks_from_several_tabs_into_a_new_tab() {
        let h = Harness::new();
        let a = h.open(doc_with(folder(
            1,
            "A",
            vec![
                bookmark(2, "a1", "https://a1.example/"),
                bookmark(3, "a2", "https://a2.example/"),
            ],
        )));
        let b = h.open(doc_with(folder(
            1,
            "B",
            vec![bookmark(2, "b", "https://b.example/")],
        )));

        let merged = merge(
            &h,
            vec![pick(a, 2), pick(b, 2), pick(a, 3)],
            ConflictStrategyView::KeepBoth,
        )
        .unwrap();

        let state = h.state();
        let docs = state.documents.read();
        let doc = &docs[&merged];
        assert!(merged != a && merged != b);
        assert_eq!(doc.root.name, "Merged");
        let titles: Vec<String> = doc
            .root
            .children
            .iter()
            .map(|n| n.as_bookmark().unwrap().title.clone())
            .collect();
        assert_eq!(titles, vec!["a1", "b", "a2"]);
        assert!(doc.path.is_none());
        assert_eq!(docs[&a].root.children.len(), 2, "sources are untouched");
    }

    #[test]
    fn keep_first_drops_a_later_duplicate_url() {
        let h = Harness::new();
        let a = h.open(doc_with(folder(
            1,
            "A",
            vec![bookmark(2, "from A", "https://same.example/")],
        )));
        let b = h.open(doc_with(folder(
            1,
            "B",
            vec![bookmark(2, "from B", "https://same.example/")],
        )));

        let merged = merge(
            &h,
            vec![pick(a, 2), pick(b, 2)],
            ConflictStrategyView::KeepFirst,
        )
        .unwrap();

        let state = h.state();
        let docs = state.documents.read();
        assert_eq!(docs[&merged].root.children.len(), 1);
    }

    #[test]
    fn merge_rejects_empty_plans_and_unknown_tabs() {
        let h = Harness::new();
        assert!(matches!(
            merge(&h, vec![], ConflictStrategyView::KeepFirst),
            Err(UiError::InvalidOperation(_))
        ));
        assert!(matches!(
            merge(&h, vec![pick(999, 1)], ConflictStrategyView::KeepFirst),
            Err(UiError::TabNotFound(999))
        ));
    }

    #[test]
    fn export_writes_a_subtree_and_reports_its_size() {
        let h = Harness::new();
        let tab = h.open(doc_with(folder(
            1,
            "Bookmarks",
            vec![
                lantern_core::model::node::Node::Folder(folder(
                    2,
                    "Keep",
                    vec![bookmark(3, "Inside", "https://inside.example/")],
                )),
                bookmark(4, "Outside", "https://outside.example/"),
            ],
        )));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.html");

        let report = block_on(export(
            tab,
            ExportScope::Subtree { root_id: 2 },
            path.clone(),
            h.state(),
        ))
        .unwrap();

        let html = std::fs::read_to_string(&path).unwrap();
        assert_eq!(report.bytes_written, html.len());
        assert!(html.contains("inside.example"));
        assert!(!html.contains("outside.example"));
    }
}
