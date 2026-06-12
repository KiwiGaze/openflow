//! Tauri IPC commands. Names map 1:1 to `COMMANDS` in `@velata/core`.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult};
use crate::llm::LlmTestResult;
use crate::models::{ModelInfoDto, DEFAULT_STT_MODEL_ID};
use crate::permissions::{self, PermissionsState};
use crate::pipeline::{Job, PipelineState, TranscriptionResult};
use crate::profiles::LlmProfile;
use crate::settings::{Appearance, Settings, SETTINGS_CHANGED_EVENT};
use crate::state::AppState;
use crate::stt_profiles::{SttProfile, CLOUD_STT_PREFIX};
use crate::{modes, shortcuts, text, tray};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub config_path: String,
}

/// Returns the current settings snapshot.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

/// Persists settings; hotkey changes re-register as one atomic unit and every
/// hotkey rolls back to the last working set when registration fails.
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
    // Transforms carry hotkeys too, but the handler resolves the instruction by
    // id at trigger time — so only a changed id↔hotkey binding needs a
    // re-register, not an instruction or name edit (which save on every
    // keystroke).
    let bindings = |s: &Settings| -> Vec<(String, String)> {
        s.transforms
            .iter()
            .map(|t| (t.id.clone(), t.hotkey.clone()))
            .collect()
    };
    let hotkeys_changed = previous.dictation_hotkey != saved.dictation_hotkey
        || previous.polish_hotkey != saved.polish_hotkey
        || previous.change_overlay_hotkey != saved.change_overlay_hotkey
        || mode_hotkeys(&previous) != mode_hotkeys(&saved)
        || bindings(&previous) != bindings(&saved);
    if hotkeys_changed {
        if let Err(message) = shortcuts::apply(&app, &saved) {
            // Roll every hotkey — the three globals AND each mode hotkey — back
            // to the last working set as one atomic unit.
            let mut reverted = saved.clone();
            reverted.dictation_hotkey = previous.dictation_hotkey.clone();
            reverted.polish_hotkey = previous.polish_hotkey.clone();
            reverted.change_overlay_hotkey = previous.change_overlay_hotkey.clone();
            for mode in &mut reverted.modes {
                mode.hotkey = previous
                    .modes
                    .iter()
                    .find(|m| m.id == mode.id)
                    .and_then(|m| m.hotkey.clone());
            }
            reverted.transforms = previous.transforms.clone();
            let restored = state.settings.set(reverted)?;
            let _ = shortcuts::apply(&app, &restored);
            let _ = app.emit(SETTINGS_CHANGED_EVENT, &restored);
            return Err(AppError::Settings(message));
        }
    }

    if previous.launch_at_login != saved.launch_at_login {
        sync_autostart(&app, saved.launch_at_login);
    }

    if previous.show_in_dock != saved.show_in_dock {
        apply_dock_policy(&app, saved.show_in_dock);
    }

    if previous.appearance != saved.appearance {
        apply_appearance(&app, saved.appearance);
    }

    if let Err(err) = tray::rebuild_menu(&app) {
        log::warn!("tray rebuild failed: {err}");
    }
    let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
    Ok(saved)
}

/// Regular keeps a Dock icon; Accessory is menu-bar-only. No-op off macOS.
/// Not an IPC command — called from `main.rs` and `save_settings`.
pub fn apply_dock_policy(app: &AppHandle, show_in_dock: bool) {
    #[cfg(target_os = "macos")]
    {
        let policy = if show_in_dock {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        let _ = app.set_activation_policy(policy);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, show_in_dock);
}

/// Forces the native window chrome to the chosen theme; `System` follows macOS.
/// Not an IPC command — called from `main.rs` and `save_settings`.
pub fn apply_appearance(app: &AppHandle, appearance: Appearance) {
    app.set_theme(match appearance {
        Appearance::System => None,
        Appearance::Light => Some(tauri::Theme::Light),
        Appearance::Dark => Some(tauri::Theme::Dark),
    });
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

/// Returns the whisper model registry with installed/downloading status.
#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Vec<ModelInfoDto> {
    state.models.list()
}

/// Starts a background model download; progress (and any failure) streams to
/// the webview via `model-download` events.
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

/// Cancels an in-flight download; no-op when none is running for this id.
#[tauri::command]
pub fn cancel_model_download(state: State<'_, AppState>, model_id: String) {
    state.models.cancel(&model_id);
}

/// Deletes the model file and unloads it from memory if it was loaded.
#[tauri::command]
pub fn delete_model(state: State<'_, AppState>, model_id: String) -> AppResult<()> {
    state.models.delete(&model_id)?;
    state.stt.unload();
    Ok(())
}

/// Returns the current pipeline state — the HUD's initial sync before events.
#[tauri::command]
pub fn get_pipeline_state(state: State<'_, AppState>) -> PipelineState {
    state.pipeline.state()
}

/// Starts recording a dictation in the active mode.
#[tauri::command]
pub fn start_dictation(state: State<'_, AppState>) -> AppResult<()> {
    state.pipeline.start(Job::Dictation, None)
}

/// Stops recording and processes the take: transcribe → clean/polish → insert.
/// Contrast with [`cancel_dictation`], which discards it.
#[tauri::command]
pub fn stop_dictation(state: State<'_, AppState>) {
    state.pipeline.finish();
}

/// Discards the in-flight recording or processing; nothing is inserted.
#[tauri::command]
pub fn cancel_dictation(state: State<'_, AppState>) {
    state.pipeline.cancel();
}

/// Rewrites the current selection with the built-in fix-grammar instruction —
/// no recording involved.
#[tauri::command]
pub async fn start_polish_selection(state: State<'_, AppState>) -> AppResult<()> {
    // Sync commands run on the main thread, but polish blocks on selection
    // capture, which round-trips keystrokes through the main thread — running
    // it inline would deadlock. Errors surface as transient HUD states inside
    // polish().
    let pipeline = state.pipeline.clone();
    tauri::async_runtime::spawn_blocking(move || pipeline.polish())
        .await
        .map_err(|e| AppError::State(format!("polish task failed: {e}")))
}

/// Returns the most recent result — backs tray “Copy Last Result” and the
/// changes overlay.
#[tauri::command]
pub fn get_last_result(state: State<'_, AppState>) -> Option<TranscriptionResult> {
    state.pipeline.last_result()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmostApp {
    pub bundle_id: String,
    pub name: String,
}

/// The app Velata last dictated into — lets the App rules UI offer a one-click
/// rule for it without the user hunting for a bundle id. None until first use.
#[tauri::command]
pub fn get_last_dictation_app(state: State<'_, AppState>) -> Option<FrontmostApp> {
    state
        .pipeline
        .last_app()
        .map(|(bundle_id, name)| FrontmostApp { bundle_id, name })
}

/// Returns the opt-in local dictation history (text only — never audio).
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<crate::history::HistoryEntry> {
    state.history.list()
}

/// Deletes every persisted history entry.
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> AppResult<()> {
    state.history.clear()?;
    Ok(())
}

/// Deletes one persisted history entry by id. A missing id is a no-op.
#[tauri::command]
pub fn delete_history_entry(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.history.delete(&id)?;
    Ok(())
}

/// Re-runs a stored transcript through a chosen mode, reusing the dictation
/// resolution (the mode's prompt + active profile, or rules cleanup with no
/// profile). Returns the new text for the user to copy — never auto-inserts.
#[tauri::command]
pub async fn reprocess_history(
    state: State<'_, AppState>,
    text: String,
    mode_id: String,
) -> Result<String, AppError> {
    let settings = state.settings.get();
    let mode = settings
        .modes
        .iter()
        .find(|m| m.id == mode_id)
        .cloned()
        .ok_or_else(|| AppError::State("that mode no longer exists".into()))?;
    if !mode.uses_llm {
        return Ok(modes::no_ai_output(&mode.id, &text));
    }
    let system = modes::dictation_system_prompt(&mode, &settings.dictionary);
    // Honor the mode's AI-profile override (07 §3), falling back to the active
    // profile, so reprocess matches what real dictation in this mode produces.
    let profile = state.profiles.resolve(
        mode.ai_profile_id.as_deref(),
        &settings.active_llm_profile_id,
    );
    match profile {
        Some(profile) => state.llm.chat(&profile, &system, &text).await,
        None => Ok(text::apply_rules_cleanup(&text)),
    }
}

/// Copies text to the clipboard for the changes overlay's Copy button. Routed
/// through the output worker, which owns the single clipboard handle.
#[tauri::command]
pub fn copy_text(state: State<'_, AppState>, text: String) -> AppResult<()> {
    state.output.copy_text(text)
}

/// Toggles the changes overlay between click-through (hidden) and interactive
/// (visible), so the always-present panel never eats clicks while faded out.
#[tauri::command]
pub fn set_changes_interactive(app: AppHandle, interactive: bool) {
    if let Some(window) = app.get_webview_window(crate::changes::CHANGES_LABEL) {
        let _ = window.set_ignore_cursor_events(!interactive);
    }
}

/// Returns the Insights snapshot: always the session aggregates (in-memory,
/// reset on quit), enriched with all-time totals, streaks, and an all-time
/// per-app breakdown when their opt-in flags are on (`app_stats_enabled`,
/// `history_enabled`). The DB reads are best-effort — a failure degrades to the
/// session view rather than failing the whole command.
#[tauri::command]
pub fn get_insights(state: State<'_, AppState>) -> crate::stats::Insights {
    use crate::stats::{AllTimeStats, AppWords, PerAppScope, PER_APP_LIMIT};

    let mut insights = state.pipeline.insights();
    let settings = state.settings.get();

    // All-time per-app from history when the user keeps it; otherwise the
    // snapshot's session tally stands.
    if settings.history_enabled {
        if let Ok(rows) = state.db.history_per_app(PER_APP_LIMIT as i64) {
            insights.per_app = rows
                .into_iter()
                .map(|(name, words)| AppWords { name, words })
                .collect();
            insights.per_app_scope = PerAppScope::AllTime;
        }
    }

    if settings.app_stats_enabled {
        if let Ok(Some(totals)) = state.db.insights_totals() {
            let ai_percent = if totals.dictations > 0 {
                (totals.ai_dictations as f64 / totals.dictations as f64 * 100.0).round() as u32
            } else {
                0
            };
            insights.all_time = Some(AllTimeStats {
                words: totals.words,
                dictations: totals.dictations,
                ai_percent,
                fixes: totals.fixes,
                words_per_minute: crate::stats::pace_wpm(totals.words, totals.duration_ms),
            });
        }
        if let Ok(days) = state.db.insights_days() {
            insights.streak = Some(crate::stats::streaks(&days, &local_day()));
        }
    }

    insights
}

/// Deletes every persisted `insights_daily` row (the "reset all-time stats"
/// action). Session counters are untouched — they live in RAM.
#[tauri::command]
pub fn clear_insights(state: State<'_, AppState>) -> AppResult<()> {
    state.db.insights_clear()
}

/// The user's LOCAL calendar day as `YYYY-MM-DD` — the streak calculator's
/// "today". Local (not UTC) so the streak matches the user's clock.
fn local_day() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Returns session-only candidate terms for the dictionary, most-seen first.
#[tauri::command]
pub fn list_dictionary_suggestions(
    state: State<'_, AppState>,
) -> Vec<crate::suggestions::DictionarySuggestion> {
    let dictionary = state.settings.get().dictionary;
    state.pipeline.dictionary_suggestions(&dictionary, 5)
}

/// Hides a suggested term for the rest of the session.
#[tauri::command]
pub fn dismiss_dictionary_suggestion(state: State<'_, AppState>, term: String) {
    state.pipeline.dismiss_suggestion(&term);
}

/// Round-trips a tiny prompt through the profile to verify the connection.
#[tauri::command]
pub async fn test_llm(
    state: State<'_, AppState>,
    profile: LlmProfile,
) -> Result<LlmTestResult, AppError> {
    Ok(state.llm.test(&profile).await)
}

/// Returns all saved LLM profiles (one JSON file each).
#[tauri::command]
pub fn list_llm_profiles(state: State<'_, AppState>) -> Vec<LlmProfile> {
    state.profiles.list()
}

/// Mode editor Preview (06 §6): builds the full system prompt exactly as the
/// pipeline would, then polishes a sample through the active profile — or runs
/// the same rules-based cleanup the pipeline uses when there is no profile, so
/// the preview is the genuine path, never a mock.
#[tauri::command]
pub async fn test_mode(
    state: State<'_, AppState>,
    prompt: String,
    sample: String,
    transforms: bool,
    ai_profile_id: Option<String>,
) -> Result<String, AppError> {
    let settings = state.settings.get();
    let system = modes::preview_system_prompt(&prompt, transforms, &settings.dictionary);
    // Preview the mode's effective AI profile (its override, else the active
    // one) so the result matches real dictation — including unsaved edits.
    let profile = state
        .profiles
        .resolve(ai_profile_id.as_deref(), &settings.active_llm_profile_id);
    match profile {
        Some(profile) => state.llm.chat(&profile, &system, &sample).await,
        None => Ok(text::apply_rules_cleanup(&sample)),
    }
}

/// Upserts a profile file (0600 — it can hold an API key); returns the fresh list.
#[tauri::command]
pub fn save_llm_profile(
    state: State<'_, AppState>,
    profile: LlmProfile,
) -> AppResult<Vec<LlmProfile>> {
    state.profiles.save(profile)
}

/// Deletes the profile; polish turns off if this was the active one.
#[tauri::command]
pub fn delete_llm_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<LlmProfile>> {
    let list = state.profiles.delete(&id)?;
    // Deleting the active profile turns polish off.
    let settings = state.settings.get();
    if settings.active_llm_profile_id == id {
        let mut next = settings;
        next.active_llm_profile_id.clear();
        let saved = state.settings.set(next)?;
        let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
    }
    Ok(list)
}

/// Returns all saved cloud STT profiles.
#[tauri::command]
pub fn list_stt_profiles(state: State<'_, AppState>) -> Vec<SttProfile> {
    state.stt_profiles.list()
}

#[tauri::command]
pub fn save_stt_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: SttProfile,
) -> AppResult<Vec<SttProfile>> {
    // Editing the endpoint or key revokes prior "audio leaves the Mac" consent
    // (08 §3): the audio would otherwise reach a place the user never approved.
    let endpoint_changed = state
        .stt_profiles
        .get(&profile.id)
        .is_some_and(|old| old.base_url != profile.base_url || old.api_key != profile.api_key);
    let profile_id = profile.id.clone();
    let list = state.stt_profiles.save(profile)?;
    if endpoint_changed {
        let mut settings = state.settings.get();
        if settings
            .confirmed_stt_profiles
            .iter()
            .any(|id| id == &profile_id)
        {
            settings
                .confirmed_stt_profiles
                .retain(|id| id != &profile_id);
            let saved = state.settings.set(settings)?;
            let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
        }
    }
    Ok(list)
}

/// Deletes the profile; its consent is revoked and an active engine falls
/// back to on-device whisper.
#[tauri::command]
pub fn delete_stt_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<SttProfile>> {
    let list = state.stt_profiles.delete(&id)?;
    // Drop any consent and reset the active engine if this profile was it — a
    // recreated id (same name, new endpoint) must not inherit old consent, and
    // a deleted active engine falls back to the on-device default (08 §3).
    let settings = state.settings.get();
    let had_consent = settings.confirmed_stt_profiles.iter().any(|c| c == &id);
    let was_active = settings.stt_model_id == format!("{CLOUD_STT_PREFIX}{id}");
    if had_consent || was_active {
        let mut next = settings;
        next.confirmed_stt_profiles.retain(|c| c != &id);
        if was_active {
            next.stt_model_id = DEFAULT_STT_MODEL_ID.into();
        }
        let saved = state.settings.set(next)?;
        let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
    }
    Ok(list)
}

/// Opens the STT profiles folder in Finder (created on demand).
#[tauri::command]
pub fn reveal_stt_profiles(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let dir = state.stt_profiles.dir();
    std::fs::create_dir_all(dir)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Settings(format!("could not open the STT profiles folder: {e}")))
}

/// Opens the LLM profiles folder in Finder (created on demand).
#[tauri::command]
pub fn reveal_llm_profiles(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let dir = state.profiles.dir();
    // Make sure there is something to show on a fresh install.
    std::fs::create_dir_all(dir)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Settings(format!("could not open the profiles folder: {e}")))
}

/// Writes an exported mode JSON to `<app-data>/exported-modes/<filename>.json`
/// and reveals the folder. A native save panel would need the dialog plugin;
/// the reveal idiom matches "Show in Finder" for profiles and adds no dependency.
#[tauri::command]
pub fn export_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    filename: String,
    contents: String,
) -> AppResult<()> {
    // `filename` is a slug from the frontend; re-check it can't escape the dir.
    if filename.is_empty()
        || filename.len() > 80
        || filename.contains(['/', '\\'])
        || filename.contains("..")
    {
        return Err(AppError::Settings("invalid export filename".into()));
    }
    let dir = state
        .profiles
        .dir()
        .parent()
        .ok_or_else(|| AppError::Settings("no data directory".into()))?
        .join("exported-modes");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{filename}.json")), contents)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Settings(format!("could not open the exported-modes folder: {e}")))
}

/// Writes the dictionary as CSV to `<app-data>/dictionary.csv` and reveals the
/// folder — same no-dependency reveal idiom as `export_mode`.
#[tauri::command]
pub fn export_dictionary(
    app: AppHandle,
    state: State<'_, AppState>,
    contents: String,
) -> AppResult<()> {
    let data_dir = state
        .profiles
        .dir()
        .parent()
        .ok_or_else(|| AppError::Settings("no data directory".into()))?;
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(data_dir.join("dictionary.csv"), contents)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(data_dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Settings(format!("could not open the data folder: {e}")))
}

/// Lists installed model names from an Ollama server — the one Ollama-native
/// call (everything else goes through the OpenAI-compatible client).
#[tauri::command]
pub async fn list_ollama_models(
    state: State<'_, AppState>,
    base_url: String,
) -> Result<Vec<String>, AppError> {
    state.llm.list_ollama_models(&base_url).await
}

/// Returns microphone and Accessibility permission state.
#[tauri::command]
pub fn check_permissions() -> PermissionsState {
    permissions::check()
}

/// Shows the system microphone consent prompt; the grant lands asynchronously
/// (the UI polls `check_permissions`).
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

/// Opens System Settings at Privacy & Security → Accessibility.
#[tauri::command]
pub fn open_accessibility_settings(app: AppHandle) {
    if let Err(err) = tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(permissions::ACCESSIBILITY_SETTINGS_URL, None::<&str>)
    {
        log::warn!("could not open accessibility settings: {err}");
    }
}

/// Opens System Settings at Privacy & Security → Microphone.
#[tauri::command]
pub fn open_microphone_settings(app: AppHandle) {
    if let Err(err) = tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(permissions::MICROPHONE_SETTINGS_URL, None::<&str>)
    {
        log::warn!("could not open microphone settings: {err}");
    }
}

/// Returns the app version and data paths shown in the About tab.
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
