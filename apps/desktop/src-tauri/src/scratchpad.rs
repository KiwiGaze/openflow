//! The Scratchpad window — the opt-in notes surface.
//!
//! Unlike the HUD and the changes overlay (always-present, never reopened), the
//! Scratchpad is a plain, decorated, resizable window created on demand and
//! destroyed on close, then recreated next time it is asked for. It needs no
//! `NSPanel` reclass or focus tricks: it is an ordinary window the user works
//! in. Its label is listed in `capabilities/default.json` so its webview can
//! reach the note IPC commands.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SCRATCHPAD_LABEL: &str = "scratchpad";

/// Emitted after any note mutation (create/update/pin/delete/restore/transform)
/// so an open Scratchpad refreshes its list from durable rows. No payload.
/// Mirrored as `EVENTS.notesChanged` in `@velata/core`.
pub const NOTES_CHANGED_EVENT: &str = "notes-changed";

/// Asks an already-open Scratchpad to switch to a note. Payload: the note id.
/// Used when the window exists (a fresh window receives the id via the URL).
/// Mirrored as `EVENTS.scratchpadOpenNote` in `@velata/core`.
pub const SCRATCHPAD_OPEN_NOTE_EVENT: &str = "scratchpad-open-note";

const DEFAULT_WIDTH: f64 = 860.0;
const DEFAULT_HEIGHT: f64 = 560.0;
const MIN_WIDTH: f64 = 640.0;
const MIN_HEIGHT: f64 = 420.0;

/// Opens the Scratchpad, creating it if absent and otherwise showing and
/// focusing the existing window. `note_id` selects a note: it rides the URL on
/// creation (the webview reads `?note=`) and arrives via
/// [`SCRATCHPAD_OPEN_NOTE_EVENT`] when the window is already up.
pub fn open(app: &AppHandle, note_id: Option<String>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SCRATCHPAD_LABEL) {
        window.show()?;
        window.set_focus()?;
        if let Some(id) = note_id {
            let _ = tauri::Emitter::emit(app, SCRATCHPAD_OPEN_NOTE_EVENT, id);
        }
        return Ok(());
    }

    let url = match &note_id {
        Some(id) => format!("scratchpad.html?note={}", encode_query(id)),
        None => "scratchpad.html".to_string(),
    };
    WebviewWindowBuilder::new(app, SCRATCHPAD_LABEL, WebviewUrl::App(url.into()))
        .title("Scratchpad")
        .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .resizable(true)
        .center()
        .build()?;
    Ok(())
}

/// Percent-encodes the few characters a note id could contain that are unsafe
/// in a URL query. Ids are `${ms}-${seq}` today (digits and a hyphen), so this
/// is belt-and-braces against a future id scheme rather than a live need.
fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::encode_query;

    #[test]
    fn encodes_only_unsafe_characters() {
        // The live id scheme passes through untouched.
        assert_eq!(encode_query("1718000000000-3"), "1718000000000-3");
        // Reserved characters are percent-encoded.
        assert_eq!(encode_query("a b&c"), "a%20b%26c");
    }
}
