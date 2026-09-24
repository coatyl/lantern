//! Tauri shell for Lantern: the IPC layer between the React UI and
//! `lantern-core` / `lantern-io`.  `main.rs` only calls [`run`].

mod commands;
mod error;
mod state;
#[cfg(test)]
mod test_support;
mod types;

use std::path::PathBuf;

use lantern_io::Settings;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = load_app_state();

    // Seed the built-in rule sets without touching user edits.  Not fatal:
    // `run_pass` falls back to the in-memory built-ins.
    if let Err(err) = lantern_io::rulestore::seed_builtins_if_absent(&app_state.rules_dir) {
        eprintln!(
            "warn: could not seed rule sets into {}: {err}",
            app_state.rules_dir.display()
        );
    }
    app_state.mark_session_started();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())
        .expect("error while building Lantern")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app_handle.state::<AppState>().mark_session_closed();
            }
        });
}

/// Load `settings.toml` (defaults if missing or unreadable) from the settings
/// directory and pick up the documents to offer for recovery if the previous
/// session ended uncleanly.  Rule sets live in `rules/` next to it.
fn load_app_state() -> AppState {
    let settings_dir = resolve_settings_dir();
    if let Err(err) = std::fs::create_dir_all(&settings_dir) {
        eprintln!(
            "warn: could not create settings dir {}: {err}",
            settings_dir.display()
        );
    }
    let settings_path = settings_dir.join("settings.toml");

    let settings = if settings_path.exists() {
        lantern_io::read_settings(&settings_path).unwrap_or_else(|e| {
            eprintln!("warn: could not read settings.toml: {e}");
            Settings::default()
        })
    } else {
        Settings::default()
    };

    let recovery_paths = if settings.crash_recovery_enabled && settings.session_was_running {
        settings.recoverable_documents.clone()
    } else {
        Vec::new()
    };

    AppState::new(
        settings_path,
        settings_dir.join("rules"),
        settings,
        recovery_paths,
    )
}

/// The settings directory, in order of precedence:
///
/// 1. `LANTERN_SETTINGS_DIR`, if set and non-empty (tests, portable zip).
/// 2. The executable's directory, if a `settings.toml` sits next to it
///    (portable install).
/// 3. The platform config directory: `%APPDATA%\Lantern` on Windows,
///    `$XDG_CONFIG_HOME/lantern` or `~/.config/lantern` on Linux,
///    `~/Library/Application Support/Lantern` on macOS.
///
/// `lantern_io::log_path` applies the same rules for the log file.
fn resolve_settings_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("LANTERN_SETTINGS_DIR") {
        return PathBuf::from(dir);
    }

    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if exe_dir.join("settings.toml").exists() {
            return exe_dir;
        }
    }

    platform_config_dir()
}

fn platform_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(appdata) = non_empty_env("APPDATA") {
        return PathBuf::from(appdata).join("Lantern");
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = non_empty_env("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("lantern");
        }
        if let Some(home) = non_empty_env("HOME") {
            return PathBuf::from(home).join(".config").join("lantern");
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = non_empty_env("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Lantern");
    }

    // Last resort, so the app still starts.
    PathBuf::from(".")
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
