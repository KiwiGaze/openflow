//! The dictation pipeline state machine.
//!
//! One pipeline instance owns the flow `record → transcribe → (refine) →
//! insert` for both jobs (dictation and selected-text refinement), emits
//! progress events to the webviews, and guards against stale work with a
//! generation counter — cancelling simply bumps the generation.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::apps;
use crate::audio::AudioSystem;
use crate::cloud_stt;
use crate::error::{AppError, AppResult};
use crate::history::HistoryStore;
use crate::hud;
use crate::llm::LlmClient;
use crate::models::ModelManager;
use crate::modes::{self, LITERAL_MODE_ID};
use crate::output::{CopyReason, InsertOutcome, OutputSystem};
use crate::profiles::{LlmProfile, ProfileManager};
use crate::settings::{HotkeyBehavior, Mode, Settings, SettingsManager, MAX_RECORDING_SECS};
use crate::shortcuts;
use crate::stt::{initial_prompt_from_dictionary, SttEngine};
use crate::stt_profiles::SttProfileManager;
use crate::text;

pub const PIPELINE_STATE_EVENT: &str = "pipeline-state";
pub const AUDIO_LEVEL_EVENT: &str = "audio-level";
pub const RESULT_EVENT: &str = "transcription-result";

/// Releases faster than this are treated as a tap, which switches the
/// hold-to-talk gesture into hands-free mode instead of stopping.
const TAP_THRESHOLD: Duration = Duration::from_millis(350);

/// How long error/notice states stay visible before auto-clearing.
const TRANSIENT_STATE_TTL: Duration = Duration::from_secs(4);

/// How long the success flash (✓ + inserted text) lingers before idle.
const SUCCESS_FLASH_TTL: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Idle,
    Recording,
    Transcribing,
    Refining,
    Inserting,
    /// Brief success flash (✓ + inserted text) before fading back to idle.
    Inserted,
    Notice,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Job {
    Dictation,
    RefineSelection,
    /// Refine the selection with the built-in instruction — no recording.
    PolishSelection,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineState {
    pub status: Status,
    pub job: Option<Job>,
    pub message: Option<String>,
    /// A one-time educational tip shown on the success flash (05 §2.3); set only
    /// when entering `Inserted`, otherwise None.
    pub hud_tip: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub raw: String,
    pub text: String,
    pub mode_id: String,
    pub refined: bool,
    pub duration_ms: u64,
}

struct Session {
    job: Job,
    generation: u64,
    started: Instant,
    /// Captured before recording starts (refine job only).
    selection: Option<String>,
    /// Mode id from a per-mode hotkey, used for this job only without changing
    /// the persistent active mode (one-shot, 07 §4). None = use the active mode.
    mode_override: Option<String>,
}

pub struct Pipeline {
    app: AppHandle,
    audio: Arc<AudioSystem>,
    stt: Arc<SttEngine>,
    llm: Arc<LlmClient>,
    output: Arc<OutputSystem>,
    settings: Arc<SettingsManager>,
    models: Arc<ModelManager>,
    profiles: Arc<ProfileManager>,
    history: Arc<HistoryStore>,
    stt_profiles: Arc<SttProfileManager>,
    state: Mutex<PipelineState>,
    state_seq: AtomicU64,
    session: Mutex<Option<Session>>,
    generation: AtomicU64,
    last_result: Mutex<Option<TranscriptionResult>>,
    /// Frontmost app `(bundle_id, name)` at the last dictation — lets the App
    /// rules UI offer "add a rule for the app you just dictated into".
    last_app: Mutex<Option<(String, String)>>,
    /// At most one HUD educational tip per app session (05 §2.1).
    tip_shown_session: AtomicBool,
    /// Consecutive empty dictations — drives the one-time accuracy tip (05 §2.3).
    empty_streak: AtomicU64,
}

impl Pipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: AppHandle,
        audio: Arc<AudioSystem>,
        stt: Arc<SttEngine>,
        llm: Arc<LlmClient>,
        output: Arc<OutputSystem>,
        settings: Arc<SettingsManager>,
        models: Arc<ModelManager>,
        profiles: Arc<ProfileManager>,
        history: Arc<HistoryStore>,
        stt_profiles: Arc<SttProfileManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            app,
            audio,
            stt,
            llm,
            output,
            settings,
            models,
            profiles,
            history,
            stt_profiles,
            state: Mutex::new(PipelineState {
                status: Status::Idle,
                job: None,
                message: None,
                hud_tip: None,
            }),
            state_seq: AtomicU64::new(0),
            session: Mutex::new(None),
            generation: AtomicU64::new(0),
            last_result: Mutex::new(None),
            last_app: Mutex::new(None),
            tip_shown_session: AtomicBool::new(false),
            empty_streak: AtomicU64::new(0),
        })
    }

    pub fn last_app(&self) -> Option<(String, String)> {
        self.last_app
            .lock()
            .expect("pipeline state poisoned")
            .clone()
    }

    pub fn state(&self) -> PipelineState {
        self.state.lock().expect("pipeline state poisoned").clone()
    }

    pub fn last_result(&self) -> Option<TranscriptionResult> {
        self.last_result
            .lock()
            .expect("pipeline state poisoned")
            .clone()
    }

    fn set_state(&self, status: Status, job: Option<Job>, message: Option<String>) -> u64 {
        let state = PipelineState {
            status,
            job,
            message,
            hud_tip: None,
        };
        *self.state.lock().expect("pipeline state poisoned") = state.clone();
        let seq = self.state_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.app.emit(PIPELINE_STATE_EVENT, &state);
        seq
    }

    /// Error/notice that clears back to idle unless the state changed since.
    fn set_transient(self: &Arc<Self>, status: Status, message: String) {
        let seq = self.set_state(status, None, Some(message));
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(TRANSIENT_STATE_TTL).await;
            if pipeline.state_seq.load(Ordering::SeqCst) == seq {
                pipeline.set_state(Status::Idle, None, None);
            }
        });
    }

    /// Confirmation flash: ✓ + a preview of the inserted text (and maybe a
    /// one-time tip), then idle. The webview ellipsizes; the state-seq guard
    /// lets a newer job pre-empt.
    fn set_success(self: &Arc<Self>, preview: String, hud_tip: Option<String>) {
        let state = PipelineState {
            status: Status::Inserted,
            job: None,
            message: Some(preview),
            hud_tip,
        };
        *self.state.lock().expect("pipeline state poisoned") = state.clone();
        let seq = self.state_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.app.emit(PIPELINE_STATE_EVENT, &state);
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SUCCESS_FLASH_TTL).await;
            if pipeline.state_seq.load(Ordering::SeqCst) == seq {
                pipeline.set_state(Status::Idle, None, None);
            }
        });
    }

    /// The HUD tip to ride a successful dictation (tip.latch — teach hands-free
    /// at the 3rd hold-dictation), or None. Marks it seen so it shows once.
    fn dictation_hud_tip(&self) -> Option<String> {
        if self.tip_shown_session.load(Ordering::SeqCst) {
            return None;
        }
        let s = self.settings.get();
        if !s.tips_enabled || s.tips_seen.iter().any(|t| t == "tip.latch") {
            return None;
        }
        if s.dictation_hotkey_behavior != HotkeyBehavior::Hold || s.dictation_count != 3 {
            return None;
        }
        self.mark_tip_shown("tip.latch", s);
        Some("Tip: tap the hotkey instead of holding to go hands-free.".into())
    }

    /// The HUD tip to ride a successful rewrite (tip.polish — teach the polish
    /// hotkey), or None. Marks it seen so it shows once.
    fn refine_hud_tip(&self) -> Option<String> {
        if self.tip_shown_session.load(Ordering::SeqCst) {
            return None;
        }
        let s = self.settings.get();
        if !s.tips_enabled || s.tips_seen.iter().any(|t| t == "tip.polish") {
            return None;
        }
        if s.polish_hotkey.is_empty() || s.active_llm_profile_id.is_empty() {
            return None;
        }
        self.mark_tip_shown("tip.polish", s);
        Some("Tip: tap your polish hotkey to fix grammar without recording.".into())
    }

    fn mark_tip_shown(&self, id: &str, mut settings: Settings) {
        self.tip_shown_session.store(true, Ordering::SeqCst);
        settings.tips_seen.push(id.to_string());
        if let Ok(saved) = self.settings.set(settings) {
            let _ = self.app.emit("settings-changed", &saved);
        }
    }

    /// The "didn't catch that" notice, upgraded once to suggest a larger model
    /// after two consecutive empty dictations (tip.accuracy, 05 §2.3). Unlike
    /// the flash tips this rides a Notice, so it is independent of the
    /// per-session flash cap; `tips_seen` still bounds it to once ever.
    fn empty_dictation_notice(&self, streak: u64) -> String {
        const BASE: &str = "Didn't catch that — try again.";
        if streak < 2 {
            return BASE.into();
        }
        let mut s = self.settings.get();
        if !s.tips_enabled
            || s.tips_seen.iter().any(|t| t == "tip.accuracy")
            || s.stt_model_id.contains("large")
            || s.stt_model_id.starts_with("cloud:")
        {
            return BASE.into();
        }
        s.tips_seen.push("tip.accuracy".into());
        if let Ok(saved) = self.settings.set(s) {
            let _ = self.app.emit("settings-changed", &saved);
        }
        "Didn't catch that. A larger speech model often helps — switch it in Settings → Models."
            .into()
    }

    // ---- Hotkey entry points -------------------------------------------

    pub fn on_hotkey_pressed(self: &Arc<Self>, job: Job, mode_override: Option<String>) {
        let status = self.state().status;
        match status {
            Status::Recording => {
                // Second press while hands-free recording → stop & process.
                let same_job = self
                    .session
                    .lock()
                    .expect("pipeline state poisoned")
                    .as_ref()
                    .map(|s| s.job == job)
                    .unwrap_or(false);
                if same_job {
                    self.finish();
                }
            }
            Status::Idle | Status::Error | Status::Notice => {
                if let Err(err) = self.start(job, mode_override) {
                    log::warn!("could not start {job:?}: {err}");
                    self.set_transient(Status::Error, err.user_message());
                }
            }
            // Busy processing a previous utterance — ignore.
            _ => {}
        }
    }

    pub fn on_hotkey_released(self: &Arc<Self>, job: Job) {
        let behavior = match job {
            Job::Dictation => self.settings.get().dictation_hotkey_behavior,
            // Refinement is always push-to-talk style.
            Job::RefineSelection => HotkeyBehavior::Hold,
            // Polish is a tap; there is no recording to stop.
            Job::PolishSelection => return,
        };
        if behavior == HotkeyBehavior::Toggle {
            return;
        }
        let held_for = {
            let session = self.session.lock().expect("pipeline state poisoned");
            match session.as_ref() {
                Some(s) if s.job == job => s.started.elapsed(),
                _ => return,
            }
        };
        // A quick tap means "hands-free": keep recording until the next press.
        if held_for < TAP_THRESHOLD {
            return;
        }
        self.finish();
    }

    // ---- Lifecycle ------------------------------------------------------

    pub fn start(self: &Arc<Self>, job: Job, mode_override: Option<String>) -> AppResult<()> {
        {
            let state = self.state();
            if !matches!(state.status, Status::Idle | Status::Error | Status::Notice) {
                return Err(AppError::State(
                    "Still finishing the last one — try again in a moment.".into(),
                ));
            }
        }

        let settings = self.settings.get();

        // Per-app rules (07 §9): for a plain dictation (no explicit mode hotkey),
        // a frontmost-app rule supplies a one-shot mode override, exactly like a
        // hotkey. Explicit overrides win; a detection failure just means no rule.
        let mode_override = match mode_override {
            Some(id) => Some(id),
            None if job == Job::Dictation => match apps::frontmost_app() {
                Some((bundle_id, name)) => {
                    *self.last_app.lock().expect("pipeline state poisoned") =
                        Some((bundle_id.clone(), name));
                    settings
                        .app_rules
                        .iter()
                        .find(|r| r.bundle_id == bundle_id)
                        .map(|r| r.mode_id.clone())
                }
                None => None,
            },
            None => None,
        };

        // A cloud engine needs no local model; only gate the on-device default.
        if !settings.stt_model_id.starts_with("cloud:")
            && !self.models.is_installed(&settings.stt_model_id)
        {
            return Err(AppError::Model(
                "No speech model yet — open Settings to download one.".into(),
            ));
        }

        let selection = if job == Job::RefineSelection {
            if self
                .profiles
                .active(&settings.active_llm_profile_id)
                .is_none()
            {
                return Err(AppError::Llm(
                    "Rewrite needs an AI profile — add one in Settings.".into(),
                ));
            }
            match self.output.capture_selection()? {
                Some(text) => Some(text),
                None => {
                    return Err(AppError::State(
                        "Select some text first, then hold the rewrite hotkey.".into(),
                    ))
                }
            }
        } else {
            None
        };

        // Name the mode in the listening label so the user sees which mode will
        // write before speaking (07 §5) — the override mode for a mode hotkey,
        // else the active mode. Rewrite is an action, not a mode.
        let recording_label = if job == Job::Dictation {
            let mode_name = mode_override
                .as_ref()
                .and_then(|id| settings.modes.iter().find(|m| &m.id == id))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| settings.active_mode().name);
            Some(truncate_mode_name(&mode_name))
        } else {
            None
        };

        self.audio.start()?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.session.lock().expect("pipeline state poisoned") = Some(Session {
            job,
            generation,
            started: Instant::now(),
            selection,
            mode_override,
        });
        self.set_state(Status::Recording, Some(job), recording_label);
        // Bind Esc so a recording started by mistake has a "never mind".
        shortcuts::set_cancel_key(&self.app, true);
        hud::position_on_cursor_monitor(&self.app);

        // Level meter for the HUD while recording.
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(60)).await;
                if pipeline.generation.load(Ordering::SeqCst) != generation
                    || pipeline.state().status != Status::Recording
                {
                    break;
                }
                let _ = pipeline.app.emit(AUDIO_LEVEL_EVENT, pipeline.audio.level());
            }
        });

        // Hard cap on recording length.
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(MAX_RECORDING_SECS)).await;
            let still_same = pipeline.generation.load(Ordering::SeqCst) == generation
                && pipeline.state().status == Status::Recording;
            if still_same {
                log::info!("max recording length reached; stopping automatically");
                pipeline.finish();
            }
        });

        Ok(())
    }

    /// Stop recording and run the rest of the pipeline asynchronously.
    pub fn finish(self: &Arc<Self>) {
        let Some(session) = self.session.lock().expect("pipeline state poisoned").take() else {
            return;
        };
        // Recording is over; Esc-to-cancel is scoped to recording only.
        shortcuts::set_cancel_key(&self.app, false);
        self.set_state(Status::Transcribing, Some(session.job), None);

        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let generation = session.generation;
            let result = pipeline.process(session).await;
            if pipeline.generation.load(Ordering::SeqCst) != generation {
                return; // cancelled mid-flight; a newer job owns the UI now
            }
            match result {
                Ok(ProcessOutcome::Inserted(preview, hud_tip)) => {
                    pipeline.set_success(preview, hud_tip);
                }
                Ok(ProcessOutcome::Notice(message)) => {
                    pipeline.set_transient(Status::Notice, message);
                }
                Err(err) => {
                    log::warn!("job failed: {err}");
                    pipeline.set_transient(Status::Error, err.user_message());
                }
            }
        });
    }

    pub fn cancel(self: &Arc<Self>) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.session.lock().expect("pipeline state poisoned") = None;
        self.audio.cancel();
        shortcuts::set_cancel_key(&self.app, false);
        self.set_state(Status::Idle, None, None);
    }

    /// Surfaces a transient notice in the HUD for callers outside the pipeline
    /// (e.g. the tray acting on nothing), reusing the existing notice surface
    /// rather than inventing another. Skipped while a job owns the HUD.
    pub fn flash_notice(self: &Arc<Self>, message: String) {
        if matches!(
            self.state().status,
            Status::Idle | Status::Notice | Status::Error | Status::Inserted
        ) {
            self.set_transient(Status::Notice, message);
        }
    }

    /// Tap entry point: refine the current selection with the built-in
    /// instruction — no recording, no Session. Runs under the same busy-state
    /// and generation contract as every other job; errors surface as
    /// transient HUD states. Blocks on selection capture, so callers must
    /// stay off the main thread (capture round-trips keystrokes through it).
    pub fn polish(self: &Arc<Self>) {
        if !matches!(
            self.state().status,
            Status::Idle | Status::Error | Status::Notice
        ) {
            return; // busy with another job
        }
        let settings = self.settings.get();
        let Some(profile) = self.profiles.active(&settings.active_llm_profile_id) else {
            self.set_transient(
                Status::Error,
                "Polishing needs an AI profile — add one in Settings.".into(),
            );
            return;
        };

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.set_state(Status::Refining, Some(Job::PolishSelection), None);
        hud::position_on_cursor_monitor(&self.app);

        let selection = match self.output.capture_selection() {
            Ok(Some(text)) => text,
            Ok(None) => {
                self.set_transient(
                    Status::Error,
                    "Select some text first, then press the polish hotkey.".into(),
                );
                return;
            }
            Err(err) => {
                log::warn!("polish selection capture failed: {err}");
                self.set_transient(Status::Error, err.user_message());
                return;
            }
        };
        if self.generation.load(Ordering::SeqCst) != generation {
            return; // cancelled while capturing
        }

        let started = Instant::now();
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let result = pipeline
                .finish_polish(&settings, &profile, selection, started)
                .await;
            if pipeline.generation.load(Ordering::SeqCst) != generation {
                return; // cancelled mid-flight; a newer job owns the UI now
            }
            match result {
                Ok(ProcessOutcome::Inserted(preview, hud_tip)) => {
                    pipeline.set_success(preview, hud_tip);
                }
                Ok(ProcessOutcome::Notice(message)) => {
                    pipeline.set_transient(Status::Notice, message);
                }
                Err(err) => {
                    log::warn!("job failed: {err}");
                    pipeline.set_transient(Status::Error, err.user_message());
                }
            }
        });
    }

    // ---- Processing -----------------------------------------------------

    /// Resolves a dictation mode's overrides once, at job start (07 §3). Each
    /// field falls back `mode override (set AND valid) → global → default`; a
    /// dangling override never fails the job — it falls through and the first
    /// dangling field (AI profile → STT → language) yields one informational
    /// notice that names the mode.
    fn resolve_dictation(&self, settings: &Settings, mode: &Mode) -> ResolvedDictation {
        let mut notice: Option<String> = None;
        let name = truncate_mode_name(&mode.name);

        // AI profile is only resolved when the mode wants AI and the master
        // switch is on, so a no-AI mode never notices a dangling profile.
        let profile = if mode.uses_llm && settings.refine_after_dictation {
            match &mode.ai_profile_id {
                Some(id) => self.profiles.get(id).or_else(|| {
                    notice.get_or_insert(format!(
                        "{name}: its AI profile is missing — used your active profile instead."
                    ));
                    self.profiles.active(&settings.active_llm_profile_id)
                }),
                None => self.profiles.active(&settings.active_llm_profile_id),
            }
        } else {
            None
        };

        let stt_model_id = match &mode.stt_model_id {
            Some(id) if self.stt_model_valid(settings, id) => id.clone(),
            Some(_) => {
                notice.get_or_insert(format!(
                    "{name}: its speech model isn’t available — used your default model instead."
                ));
                settings.stt_model_id.clone()
            }
            None => settings.stt_model_id.clone(),
        };

        let language = match &mode.language {
            Some(lang) if is_known_language(lang) => lang.clone(),
            Some(_) => {
                notice.get_or_insert(format!(
                    "{name}: its language setting was invalid — used your default."
                ));
                settings.language.clone()
            }
            None => settings.language.clone(),
        };

        ResolvedDictation {
            mode: mode.clone(),
            stt_model_id,
            language,
            profile,
            notice,
        }
    }

    /// Whether a resolved STT model id is usable: a `cloud:<profileId>` id needs
    /// the profile present AND its consent confirmed (08 §3); a bare id needs the
    /// whisper model installed.
    fn stt_model_valid(&self, settings: &Settings, id: &str) -> bool {
        match id.strip_prefix("cloud:") {
            Some(pid) => {
                self.stt_profiles.get(pid).is_some()
                    && settings.confirmed_stt_profiles.iter().any(|c| c == pid)
            }
            None => self.models.is_installed(id),
        }
    }

    /// An installed local whisper model to fall back to (08 §4.3) — the global
    /// if it is local and installed, else any installed model.
    fn installed_local_model(&self, settings: &Settings) -> Option<String> {
        if !settings.stt_model_id.starts_with("cloud:")
            && self.models.is_installed(&settings.stt_model_id)
        {
            return Some(settings.stt_model_id.clone());
        }
        self.models
            .list()
            .into_iter()
            .find(|m| m.installed)
            .map(|m| m.id.to_string())
    }

    /// Today's local transcription: whisper inference inside spawn_blocking.
    async fn local_transcribe(
        self: &Arc<Self>,
        model_id: String,
        samples: Vec<f32>,
        language: String,
        prompt: Option<String>,
    ) -> AppResult<String> {
        let model_path = self.models.path_for(&model_id)?;
        let stt = Arc::clone(&self.stt);
        tauri::async_runtime::spawn_blocking(move || {
            stt.transcribe(
                &model_id,
                &model_path,
                &samples,
                &language,
                prompt.as_deref(),
            )
        })
        .await
        .map_err(|e| AppError::Stt(format!("transcription join failed: {e}")))?
    }

    async fn process(self: &Arc<Self>, session: Session) -> AppResult<ProcessOutcome> {
        let settings = self.settings.get();
        let job = session.job;
        let started = session.started;

        // Resolve mode overrides once, up front (dictation only). A per-mode
        // hotkey resolves its own mode for this job only; otherwise the active
        // mode. Rewrite uses the global speech model + language.
        let resolved = if job == Job::Dictation {
            let mode = session
                .mode_override
                .as_ref()
                .and_then(|id| settings.modes.iter().find(|m| &m.id == id).cloned())
                .unwrap_or_else(|| settings.active_mode());
            Some(self.resolve_dictation(&settings, &mode))
        } else {
            None
        };

        // Stop capture and get 16 kHz samples (resampled on the audio thread).
        let audio = Arc::clone(&self.audio);
        let recorded = tauri::async_runtime::spawn_blocking(move || audio.stop())
            .await
            .map_err(|e| AppError::State(format!("audio join failed: {e}")))??;

        let model_id = match &resolved {
            Some(r) => r.stt_model_id.clone(),
            None => settings.stt_model_id.clone(),
        };
        let language = match &resolved {
            Some(r) => r.language.clone(),
            None => settings.language.clone(),
        };
        let prompt = initial_prompt_from_dictionary(&settings.dictionary);
        log::info!(
            "recorded {:.1}s of audio ({} samples at 16 kHz)",
            recorded.duration.as_secs_f32(),
            recorded.samples.len()
        );
        let samples = recorded.samples;

        let raw = match model_id.strip_prefix("cloud:") {
            // Cloud STT uploads audio off the Mac. Gate: the profile must exist
            // AND its consent be confirmed, or nothing is uploaded (08 §3).
            Some(pid) => {
                let confirmed = settings.confirmed_stt_profiles.iter().any(|c| c == pid);
                match (self.stt_profiles.get(pid), confirmed) {
                    (Some(profile), true) => {
                        match cloud_stt::transcribe(&profile, &samples, &language, prompt.as_deref())
                            .await
                        {
                            Ok(text) => text,
                            // Failure policy (08 §4.3): fall back only toward LESS
                            // data leaving — on-device whisper if installed, never
                            // escalating. Output is never silently dropped.
                            Err(err) => match self.installed_local_model(&settings) {
                                Some(local_id) => {
                                    log::warn!("cloud STT failed, used on-device instead: {err}");
                                    self.local_transcribe(local_id, samples, language, prompt)
                                        .await?
                                }
                                None => {
                                    return Err(AppError::Stt(format!(
                                        "Cloud transcription failed ({err}). Switch to the on-device model in Settings."
                                    )))
                                }
                            },
                        }
                    }
                    _ => {
                        return Ok(ProcessOutcome::Notice(
                            "Cloud transcription isn't set up — choose a speech engine in Settings → Models."
                                .into(),
                        ))
                    }
                }
            }
            // Local whisper.cpp — exactly today's path.
            None => {
                self.local_transcribe(model_id, samples, language, prompt)
                    .await?
            }
        };

        let cleaned = text::clean_transcript(&raw);
        if cleaned.is_empty() {
            // The accuracy tip is about dictation; an empty Rewrite instruction
            // is a different situation, so only dictation feeds the streak.
            let message = if job == Job::Dictation {
                let streak = self.empty_streak.fetch_add(1, Ordering::SeqCst) + 1;
                self.empty_dictation_notice(streak)
            } else {
                "Didn't catch that — try again.".into()
            };
            return Ok(ProcessOutcome::Notice(message));
        }
        if job == Job::Dictation {
            self.empty_streak.store(0, Ordering::SeqCst);
        }
        let with_dictionary = text::apply_dictionary(&cleaned, &settings.dictionary);

        match job {
            Job::Dictation => {
                let resolved = resolved.expect("dictation always resolves a mode");
                self.finish_dictation(&settings, resolved, with_dictionary, started)
                    .await
            }
            Job::RefineSelection => {
                self.finish_refine(
                    &settings,
                    session.selection.unwrap_or_default(),
                    with_dictionary,
                    started,
                )
                .await
            }
            // Sessions are only created for recording jobs; polish runs
            // through `polish()` without one.
            Job::PolishSelection => Err(AppError::State(
                "polish does not use a recording session".into(),
            )),
        }
    }

    async fn finish_dictation(
        self: &Arc<Self>,
        settings: &Settings,
        resolved: ResolvedDictation,
        transcript: String,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        let ResolvedDictation {
            mode,
            profile,
            notice,
            ..
        } = resolved;

        let mut refined = false;
        let mut llm_warning: Option<String> = None;
        let final_text = if let Some(profile) = profile {
            self.set_state(Status::Refining, Some(Job::Dictation), None);
            let system = modes::dictation_system_prompt(&mode, &settings.dictionary);
            match self.llm.chat(&profile, &system, &transcript).await {
                Ok(text) => {
                    refined = true;
                    text
                }
                Err(err) => {
                    // Never lose a dictation to a flaky provider: fall back to
                    // the rules-based cleanup and tell the user.
                    log::warn!("LLM refinement failed, falling back to rules: {err}");
                    llm_warning =
                        Some("AI cleanup unavailable — inserted the plain transcript.".into());
                    text::apply_rules_cleanup(&transcript)
                }
            }
        } else if mode.id == LITERAL_MODE_ID {
            transcript.clone()
        } else {
            text::apply_rules_cleanup(&transcript)
        };

        let outcome = self.insert(
            settings,
            &transcript,
            final_text,
            &mode.id,
            refined,
            started,
        )?;
        // Count this successful dictation — the only tip-system counter (05); a
        // count, never a log. Emit so an open settings webview re-evaluates tips.
        let mut counted = self.settings.get();
        counted.dictation_count = counted.dictation_count.saturating_add(1);
        if let Ok(saved) = self.settings.set(counted) {
            let _ = self.app.emit("settings-changed", &saved);
        }
        // Opt-in history: log the text (never audio) when the user turned it on.
        if settings.history_enabled {
            if let Some(result) = self.last_result() {
                self.history
                    .append(result.raw, result.text, result.mode_id, result.refined);
            }
        }
        // The text already pasted; both notices are informational. A flaky-LLM
        // warning is more useful than a dangling-override notice, so it wins.
        if let Some(warning) = llm_warning {
            return Ok(ProcessOutcome::Notice(warning));
        }
        if let Some(notice) = notice {
            return Ok(ProcessOutcome::Notice(notice));
        }
        // A clean insert may carry a one-time HUD tip; a clipboard-fallback
        // Notice does not.
        Ok(match outcome {
            ProcessOutcome::Inserted(preview, _) => {
                ProcessOutcome::Inserted(preview, self.dictation_hud_tip())
            }
            other => other,
        })
    }

    async fn finish_refine(
        self: &Arc<Self>,
        settings: &Settings,
        selection: String,
        instruction: String,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        self.set_state(Status::Refining, Some(Job::RefineSelection), None);
        // Re-resolved at use time: the profile may have changed mid-recording.
        let Some(profile) = self.profiles.active(&settings.active_llm_profile_id) else {
            return Err(AppError::Llm(
                "Rewrite needs an AI profile — add one in Settings.".into(),
            ));
        };
        let system = modes::selection_system_prompt();
        let user = modes::selection_user_prompt(&selection, &instruction);
        // Unlike dictation there is no fallback: replacing the user's
        // selection with something other than what they asked for is worse
        // than doing nothing.
        let rewritten = self.llm.chat(&profile, &system, &user).await?;
        let outcome = self.insert(settings, &instruction, rewritten, "refine", true, started)?;
        Ok(match outcome {
            ProcessOutcome::Inserted(preview, _) => {
                ProcessOutcome::Inserted(preview, self.refine_hud_tip())
            }
            other => other,
        })
    }

    async fn finish_polish(
        self: &Arc<Self>,
        settings: &Settings,
        profile: &LlmProfile,
        selection: String,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        let system = modes::selection_system_prompt();
        // An empty instruction selects the built-in fix-grammar default.
        let user = modes::selection_user_prompt(&selection, "");
        // Like rewrite: no fallback — wrong text over the user's selection is
        // worse than nothing.
        let polished = self.llm.chat(profile, &system, &user).await?;
        self.insert(settings, &selection, polished, "polish", true, started)
    }

    fn insert(
        self: &Arc<Self>,
        settings: &Settings,
        raw: &str,
        final_text: String,
        mode_id: &str,
        refined: bool,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        self.set_state(Status::Inserting, None, None);
        let outcome = self.output.insert(
            final_text.clone(),
            settings.insert_method,
            settings.restore_clipboard,
        )?;

        let result = TranscriptionResult {
            raw: raw.to_string(),
            text: final_text,
            mode_id: mode_id.to_string(),
            refined,
            duration_ms: started.elapsed().as_millis() as u64,
        };
        *self.last_result.lock().expect("pipeline state poisoned") = Some(result.clone());
        let _ = self.app.emit(RESULT_EVENT, &result);

        match outcome {
            InsertOutcome::Pasted | InsertOutcome::CopiedToClipboard(CopyReason::ChosenMethod) => {
                Ok(ProcessOutcome::Inserted(result.text.clone(), None))
            }
            InsertOutcome::CopiedToClipboard(CopyReason::NoAccessibility) => {
                Ok(ProcessOutcome::Notice(
                    "Copied to clipboard — press ⌘V to paste (grant Accessibility to auto-paste)"
                        .into(),
                ))
            }
            InsertOutcome::CopiedToClipboard(CopyReason::PasteFailed) => {
                Ok(ProcessOutcome::Notice(
                    "Paste didn't go through — the result is on the clipboard, press ⌘V".into(),
                ))
            }
        }
    }
}

/// A dictation mode's overrides resolved against the globals (07 §3).
struct ResolvedDictation {
    mode: Mode,
    stt_model_id: String,
    language: String,
    profile: Option<LlmProfile>,
    /// First dangling-override notice, if any; informational, never fatal.
    notice: Option<String>,
}

/// Mode names render up to 16 chars in the HUD and dangling notices (07 §5);
/// the pill must not resize.
fn truncate_mode_name(name: &str) -> String {
    const MAX: usize = 16;
    if name.chars().count() > MAX {
        let head: String = name.chars().take(MAX - 1).collect();
        format!("{head}…")
    } else {
        name.to_string()
    }
}

/// A mode language override is valid if it is `auto` or a 2-letter code; a
/// hand-edited garbage value falls back to the global language (07 §3).
fn is_known_language(lang: &str) -> bool {
    lang == "auto" || (lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase()))
}

enum ProcessOutcome {
    /// Inserted cleanly; carries the text for the success flash and an optional
    /// one-time HUD tip.
    Inserted(String, Option<String>),
    Notice(String),
}

#[cfg(test)]
mod tests {
    use super::{is_known_language, truncate_mode_name};

    #[test]
    fn truncates_long_mode_names_to_16_chars() {
        assert_eq!(truncate_mode_name("Notes"), "Notes");
        let long = truncate_mode_name("Quarterly Board Update");
        assert_eq!(long.chars().count(), 16);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn language_override_validity() {
        assert!(is_known_language("auto"));
        assert!(is_known_language("de"));
        assert!(!is_known_language("English"));
        assert!(!is_known_language("EN"));
        assert!(!is_known_language(""));
    }
}
