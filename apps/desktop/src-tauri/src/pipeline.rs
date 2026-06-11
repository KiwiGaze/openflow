//! The dictation pipeline state machine.
//!
//! One pipeline instance owns the flow `record → transcribe → (refine) →
//! insert` for both jobs (dictation and selected-text refinement), emits
//! progress events to the webviews, and guards against stale work with a
//! generation counter — cancelling simply bumps the generation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::audio::AudioSystem;
use crate::error::{AppError, AppResult};
use crate::hud;
use crate::llm::LlmClient;
use crate::models::ModelManager;
use crate::modes::{self, LITERAL_MODE_ID};
use crate::output::{CopyReason, InsertOutcome, OutputSystem};
use crate::profiles::{LlmProfile, ProfileManager};
use crate::settings::{HotkeyBehavior, Settings, SettingsManager, MAX_RECORDING_SECS};
use crate::stt::{initial_prompt_from_dictionary, SttEngine};
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
    state: Mutex<PipelineState>,
    state_seq: AtomicU64,
    session: Mutex<Option<Session>>,
    generation: AtomicU64,
    last_result: Mutex<Option<TranscriptionResult>>,
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
            state: Mutex::new(PipelineState {
                status: Status::Idle,
                job: None,
                message: None,
            }),
            state_seq: AtomicU64::new(0),
            session: Mutex::new(None),
            generation: AtomicU64::new(0),
            last_result: Mutex::new(None),
        })
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

    /// Confirmation flash: ✓ + a preview of the inserted text, then idle.
    /// The webview ellipsizes; the state-seq guard lets a newer job pre-empt.
    fn set_success(self: &Arc<Self>, preview: String) {
        let seq = self.set_state(Status::Inserted, None, Some(preview));
        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SUCCESS_FLASH_TTL).await;
            if pipeline.state_seq.load(Ordering::SeqCst) == seq {
                pipeline.set_state(Status::Idle, None, None);
            }
        });
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
                    .map(|s| s.job == job)
                    .unwrap_or(false);
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

        if !self.models.is_installed(&settings.stt_model_id) {
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

        self.audio.start()?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.session.lock().expect("pipeline state poisoned") = Some(Session {
            job,
            generation,
            started: Instant::now(),
            selection,
        });
        self.set_state(Status::Recording, Some(job), None);
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
        self.set_state(Status::Transcribing, Some(session.job), None);

        let pipeline = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let generation = session.generation;
            let result = pipeline.process(session).await;
            if pipeline.generation.load(Ordering::SeqCst) != generation {
                return; // cancelled mid-flight; a newer job owns the UI now
            }
            match result {
                Ok(ProcessOutcome::Inserted(preview)) => {
                    pipeline.set_success(preview);
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
        self.set_state(Status::Idle, None, None);
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
                Ok(ProcessOutcome::Inserted(preview)) => {
                    pipeline.set_success(preview);
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

    async fn process(self: &Arc<Self>, session: Session) -> AppResult<ProcessOutcome> {
        let settings = self.settings.get();
        let job = session.job;
        let started = session.started;

        // Stop capture and get 16 kHz samples (resampled on the audio thread).
        let audio = Arc::clone(&self.audio);
        let recorded = tauri::async_runtime::spawn_blocking(move || audio.stop())
            .await
            .map_err(|e| AppError::State(format!("audio join failed: {e}")))??;

        // Transcribe on a blocking thread; whisper inference is CPU/GPU heavy.
        let stt = Arc::clone(&self.stt);
        let model_id = settings.stt_model_id.clone();
        let model_path = self.models.path_for(&model_id)?;
        let language = settings.language.clone();
        let prompt = initial_prompt_from_dictionary(&settings.dictionary);
        log::info!(
            "recorded {:.1}s of audio ({} samples at 16 kHz)",
            recorded.duration.as_secs_f32(),
            recorded.samples.len()
        );
        let samples = recorded.samples;
        let raw = tauri::async_runtime::spawn_blocking(move || {
            stt.transcribe(
                &model_id,
                &model_path,
                &samples,
                &language,
                prompt.as_deref(),
            )
        })
        .await
        .map_err(|e| AppError::Stt(format!("transcription join failed: {e}")))??;

        let cleaned = text::clean_transcript(&raw);
        if cleaned.is_empty() {
            return Ok(ProcessOutcome::Notice(
                "Didn't catch that — try again.".into(),
            ));
        }
        let with_dictionary = text::apply_dictionary(&cleaned, &settings.dictionary);

        match job {
            Job::Dictation => {
                self.finish_dictation(&settings, with_dictionary, started)
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
        transcript: String,
        started: Instant,
    ) -> AppResult<ProcessOutcome> {
        let mode = settings.active_mode();
        // The refine toggle is the master switch; the per-mode flag still
        // decides whether this mode wants AI at all.
        let profile = if mode.uses_llm && settings.refine_after_dictation {
            self.profiles.active(&settings.active_llm_profile_id)
        } else {
            None
        };

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
        if let Some(warning) = llm_warning {
            return Ok(ProcessOutcome::Notice(warning));
        }
        Ok(outcome)
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
        self.insert(settings, &instruction, rewritten, "refine", true, started)
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
                Ok(ProcessOutcome::Inserted(result.text.clone()))
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
    /// Inserted cleanly; carries the text for the success flash.
    Inserted(String),
    Notice(String),
}
