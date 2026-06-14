//! Tauri IPC commands. Names map 1:1 to `COMMANDS` in `@velata/core`.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult};
use crate::llm::LlmTestResult;
use crate::models::{ModelInfoDto, DEFAULT_STT_MODEL_ID};
use crate::notes::{self, Note, NoteSummary, NoteVersion};
use crate::permissions::{self, PermissionsState};
use crate::pipeline::{Job, PipelineState, TranscriptionResult};
use crate::profiles::LlmProfile;
use crate::scratchpad::{self, NOTES_CHANGED_EVENT};
use crate::settings::{Appearance, Settings, SETTINGS_CHANGED_EVENT};
use crate::state::AppState;
use crate::stt_profiles::{SttProfile, CLOUD_STT_PREFIX};
use crate::{prompts, shortcuts};

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

    let saved = state.settings.set(settings)?;

    // Prompts carry shortcuts, but the handler resolves the instruction by id at
    // trigger time — so only a changed id↔shortcut binding needs a re-register,
    // not an instruction or name edit (which save on every keystroke).
    let bindings = |s: &Settings| -> Vec<(String, String)> {
        s.prompts
            .iter()
            .map(|p| (p.id.clone(), p.shortcut.clone()))
            .collect()
    };
    let hotkeys_changed = previous.push_to_talk_hotkey != saved.push_to_talk_hotkey
        || previous.hands_free_hotkey != saved.hands_free_hotkey
        || previous.see_changes_hotkey != saved.see_changes_hotkey
        || bindings(&previous) != bindings(&saved);
    if hotkeys_changed {
        if let Err(message) = shortcuts::apply(&app, &saved) {
            // Roll every hotkey — push-to-talk, hands-free, see-changes, and each
            // prompt shortcut — back to the last working set as one atomic unit.
            let mut reverted = saved.clone();
            reverted.push_to_talk_hotkey = previous.push_to_talk_hotkey.clone();
            reverted.hands_free_hotkey = previous.hands_free_hotkey.clone();
            reverted.see_changes_hotkey = previous.see_changes_hotkey.clone();
            reverted.prompts = previous.prompts.clone();
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

    let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
    Ok(saved)
}

/// Sets (or clears, with null) the prompt that runs automatically on the
/// transcript after dictation. A dangling id is dropped by `normalize`. Persists
/// and emits `settings-changed` so the HUD circle and any open webview re-sync.
#[tauri::command]
pub fn set_post_dictation_transform(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
) -> AppResult<()> {
    let mut settings = state.settings.get();
    settings.post_dictation_transform_id = id;
    let saved = state.settings.set(settings)?;
    let _ = app.emit(SETTINGS_CHANGED_EVENT, &saved);
    Ok(())
}

/// The App window (the Features half of the former single window).
pub const MAIN_WINDOW_LABEL: &str = "main";
/// The Settings window (the configuration half).
pub const SETTINGS_WINDOW_LABEL: &str = "settings";

/// Asks the (already-open, hide-on-close) Settings window to switch to a tab.
/// Payload: the tab id string. Mirrored as `EVENTS.settingsNavigate` in
/// `@velata/core`. Lets a deep link from the App window land on a specific
/// Settings tab instead of the window's default.
pub const SETTINGS_NAVIGATE_EVENT: &str = "settings-navigate";

/// Shows, unminimizes, and focuses a window, bringing the app out of Accessory
/// mode so it appears in the Dock and can take focus while open. Shared by the
/// `open_main_window`/`open_settings_window` commands, the tray, and startup —
/// best-effort, like all window show paths. No-op if the label is unknown.
pub fn show_window(app: &AppHandle, label: &str) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
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

/// Starts recording a dictation.
#[tauri::command]
pub fn start_dictation(state: State<'_, AppState>) -> AppResult<()> {
    state.pipeline.start(Job::Dictation)
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

/// Returns the most recent result — backs tray “Copy Last Result” and the
/// changes overlay.
#[tauri::command]
pub fn get_last_result(state: State<'_, AppState>) -> Option<TranscriptionResult> {
    state.pipeline.last_result()
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

/// Reports whether the HUD's post-dictation dropdown is open. While open, the
/// HUD's cursor poll forces the whole (never-shown) window interactive so a
/// radio click anywhere in the frame lands; otherwise only the circle is.
#[tauri::command]
pub fn set_hud_menu_open(open: bool) {
    crate::hud::set_menu_open(open);
}

/// Returns the lifetime Insights, always computed from `insights_daily` (which
/// is written on every dictation): total words, total dictations, speaking pace,
/// and the current day streak — counts and dates only, never words or audio.
/// There is no enable toggle and no reset. Empty stores read as all-zero (the
/// Home header hides the row at zero activity). The DB reads are best-effort: a
/// failure logs and yields zeros rather than failing the whole command.
#[tauri::command]
pub fn get_insights(state: State<'_, AppState>) -> crate::stats::Insights {
    use crate::stats::{local_day, pace_wpm, streaks, Insights};

    let (words, dictations, words_per_minute) = match state.db.insights_totals() {
        // No day rows yet → zeros, so the header hides until the first dictation.
        Ok(None) => (0, 0, 0),
        Ok(Some(totals)) => (
            totals.words,
            totals.dictations,
            pace_wpm(totals.words, totals.duration_ms),
        ),
        Err(err) => {
            log::warn!("could not read insights totals: {err}");
            (0, 0, 0)
        }
    };
    let streak = match state.db.insights_days() {
        Ok(days) => streaks(&days, &local_day()),
        Err(err) => {
            log::warn!("could not read insights days: {err}");
            0
        }
    };

    Insights {
        words,
        dictations,
        words_per_minute,
        streak,
    }
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

/// Writes the dictionary as CSV to `<app-data>/dictionary.csv` and reveals the
/// folder — a no-dependency "Show in Finder" idiom (a native save panel would
/// need the dialog plugin).
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

/// Lists input device names for the microphone picker (local hardware
/// enumeration — never network). Enumeration failures degrade to whatever names
/// were collected, so the picker can always offer at least the system default.
#[tauri::command]
pub fn list_input_devices() -> Vec<String> {
    crate::audio::list_input_device_names()
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

// ---- Scratchpad notes ------------------------------------------------------
//
// Notes are opt-in user content: every note command refuses while the
// Scratchpad is off, so nothing is read or written behind the user's back. The
// window itself is not gated (it shows the enable card when off); only the data
// commands are.

/// The gate every note command shares: refuses unless the Scratchpad is on.
fn ensure_scratchpad_on(state: &AppState) -> AppResult<()> {
    if state.settings.get().scratchpad_enabled {
        Ok(())
    } else {
        Err(AppError::Settings("Scratchpad is turned off.".into()))
    }
}

/// Lists non-deleted notes (pinned first, then most recent). `search` filters
/// title and body case-insensitively. Refuses while the Scratchpad is off.
#[tauri::command]
pub fn list_notes(
    state: State<'_, AppState>,
    search: Option<String>,
) -> AppResult<Vec<NoteSummary>> {
    ensure_scratchpad_on(&state)?;
    state.db.notes_list(search.as_deref())
}

/// Returns one non-deleted note by id, or null. Refuses while off.
#[tauri::command]
pub fn get_note(state: State<'_, AppState>, id: String) -> AppResult<Option<Note>> {
    ensure_scratchpad_on(&state)?;
    state.db.note_get(&id)
}

/// Creates an empty note, snapshots its (empty) body as a "created" version,
/// and returns it. Refuses while off.
#[tauri::command]
pub fn create_note(app: AppHandle, state: State<'_, AppState>) -> AppResult<Note> {
    ensure_scratchpad_on(&state)?;
    let note = state.db.note_create()?;
    state
        .db
        .note_version_add(&note.id, &note.content, "created", None)?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(note)
}

/// Updates a note's title and body. Rejects an oversized body rather than
/// truncating it. Refuses while off.
#[tauri::command]
pub fn update_note(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    title: String,
    content: String,
) -> AppResult<()> {
    ensure_scratchpad_on(&state)?;
    state.db.note_update(&id, &title, &content)?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(())
}

/// Pins or unpins a note. Refuses while off.
#[tauri::command]
pub fn set_note_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> AppResult<()> {
    ensure_scratchpad_on(&state)?;
    state.db.note_set_pinned(&id, pinned)?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(())
}

/// Soft-deletes a note (sets `deleted_at`; the bytes stay on disk). Refuses
/// while off.
#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    ensure_scratchpad_on(&state)?;
    state.db.note_soft_delete(&id)?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(())
}

/// Returns a note's version history, newest first. Refuses while off.
#[tauri::command]
pub fn list_note_versions(
    state: State<'_, AppState>,
    note_id: String,
) -> AppResult<Vec<NoteVersion>> {
    ensure_scratchpad_on(&state)?;
    state.db.note_versions(&note_id)
}

/// Restores a note to an earlier version: snapshots the CURRENT body as a
/// "restore" version first (so the restore is itself reversible), then sets the
/// note's content to the chosen version's. Refuses while off.
#[tauri::command]
pub fn restore_note_version(
    app: AppHandle,
    state: State<'_, AppState>,
    version_id: String,
) -> AppResult<Note> {
    ensure_scratchpad_on(&state)?;
    let version = state
        .db
        .note_version_get(&version_id)?
        .ok_or_else(|| AppError::State("that version no longer exists".into()))?;
    let current = state
        .db
        .note_get(&version.note_id)?
        .ok_or_else(|| AppError::State("that note no longer exists".into()))?;
    state
        .db
        .note_version_add(&version.note_id, &current.content, "restore", None)?;
    state
        .db
        .note_update(&version.note_id, &current.title, &version.content)?;
    let note = state
        .db
        .note_get(&version.note_id)?
        .ok_or_else(|| AppError::State("that note no longer exists".into()))?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(note)
}

/// Rewrites a note's body with a transform (Polish when `transform_id` is null,
/// else the named settings transform). The instruction is resolved server-side;
/// the note's plain text (tags stripped) is sent through the one LLM client and
/// the returned text becomes the new body as escaped paragraphs. A "transform"
/// version of the prior body is snapshotted first. Refuses while off, and
/// refuses honestly when no AI provider is configured (no silent fallback —
/// wrong text over the user's note is worse than an error). Refuses while off.
#[tauri::command]
pub async fn transform_note_text(
    app: AppHandle,
    state: State<'_, AppState>,
    note_id: String,
    transform_id: Option<String>,
) -> Result<Note, AppError> {
    ensure_scratchpad_on(&state)?;
    let settings = state.settings.get();

    // Resolve the instruction by id before touching the note, so an unknown id
    // fails fast without snapshotting or calling the LLM. A null id means Polish:
    // honor the user-edited Polish prompt, matching `run_prompt("polish")`.
    let instruction = match &transform_id {
        None => settings
            .prompts
            .iter()
            .find(|p| p.id == prompts::POLISH_PROMPT_ID)
            .map(|p| p.instruction.clone())
            .unwrap_or_else(|| prompts::DEFAULT_POLISH_INSTRUCTION.to_string()),
        Some(id) => settings
            .prompts
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.instruction.clone())
            .ok_or_else(|| AppError::State("that prompt no longer exists".into()))?,
    };

    let note = state
        .db
        .note_get(&note_id)?
        .ok_or_else(|| AppError::State("that note no longer exists".into()))?;

    // Transforms need a provider; say so plainly rather than falling back to a
    // rules cleanup that would silently change the user's note.
    let profile = state
        .profiles
        .active(&settings.active_llm_profile_id)
        .ok_or_else(|| AppError::Llm("Add an AI provider to use transforms.".into()))?;

    // Snapshot the body before the destructive rewrite.
    state.db.note_version_add(
        &note_id,
        &note.content,
        "transform",
        transform_id.as_deref(),
    )?;

    // The LLM works on plain text and returns plain text; re-wrap as minimal
    // HTML for the editor. The selection-rewrite prompt treats the body as data.
    let plain = notes::strip_tags(&note.content);
    let system = prompts::selection_system_prompt();
    let user = prompts::selection_user_prompt(&plain, &instruction);
    let rewritten = state.llm.chat(&profile, &system, &user).await?;
    let content = notes::text_to_html(&rewritten);

    state.db.note_update(&note_id, &note.title, &content)?;
    let updated = state
        .db
        .note_get(&note_id)?
        .ok_or_else(|| AppError::State("that note no longer exists".into()))?;
    let _ = app.emit(NOTES_CHANGED_EVENT, ());
    Ok(updated)
}

/// Opens the Scratchpad window (creating it if absent, else showing and
/// focusing it). Not gated on `scratchpad_enabled` — the window shows the
/// enable card when off, and this is the entry point either way. `note_id`
/// selects a note on open.
#[tauri::command]
pub fn open_scratchpad_window(app: AppHandle, note_id: Option<String>) -> AppResult<()> {
    scratchpad::open(&app, note_id)
        .map_err(|e| AppError::State(format!("could not open the Scratchpad: {e}")))
}

/// Shows and focuses the App window — the "‹ Velata" action in the Settings
/// window's sidebar.
#[tauri::command]
pub fn open_main_window(app: AppHandle) {
    show_window(&app, MAIN_WINDOW_LABEL);
}

/// Shows and focuses the Settings window — the "⚙ Settings" action in the App
/// window's sidebar. When `tab` is given, asks the already-open window to switch
/// to that tab so a deep link from the App window lands where it meant to.
#[tauri::command]
pub fn open_settings_window(app: AppHandle, tab: Option<String>) {
    show_window(&app, SETTINGS_WINDOW_LABEL);
    if let Some(tab) = tab {
        let _ = app.emit(SETTINGS_NAVIGATE_EVENT, tab);
    }
}
