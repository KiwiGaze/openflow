//! Global hotkey registration and dispatch.
//!
//! Uses Carbon `RegisterEventHotKey` under the hood (via the global-shortcut
//! plugin) — no Input Monitoring permission required. The handler receives
//! Pressed/Released/Repeat states, which drive hold-to-talk:
//! press starts, release stops (a sub-350 ms tap latches hands-free mode).

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use crate::pipeline::Job;
use crate::settings::Settings;
use crate::state::AppState;

/// (Re-)registers every hotkey from settings — the dictation key, the optional
/// see-changes key, and each prompt with a bound shortcut (Polish included).
/// Returns a user-readable error when an accelerator cannot be parsed or
/// registered (e.g. taken by another app) or two collide, leaving no partial
/// registrations behind.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| format!("could not reset hotkeys: {e}"))?;

    let dictation = Shortcut::from_str(&settings.dictation_hotkey)
        .map_err(|e| format!("dictation hotkey “{}”: {e}", settings.dictation_hotkey))?;

    // The see-changes hotkey is optional: an empty string disables it.
    let changes = if settings.change_overlay_hotkey.trim().is_empty() {
        None
    } else {
        Some(
            Shortcut::from_str(&settings.change_overlay_hotkey).map_err(|e| {
                format!(
                    "see-changes hotkey “{}”: {e}",
                    settings.change_overlay_hotkey
                )
            })?,
        )
    };

    // Prompts with a bound shortcut; unbound ones exist but never register.
    // Polish's default ⌥⇧P is included here automatically.
    let mut prompts: Vec<(String, Shortcut)> = Vec::new();
    for p in &settings.prompts {
        if p.shortcut.trim().is_empty() {
            continue;
        }
        let sc = Shortcut::from_str(&p.shortcut)
            .map_err(|e| format!("prompt “{}” shortcut “{}”: {e}", p.name, p.shortcut))?;
        prompts.push((p.id.clone(), sc));
    }

    // Every hotkey must be pairwise distinct — the dictation key, the optional
    // see-changes key, and each bound prompt shortcut. (Shortcut is Copy, so
    // collecting them here leaves the originals usable for registration below.)
    let mut all: Vec<Shortcut> = vec![dictation];
    all.extend(changes.iter().copied());
    all.extend(prompts.iter().map(|(_, sc)| *sc));
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            if all[i] == all[j] {
                return Err("two hotkeys are the same — every hotkey must be different".into());
            }
        }
    }

    shortcuts
        .on_shortcut(dictation, move |app, _shortcut, event| {
            dispatch(app, Job::Dictation, event.state());
        })
        .map_err(|e| format!("dictation hotkey “{}”: {e}", settings.dictation_hotkey))?;

    // Each prompt is a tap; the handler resolves the instruction by id at
    // trigger time, so edits don't require re-binding.
    for (id, sc) in prompts {
        if let Err(e) = shortcuts.on_shortcut(sc, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let pipeline = app.state::<AppState>().pipeline.clone();
                let id = id.clone();
                // Offload: running a prompt blocks on selection capture.
                tauri::async_runtime::spawn_blocking(move || pipeline.run_prompt(&id));
            }
        }) {
            let _ = shortcuts.unregister_all();
            return Err(format!("a prompt shortcut could not be registered: {e}"));
        }
    }

    // See-changes is a tap: only Pressed matters, and it does no clipboard or
    // keystroke work, so it can run inline on the hotkey thread.
    if let Some(changes) = changes {
        if let Err(e) = shortcuts.on_shortcut(changes, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::changes::request_toggle(app);
            }
        }) {
            let _ = shortcuts.unregister_all();
            return Err(format!(
                "see-changes hotkey “{}”: {e}",
                settings.change_overlay_hotkey
            ));
        }
    }

    log::info!(
        "hotkeys registered: dictation={} see-changes={} prompts={}",
        settings.dictation_hotkey,
        settings.change_overlay_hotkey,
        settings
            .prompts
            .iter()
            .filter(|p| !p.shortcut.trim().is_empty())
            .count(),
    );
    Ok(())
}

/// Esc cancels an in-progress recording — a "never mind" for a mistaken
/// activation (UX-12). It is bound only while recording (registered on start,
/// dropped on finish/cancel) so Esc stays free for every other app the rest of
/// the time. Carbon registration must happen on the main thread, so this
/// marshals there regardless of the caller's thread; both register and
/// unregister are idempotent (unbinding a free key is a no-op we log at debug).
pub fn set_cancel_key(app: &AppHandle, active: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let shortcuts = handle.global_shortcut();
        let esc = Shortcut::new(None, Code::Escape);
        if active {
            if let Err(err) = shortcuts.on_shortcut(esc, |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    app.state::<AppState>().pipeline.clone().cancel();
                }
            }) {
                log::warn!("could not bind Esc-to-cancel: {err}");
            }
        } else if let Err(err) = shortcuts.unregister(esc) {
            log::debug!("Esc-to-cancel already unbound: {err}");
        }
    });
}

fn dispatch(app: &AppHandle, job: Job, state: ShortcutState) {
    let pipeline = app.state::<AppState>().pipeline.clone();
    // The hotkey handler runs on the main thread; start/finish block on
    // worker-thread replies, so offload to keep the handler responsive.
    //
    // Pressed/Released are offloaded independently, so the pool could in
    // principle run Released before its Pressed. Harmless here: a reorder only
    // changes behavior for holds longer than TAP_THRESHOLD, but the pool is
    // never saturated (one brief task at a time), so tasks start in order well
    // within that window — and sub-threshold taps already treat Released as a
    // no-op.
    tauri::async_runtime::spawn_blocking(move || match state {
        ShortcutState::Pressed => pipeline.on_hotkey_pressed(job),
        ShortcutState::Released => pipeline.on_hotkey_released(job),
    });
}
