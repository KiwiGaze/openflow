//! Global hotkey registration and dispatch.
//!
//! Uses Carbon `RegisterEventHotKey` under the hood (via the global-shortcut
//! plugin) — no Input Monitoring permission required. The handler receives
//! Pressed/Released/Repeat states, which drive hold-to-talk:
//! press starts, release stops (a sub-350 ms tap latches hands-free mode).

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use crate::fn_gesture;
use crate::pipeline::Job;
use crate::settings::{Hotkey, HotkeyKind, Settings};
use crate::state::AppState;

/// Accelerator stand-in for the `fn`-key push-to-talk gesture when Input
/// Monitoring is not granted (so the tap can't observe `fn`). A held `Alt+Space`
/// is push-to-talk; a quick tap of it latches hands-free via the same pipeline
/// path — so the fallback keeps both behaviors the `fn` key would give. Bypassed
/// when the `fn` monitor is live.
const PUSH_TO_TALK_FALLBACK: &str = "Alt+Space";

/// Whether a hotkey is the observable `fn`-key gesture: a `Hold`/`DoubleTap`
/// whose key is `fn`. Only push-to-talk uses this; when the `fn` monitor is live
/// such a trigger is driven by the [`crate::fn_gesture`] CGEventTap, not by an
/// accelerator fallback. A `Hold`/`DoubleTap` with any other key is not a thing
/// the UI can produce, but would still fall back rather than feed the tap.
pub fn is_fn_gesture(hotkey: &Hotkey) -> bool {
    matches!(hotkey.kind, HotkeyKind::Hold | HotkeyKind::DoubleTap) && hotkey.key == "fn"
}

/// Resolves a gesture trigger to the accelerator string to actually register, or
/// `None` to register nothing. An `Accelerator` hotkey uses its own `key` (an
/// empty `key` disables the trigger); a `Hold`/`DoubleTap` gesture — used when the
/// `fn` monitor is not live — falls back to `fallback`.
fn resolve(hotkey: &Hotkey, fallback: &str) -> Option<String> {
    match hotkey.kind {
        HotkeyKind::Accelerator => {
            let key = hotkey.key.trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        }
        HotkeyKind::Hold | HotkeyKind::DoubleTap => Some(fallback.to_string()),
    }
}

/// (Re-)registers every hotkey from settings — push-to-talk, hands-free, the
/// optional see-changes key, and each prompt with a bound shortcut (Polish
/// included). Returns a user-readable error when an accelerator cannot be parsed
/// or registered (e.g. taken by another app) or two collide, leaving no partial
/// registrations behind.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| format!("could not reset hotkeys: {e}"))?;

    // Start the `fn` CGEventTap once if a trigger is an `fn` gesture and Input
    // Monitoring is granted. When it is live it OWNS the `fn` gesture triggers,
    // so their accelerator fallback must NOT also register (or both would fire);
    // when it is not (ungranted/unavailable, or the trigger is an accelerator)
    // the fallback registers as before. Idempotent across saves — the monitor is
    // started once and routes by live settings (`fn_gesture::route_gesture`).
    let fn_active = fn_gesture::ensure_monitor(app);

    // Push-to-talk: the `fn` gesture driven by the live tap registers nothing
    // here; otherwise it falls back to Alt+Space so dictation stays usable
    // (held = push-to-talk, tapped = hands-free latch). Empty disables it.
    let push_to_talk = if fn_active && is_fn_gesture(&settings.push_to_talk_hotkey) {
        None
    } else {
        resolve(&settings.push_to_talk_hotkey, PUSH_TO_TALK_FALLBACK)
            .map(|a| Shortcut::from_str(&a).map_err(|e| format!("push-to-talk hotkey “{a}”: {e}")))
            .transpose()?
    };
    // Hands-free's primary mechanism is the tap-latch on the push-to-talk key
    // (see `fn_gesture::route_gesture`); this is the OPTIONAL separate
    // accelerator to toggle it, disabled (empty) by default. Always an
    // accelerator — no `fn` gesture, no fallback.
    let hands_free_key = settings.hands_free_hotkey.key.trim();
    let hands_free = if hands_free_key.is_empty() {
        None
    } else {
        Some(
            Shortcut::from_str(hands_free_key)
                .map_err(|e| format!("hands-free hotkey “{hands_free_key}”: {e}"))?,
        )
    };
    // See-changes is an accelerator trigger; a stray gesture here falls back to
    // the historical default rather than disabling it.
    let changes = resolve(&settings.see_changes_hotkey, "Alt+O")
        .map(|a| Shortcut::from_str(&a).map_err(|e| format!("see-changes hotkey “{a}”: {e}")))
        .transpose()?;

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

    // Every registered hotkey must be pairwise distinct — push-to-talk,
    // hands-free, see-changes, and each bound prompt shortcut. (Shortcut is Copy,
    // so collecting them here leaves the originals usable for registration
    // below.) With the gesture defaults, push-to-talk and hands-free resolve to
    // the two distinct fallbacks, so both register.
    let mut all: Vec<Shortcut> = Vec::new();
    all.extend(push_to_talk.iter().copied());
    all.extend(hands_free.iter().copied());
    all.extend(changes.iter().copied());
    all.extend(prompts.iter().map(|(_, sc)| *sc));
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            if all[i] == all[j] {
                return Err("two hotkeys are the same — every hotkey must be different".into());
            }
        }
    }

    // Push-to-talk is the hold path: press starts, release stops (a sub-threshold
    // tap latches hands-free instead — see `pipeline::on_hotkey_released`).
    if let Some(push_to_talk) = push_to_talk {
        shortcuts
            .on_shortcut(push_to_talk, move |app, _shortcut, event| {
                dispatch(app, Job::Dictation, event.state());
            })
            .map_err(|e| format!("push-to-talk hotkey: {e}"))?;
    }

    // The optional hands-free accelerator toggles via press only: a press
    // starts, the next press finishes (the existing "press while recording →
    // finish" path). Release is ignored. (The primary hands-free is the tap-latch
    // on the push-to-talk key.) Offloaded like the prompt handler — start/finish
    // block on worker replies.
    if let Some(hands_free) = hands_free {
        if let Err(e) = shortcuts.on_shortcut(hands_free, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let pipeline = app.state::<AppState>().pipeline.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    pipeline.on_hotkey_pressed(Job::Dictation)
                });
            }
        }) {
            let _ = shortcuts.unregister_all();
            return Err(format!("hands-free hotkey could not be registered: {e}"));
        }
    }

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
            return Err(format!("see-changes hotkey could not be registered: {e}"));
        }
    }

    log::info!(
        "hotkeys registered: push-to-talk={} hands-free={} see-changes={} prompts={}",
        push_to_talk.is_some(),
        hands_free.is_some(),
        changes.is_some(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hk(kind: HotkeyKind, key: &str) -> Hotkey {
        Hotkey {
            kind,
            key: key.into(),
        }
    }

    #[test]
    fn accelerator_kind_uses_its_own_key() {
        assert_eq!(
            resolve(&hk(HotkeyKind::Accelerator, "Alt+O"), PUSH_TO_TALK_FALLBACK),
            Some("Alt+O".to_string())
        );
    }

    #[test]
    fn empty_accelerator_disables_the_trigger() {
        assert_eq!(
            resolve(&hk(HotkeyKind::Accelerator, ""), PUSH_TO_TALK_FALLBACK),
            None
        );
        assert_eq!(
            resolve(&hk(HotkeyKind::Accelerator, "   "), PUSH_TO_TALK_FALLBACK),
            None
        );
    }

    #[test]
    fn gesture_kinds_resolve_to_their_fallback() {
        // `resolve` always yields the fallback for a gesture; `apply` decides
        // whether to register it (skipped when the `fn` tap is live). Only
        // push-to-talk uses a gesture today.
        assert_eq!(
            resolve(&hk(HotkeyKind::Hold, "fn"), PUSH_TO_TALK_FALLBACK),
            Some(PUSH_TO_TALK_FALLBACK.to_string())
        );
    }

    #[test]
    fn fn_gestures_are_recognized() {
        assert!(is_fn_gesture(&hk(HotkeyKind::Hold, "fn")));
        assert!(is_fn_gesture(&hk(HotkeyKind::DoubleTap, "fn")));
    }

    #[test]
    fn accelerators_and_non_fn_keys_are_not_fn_gestures() {
        // An accelerator is never the `fn` tap's job, and a gesture on any other
        // key is not the observable `fn` modifier.
        assert!(!is_fn_gesture(&hk(HotkeyKind::Accelerator, "Alt+Space")));
        assert!(!is_fn_gesture(&hk(HotkeyKind::Accelerator, "fn")));
        assert!(!is_fn_gesture(&hk(HotkeyKind::Hold, "F13")));
    }
}
