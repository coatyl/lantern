//! User preferences and the recent-files list.

use crate::error::CommandResult;
use crate::state::AppState;
use crate::types::AppSettings;

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> CommandResult<AppSettings> {
    let s = state.settings.read();
    Ok(AppSettings {
        theme: s.theme.clone().into(),
        dead_link_checker_opt_in: s.dead_link_checker_opt_in,
        recent_files_max: s.recent_files_max as u32,
        crash_recovery_enabled: s.crash_recovery_enabled,
        list_density: s.list_density.into(),
        settings_path: state.settings_path.to_string_lossy().into_owned(),
        rules_dir: state.rules_dir.to_string_lossy().into_owned(),
    })
}

/// Save the preferences in `settings` and return what was stored.  The
/// read-only path fields are ignored.
#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
    state: tauri::State<'_, AppState>,
) -> CommandResult<AppSettings> {
    {
        let mut s = state.settings.write();
        s.theme = settings.theme.into();
        s.dead_link_checker_opt_in = settings.dead_link_checker_opt_in;
        s.recent_files_max = settings.recent_files_max as usize;
        s.crash_recovery_enabled = settings.crash_recovery_enabled;
        s.list_density = settings.list_density.into();
    }
    state.persist_runtime_state();
    get_settings(state).await
}

/// Recently opened file paths, most recent first.
#[tauri::command]
pub async fn list_recent_files(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    Ok(state
        .recent_files
        .read()
        .iter()
        .rev()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

#[tauri::command]
pub async fn clear_recent_files(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    state.recent_files.write().clear();
    state.persist_runtime_state();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{block_on, Harness};
    use crate::types::ThemeSetting;

    #[test]
    fn update_settings_persists_preferences() {
        let h = Harness::new();
        let mut settings = block_on(get_settings(h.state())).unwrap();
        settings.theme = ThemeSetting::Dark;
        settings.dead_link_checker_opt_in = true;

        let stored = block_on(update_settings(settings, h.state())).unwrap();
        assert_eq!(stored.theme, ThemeSetting::Dark);

        let on_disk = lantern_io::read_settings(&h.state().settings_path).unwrap();
        assert_eq!(on_disk.theme, lantern_io::Theme::Dark);
        assert!(on_disk.dead_link_checker_opt_in);
    }
}
