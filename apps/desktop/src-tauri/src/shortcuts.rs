//! Global hotkey registration and dispatch.
//!
//! Uses Carbon `RegisterEventHotKey` under the hood (via the global-shortcut
//! plugin) — no Input Monitoring permission required. The handler receives
//! Pressed/Released/Repeat states, which drive hold-to-talk:
//! press starts, release stops (a sub-350 ms tap latches hands-free mode).

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::pipeline::Job;
use crate::settings::Settings;
use crate::state::AppState;

/// (Re-)registers every hotkey from settings — the three fixed ones plus each
/// bound transform. Returns a user-readable error when an accelerator cannot be
/// parsed or registered (e.g. taken by another app) or two collide, leaving no
/// partial registrations behind.
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

    // Transforms with a bound hotkey; unbound ones exist but never register.
    let mut transforms: Vec<(String, Shortcut)> = Vec::new();
    for t in &settings.transforms {
        if t.hotkey.trim().is_empty() {
            continue;
        }
        let sc = Shortcut::from_str(&t.hotkey)
            .map_err(|e| format!("transform “{}” hotkey “{}”: {e}", t.name, t.hotkey))?;
        transforms.push((t.id.clone(), sc));
    }

    // Every hotkey must be pairwise distinct — the three fixed ones, the
    // optional see-changes key, and each bound transform. (Shortcut is Copy, so
    // collecting them here leaves the originals usable for registration below.)
    let mut all: Vec<Shortcut> = vec![dictation, refine, polish];
    all.extend(changes.iter().copied());
    all.extend(transforms.iter().map(|(_, sc)| *sc));
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

    // Each transform is a tap like polish; the handler resolves the instruction
    // by id at trigger time, so edits don't require re-binding.
    for (id, sc) in transforms {
        if let Err(e) = shortcuts.on_shortcut(sc, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let pipeline = app.state::<AppState>().pipeline.clone();
                let id = id.clone();
                tauri::async_runtime::spawn_blocking(move || pipeline.run_transform(&id));
            }
        }) {
            let _ = shortcuts.unregister_all();
            return Err(format!("a transform hotkey could not be registered: {e}"));
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
        "hotkeys registered: dictation={} rewrite={} polish={} transforms={} see-changes={}",
        settings.dictation_hotkey,
        settings.refine_hotkey,
        settings.polish_hotkey,
        settings
            .transforms
            .iter()
            .filter(|t| !t.hotkey.trim().is_empty())
            .count(),
        settings.change_overlay_hotkey
    );
    Ok(())
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
        ShortcutState::Pressed => pipeline.on_hotkey_pressed(job),
        ShortcutState::Released => pipeline.on_hotkey_released(job),
    });
}
