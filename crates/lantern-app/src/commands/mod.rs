//! Tauri command handlers, grouped by domain.
//!
//! Commands are thin: they find the tab, call into `lantern-core` /
//! `lantern-io`, and convert the result into the view models in
//! [`crate::types`].  The UI reaches them only through `ui/src/ipc/index.ts`.

mod about;
mod browse;
#[cfg(feature = "checker")]
mod dead_links;
mod documents;
mod edit;
mod filter;
mod rule_sets;
mod sanitize;
mod search;
mod settings;
mod tabs;

use tauri::ipc::Invoke;
use tauri::Runtime;

/// The invoke handler for every command the UI can call.
pub(crate) fn handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        tabs::open_file,
        tabs::close_tab,
        tabs::list_tabs,
        tabs::get_recovery_state,
        tabs::restore_recovery_session,
        tabs::dismiss_recovery_session,
        browse::get_tree,
        browse::get_tree_root,
        browse::get_tree_children,
        browse::get_folder_items,
        search::search,
        edit::undo,
        edit::redo,
        edit::rename_node,
        edit::delete_node,
        edit::create_bookmark,
        edit::create_folder,
        edit::create_separator,
        edit::move_node,
        sanitize::run_pass,
        sanitize::apply_changeset,
        rule_sets::list_rule_sets,
        rule_sets::get_rule_set,
        rule_sets::save_rule_set,
        rule_sets::save_rule_set_with_configs,
        rule_sets::delete_rule_set,
        rule_sets::duplicate_rule_set,
        rule_sets::list_treatments,
        documents::export,
        documents::compare_tabs,
        documents::merge_documents,
        settings::get_settings,
        settings::update_settings,
        settings::list_recent_files,
        settings::clear_recent_files,
        #[cfg(feature = "checker")]
        dead_links::check_dead_links,
        about::list_shortcuts,
        about::get_logs,
        about::get_build_info,
    ]
}

#[cfg(test)]
mod tests {
    //! Contract between `ui/src/ipc/index.ts` and the registered commands.
    //!
    //! Every `invoke("command", { args })` in the wrapper is sent through the
    //! real handler on a mock runtime with the argument keys the UI uses.
    //! Tauri answers a wrong command name or argument key with a plain string
    //! error; a command that ran answers with a value or a `UiError` object.

    use std::path::Path;

    use serde_json::{json, Map, Value};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{get_ipc_response, INVOKE_KEY};
    use tauri::webview::InvokeRequest;

    use crate::test_support::Harness;

    /// `(command, argument keys)` for every `invoke` call in the wrapper.
    fn wrapper_calls() -> Vec<(String, Vec<String>)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/ipc/index.ts");
        let src = std::fs::read_to_string(&path).unwrap();
        let re =
            regex::Regex::new(r#"invoke<[^(]*>\([ \n]*"([a-z_]+)"[ \n]*(?:,[ \n]*\{([^}]*)\})?"#)
                .unwrap();
        re.captures_iter(&src)
            .map(|c| {
                let keys = c.get(2).map_or(Vec::new(), |body| {
                    body.as_str()
                        .split(',')
                        .map(|arg| arg.split(':').next().unwrap().trim().to_owned())
                        .filter(|key| !key.is_empty())
                        .collect()
                });
                (c[1].to_owned(), keys)
            })
            .collect()
    }

    /// A well-typed value for `key` that cannot touch real user data.
    fn sample(command: &str, key: &str) -> Value {
        match (command, key) {
            ("export", "scope") => json!({ "kind": "whole_document" }),
            (_, "tab" | "tabId" | "left" | "right" | "changesetId") => json!(999),
            (_, "folderId" | "parentId" | "nodeId" | "newParentId" | "newIndex") => json!(0),
            (_, "scope" | "sort" | "offset" | "limit" | "filter") => Value::Null,
            (_, "path") => json!("/nonexistent/lantern-ipc-contract.html"),
            (_, "query") => json!({ "query": "x" }),
            (_, "approvals" | "treatmentIds" | "treatments" | "picks") => json!([]),
            (_, "strategy") => json!("keep_first"),
            (_, "settings") => json!({
                "theme": "system",
                "dead_link_checker_opt_in": false,
                "recent_files_max": 10,
                "crash_recovery_enabled": true,
                "list_density": "comfortable",
                "settings_path": "",
                "rules_dir": "",
            }),
            (
                _,
                "name" | "newName" | "title" | "url" | "ruleSetName" | "source" | "mergedRootName",
            ) => json!("IPC contract"),
            _ => panic!("no sample value for `{key}` (used by `{command}`); add one"),
        }
    }

    #[test]
    fn every_wrapper_call_reaches_its_command_with_valid_arguments() {
        let calls = wrapper_calls();
        assert!(calls.len() > 30, "parsed only {} invoke calls", calls.len());

        let harness = Harness::new();
        let webview = tauri::WebviewWindowBuilder::new(&harness.app, "main", Default::default())
            .build()
            .unwrap();

        for (command, keys) in calls {
            let args: Map<String, Value> = keys
                .iter()
                .map(|key| (key.clone(), sample(&command, key)))
                .collect();
            let request = InvokeRequest {
                cmd: command.clone(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(),
                body: InvokeBody::Json(Value::Object(args)),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            };
            if let Err(err) = get_ipc_response(&webview, request) {
                assert!(
                    err.get("kind").is_some(),
                    "`{command}` rejected the wrapper's call: {err}"
                );
            }
        }
    }
}
