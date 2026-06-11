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

/// (Re-)registers all three hotkeys from settings. Returns a user-readable
/// error when an accelerator cannot be parsed or registered (e.g. taken by
/// another app), leaving no partial registrations behind.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| format!("could not reset hotkeys: {e}"))?;

    let dictation = Shortcut::from_str(&settings.dictation_hotkey)
        .map_err(|e| format!("dictation hotkey “{}”: {e}", settings.dictation_hotkey))?;
    let refine = Shortcut::from_str(&settings.refine_hotkey)
        .map_err(|e| format!("rewrite hotkey “{}”: {e}", settings.refine_hotkey))?;
    let polish = Shortcut::from_str(&settings.polish_hotkey)
        .map_err(|e| format!("polish hotkey “{}”: {e}", settings.polish_hotkey))?;
    // Parse every non-null mode hotkey alongside the globals.
    let mut mode_hotkeys: Vec<(String, Shortcut)> = Vec::new();
    for mode in &settings.modes {
        if let Some(hk) = mode.hotkey.as_deref().filter(|h| !h.is_empty()) {
            let shortcut = Shortcut::from_str(hk)
                .map_err(|e| format!("mode “{}” hotkey “{hk}”: {e}", mode.name))?;
            mode_hotkeys.push((mode.id.clone(), shortcut));
        }
    }

    // Every accelerator — the three globals and every mode hotkey — must be
    // pairwise distinct (07 §4). Compare by reference so the originals survive
    // for registration below.
    let mut all: Vec<&Shortcut> = vec![&dictation, &refine, &polish];
    all.extend(mode_hotkeys.iter().map(|(_, sc)| sc));
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            if all[i] == all[j] {
                return Err("every hotkey, including per-mode hotkeys, must be different".into());
            }
        }
    }

    shortcuts
        .on_shortcut(dictation, move |app, _shortcut, event| {
            dispatch(app, Job::Dictation, event.state());
        })
        .map_err(|e| format!("dictation hotkey “{}”: {e}", settings.dictation_hotkey))?;

    if let Err(e) = shortcuts.on_shortcut(refine, move |app, _shortcut, event| {
        dispatch(app, Job::RefineSelection, event.state());
    }) {
        let _ = shortcuts.unregister_all();
        return Err(format!("rewrite hotkey “{}”: {e}", settings.refine_hotkey));
    }

    // Polish is a tap: only Pressed matters, Released is a no-op.
    if let Err(e) = shortcuts.on_shortcut(polish, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let pipeline = app.state::<AppState>().pipeline.clone();
            // Same offload as dispatch(): polish blocks on selection capture.
            tauri::async_runtime::spawn_blocking(move || pipeline.polish());
        }
    }) {
        let _ = shortcuts.unregister_all();
        return Err(format!("polish hotkey “{}”: {e}", settings.polish_hotkey));
    }

    // Per-mode hotkeys: one-shot dictation in that mode.
    for (mode_id, shortcut) in mode_hotkeys {
        if let Err(e) = shortcuts.on_shortcut(shortcut, move |app, _shortcut, event| {
            dispatch_mode(app, mode_id.clone(), event.state());
        }) {
            let _ = shortcuts.unregister_all();
            return Err(format!("could not register a mode hotkey: {e}"));
        }
    }

    log::info!(
        "hotkeys registered: dictation={} rewrite={} polish={} mode-hotkeys={}",
        settings.dictation_hotkey,
        settings.refine_hotkey,
        settings.polish_hotkey,
        settings.modes.iter().filter(|m| m.hotkey.is_some()).count()
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
    // The hotkey handler runs on the main thread. Starting a refine job
    // blocks on the output worker (selection capture), which round-trips
    // keystrokes through the main thread — handling it inline would deadlock.
    //
    // Pressed/Released are offloaded independently, so the pool could in
    // principle run Released before its Pressed. Harmless here: a reorder only
    // changes behavior for holds longer than TAP_THRESHOLD, but the pool is
    // never saturated (one brief task at a time), so tasks start in order well
    // within that window — and sub-threshold taps already treat Released as a
    // no-op.
    tauri::async_runtime::spawn_blocking(move || match state {
        ShortcutState::Pressed => pipeline.on_hotkey_pressed(job, None),
        ShortcutState::Released => pipeline.on_hotkey_released(job),
    });
}

/// A per-mode hotkey: dictates once in `mode_id` without changing the active
/// mode (one-shot, 07 §4). Same record/finish flow as the plain dictation key.
fn dispatch_mode(app: &AppHandle, mode_id: String, state: ShortcutState) {
    let pipeline = app.state::<AppState>().pipeline.clone();
    tauri::async_runtime::spawn_blocking(move || match state {
        ShortcutState::Pressed => pipeline.on_hotkey_pressed(Job::Dictation, Some(mode_id)),
        ShortcutState::Released => pipeline.on_hotkey_released(Job::Dictation),
    });
}
