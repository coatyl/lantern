//! Settings panes backed by static or compile-time data: keyboard shortcuts,
//! the log viewer and the About pane.

use std::path::Path;

use crate::error::{CommandResult, UiError};
use crate::types::{BuildInfo, LogEntry, LogLevel, ShortcutBinding};

/// Number of log lines the log viewer shows.
const LOG_LINES: usize = 200;

/// The keyboard shortcuts the UI binds, for the Keyboard pane.
#[tauri::command]
pub fn list_shortcuts() -> CommandResult<Vec<ShortcutBinding>> {
    // Keep in step with the key handlers in ui/src/App.tsx, ListPane and
    // ReviewPane: this list is documentation, not configuration.
    const SHORTCUTS: &[(&str, &str, &str, &str)] = &[
        ("open_file", "Open files", "Ctrl+O", "File"),
        ("save_copy", "Save a clean copy", "Ctrl+S", "File"),
        ("save_copy_as", "Save copy as…", "Ctrl+Shift+S", "File"),
        ("close_tab", "Close tab", "Ctrl+W", "File"),
        ("close_all_tabs", "Close all tabs", "Ctrl+Shift+W", "File"),
        (
            "next_tab",
            "Next / previous tab",
            "Ctrl+Tab / Ctrl+Shift+Tab",
            "View",
        ),
        ("library", "Library", "Ctrl+Shift+L", "View"),
        ("search", "Search this document", "Ctrl+F", "View"),
        ("undo", "Undo", "Ctrl+Z", "Edit"),
        ("redo", "Redo", "Ctrl+Y / Ctrl+Shift+Z", "Edit"),
        ("rename", "Rename", "F2", "Edit"),
        ("delete", "Delete", "Delete", "Edit"),
        ("run_pass", "Run the selected rule set", "Ctrl+R", "Tools"),
        (
            "apply_review",
            "Apply reviewed changes",
            "Ctrl+Enter",
            "Tools",
        ),
        ("command_palette", "Command palette", "Ctrl+K / ⌘K", "Tools"),
        ("compare_tabs", "Compare tabs", "Ctrl+Shift+D", "Tools"),
        ("merge_documents", "Merge documents", "Ctrl+M", "Tools"),
        ("settings", "Settings", "Ctrl+,", "Tools"),
    ];
    Ok(SHORTCUTS
        .iter()
        .map(|&(action_id, label, key_combo, category)| ShortcutBinding {
            action_id: action_id.to_owned(),
            label: label.to_owned(),
            key_combo: key_combo.to_owned(),
            category: category.to_owned(),
        })
        .collect())
}

/// The last lines of `<settings dir>/logs/lantern.log`; empty if no log has
/// been written yet.
#[tauri::command]
pub async fn get_logs() -> CommandResult<Vec<LogEntry>> {
    read_logs(&lantern_io::log_path())
}

fn read_logs(path: &Path) -> CommandResult<Vec<LogEntry>> {
    match lantern_io::read_recent(path, LOG_LINES) {
        Ok(raw) => Ok(raw
            .into_iter()
            .map(|r| LogEntry {
                timestamp: r.timestamp,
                level: LogLevel::from_token(&r.level),
                message: r.message,
            })
            .collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(UiError::Io(e.to_string())),
    }
}

/// Compile-time facts for the About pane.
#[tauri::command]
pub fn get_build_info() -> CommandResult<BuildInfo> {
    Ok(BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_flavor: if cfg!(feature = "checker") {
            "default"
        } else {
            "offline-only"
        }
        .to_string(),
        rust_version: option_env!("LANTERN_RUSTC_VERSION")
            .unwrap_or("unknown")
            .to_string(),
        git_commit: None,
        license: "Apache-2.0 OR MIT".to_string(),
        adr_index_path: "private/adrs/".to_string(),
        // `build.rs` forwards LANTERN_SIGNED, which the Windows signing job
        // sets once signtool succeeds.
        signed: matches!(option_env!("LANTERN_SIGNED"), Some("1" | "true")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shortcut_action_ids_are_unique() {
        let shortcuts = list_shortcuts().unwrap();
        let ids: HashSet<&str> = shortcuts.iter().map(|s| s.action_id.as_str()).collect();
        assert_eq!(ids.len(), shortcuts.len());
    }

    #[test]
    fn logs_are_empty_without_a_file_and_parsed_with_one() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_logs(&dir.path().join("missing.log"))
            .unwrap()
            .is_empty());

        let path = dir.path().join("lantern.log");
        std::fs::write(
            &path,
            "2026-05-05T17:23:45Z INFO startup complete\n\
             2026-05-05T17:23:46Z WARN checker disabled\n\
             2026-05-05T17:23:47Z ERROR could not parse rule set\n",
        )
        .unwrap();
        let entries = read_logs(&path).unwrap();
        let levels: Vec<&LogLevel> = entries.iter().map(|e| &e.level).collect();
        assert!(matches!(
            levels[..],
            [LogLevel::Info, LogLevel::Warn, LogLevel::Error]
        ));
        assert_eq!(entries[0].message, "startup complete");
        assert_eq!(entries[2].timestamp, "2026-05-05T17:23:47Z");
    }

    #[test]
    fn build_info_carries_the_rustc_version_from_the_build_script() {
        let info = get_build_info().unwrap();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(
            info.rust_version.starts_with("rustc "),
            "{}",
            info.rust_version
        );
    }
}
