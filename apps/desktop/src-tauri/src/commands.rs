//! Tauri IPC commands. Names map 1:1 to `COMMANDS` in `@openflow/core`.

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult};
use crate::llm::LlmTestResult;
use crate::models::ModelInfoDto;
use crate::permissions::{self, PermissionsState};
use crate::pipeline::{Job, PipelineState, TranscriptionResult};
use crate::profiles::LlmProfile;
use crate::settings::Settings;
use crate::state::AppState;
use crate::{shortcuts, tray};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub config_path: String,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let previous = state.settings.get();

    // Cap per-mode hotkeys before persisting (07 §4); the optimistic UI reverts
    // on the error.
    let mode_hotkey_count = settings
        .modes
        .iter()
        .filter(|m| m.hotkey.as_deref().is_some_and(|h| !h.is_empty()))
        .count();
    if mode_hotkey_count > 5 {
        return Err(AppError::Settings(
            "At most 5 modes can have their own hotkey. Remove one first.".into(),
        ));
    }

    let saved = state.settings.set(settings)?;

    let mode_hotkeys = |s: &Settings| -> Vec<(String, String)> {
        s.modes
            .iter()
            .filter_map(|m| m.hotkey.as_deref().map(|h| (m.id.clone(), h.to_string())))
            .collect()
    };
    let hotkeys_changed = previous.dictation_hotkey != saved.dictation_hotkey
        || previous.refine_hotkey != saved.refine_hotkey
        || previous.polish_hotkey != saved.polish_hotkey
        || mode_hotkeys(&previous) != mode_hotkeys(&saved);
    if hotkeys_changed {
        if let Err(message) = shortcuts::apply(&app, &saved) {
            // Roll every hotkey — the three globals AND each mode hotkey — back
            // to the last working set as one atomic unit.
            let mut reverted = saved.clone();
            reverted.dictation_hotkey = previous.dictation_hotkey.clone();
            reverted.refine_hotkey = previous.refine_hotkey.clone();
            reverted.polish_hotkey = previous.polish_hotkey.clone();
            for mode in &mut reverted.modes {
                mode.hotkey = previous
                    .modes
                    .iter()
                    .find(|m| m.id == mode.id)
                    .and_then(|m| m.hotkey.clone());
            }
            let restored = state.settings.set(reverted)?;
            let _ = shortcuts::apply(&app, &restored);
            let _ = app.emit("settings-changed", &restored);
            return Err(AppError::Settings(message));
        }
    }

    if previous.launch_at_login != saved.launch_at_login {
        sync_autostart(&app, saved.launch_at_login);
    }

    if let Err(err) = tray::rebuild_menu(&app) {
        log::warn!("tray rebuild failed: {err}");
    }
    let _ = app.emit("settings-changed", &saved);
    Ok(saved)
}

fn sync_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = result {
        log::warn!("autostart sync failed: {err}");
    }
}

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Vec<ModelInfoDto> {
    state.models.list()
}

#[tauri::command]
pub fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> AppResult<()> {
    let models = state.models.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = models.download(&app, &model_id).await {
            log::warn!("model download failed: {err}");
        }
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_model_download(state: State<'_, AppState>, model_id: String) {
    state.models.cancel(&model_id);
}

#[tauri::command]
pub fn delete_model(state: State<'_, AppState>, model_id: String) -> AppResult<()> {
    state.models.delete(&model_id)?;
    state.stt.unload();
    Ok(())
}

#[tauri::command]
pub fn get_pipeline_state(state: State<'_, AppState>) -> PipelineState {
    state.pipeline.state()
}

#[tauri::command]
pub fn start_dictation(state: State<'_, AppState>) -> AppResult<()> {
    state.pipeline.start(Job::Dictation, None)
}

#[tauri::command]
pub fn stop_dictation(state: State<'_, AppState>) {
    state.pipeline.finish();
}

#[tauri::command]
pub fn cancel_dictation(state: State<'_, AppState>) {
    state.pipeline.cancel();
}

#[tauri::command]
pub async fn start_refine_selection(state: State<'_, AppState>) -> AppResult<()> {
    // Sync commands run on the main thread, but starting a refine job blocks
    // on the output worker (selection capture), which round-trips keystrokes
    // through the main thread — running it inline would deadlock.
    let pipeline = state.pipeline.clone();
    tauri::async_runtime::spawn_blocking(move || pipeline.start(Job::RefineSelection, None))
        .await
        .map_err(|e| AppError::State(format!("refine task failed: {e}")))?
}

#[tauri::command]
pub async fn start_polish_selection(state: State<'_, AppState>) -> AppResult<()> {
    // Same offload as start_refine_selection: polish blocks on selection
    // capture, which round-trips keystrokes through the main thread.
    // Errors surface as transient HUD states inside polish().
    let pipeline = state.pipeline.clone();
    tauri::async_runtime::spawn_blocking(move || pipeline.polish())
        .await
        .map_err(|e| AppError::State(format!("polish task failed: {e}")))
}

#[tauri::command]
pub fn get_last_result(state: State<'_, AppState>) -> Option<TranscriptionResult> {
    state.pipeline.last_result()
}

#[tauri::command]
pub async fn test_llm(
    state: State<'_, AppState>,
    profile: LlmProfile,
) -> Result<LlmTestResult, AppError> {
    Ok(state.llm.test(&profile).await)
}

#[tauri::command]
pub fn list_llm_profiles(state: State<'_, AppState>) -> Vec<LlmProfile> {
    state.profiles.list()
}

#[tauri::command]
pub fn save_llm_profile(
    state: State<'_, AppState>,
    profile: LlmProfile,
) -> AppResult<Vec<LlmProfile>> {
    state.profiles.save(profile)
}

#[tauri::command]
pub fn delete_llm_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<LlmProfile>> {
    let list = state.profiles.delete(&id)?;
    // Deleting the active profile turns refinement off.
    let settings = state.settings.get();
    if settings.active_llm_profile_id == id {
        let mut next = settings;
        next.active_llm_profile_id.clear();
        let saved = state.settings.set(next)?;
        let _ = app.emit("settings-changed", &saved);
    }
    Ok(list)
}

#[tauri::command]
pub fn reveal_llm_profiles(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let dir = state.profiles.dir();
    // Make sure there is something to show on a fresh install.
    std::fs::create_dir_all(dir)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Settings(format!("could not open the profiles folder: {e}")))
}

#[tauri::command]
pub async fn list_ollama_models(
    state: State<'_, AppState>,
    base_url: String,
) -> Result<Vec<String>, AppError> {
    state.llm.list_ollama_models(&base_url).await
}

#[tauri::command]
pub fn check_permissions() -> PermissionsState {
    permissions::check()
}

#[tauri::command]
pub fn request_microphone_permission() {
    permissions::request_microphone();
}

/// Shows the system Accessibility consent dialog (or returns true if already
/// trusted).
#[tauri::command]
pub fn prompt_accessibility_permission() -> bool {
    permissions::accessibility_trusted(true)
}

#[tauri::command]
pub fn open_accessibility_settings(app: AppHandle) {
    if let Err(err) = tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(permissions::ACCESSIBILITY_SETTINGS_URL, None::<&str>)
    {
        log::warn!("could not open accessibility settings: {err}");
    }
}

#[tauri::command]
pub fn open_microphone_settings(app: AppHandle) {
    if let Err(err) = tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(permissions::MICROPHONE_SETTINGS_URL, None::<&str>)
    {
        log::warn!("could not open microphone settings: {err}");
    }
}

#[tauri::command]
pub fn get_app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state
            .models
            .models_dir()
            .parent()
            .unwrap_or(state.models.models_dir())
            .display()
            .to_string(),
        config_path: state.settings.path().display().to_string(),
    }
}
