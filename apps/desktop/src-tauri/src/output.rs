//! Clipboard and keystroke output on a dedicated worker thread.
//!
//! The worker owns the clipboard and the settle delays, and the app talks to
//! it over a channel. Keystroke synthesis itself runs on the main thread:
//! enigo resolves `Key::Unicode` to a keycode through the TIS keyboard-layout
//! APIs, which macOS aborts with `dispatch_assert_queue` on any other thread.
//! Callers of [`OutputSystem`] must therefore never block the main thread
//! while waiting on the worker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::Arc;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use tauri::AppHandle;

use crate::error::{AppError, AppResult};
use crate::permissions;
use crate::settings::InsertMethod;

/// How the final text reached the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Pasted,
    /// Text was left on the clipboard; the reason decides what to tell the user.
    CopiedToClipboard(CopyReason),
}

/// Why the text ended up on the clipboard instead of being pasted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyReason {
    /// The user chose the clipboard-only insert method.
    ChosenMethod,
    /// Pasting needs the Accessibility permission and it is missing.
    NoAccessibility,
    /// The ⌘V keystroke failed after the clipboard was already written.
    PasteFailed,
}

/// Marker used to detect "nothing was selected" when capturing a selection.
const SELECTION_PROBE: &str = "\u{200B}openflow-selection-probe\u{200B}";

/// Time for the target app to process a synthetic Cmd+C / Cmd+V.
const KEYSTROKE_SETTLE: Duration = Duration::from_millis(140);
/// Time between writing the clipboard and sending Cmd+V.
const CLIPBOARD_SETTLE: Duration = Duration::from_millis(60);
/// Upper bound for the main run loop to service a keystroke request; a busy
/// main thread degrades to an error instead of hanging the pipeline.
const KEYSTROKE_DISPATCH_TIMEOUT: Duration = Duration::from_secs(2);

enum OutputCmd {
    Insert {
        text: String,
        method: InsertMethod,
        restore_clipboard: bool,
        respond: SyncSender<AppResult<InsertOutcome>>,
    },
    CaptureSelection {
        respond: SyncSender<AppResult<Option<String>>>,
    },
    CopyText {
        text: String,
        respond: SyncSender<AppResult<()>>,
    },
}

pub struct OutputSystem {
    tx: Sender<OutputCmd>,
}

impl OutputSystem {
    pub fn spawn(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<OutputCmd>();
        std::thread::Builder::new()
            .name("openflow-output".into())
            .spawn(move || worker(app, rx))
            .expect("failed to spawn output thread");
        Self { tx }
    }

    /// Pastes (or copies) `text` into the frontmost app and blocks until the
    /// worker replies. Never call on the main thread: the paste round-trips
    /// keystrokes through it and would deadlock — offload first, as
    /// `commands::start_refine_selection` does.
    pub fn insert(
        &self,
        text: String,
        method: InsertMethod,
        restore_clipboard: bool,
    ) -> AppResult<InsertOutcome> {
        let (respond, wait) = mpsc::sync_channel(1);
        self.tx
            .send(OutputCmd::Insert {
                text,
                method,
                restore_clipboard,
                respond,
            })
            .map_err(|_| AppError::Output("output thread is gone".into()))?;
        wait.recv()
            .map_err(|_| AppError::Output("output thread did not respond".into()))?
    }

    /// Writes text to the clipboard via the worker that owns the clipboard
    /// handle. Does not paste — used by the changes overlay's Copy button.
    /// Blocks until the worker replies.
    pub fn copy_text(&self, text: String) -> AppResult<()> {
        let (respond, wait) = mpsc::sync_channel(1);
        self.tx
            .send(OutputCmd::CopyText { text, respond })
            .map_err(|_| AppError::Output("output thread is gone".into()))?;
        wait.recv()
            .map_err(|_| AppError::Output("output thread did not respond".into()))?
    }

    /// Returns the currently selected text in the frontmost app, or `None`
    /// when nothing is selected. Uses the Cmd+C clipboard round-trip, so the
    /// same main-thread rule as [`OutputSystem::insert`] applies: calling this
    /// on the main thread deadlocks.
    pub fn capture_selection(&self) -> AppResult<Option<String>> {
        let (respond, wait) = mpsc::sync_channel(1);
        self.tx
            .send(OutputCmd::CaptureSelection { respond })
            .map_err(|_| AppError::Output("output thread is gone".into()))?;
        wait.recv()
            .map_err(|_| AppError::Output("output thread did not respond".into()))?
    }
}

struct Worker {
    app: AppHandle,
    clipboard: Option<Clipboard>,
}

/// Presses Cmd+<letter>. Must run on the main thread: enigo maps
/// `Key::Unicode` to a keycode through the TIS keyboard-layout APIs, which
/// trap (`dispatch_assert_queue`) on any other thread. The short-lived
/// `Enigo` also releases Cmd on drop if the sequence fails halfway.
fn press_cmd_shortcut(letter: char) -> AppResult<()> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| AppError::Output(format!("keyboard synthesis unavailable: {e}")))?;
    let fail = |e: enigo::InputError| AppError::Output(format!("keystroke failed: {e}"));
    enigo.key(Key::Meta, Direction::Press).map_err(fail)?;
    enigo
        .key(Key::Unicode(letter), Direction::Click)
        .map_err(fail)?;
    enigo.key(Key::Meta, Direction::Release).map_err(fail)?;
    Ok(())
}

impl Worker {
    fn clipboard(&mut self) -> AppResult<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                Clipboard::new()
                    .map_err(|e| AppError::Output(format!("clipboard unavailable: {e}")))?,
            );
        }
        Ok(self.clipboard.as_mut().expect("just set"))
    }

    fn send_shortcut(&self, letter: char) -> AppResult<()> {
        let (respond, wait) = mpsc::sync_channel(1);
        // Once we stop waiting (timeout), the closure may still be queued on the
        // main thread. Gate it so a timed-out request becomes a no-op instead of
        // firing a stray ⌘V/⌘C after the paste/capture flow has already unwound.
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_main = Arc::clone(&cancelled);
        self.app
            .run_on_main_thread(move || {
                if cancelled_main.load(Ordering::Acquire) {
                    return;
                }
                let _ = respond.send(press_cmd_shortcut(letter));
            })
            .map_err(|e| AppError::Output(format!("keystroke dispatch failed: {e}")))?;
        match wait.recv_timeout(KEYSTROKE_DISPATCH_TIMEOUT) {
            Ok(result) => result,
            Err(_) => {
                cancelled.store(true, Ordering::Release);
                Err(AppError::Output(
                    "main thread did not respond to keystroke".into(),
                ))
            }
        }
    }

    fn insert(
        &mut self,
        text: &str,
        method: InsertMethod,
        restore_clipboard: bool,
    ) -> AppResult<InsertOutcome> {
        let can_paste = permissions::accessibility_trusted(false);
        let paste_requested = method == InsertMethod::Paste;

        if !paste_requested || !can_paste {
            self.clipboard()?
                .set_text(text)
                .map_err(|e| AppError::Output(format!("could not write clipboard: {e}")))?;
            let reason = if paste_requested {
                CopyReason::NoAccessibility
            } else {
                CopyReason::ChosenMethod
            };
            return Ok(InsertOutcome::CopiedToClipboard(reason));
        }

        let saved = self.clipboard()?.get_text().ok();
        self.clipboard()?
            .set_text(text)
            .map_err(|e| AppError::Output(format!("could not write clipboard: {e}")))?;
        std::thread::sleep(CLIPBOARD_SETTLE);
        if let Err(err) = self.send_shortcut('v') {
            // The result is already on the clipboard — degrade instead of
            // erroring, and skip the restore that would wipe it.
            log::warn!("paste keystroke failed; leaving the result on the clipboard: {err}");
            return Ok(InsertOutcome::CopiedToClipboard(CopyReason::PasteFailed));
        }
        std::thread::sleep(KEYSTROKE_SETTLE);

        if restore_clipboard {
            if let Some(previous) = saved {
                // Best effort: a failed restore must not fail the insert.
                let _ = self.clipboard().map(|c| c.set_text(previous));
            }
        }
        Ok(InsertOutcome::Pasted)
    }

    fn copy_text(&mut self, text: &str) -> AppResult<()> {
        self.clipboard()?
            .set_text(text)
            .map_err(|e| AppError::Output(format!("could not write clipboard: {e}")))
    }

    fn capture_selection(&mut self) -> AppResult<Option<String>> {
        if !permissions::accessibility_trusted(false) {
            return Err(AppError::Output(
                "Accessibility permission is required to read the selection".into(),
            ));
        }

        let saved = self.clipboard()?.get_text().ok();
        self.clipboard()?
            .set_text(SELECTION_PROBE)
            .map_err(|e| AppError::Output(format!("could not write clipboard: {e}")))?;
        std::thread::sleep(CLIPBOARD_SETTLE);
        self.send_shortcut('c')?;
        std::thread::sleep(KEYSTROKE_SETTLE);

        let captured = self.clipboard()?.get_text().ok();

        // Put the user's clipboard back before returning.
        if let Some(previous) = saved {
            let _ = self.clipboard().map(|c| c.set_text(previous));
        }

        match captured {
            Some(text) if text != SELECTION_PROBE && !text.is_empty() => Ok(Some(text)),
            _ => Ok(None),
        }
    }
}

fn worker(app: AppHandle, rx: mpsc::Receiver<OutputCmd>) {
    let mut state = Worker {
        app,
        clipboard: None,
    };
    while let Ok(cmd) = rx.recv() {
        match cmd {
            OutputCmd::Insert {
                text,
                method,
                restore_clipboard,
                respond,
            } => {
                let _ = respond.send(state.insert(&text, method, restore_clipboard));
            }
            OutputCmd::CaptureSelection { respond } => {
                let _ = respond.send(state.capture_selection());
            }
            OutputCmd::CopyText { text, respond } => {
                let _ = respond.send(state.copy_text(&text));
            }
        }
    }
}
