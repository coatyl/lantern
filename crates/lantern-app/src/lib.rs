//! Tauri application entry point (library side).
//!
//! The library crate lets integration tests import and exercise commands
//! without spinning up a full Tauri window.  `main.rs` just calls `run()`.

pub mod commands;
pub mod error;
pub mod state;
pub mod types;

use lantern_io::Settings;
use state::AppState;
use std::path::PathBuf;
use tauri::Manager;

/// Build and run the Tauri application.
///
/// Called from `main.rs`.  Also the entry point for mobile targets
/// (annotated with `#[tauri::mobile_entry_point]`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup = load_startup_state();
    let app_state = AppState::new(
        startup.settings_path,
        startup.rules_dir,
        startup.settings,
        startup.recovery_paths,
    );

    // Seed built-in rule sets on first launch (non-destructive: existing user
    // edits are preserved).  Failures are logged but not fatal; `run_pass`
    // falls back to the in-memory catalogue if the directory is unusable.
    if let Err(err) = lantern_io::rulestore::seed_builtins_if_absent(&app_state.rules_dir) {
        eprintln!(
            "warn: could not seed rule sets into {}: {err}",
            app_state.rules_dir.display()
        );
    }

    app_state.mark_session_started();

    // The dead-link checker command is the only network-touching command in
    // the app.  In v0.0.6 it was moved behind the `checker` Cargo feature so
    // an offline-only flavor (`--no-default-features`) can be produced with
    // zero networking symbols.  We branch on the feature here rather than
    // inside `generate_handler!` because the macro re-tokenises its inputs in
    // ways that don't always cooperate with `#[cfg]` on a single arm.
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state);

    #[cfg(feature = "checker")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::open_file,
        commands::close_tab,
        commands::list_tabs,
        commands::get_tree,
        commands::get_tree_children,
        commands::get_tree_root,
        commands::get_folder_items,
        commands::search,
        commands::run_pass,
        commands::apply_changeset,
        commands::undo,
        commands::redo,
        commands::export,
        commands::rename_node,
        commands::delete_node,
        commands::create_bookmark,
        commands::create_folder,
        commands::create_separator,
        commands::move_node,
        commands::list_recent_files,
        commands::clear_recent_files,
        commands::get_recovery_state,
        commands::restore_recovery_session,
        commands::dismiss_recovery_session,
        commands::get_settings,
        commands::update_settings,
        commands::list_rule_sets,
        commands::get_rule_set,
        commands::save_rule_set,
        commands::save_rule_set_with_configs,
        commands::delete_rule_set,
        commands::duplicate_rule_set,
        commands::list_treatments,
        commands::check_dead_links,
        commands::compare_tabs,
        commands::merge_documents,
        commands::list_shortcuts,
        commands::get_logs,
        commands::get_build_info,
    ]);

    #[cfg(not(feature = "checker"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::open_file,
        commands::close_tab,
        commands::list_tabs,
        commands::get_tree,
        commands::get_tree_children,
        commands::get_tree_root,
        commands::get_folder_items,
        commands::search,
        commands::run_pass,
        commands::apply_changeset,
        commands::undo,
        commands::redo,
        commands::export,
        commands::rename_node,
        commands::delete_node,
        commands::create_bookmark,
        commands::create_folder,
        commands::create_separator,
        commands::move_node,
        commands::list_recent_files,
        commands::clear_recent_files,
        commands::get_recovery_state,
        commands::restore_recovery_session,
        commands::dismiss_recovery_session,
        commands::get_settings,
        commands::update_settings,
        commands::list_rule_sets,
        commands::get_rule_set,
        commands::save_rule_set,
        commands::save_rule_set_with_configs,
        commands::delete_rule_set,
        commands::duplicate_rule_set,
        commands::list_treatments,
        commands::compare_tabs,
        commands::merge_documents,
        commands::list_shortcuts,
        commands::get_logs,
        commands::get_build_info,
    ]);

    builder
        .build(tauri::generate_context!())
        .expect("error while building Lantern")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app_handle.state::<AppState>().mark_session_closed();
            }
        });
}

// ---------------------------------------------------------------------------
// Startup helpers
// ---------------------------------------------------------------------------

struct StartupState {
    settings_path: PathBuf,
    rules_dir: PathBuf,
    settings: Settings,
    recovery_paths: Vec<PathBuf>,
}

/// Resolve the settings directory, load `settings.toml` (or defaults if
/// missing), and pick up any recoverable-document paths from an unclean prior
/// session.
///
/// # Path resolution
///
/// 1. If `LANTERN_SETTINGS_DIR` is set (test / portable override), use it.
/// 2. If a `settings.toml` sits next to the executable (portable install),
///    use that directory.
/// 3. Otherwise: `%APPDATA%\Lantern\` on Windows, `$XDG_CONFIG_HOME/lantern/`
///    on Linux, `~/Library/Application Support/Lantern/` on macOS.
///
/// The directory is created on demand.  Rule sets live in a `rules/`
/// sub-directory of the same parent.
fn load_startup_state() -> StartupState {
    let settings_dir = resolve_settings_dir();
    if let Err(err) = std::fs::create_dir_all(&settings_dir) {
        eprintln!(
            "warn: could not create settings dir {}: {err}",
            settings_dir.display()
        );
    }

    let settings_path = settings_dir.join("settings.toml");
    let rules_dir = settings_dir.join("rules");

    let settings = if settings_path.exists() {
        match lantern_io::read_settings(&settings_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warn: could not read settings.toml: {e}");
                Settings::default()
            }
        }
    } else {
        Settings::default()
    };

    let recovery_paths = if settings.crash_recovery_enabled && settings.session_was_running {
        settings.recoverable_documents.clone()
    } else {
        Vec::new()
    };

    StartupState {
        settings_path,
        rules_dir,
        settings,
        recovery_paths,
    }
}

/// Resolve the absolute settings *directory* (not the file) for the current
/// build flavor.  Factored out of [`load_startup_state`] so tests and the
/// `get_settings` command can show the user where preferences live.
pub fn resolve_settings_dir() -> PathBuf {
    // Explicit override wins.  Used by CI and the portable zip.
    if let Ok(override_dir) = std::env::var("LANTERN_SETTINGS_DIR") {
        if !override_dir.is_empty() {
            return PathBuf::from(override_dir);
        }
    }

    // Portable install: settings.toml next to lantern.exe.
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if exe_dir.join("settings.toml").exists() {
            return exe_dir;
        }
    }

    // Installed build: platform config directory.
    platform_config_dir()
}

/// `%APPDATA%\Lantern` on Windows, equivalent XDG / macOS paths elsewhere.
fn platform_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("Lantern");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("lantern");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home).join(".config").join("lantern");
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Lantern");
            }
        }
    }

    // Ultimate fallback: the current working directory.  Not ideal for a
    // long-running daemon but fine for a hand-launched binary and guarantees
    // the app still starts.
    PathBuf::from(".")
}
