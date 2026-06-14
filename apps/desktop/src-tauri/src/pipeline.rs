//! The dictation pipeline state machine.
//!
//! One pipeline instance owns the flow `record → transcribe → (transform) →
//! insert` for dictation plus the no-recording selection jobs (per-prompt
//! shortcuts), emits progress events to the webviews, and guards against stale
//! work with a generation counter — cancelling simply bumps the generation.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::apps;
use crate::audio::AudioSystem;
use crate::cloud_stt;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::history::HistoryStore;
use crate::hud;
use crate::llm::LlmClient;
use crate::models::ModelManager;
use crate::output::{CopyReason, InsertOutcome, OutputSystem};
use crate::profiles::{LlmProfile, ProfileManager};
use crate::prompts;
use crate::settings::{
    HotkeyBehavior, InsertMethod, Prompt, Settings, SettingsManager, MAX_RECORDING_SECS,
    SETTINGS_CHANGED_EVENT,
};
use crate::shortcuts;
use crate::stats::{local_day, word_count};
use crate::stt::{initial_prompt_from_dictionary, SttEngine};
use crate::stt_profiles::{SttProfileManager, CLOUD_STT_PREFIX};
use crate::suggestions::{DictionarySuggestion, Suggestions};
use crate::text;

pub const PIPELINE_STATE_EVENT: &str = "pipeline-state";
pub const AUDIO_LEVEL_EVENT: &str = "audio-level";
pub const RESULT_EVENT: &str = "transcription-result";
/// Fired (no payload) once a history append has actually committed to the DB,
/// so views refresh from durable rows rather than racing the write. Mirrored as
/// `EVENTS.historyChanged` in `@velata/core`.
pub const HISTORY_CHANGED_EVENT: &str = "history-changed";
/// Fired (no payload) once the `insights_daily` upsert has committed, so the
/// Home header refetches from the durable row rather than racing the off-thread
/// write. Mirrored as `EVENTS.insightsChanged` in `@velata/core`.
pub const INSIGHTS_CHANGED_EVENT: &str = "insights-changed";

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
    Polishing,
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
    /// Apply a prompt's instruction to the selection — no recording. The prompt
    /// (Polish or a custom one) is resolved at dispatch time by id.
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
    /// dictation, the original selection for a prompt transform.
    pub original: String,
    pub text: String,
    pub polished: bool,
    pub duration_ms: u64,
}

struct Session {
    job: Job,
    generation: u64,
    started: Instant,
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
    /// Direct store handle for the opt-in `insights_daily` upsert at the
    /// dictation success point; history goes through `history` above.
    db: Arc<Db>,
    stt_profiles: Arc<SttProfileManager>,
    state: Mutex<PipelineState>,
    state_seq: AtomicU64,
    session: Mutex<Option<Session>>,
    generation: AtomicU64,
    last_result: Mutex<Option<TranscriptionResult>>,
    /// Frontmost app `(bundle_id, name)` at the last dictation — supplies the
    /// opt-in history row's app name.
    last_app: Mutex<Option<(String, String)>>,
    /// At most one HUD educational tip per app session (05 §2.1).
    tip_shown_session: AtomicBool,
    /// Consecutive empty dictations — drives the one-time accuracy tip (05 §2.3).
    empty_streak: AtomicU64,
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
        db: Arc<Db>,
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
            db,
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
            suggestions: Suggestions::new(),
        })
    }

    pub fn last_app(&self) -> Option<(String, String)> {
        self.last_app
            .lock()
            .expect("pipeline state poisoned")
            .clone()
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

    pub fn on_hotkey_pressed(self: &Arc<Self>, job: Job) {
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
                if let Err(err) = self.start(job) {
                    log::warn!("could not start {job:?}: {err}");
                    self.set_transient(Status::Error, err.user_message());
                }
            }
            // Busy processing a previous utterance — ignore.
            _ => {}
        }
    }

    pub fn on_hotkey_released(self: &Arc<Self>, job: Job) {
        // Prompt transforms are taps; only dictation records.
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

    /// Starts a recording job. Only dictation records; prompt transforms capture
    /// the selection without one and never come through here.
    pub fn start(self: &Arc<Self>, job: Job) -> AppResult<()> {
        {
            let state = self.state();
            if !matches!(state.status, Status::Idle | Status::Error | Status::Notice) {
                return Err(AppError::State(
                    "Still finishing the last one — try again in a moment.".into(),
                ));
            }
        }

        let settings = self.settings.get();

        // Capture the frontmost app for the opt-in history row's app name. A
        // detection failure clears it so stale data can't leak forward.
        *self.last_app.lock().expect("pipeline state poisoned") = apps::frontmost_app();

        // Preflight the global speech model: a cloud engine needs no local file.
        if !settings.stt_model_id.starts_with(CLOUD_STT_PREFIX)
            && !self.models.is_installed(&settings.stt_model_id)
        {
            return Err(AppError::Model(
                "No speech model yet — open Settings to download one.".into(),
            ));
        }

        self.audio.start(settings.input_device_name.as_deref())?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.session.lock().expect("pipeline state poisoned") = Some(Session {
            job,
            generation,
            started: Instant::now(),
        });
        // No per-mode label any more; the HUD shows a generic "Listening…".
        self.set_state(Status::Recording, Some(job), None);
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

    /// Tap entry point: apply a prompt's instruction to the selection. The
    /// prompt (Polish or a custom one) is resolved by id at dispatch time so
    /// edits take effect without re-binding the shortcut; a prompt deleted
    /// between keypress and dispatch is a silent no-op (its shortcut is already
    /// being unregistered).
    pub fn run_prompt(self: &Arc<Self>, prompt_id: &str) {
        let Some(prompt) = self
            .settings
            .get()
            .prompts
            .into_iter()
            .find(|p| p.id == prompt_id)
        else {
            return;
        };
        self.run_selection_prompt(prompt.instruction, prompt.name);
    }

    /// Shared body for the no-recording prompt-transform job: resolve the active
    /// profile, capture the selection, then rewrite it with `instruction` and
    /// insert. Runs under the same busy-state and generation contract as every
    /// other job; errors surface as transient HUD states. Blocks on selection
    /// capture, so callers must stay off the main thread (capture round-trips
    /// keystrokes through it).
    fn run_selection_prompt(self: &Arc<Self>, instruction: String, hud_label: String) {
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
                "This prompt needs an AI profile — add one in Settings.".into(),
            );
            return;
        };

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        // The prompt name rides in the message so the HUD can read "Polish…"
        // instead of a generic label.
        self.set_state(Status::Polishing, Some(Job::Transform), Some(hud_label));
        hud::position_on_cursor_monitor(&self.app);

        let selection = match self.output.capture_selection() {
            Ok(Some(text)) => text,
            Ok(None) => {
                self.set_transient(
                    Status::Error,
                    "Select some text first, then press the prompt's shortcut.".into(),
                );
                return;
            }
            Err(err) => {
                log::warn!("prompt selection capture failed: {err}");
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
                .finish_selection_prompt(&profile, selection, &instruction, started)
                .await;
            if pipeline.generation.load(Ordering::SeqCst) != generation {
                return; // cancelled mid-flight; a newer job owns the UI now
            }
            pipeline.publish_outcome(result);
        });
    }

    // ---- Processing -----------------------------------------------------

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

        // Stop capture and get 16 kHz samples (resampled on the audio thread).
        let audio = Arc::clone(&self.audio);
        let recorded = tauri::async_runtime::spawn_blocking(move || audio.stop())
            .await
            .map_err(|e| AppError::State(format!("audio join failed: {e}")))??;

        let model_id = settings.stt_model_id.clone();
        let language = settings.language.clone();
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
        let (with_dictionary, dict_fixes) = text::apply_dictionary(&cleaned, &settings.dictionary);

        match job {
            Job::Dictation => {
                self.finish_dictation(&settings, with_dictionary, dict_fixes, started, record_ms)
                    .await
            }
            // Sessions are only created for recording jobs; prompt transforms
            // run through `run_selection_prompt()` without one.
            Job::Transform => Err(AppError::State(
                "a prompt transform does not use a recording session".into(),
            )),
        }
    }

    async fn finish_dictation(
        self: &Arc<Self>,
        settings: &Settings,
        transcript: String,
        dict_fixes: usize,
        started: Instant,
        record_ms: u64,
    ) -> AppResult<ProcessOutcome> {
        // Watch for distinctive terms worth suggesting for the dictionary.
        self.suggestions.observe(&transcript);

        // The optional post-dictation transform runs the selected prompt over
        // the transcript — but only when the prompt still exists AND an active
        // profile can run it. Anything else inserts the plain transcript
        // (artifact-stripped + dictionary-replaced upstream); there is no tidy.
        let transform = post_dictation_prompt(settings);
        let profile = transform.and(self.profiles.active(&settings.active_llm_profile_id));

        let mut polished = false;
        let mut llm_warning: Option<String> = None;
        let final_text = match (transform, profile) {
            (Some(prompt), Some(profile)) => {
                self.set_state(Status::Polishing, Some(Job::Dictation), None);
                let system = prompts::selection_system_prompt();
                let user = prompts::selection_user_prompt(&transcript, &prompt.instruction);
                match self.llm.chat(&profile, &system, &user).await {
                    Ok(text) => {
                        polished = true;
                        text
                    }
                    Err(err) => {
                        // Never lose a dictation to a flaky provider: fall back to
                        // the plain transcript and tell the user.
                        log::warn!("post-dictation transform failed, inserting plain: {err}");
                        llm_warning =
                            Some("AI unavailable — inserted the plain transcript.".into());
                        transcript.clone()
                    }
                }
            }
            // No transform, a deleted prompt, or no profile → plain transcript.
            _ => transcript.clone(),
        };

        // Snippets expand on the final text only: dictation-only, verbatim, and
        // after any transform pass so the expansion is never reworded. Prompt
        // transforms edit existing text and deliberately skip this.
        let final_text = text::apply_snippets(&final_text, &settings.snippets);

        let words = word_count(&final_text);
        // Frontmost-app display name at dictation time, for the opt-in history
        // row (never any dictated content).
        let app_name = self.last_app().map(|(_, name)| name);
        // Clone only when the opt-in history will consume it below — `insert`
        // takes the text by move.
        let history_text = settings.history_enabled.then(|| final_text.clone());
        let outcome = self.insert(&transcript, &transcript, final_text, polished, started)?;
        // Count this successful dictation — the only tip-system counter (05); a
        // count, never a log. Emit so an open settings webview re-evaluates tips.
        let mut counted = self.settings.get();
        counted.dictation_count = counted.dictation_count.saturating_add(1);
        if let Ok(saved) = self.settings.set(counted) {
            let _ = self.app.emit(SETTINGS_CHANGED_EVENT, &saved);
        }
        // Persistence at the success point, off the async executor (convention:
        // file/DB I/O blocks). Two writes, different policies:
        //   • `insights_daily` is ALWAYS written (counts/dates only, never words
        //     or audio) for the LOCAL calendar day — lifetime insights are
        //     always kept, with no enable toggle and no reset;
        //   • history (text, never audio) is written only when `history_enabled`.
        // insights emits only after its upsert commits (the Ok arm); history
        // emits after the append attempt (errors are swallowed inside `append`).
        // Either way the event follows the write, so views refetch durable rows
        // instead of racing the off-thread write.
        let app = self.app.clone();
        let history = Arc::clone(&self.history);
        let db = Arc::clone(&self.db);
        let raw = transcript.clone();
        let retention_days = settings.history_retention_days;
        tauri::async_runtime::spawn_blocking(move || {
            match db.insights_upsert_daily(
                &local_day(),
                words as i64,
                polished,
                dict_fixes as i64,
                record_ms as i64,
            ) {
                Ok(()) => {
                    let _ = app.emit(INSIGHTS_CHANGED_EVENT, ());
                }
                Err(err) => log::warn!("could not persist insights: {err}"),
            }
            if let Some(text) = history_text {
                history.append(
                    raw,
                    text,
                    app_name,
                    Some(record_ms as i64),
                    words as i64,
                    polished,
                    retention_days,
                );
                let _ = app.emit(HISTORY_CHANGED_EVENT, ());
            }
        });
        // A clipboard fallback (paste failed / no Accessibility) is the
        // highest-priority message — the user must know to press ⌘V to get their
        // text. The flaky-AI warning only matters once the text auto-pasted.
        match outcome {
            ProcessOutcome::Inserted(preview, _) => {
                if let Some(warning) = llm_warning {
                    Ok(ProcessOutcome::Notice(warning))
                } else {
                    Ok(ProcessOutcome::Inserted(preview, self.dictation_hud_tip()))
                }
            }
            // Clipboard-fallback notice from insert(): surface it, never hide it.
            clipboard_notice => Ok(clipboard_notice),
        }
    }

    async fn finish_selection_prompt(
        self: &Arc<Self>,
        profile: &LlmProfile,
        selection: String,
        instruction: &str,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        let system = prompts::selection_system_prompt();
        // The prompt passes its own instruction; an empty one still falls back
        // to the fix-grammar default inside `selection_user_prompt`.
        let user = prompts::selection_user_prompt(&selection, instruction);
        // No fallback — wrong text over the user's selection is worse than
        // nothing.
        let result = self.llm.chat(profile, &system, &user).await?;
        // `original` is the text the diff is measured against — the selection.
        self.insert(&selection, &selection, result, true, started)
    }

    fn insert(
        self: &Arc<Self>,
        raw: &str,
        original: &str,
        final_text: String,
        polished: bool,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        self.set_state(Status::Inserting, None, None);
        // Paste is the fixed insert behavior; the clipboard is always restored
        // afterward. A degrade to clipboard-only happens only without
        // Accessibility, or if the paste keystroke fails.
        let outcome = self
            .output
            .insert(final_text.clone(), InsertMethod::Paste, true)?;

        let result = TranscriptionResult {
            raw: raw.to_string(),
            original: original.to_string(),
            text: final_text,
            polished,
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

enum ProcessOutcome {
    /// Inserted cleanly; carries the text for the success flash and an optional
    /// one-time HUD tip.
    Inserted(String, Option<String>),
    Notice(String),
}

/// The prompt the post-dictation transform should run, or None to insert the
/// plain transcript. `None` covers no transform set AND a dangling id (a prompt
/// deleted after it was chosen) — both mean "insert the plain transcript". Pure,
/// so the routing decision is unit-tested without the pipeline. The profile gate
/// (no active profile → also plain) lives in `finish_dictation`, which needs the
/// live profile manager.
fn post_dictation_prompt(settings: &Settings) -> Option<&Prompt> {
    let id = settings.post_dictation_transform_id.as_deref()?;
    settings.prompts.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::post_dictation_prompt;
    use crate::settings::{Prompt, Settings};

    fn prompt(id: &str) -> Prompt {
        Prompt {
            id: id.into(),
            name: id.into(),
            instruction: "do the thing".into(),
            shortcut: String::new(),
            built_in: false,
        }
    }

    #[test]
    fn no_post_dictation_transform_inserts_plain() {
        let settings = Settings {
            post_dictation_transform_id: None,
            ..Default::default()
        };
        assert!(post_dictation_prompt(&settings).is_none());
    }

    #[test]
    fn a_set_existing_id_selects_that_prompt() {
        let settings = Settings {
            prompts: vec![prompt("concise"), prompt("formal")],
            post_dictation_transform_id: Some("formal".into()),
            ..Default::default()
        };
        assert_eq!(
            post_dictation_prompt(&settings).map(|p| p.id.as_str()),
            Some("formal")
        );
    }

    #[test]
    fn a_dangling_id_inserts_plain() {
        // A prompt chosen then deleted leaves a dangling id; routing must fall
        // back to the plain transcript rather than treating it as a transform.
        let settings = Settings {
            prompts: vec![prompt("concise")],
            post_dictation_transform_id: Some("deleted".into()),
            ..Default::default()
        };
        assert!(post_dictation_prompt(&settings).is_none());
    }
}
