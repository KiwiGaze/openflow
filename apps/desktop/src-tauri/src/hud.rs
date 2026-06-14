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

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, WebviewWindow};

pub const HUD_LABEL: &str = "hud";

/// Gap between the HUD and the bottom edge of the screen, in logical px.
const BOTTOM_MARGIN: f64 = 96.0;

/// Distance from the window's bottom-right corner to the circle's center, in
/// logical px: CSS pins `.hud-circle` `right:12px; bottom:12px` at 28px wide, so
/// its center sits 12 + 14 = 26px in from each edge.
const CIRCLE_INSET: f64 = 26.0;
/// Circle radius (28px / 2) plus a few px of slop so the edge stays clickable.
const CIRCLE_HIT_RADIUS: f64 = 14.0 + 4.0;

/// True while the post-dictation dropdown is open. Set by `set_hud_menu_open`
/// (the `set_hud_menu_open` command) and read by the cursor poll, which forces
/// the whole window interactive while the menu is open so a radio click anywhere
/// in the frame lands — the HTML menu can sit outside the circle's hit rect.
static MENU_OPEN: AtomicBool = AtomicBool::new(false);

/// Records whether the dropdown is open. Off the main thread; the poll reads it.
pub fn set_menu_open(open: bool) {
    MENU_OPEN.store(open, Ordering::Relaxed);
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let Some(hud) = app.get_webview_window(HUD_LABEL) else {
        log::warn!("hud window missing from config");
        return Ok(());
    };
    hud.set_ignore_cursor_events(true)?;
    allow_over_fullscreen(&hud);
    // Reclass into a non-activating panel so clicks on the circle never pull
    // focus from the app the user is dictating into (mirrors changes.rs).
    make_nonactivating_panel(&hud);
    position_on_cursor_monitor(app);
    hud.show()?;
    spawn_cursor_poll(app.clone());
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
    let Some(monitor) = monitor_under_cursor(app)? else {
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

/// The monitor holding the cursor — the best proxy for where the user is
/// working — falling back to the primary monitor. Shared with the changes
/// overlay so both windows always agree on the target monitor.
pub fn monitor_under_cursor(app: &AppHandle) -> tauri::Result<Option<Monitor>> {
    let Ok(point) = app.cursor_position() else {
        return app.primary_monitor();
    };
    Ok(app
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
        .or(app.primary_monitor()?))
}

/// Whether the cursor sits within the circle's hit box. All inputs are top-left
/// physical pixels and share one coordinate space: `cursor`, the window
/// `position`/`size`, and `scale` from `scale_factor()`. The circle's geometry
/// comes from CSS in logical px, so its insets are scaled here. Split out so the
/// geometry stays unit-testable without a live window.
fn cursor_over_circle(
    cursor: (f64, f64),
    position: (f64, f64),
    size: (f64, f64),
    scale: f64,
) -> bool {
    let center_x = position.0 + size.0 - CIRCLE_INSET * scale;
    let center_y = position.1 + size.1 - CIRCLE_INSET * scale;
    let reach = CIRCLE_HIT_RADIUS * scale;
    (cursor.0 - center_x).abs() <= reach && (cursor.1 - center_y).abs() <= reach
}

/// Makes only the circle (and the open dropdown) clickable without ever
/// hiding/showing the HUD or stealing focus. Every ~30 ms it toggles
/// `set_ignore_cursor_events` so the rest of the transparent frame stays
/// click-through. The window is created once and never destroyed, so the cached
/// handle stays valid; its getters still read the live position. Toggling only
/// on change keeps the hot path quiet — this is deliberately unoptimized for
/// now (a later slice can event-drive it). `set_ignore_cursor_events` is
/// `Send`-safe: Tauri marshals it to the main thread.
fn spawn_cursor_poll(app: AppHandle) {
    let Some(hud) = app.get_webview_window(HUD_LABEL) else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("velata-hud-cursor".into())
        .spawn(move || {
            // Mirror init's `set_ignore_cursor_events(true)`, so the first
            // interactive transition actually flips the flag.
            let mut interactive = false;
            loop {
                std::thread::sleep(Duration::from_millis(30));
                let want = MENU_OPEN.load(Ordering::Relaxed) || cursor_in_circle(&app, &hud);
                if want != interactive {
                    let _ = hud.set_ignore_cursor_events(!want);
                    interactive = want;
                }
            }
        });
}

/// Reads the live cursor and window geometry and hit-tests the circle. Any
/// getter error yields `false` (stay click-through) rather than panicking on the
/// poll thread.
fn cursor_in_circle(app: &AppHandle, hud: &WebviewWindow) -> bool {
    let (Ok(cursor), Ok(position), Ok(size), Ok(scale)) = (
        app.cursor_position(),
        hud.outer_position(),
        hud.outer_size(),
        hud.scale_factor(),
    ) else {
        return false;
    };
    cursor_over_circle(
        (cursor.x, cursor.y),
        (position.x as f64, position.y as f64),
        (size.width as f64, size.height as f64),
        scale,
    )
}

/// Reclasses the HUD's `NSWindow` into a non-activating `NSPanel` so clicks on
/// the circle are received without activating Velata, leaving the app the user
/// dictated into focused. Mirrors changes.rs; see its doc comment for why the
/// raw `object_setClass` is safe with tao's larger window class.
#[cfg(target_os = "macos")]
fn make_nonactivating_panel(hud: &WebviewWindow) {
    use objc2::ffi::object_setClass;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_app_kit::{NSPanel, NSWindow, NSWindowStyleMask};

    let ptr = match hud.ns_window() {
        Ok(ptr) => ptr,
        Err(err) => {
            log::warn!("could not reach HUD window for panel conversion: {err}");
            return;
        }
    };

    // SAFETY: the HUD window is created at startup and never closed, so its
    // `NSWindow` outlives this call. `init` runs on the main thread (Tauri
    // `setup`), where mutating an AppKit window is valid.
    unsafe {
        let object = ptr.cast::<AnyObject>();
        let panel_class: *const AnyClass = objc2::class!(NSPanel);
        let _ = object_setClass(object, panel_class);

        let panel = &*ptr.cast::<NSPanel>();
        let win = &*ptr.cast::<NSWindow>();
        win.setStyleMask(win.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        // Only grab keyboard if a control needs it (none does), so the app the
        // user dictated into keeps key focus.
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setFloatingPanel(true);
        // LOAD-BEARING: without this the panel hides whenever Velata is not
        // frontmost — i.e. always, while the user dictates into another app.
        win.setHidesOnDeactivate(false);
    }
}

#[cfg(not(target_os = "macos"))]
fn make_nonactivating_panel(_hud: &WebviewWindow) {}

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

#[cfg(test)]
mod hit_test {
    use super::cursor_over_circle;

    // A 600x280 window whose top-left is at (1000, 1000) in physical px.
    const POS: (f64, f64) = (1000.0, 1000.0);
    const SIZE: (f64, f64) = (600.0, 280.0);

    #[test]
    fn center_is_inside_at_1x() {
        // Center sits 26px in from the bottom-right corner at scale 1.
        let center = (POS.0 + SIZE.0 - 26.0, POS.1 + SIZE.1 - 26.0);
        assert!(cursor_over_circle(center, POS, SIZE, 1.0));
    }

    #[test]
    fn retina_doubles_the_inset() {
        // On a 2x display the CSS inset is 52 physical px from each edge, so the
        // hit center is at 52px in, not 26px. Probe a point ~70px in (well past
        // the 36px reach at 1x scale): out at 1x, in at 2x — proving scale is
        // applied to the inset, not ignored.
        let deep = (POS.0 + SIZE.0 - 70.0, POS.1 + SIZE.1 - 70.0);
        assert!(!cursor_over_circle(deep, POS, SIZE, 1.0));
        assert!(cursor_over_circle(deep, POS, SIZE, 2.0));
    }

    #[test]
    fn far_from_corner_is_outside() {
        assert!(!cursor_over_circle((POS.0, POS.1), POS, SIZE, 1.0));
    }
}
