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

/// (Re-)registers both hotkeys from settings. Returns a user-readable error
/// when an accelerator cannot be parsed or registered (e.g. taken by another
/// app), leaving no partial registrations behind.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| format!("could not reset hotkeys: {e}"))?;

    let dictation = Shortcut::from_str(&settings.dictation_hotkey)
        .map_err(|e| format!("dictation hotkey “{}”: {e}", settings.dictation_hotkey))?;
    let refine = Shortcut::from_str(&settings.refine_hotkey)
        .map_err(|e| format!("rewrite hotkey “{}”: {e}", settings.refine_hotkey))?;
    if dictation == refine {
        return Err("the dictation and rewrite hotkeys must be different".into());
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

    log::info!(
        "hotkeys registered: dictation={} rewrite={}",
        settings.dictation_hotkey,
        settings.refine_hotkey
    );
    Ok(())
}

fn dispatch(app: &AppHandle, job: Job, state: ShortcutState) {
    let pipeline = app.state::<AppState>().pipeline.clone();
    match state {
        ShortcutState::Pressed => pipeline.on_hotkey_pressed(job),
        ShortcutState::Released => pipeline.on_hotkey_released(job),
    }
}
