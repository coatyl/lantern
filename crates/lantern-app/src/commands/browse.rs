//! Tree and list panes: folder hierarchy and folder contents.

use lantern_core::model::node::{Folder, Node};

use super::filter::item_passes_filter;
use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// The whole folder tree of a tab, used by the merge picker.
#[tauri::command]
pub async fn get_tree(tab: TabId, state: tauri::State<'_, AppState>) -> CommandResult<TreeView> {
    state.read_doc(tab, |doc| {
        Ok(TreeView {
            root: folder_to_tree_node(&doc.root),
        })
    })
}

/// The folders directly under the document root.  The tree pane loads deeper
/// levels on expand through [`get_tree_children`].
#[tauri::command]
pub async fn get_tree_root(
    tab_id: TabId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<TreeNodeLazy>> {
    state.read_doc(tab_id, |doc| Ok(lazy_children(&doc.root)))
}

/// The folders directly under `parent_id`.
#[tauri::command]
pub async fn get_tree_children(
    tab_id: TabId,
    parent_id: NodeId,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<TreeNodeLazy>> {
    state.read_doc(tab_id, |doc| {
        Ok(lazy_children(folder_or_err(&doc.root, parent_id)?))
    })
}

/// One page of the items inside `folder_id` (0 = document root) for the list
/// pane.  Omit `offset` and `limit` to get everything.
///
/// `filter` applies every axis except depth, which only means something in a
/// recursive search.
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
    let mut items = state.read_doc(tab, |doc| {
        let folder = if folder_id == 0 {
            &doc.root
        } else {
            folder_or_err(&doc.root, folder_id)?
        };
        Ok(folder
            .children
            .iter()
            .map(node_to_item)
            .filter(|item| {
                filter
                    .as_ref()
                    .map_or(true, |f| item_passes_filter(item, None, f))
            })
            .collect::<Vec<_>>())
    })?;

    if let Some(spec) = sort {
        sort_items(&mut items, &spec);
    }

    let total = items.len();
    let start = offset.unwrap_or(0).min(total);
    let end = limit.map_or(total, |lim| start.saturating_add(lim).min(total));
    items.truncate(end);
    items.drain(..start);

    Ok(ItemPage { items, total })
}

pub(super) fn find_folder(folder: &Folder, id: NodeId) -> Option<&Folder> {
    if folder.id == id {
        return Some(folder);
    }
    folder.children.iter().find_map(|child| match child {
        Node::Folder(f) => find_folder(f, id),
        _ => None,
    })
}

pub(super) fn folder_or_err(root: &Folder, id: NodeId) -> CommandResult<&Folder> {
    find_folder(root, id).ok_or_else(|| UiError::InvalidOperation(format!("folder {id} not found")))
}

pub(super) fn node_to_item(node: &Node) -> FolderItem {
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

/// Host of `raw` without a leading `www.`.
fn extract_domain(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    url.host_str()
        .map(|h| h.trim_start_matches("www.").to_owned())
}

fn folder_to_tree_node(folder: &Folder) -> TreeNode {
    TreeNode {
        id: folder.id,
        name: folder.name.clone(),
        children: folder
            .children
            .iter()
            .filter_map(Node::as_folder)
            .map(folder_to_tree_node)
            .collect(),
    }
}

/// Child folders of `parent`; `has_children` says whether a row gets an
/// expand chevron.
fn lazy_children(parent: &Folder) -> Vec<TreeNodeLazy> {
    parent
        .children
        .iter()
        .filter_map(Node::as_folder)
        .map(|f| TreeNodeLazy {
            id: f.id,
            name: f.name.clone(),
            has_children: f.children.iter().any(|c| matches!(c, Node::Folder(_))),
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, bookmark, doc_with, folder, Harness};

    /// Root 1 holds "Top A" (2) > "Nested A1" (3) > bookmark, "Top B" (5) with
    /// only a bookmark, and a loose bookmark that the tree must skip.
    fn open_tree(h: &Harness) -> TabId {
        h.open(doc_with(folder(
            1,
            "Bookmarks",
            vec![
                Node::Folder(folder(
                    2,
                    "Top A",
                    vec![Node::Folder(folder(
                        3,
                        "Nested A1",
                        vec![bookmark(4, "Inner", "https://a.example/")],
                    ))],
                )),
                Node::Folder(folder(
                    5,
                    "Top B",
                    vec![bookmark(6, "Beep", "https://b.example/")],
                )),
                bookmark(7, "Loose", "https://www.c.example/"),
            ],
        )))
    }

    fn rows(rows: &[TreeNodeLazy]) -> Vec<(NodeId, &str, bool)> {
        rows.iter()
            .map(|r| (r.id, r.name.as_str(), r.has_children))
            .collect()
    }

    #[test]
    fn tree_root_lists_top_level_folders_with_chevron_flags() {
        let h = Harness::new();
        let tab = open_tree(&h);
        let root = block_on(get_tree_root(tab, h.state())).unwrap();
        assert_eq!(rows(&root), vec![(2, "Top A", true), (5, "Top B", false)]);
    }

    #[test]
    fn tree_children_lists_immediate_folders_only() {
        let h = Harness::new();
        let tab = open_tree(&h);
        let a = block_on(get_tree_children(tab, 2, h.state())).unwrap();
        assert_eq!(rows(&a), vec![(3, "Nested A1", false)]);
        let b = block_on(get_tree_children(tab, 5, h.state())).unwrap();
        assert!(b.is_empty());
    }

    #[test]
    fn unknown_tab_or_folder_is_an_error() {
        let h = Harness::new();
        let tab = open_tree(&h);
        assert!(matches!(
            block_on(get_tree_root(999, h.state())),
            Err(UiError::TabNotFound(999))
        ));
        assert!(matches!(
            block_on(get_tree_children(999, 1, h.state())),
            Err(UiError::TabNotFound(999))
        ));
        assert!(matches!(
            block_on(get_tree_children(tab, 99_999, h.state())),
            Err(UiError::InvalidOperation(_))
        ));
    }

    #[test]
    fn folder_items_are_sorted_then_paged() {
        let h = Harness::new();
        let tab = open_tree(&h);
        let sort = || {
            Some(SortSpec {
                column: SortColumn::Title,
                descending: true,
            })
        };
        let page = |offset, limit| {
            block_on(get_folder_items(
                tab,
                0,
                sort(),
                offset,
                limit,
                None,
                h.state(),
            ))
            .unwrap()
        };

        let all = page(None, None);
        assert_eq!(all.total, 3);
        let titles: Vec<&str> = all.items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["Top B", "Top A", "Loose"]);
        assert_eq!(all.items[2].domain.as_deref(), Some("c.example"));

        let second = page(Some(1), Some(1));
        assert_eq!((second.total, second.items.len()), (3, 1));
        assert_eq!(second.items[0].title, "Top A");
        assert!(page(Some(5), Some(2)).items.is_empty());
    }
}
