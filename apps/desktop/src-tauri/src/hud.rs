//! The recording HUD overlay.
//!
//! Tauri windows on macOS can steal focus when shown even with
//! `focusable: false` (tauri#14102), which would defocus the app the user is
//! dictating into. To sidestep that entirely, the HUD window is shown once at
//! startup and stays visible forever: transparent, click-through, on every
//! workspace, and — via `NSWindowCollectionBehaviorFullScreenAuxiliary` — over
//! other apps' native full-screen Spaces too. The webview content fades in/out
//! based on pipeline events, so "showing" the HUD never touches window ordering
//! or focus.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

pub const HUD_LABEL: &str = "hud";

/// Gap between the HUD and the bottom edge of the screen, in logical px.
const BOTTOM_MARGIN: f64 = 96.0;

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let Some(hud) = app.get_webview_window(HUD_LABEL) else {
        log::warn!("hud window missing from config");
        return Ok(());
    };
    hud.set_ignore_cursor_events(true)?;
    allow_over_fullscreen(&hud);
    position_on_cursor_monitor(app);
    hud.show()?;
    Ok(())
}

/// Centers the HUD at the bottom of whichever monitor holds the cursor —
/// a good proxy for where the user is typing.
pub fn position_on_cursor_monitor(app: &AppHandle) {
    let Some(hud) = app.get_webview_window(HUD_LABEL) else {
        return;
    };
    if let Err(err) = try_position(app, &hud) {
        log::warn!("could not position HUD: {err}");
    }
}

fn try_position(app: &AppHandle, hud: &WebviewWindow) -> tauri::Result<()> {
    let cursor = app.cursor_position().ok();
    let monitor = match cursor {
        Some(point) => app
            .available_monitors()?
            .into_iter()
            .find(|m| {
                let pos = m.position();
                let size = m.size();
                point.x >= pos.x as f64
                    && point.x < (pos.x + size.width as i32) as f64
                    && point.y >= pos.y as f64
                    && point.y < (pos.y + size.height as i32) as f64
            })
            .or(app.primary_monitor()?),
        None => app.primary_monitor()?,
    };
    let Some(monitor) = monitor else {
        return Ok(());
    };

    let window_size = hud.outer_size()?;
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let margin = (BOTTOM_MARGIN * monitor.scale_factor()) as i32;

    let x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;
    let y = monitor_pos.y + monitor_size.height as i32 - window_size.height as i32 - margin;
    hud.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

/// Adds `FullScreenAuxiliary` to a collection behavior, leaving the other bits
/// untouched. Split out from the AppKit glue so the OR-don't-replace contract
/// stays unit-testable.
#[cfg(target_os = "macos")]
fn with_fullscreen_overlay(
    behavior: objc2_app_kit::NSWindowCollectionBehavior,
) -> objc2_app_kit::NSWindowCollectionBehavior {
    behavior | objc2_app_kit::NSWindowCollectionBehavior::FullScreenAuxiliary
}

/// Lets the HUD render over other apps' native full-screen Spaces.
///
/// macOS puts each native full-screen app in its own Space, and a window only
/// appears there if its collection behavior includes `FullScreenAuxiliary`.
/// `visibleOnAllWorkspaces: true` only sets `CanJoinAllSpaces`, which covers
/// ordinary Spaces but not full-screen ones, and tao/Tauri expose no setting
/// for the extra flag. We OR it onto the window's current behavior so the
/// `CanJoinAllSpaces` bit Tauri already set survives.
#[cfg(target_os = "macos")]
fn allow_over_fullscreen(hud: &WebviewWindow) {
    use objc2_app_kit::NSWindow;

    let ptr = match hud.ns_window() {
        Ok(ptr) => ptr.cast::<NSWindow>(),
        Err(err) => {
            log::warn!("could not reach HUD window for full-screen overlay: {err}");
            return;
        }
    };
    // SAFETY: the HUD window is created at startup and never closed, so its
    // `NSWindow` outlives this borrow. `init` runs on the main thread (Tauri
    // `setup`), where mutating an AppKit window is valid.
    let ns_window = unsafe { &*ptr };
    ns_window.setCollectionBehavior(with_fullscreen_overlay(ns_window.collectionBehavior()));
}

#[cfg(not(target_os = "macos"))]
fn allow_over_fullscreen(_hud: &WebviewWindow) {}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::with_fullscreen_overlay;
    use objc2_app_kit::NSWindowCollectionBehavior as Behavior;

    #[test]
    fn adds_fullscreen_auxiliary_and_keeps_existing_bits() {
        let result = with_fullscreen_overlay(Behavior::CanJoinAllSpaces);
        assert!(result.contains(Behavior::FullScreenAuxiliary));
        assert!(result.contains(Behavior::CanJoinAllSpaces));
    }

    #[test]
    fn is_idempotent() {
        let once = with_fullscreen_overlay(Behavior::CanJoinAllSpaces);
        assert_eq!(once, with_fullscreen_overlay(once));
    }
}
