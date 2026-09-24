//! Rule-set passes: propose a change set, then apply the approved part.

use std::collections::{HashMap, HashSet};

use lantern_core::model::document::{Document, Field};
use lantern_core::model::node::{Folder, Node};
use lantern_core::sanitize::diff::char_diff;
use lantern_core::sanitize::pass::PassTarget;
use lantern_core::sanitize::treatment::{Change, ChangeKind, ChangeSet};
use lantern_io::rulestore;

use crate::error::{CommandResult, UiError};
use crate::state::AppState;
use crate::types::*;

/// Run a rule set over the document (or over the folder `scope` and
/// everything below it) and return the proposed changes for review.
///
/// The change set stays pending until `apply_changeset` or until its tab
/// closes.  A built-in rule set works even if its file is missing.
#[tauri::command]
pub async fn run_pass(
    tab: TabId,
    rule_set_name: String,
    scope: Option<NodeId>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ChangeSetPreview> {
    let rule_set = rulestore::load_rule_set(&state.rules_dir, &rule_set_name)
        .map_err(|e| UiError::InvalidOperation(e.to_string()))?;
    let target = scope.map_or(PassTarget::AllNodes, PassTarget::Subtree);

    let cs_id = state.alloc_changeset_id();
    let (changeset, preview) = state.read_doc(tab, |doc| {
        let changeset = lantern_core::sanitize::run_pass(doc, &rule_set, target);
        let preview = preview(cs_id, &changeset, doc);
        Ok((changeset, preview))
    })?;
    state
        .pending_changesets
        .write()
        .insert(cs_id, (tab, changeset));
    Ok(preview)
}

/// Apply a pending change set to the tab it was computed for.
///
/// `approvals[i]` overrides the approval of `ChangeSetPreview.changes[i]`;
/// changes past the end of `approvals` keep their initial approval.
#[tauri::command]
pub async fn apply_changeset(
    tab: TabId,
    changeset_id: ChangeSetId,
    approvals: Vec<bool>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ApplyReport> {
    let mut cs = {
        let mut pending = state.pending_changesets.write();
        match pending.get(&changeset_id) {
            Some((owner, _)) if *owner == tab => pending.remove(&changeset_id).map(|(_, cs)| cs),
            _ => None,
        }
    }
    .ok_or(UiError::ChangeSetNotFound(changeset_id))?;

    for (change, approved) in cs.changes.iter_mut().zip(approvals) {
        change.approved = approved;
    }
    let applied_count = cs.changes.iter().filter(|c| c.approved).count();
    let skipped_count = cs.changes.len() - applied_count;

    state.edit_doc(tab, |doc| {
        doc.apply(&cs).map_err(|e| UiError::Internal(e.to_string()))
    })?;

    Ok(ApplyReport {
        applied_count,
        skipped_count,
    })
}

fn preview(cs_id: ChangeSetId, cs: &ChangeSet, doc: &Document) -> ChangeSetPreview {
    let wanted: HashSet<NodeId> = cs.changes.iter().map(|c| c.node_id).collect();
    let contexts = node_contexts(&doc.root, &wanted);
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

/// What the review surface shows about the node a change touches.
struct NodeContext {
    title: String,
    url: Option<String>,
    /// Folder names from below the root down to the node's parent.
    location: Vec<String>,
}

/// Context for every node in `wanted`, gathered in one walk.
fn node_contexts(root: &Folder, wanted: &HashSet<NodeId>) -> HashMap<NodeId, NodeContext> {
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
                let location = path.clone();
                out.insert(
                    child.id(),
                    NodeContext {
                        title,
                        url,
                        location,
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
    walk(root, &mut Vec::new(), wanted, &mut out);
    out
}

/// The `ChangeEntry.field` value the UI groups changes by.
fn field_name(field: Field) -> &'static str {
    match field {
        Field::Url => "url",
        Field::Title => "title",
        Field::FolderName => "folder_name",
    }
}

fn change_to_entry(index: usize, c: &Change, ctx: Option<&NodeContext>) -> ChangeEntry {
    let (field, before, after) = match &c.kind {
        ChangeKind::SetField {
            field,
            before,
            after,
        } => (field_name(*field).to_owned(), before.clone(), after.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, bookmark, doc_with, folder, Harness};

    /// A tracked URL two folders deep, below a folder name with stray spaces.
    fn open_tracked(h: &Harness) -> TabId {
        h.open(doc_with(folder(
            1,
            "Bookmarks",
            vec![Node::Folder(folder(
                2,
                "Work  ",
                vec![Node::Folder(folder(
                    3,
                    "Docs",
                    vec![bookmark(4, "Spec", "https://example.com/?utm_source=x")],
                ))],
            ))],
        )))
    }

    #[test]
    fn preview_names_each_node_its_field_and_folder_path() {
        let h = Harness::new();
        let tab = open_tracked(&h);
        let preview = block_on(run_pass(tab, "Minimal clean".into(), None, h.state())).unwrap();
        let entry = |id| preview.changes.iter().find(|c| c.node_id == id).unwrap();

        let url = entry(4);
        assert_eq!(url.field, "url");
        assert_eq!(url.node_title, "Spec");
        assert_eq!(
            url.node_url.as_deref(),
            Some("https://example.com/?utm_source=x")
        );
        assert_eq!(url.location, vec!["Work  ", "Docs"]);

        let folder_name = entry(2);
        assert_eq!(folder_name.field, "folder_name");
        assert_eq!(folder_name.node_title, "Work  ");
        assert_eq!(folder_name.node_url, None);
        assert!(folder_name.location.is_empty());
    }

    #[test]
    fn a_change_set_applies_only_to_its_own_tab_and_only_once() {
        let h = Harness::new();
        let tab = open_tracked(&h);
        let other = open_tracked(&h);
        let preview = block_on(run_pass(tab, "Minimal clean".into(), None, h.state())).unwrap();
        assert_eq!(preview.changes.len(), 2);
        let id = preview.changeset_id;
        let apply = |on| block_on(apply_changeset(on, id, vec![true, false], h.state()));

        assert!(matches!(apply(other), Err(UiError::ChangeSetNotFound(_))));
        let report = apply(tab).unwrap();
        assert_eq!((report.applied_count, report.skipped_count), (1, 1));
        assert!(matches!(apply(tab), Err(UiError::ChangeSetNotFound(_))));
    }
}
