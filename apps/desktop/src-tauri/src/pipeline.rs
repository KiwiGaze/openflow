//! The dictation pipeline state machine.
//!
//! One pipeline instance owns the flow `record → transcribe → (refine) →
//! insert` for dictation plus the no-recording selection jobs (polish,
//! transforms), emits progress events to the webviews, and guards against
//! stale work with a generation counter — cancelling simply bumps the
//! generation.

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
use crate::modes;
use crate::output::{CopyReason, InsertOutcome, OutputSystem};
use crate::profiles::{LlmProfile, ProfileManager};
use crate::settings::{
    HotkeyBehavior, Mode, Settings, SettingsManager, MAX_RECORDING_SECS, SETTINGS_CHANGED_EVENT,
};
use crate::shortcuts;
use crate::stats::{word_count, Insights, Stats};
use crate::stt::{initial_prompt_from_dictionary, SttEngine};
use crate::stt_profiles::{SttProfileManager, CLOUD_STT_PREFIX};
use crate::suggestions::{DictionarySuggestion, Suggestions};
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
    /// Refine the selection with the built-in instruction — no recording.
    PolishSelection,
    /// Apply a user-defined transform's instruction to the selection — no
    /// recording. Which transform is resolved at dispatch time by id.
    Transform,
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
    /// The "before" the change-overlay diffs against: the transcript for
    /// dictation, the original selection for polish/transforms.
    pub original: String,
    pub text: String,
    pub mode_id: String,
    pub refined: bool,
    pub duration_ms: u64,
}

struct Session {
    job: Job,
    generation: u64,
    started: Instant,
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
    /// Session-only usage aggregates for the Insights view (in-RAM, never
    /// persisted or transmitted).
    stats: Stats,
    /// Session-only candidate-term tally driving dictionary suggestions
    /// (in-RAM, never persisted or transmitted).
    suggestions: Suggestions,
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
            stats: Stats::new(),
            suggestions: Suggestions::new(),
        })
    }

    pub fn last_app(&self) -> Option<(String, String)> {
        self.last_app
            .lock()
            .expect("pipeline state poisoned")
            .clone()
    }

    /// Snapshot of this session's usage aggregates for the Insights view.
    pub fn insights(&self) -> Insights {
        self.stats.snapshot()
    }

    /// Top dictionary suggestions seen this session, excluding known terms.
    pub fn dictionary_suggestions(
        &self,
        dictionary: &[crate::settings::DictionaryEntry],
        limit: usize,
    ) -> Vec<DictionarySuggestion> {
        self.suggestions.top(dictionary, limit)
    }

    /// Suppresses a suggested term for the rest of the session.
    pub fn dismiss_suggestion(&self, term: &str) {
        self.suggestions.dismiss(term);
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
        // Mirror the live-mic state into the menu bar (privacy signal).
        crate::tray::set_recording(&self.app, matches!(status, Status::Recording));
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

    fn mark_tip_shown(&self, id: &str, mut settings: Settings) {
        self.tip_shown_session.store(true, Ordering::SeqCst);
        settings.tips_seen.push(id.to_string());
        if let Ok(saved) = self.settings.set(settings) {
            let _ = self.app.emit(SETTINGS_CHANGED_EVENT, &saved);
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
            || s.stt_model_id.starts_with(CLOUD_STT_PREFIX)
        {
            return BASE.into();
        }
        s.tips_seen.push("tip.accuracy".into());
        if let Ok(saved) = self.settings.set(s) {
            let _ = self.app.emit(SETTINGS_CHANGED_EVENT, &saved);
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
                    .is_some_and(|s| s.job == job);
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
        // Polish and transforms are taps; only dictation records.
        if job != Job::Dictation {
            return;
        }
        if self.settings.get().dictation_hotkey_behavior == HotkeyBehavior::Toggle {
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

    /// Starts a recording job. Only dictation records; polish and transforms
    /// capture the selection without one and never come through here.
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
            None => match apps::frontmost_app() {
                Some((bundle_id, name)) => {
                    *self.last_app.lock().expect("pipeline state poisoned") =
                        Some((bundle_id.clone(), name));
                    settings
                        .app_rules
                        .iter()
                        .find(|r| r.bundle_id == bundle_id)
                        .map(|r| r.mode_id.clone())
                }
                None => {
                    // Detection failed: clear the stale app so the "add a rule"
                    // flow can't target whatever was frontmost last time.
                    *self.last_app.lock().expect("pipeline state poisoned") = None;
                    None
                }
            },
        };

        // Preflight the model this job will actually use: a dictation mode can
        // override the speech model (07 §3), so checking the global default
        // would wrongly block a valid per-mode override. A cloud engine needs
        // no local model.
        let mode = mode_override
            .as_ref()
            .and_then(|id| settings.modes.iter().find(|m| &m.id == id).cloned())
            .unwrap_or_else(|| settings.active_mode());
        let effective_stt = match &mode.stt_model_id {
            Some(id) if self.stt_model_valid(&settings, id) => id.clone(),
            _ => settings.stt_model_id.clone(),
        };
        if !effective_stt.starts_with(CLOUD_STT_PREFIX) && !self.models.is_installed(&effective_stt)
        {
            return Err(AppError::Model(
                "No speech model yet — open Settings to download one.".into(),
            ));
        }

        // Name the mode in the listening label so the user sees which mode will
        // write before speaking (07 §5) — the override mode for a mode hotkey,
        // else the active mode.
        let recording_label = Some(truncate_mode_name(&mode.name));

        self.audio.start()?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.session.lock().expect("pipeline state poisoned") = Some(Session {
            job,
            generation,
            started: Instant::now(),
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
            pipeline.publish_outcome(result);
        });
    }

    /// Renders a finished job's outcome as HUD state. Callers must re-check
    /// the generation first — publishing is the final step of every job.
    fn publish_outcome(self: &Arc<Self>, result: AppResult<ProcessOutcome>) {
        match result {
            Ok(ProcessOutcome::Inserted(preview, hud_tip)) => {
                self.set_success(preview, hud_tip);
            }
            Ok(ProcessOutcome::Notice(message)) => {
                self.set_transient(Status::Notice, message);
            }
            Err(err) => {
                log::warn!("job failed: {err}");
                self.set_transient(Status::Error, err.user_message());
            }
        }
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

    /// Tap entry point: polish the current selection with the built-in
    /// fix-grammar instruction — no recording, no Session.
    pub fn polish(self: &Arc<Self>) {
        self.refine_selection(Job::PolishSelection, String::new(), "polish", None);
    }

    /// Tap entry point: apply a user-defined transform to the selection. The
    /// transform is resolved by id at dispatch time so edits take effect
    /// without re-binding the hotkey; a transform deleted between keypress and
    /// dispatch is a silent no-op (its hotkey is already being unregistered).
    pub fn run_transform(self: &Arc<Self>, transform_id: &str) {
        let Some(transform) = self
            .settings
            .get()
            .transforms
            .into_iter()
            .find(|t| t.id == transform_id)
        else {
            return;
        };
        self.refine_selection(
            Job::Transform,
            transform.instruction,
            "transform",
            Some(transform.name),
        );
    }

    /// Shared body for the no-recording selection jobs (Polish, Transform):
    /// resolve the active profile, capture the selection, then refine it with
    /// `instruction` and insert. Runs under the same busy-state and generation
    /// contract as every other job; errors surface as transient HUD states.
    /// Blocks on selection capture, so callers must stay off the main thread
    /// (capture round-trips keystrokes through it).
    fn refine_selection(
        self: &Arc<Self>,
        job: Job,
        instruction: String,
        mode_id: &'static str,
        hud_label: Option<String>,
    ) {
        if !matches!(
            self.state().status,
            Status::Idle | Status::Error | Status::Notice
        ) {
            return; // busy with another job
        }
        let settings = self.settings.get();
        let Some(profile) = self.profiles.active(&settings.active_llm_profile_id) else {
            let what = if job == Job::PolishSelection {
                "Polishing"
            } else {
                "This transform"
            };
            self.set_transient(
                Status::Error,
                format!("{what} needs an AI profile — add one in Settings."),
            );
            return;
        };

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        // The transform name rides in the message so the HUD can read
        // "Concise…" instead of a generic label.
        self.set_state(Status::Refining, Some(job), hud_label);
        hud::position_on_cursor_monitor(&self.app);

        let selection = match self.output.capture_selection() {
            Ok(Some(text)) => text,
            Ok(None) => {
                let key = if job == Job::PolishSelection {
                    "the polish hotkey"
                } else {
                    "the transform's hotkey"
                };
                self.set_transient(
                    Status::Error,
                    format!("Select some text first, then press {key}."),
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
                .finish_selection_refine(
                    &settings,
                    &profile,
                    selection,
                    &instruction,
                    mode_id,
                    started,
                )
                .await;
            if pipeline.generation.load(Ordering::SeqCst) != generation {
                return; // cancelled mid-flight; a newer job owns the UI now
            }
            pipeline.publish_outcome(result);
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
        match id.strip_prefix(CLOUD_STT_PREFIX) {
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
        if !settings.stt_model_id.starts_with(CLOUD_STT_PREFIX)
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

        // Resolve mode overrides once, up front. A per-mode hotkey resolves its
        // own mode for this job only; otherwise the active mode.
        let mode = session
            .mode_override
            .as_ref()
            .and_then(|id| settings.modes.iter().find(|m| &m.id == id).cloned())
            .unwrap_or_else(|| settings.active_mode());
        let resolved = self.resolve_dictation(&settings, &mode);

        // Stop capture and get 16 kHz samples (resampled on the audio thread).
        let audio = Arc::clone(&self.audio);
        let recorded = tauri::async_runtime::spawn_blocking(move || audio.stop())
            .await
            .map_err(|e| AppError::State(format!("audio join failed: {e}")))??;

        let model_id = resolved.stt_model_id.clone();
        let language = resolved.language.clone();
        let prompt = initial_prompt_from_dictionary(&settings.dictionary);
        log::info!(
            "recorded {:.1}s of audio ({} samples at 16 kHz)",
            recorded.duration.as_secs_f32(),
            recorded.samples.len()
        );
        // Speech duration drives the Insights pace (words ÷ minutes spoken).
        let record_ms = recorded.duration.as_millis() as u64;
        let samples = recorded.samples;

        let raw = match model_id.strip_prefix(CLOUD_STT_PREFIX) {
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
            let streak = self.empty_streak.fetch_add(1, Ordering::SeqCst) + 1;
            return Ok(ProcessOutcome::Notice(self.empty_dictation_notice(streak)));
        }
        self.empty_streak.store(0, Ordering::SeqCst);
        let with_dictionary = text::apply_dictionary(&cleaned, &settings.dictionary);

        match job {
            Job::Dictation => {
                self.finish_dictation(&settings, resolved, with_dictionary, started, record_ms)
                    .await
            }
            // Sessions are only created for recording jobs; polish and
            // transforms run through `refine_selection()` without one.
            Job::PolishSelection | Job::Transform => Err(AppError::State(
                "selection refinement does not use a recording session".into(),
            )),
        }
    }

    async fn finish_dictation(
        self: &Arc<Self>,
        settings: &Settings,
        resolved: ResolvedDictation,
        transcript: String,
        started: Instant,
        record_ms: u64,
    ) -> AppResult<ProcessOutcome> {
        // Watch for distinctive terms worth suggesting for the dictionary.
        self.suggestions.observe(&transcript);
        // Mode, profile, and language were resolved once at job start (07 §3).
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
        } else {
            modes::no_ai_output(&mode.id, &transcript)
        };

        // Snippets expand on the final text only: dictation-only, verbatim,
        // and after any LLM pass so the expansion is never reworded. Selection
        // jobs (polish, transforms) edit existing text and deliberately skip this.
        let final_text = text::apply_snippets(&final_text, &settings.snippets);

        let words = word_count(&final_text);
        // Clone only when the opt-in history will consume it below — `insert`
        // takes the text by move.
        let history_text = settings.history_enabled.then(|| final_text.clone());
        let outcome = self.insert(
            settings,
            &transcript,
            &transcript,
            final_text,
            &mode.id,
            refined,
            started,
        )?;
        // Record the session aggregate only once the text actually reached the
        // user (insert returns Ok even on the clipboard fallback).
        self.stats
            .record_dictation(words, record_ms, &mode.id, refined);
        // Count this successful dictation — the only tip-system counter (05); a
        // count, never a log. Emit so an open settings webview re-evaluates tips.
        let mut counted = self.settings.get();
        counted.dictation_count = counted.dictation_count.saturating_add(1);
        if let Ok(saved) = self.settings.set(counted) {
            let _ = self.app.emit(SETTINGS_CHANGED_EVENT, &saved);
        }
        // Opt-in history: log the text (never audio) when the user turned it
        // on. Uses the in-scope values rather than re-reading `last_result` —
        // `insert` stores the same text verbatim, so these are identical.
        if let Some(text) = history_text {
            self.history
                .append(transcript.clone(), text, mode.id.clone(), refined);
        }
        // A clipboard fallback (paste failed / no Accessibility) is the
        // highest-priority message — the user must know to press ⌘V to get
        // their text. AI/override notes only matter once the text auto-pasted;
        // between those two a flaky-LLM warning beats a dangling-override note.
        match outcome {
            ProcessOutcome::Inserted(preview, _) => {
                if let Some(warning) = llm_warning {
                    Ok(ProcessOutcome::Notice(warning))
                } else if let Some(notice) = notice {
                    Ok(ProcessOutcome::Notice(notice))
                } else {
                    Ok(ProcessOutcome::Inserted(preview, self.dictation_hud_tip()))
                }
            }
            // Clipboard-fallback notice from insert(): surface it, never hide it.
            clipboard_notice => Ok(clipboard_notice),
        }
    }

    async fn finish_selection_refine(
        self: &Arc<Self>,
        settings: &Settings,
        profile: &LlmProfile,
        selection: String,
        instruction: &str,
        mode_id: &str,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        let system = modes::selection_system_prompt();
        // An empty instruction selects the built-in fix-grammar default
        // (Polish); a transform passes its own instruction.
        let user = modes::selection_user_prompt(&selection, instruction);
        // No fallback — wrong text over the user's selection is worse than
        // nothing.
        let result = self.llm.chat(profile, &system, &user).await?;
        // `original` is the text the diff is measured against — the selection.
        self.insert(
            settings, &selection, &selection, result, mode_id, true, started,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(
        self: &Arc<Self>,
        settings: &Settings,
        raw: &str,
        original: &str,
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
            original: original.to_string(),
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
