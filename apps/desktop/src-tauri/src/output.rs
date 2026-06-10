//! Clipboard and keystroke output on a dedicated worker thread.
//!
//! CGEvent posting (enigo) has thread-affinity constraints that clash with
//! async executors, so a single OS thread owns both the `Enigo` handle and
//! the clipboard, and the app talks to it over a channel.

use std::sync::mpsc::{self, Sender, SyncSender};
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

use crate::error::{AppError, AppResult};
use crate::permissions;
use crate::settings::InsertMethod;

/// How the final text reached the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Pasted,
    /// Text was left on the clipboard (chosen method, or paste unavailable
    /// because Accessibility permission is missing).
    CopiedToClipboard,
}

/// Marker used to detect "nothing was selected" when capturing a selection.
const SELECTION_PROBE: &str = "\u{200B}openflow-selection-probe\u{200B}";

/// Time for the target app to process a synthetic Cmd+C / Cmd+V.
const KEYSTROKE_SETTLE: Duration = Duration::from_millis(140);
/// Time between writing the clipboard and sending Cmd+V.
const CLIPBOARD_SETTLE: Duration = Duration::from_millis(60);

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
}

pub struct OutputSystem {
    tx: Sender<OutputCmd>,
}

impl OutputSystem {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<OutputCmd>();
        std::thread::Builder::new()
            .name("openflow-output".into())
            .spawn(move || worker(rx))
            .expect("failed to spawn output thread");
        Self { tx }
    }

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

    /// Returns the currently selected text in the frontmost app, or `None`
    /// when nothing is selected. Uses the Cmd+C clipboard round-trip.
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
    enigo: Option<Enigo>,
    clipboard: Option<Clipboard>,
}

impl Worker {
    fn enigo(&mut self) -> AppResult<&mut Enigo> {
        if self.enigo.is_none() {
            self.enigo =
                Some(Enigo::new(&EnigoSettings::default()).map_err(|e| {
                    AppError::Output(format!("keyboard synthesis unavailable: {e}"))
                })?);
        }
        Ok(self.enigo.as_mut().expect("just set"))
    }

    fn clipboard(&mut self) -> AppResult<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                Clipboard::new()
                    .map_err(|e| AppError::Output(format!("clipboard unavailable: {e}")))?,
            );
        }
        Ok(self.clipboard.as_mut().expect("just set"))
    }

    fn send_shortcut(&mut self, letter: char) -> AppResult<()> {
        let enigo = self.enigo()?;
        let fail = |e: enigo::InputError| AppError::Output(format!("keystroke failed: {e}"));
        enigo.key(Key::Meta, Direction::Press).map_err(fail)?;
        enigo
            .key(Key::Unicode(letter), Direction::Click)
            .map_err(fail)?;
        enigo.key(Key::Meta, Direction::Release).map_err(fail)?;
        Ok(())
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
            return Ok(InsertOutcome::CopiedToClipboard);
        }

        let saved = self.clipboard()?.get_text().ok();
        self.clipboard()?
            .set_text(text)
            .map_err(|e| AppError::Output(format!("could not write clipboard: {e}")))?;
        std::thread::sleep(CLIPBOARD_SETTLE);
        self.send_shortcut('v')?;
        std::thread::sleep(KEYSTROKE_SETTLE);

        if restore_clipboard {
            if let Some(previous) = saved {
                // Best effort: a failed restore must not fail the insert.
                let _ = self.clipboard().map(|c| c.set_text(previous));
            }
        }
        Ok(InsertOutcome::Pasted)
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

fn worker(rx: mpsc::Receiver<OutputCmd>) {
    let mut state = Worker {
        enigo: None,
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
        }
    }
}
