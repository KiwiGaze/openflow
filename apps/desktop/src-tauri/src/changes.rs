//! The "see changes" overlay.
//!
//! A second always-present overlay (a sibling of the HUD) that shows a
//! word-level diff of the last result when the user presses the see-changes
//! hotkey. Unlike the HUD it must take clicks (Copy / Close buttons), so it
//! cannot be click-through. A plain window would activate the app on click and
//! send the target app's `⌘Z` to us instead — so the window is reclassed into a
//! macOS non-activating `NSPanel`: it receives mouse events without activating
//! OpenFlow, leaving the app the user dictated into focused and its undo intact.
//!
//! Like the HUD, the window is created once and never reopened (tauri#14102);
//! the webview content fades in and out, and the panel toggles between
//! click-through (hidden, so the empty frame never eats clicks) and interactive
//! (visible) via `set_changes_interactive`.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use crate::hud::monitor_under_cursor;
use crate::state::AppState;

pub const CHANGES_LABEL: &str = "changes";
pub const CHANGES_TOGGLE_EVENT: &str = "changes-toggle";

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(CHANGES_LABEL) else {
        log::warn!("changes window missing from config");
        return Ok(());
    };
    make_nonactivating_panel(&window);
    position_on_cursor_monitor(app);
    window.show()?;
    // Starts hidden (content faded out), so keep it click-through until the
    // webview reports it became visible.
    window.set_ignore_cursor_events(true)?;
    Ok(())
}

/// Reads the last result and asks the overlay to toggle. Does nothing when
/// there is no result to show. Cheap enough to run inline on the hotkey thread:
/// no clipboard or keystroke work, unlike the dictation jobs.
pub fn request_toggle(app: &AppHandle) {
    let Some(result) = app.state::<AppState>().pipeline.last_result() else {
        return;
    };
    position_on_cursor_monitor(app);
    let _ = app.emit(CHANGES_TOGGLE_EVENT, &result);
}

/// Centers the overlay on whichever monitor holds the cursor — a good proxy for
/// where the user is working.
pub fn position_on_cursor_monitor(app: &AppHandle) {
    let Some(window) = app.get_webview_window(CHANGES_LABEL) else {
        return;
    };
    if let Err(err) = try_position(app, &window) {
        log::warn!("could not position changes overlay: {err}");
    }
}

fn try_position(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = monitor_under_cursor(app)? else {
        return Ok(());
    };

    let window_size = window.outer_size()?;
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;
    let y = monitor_pos.y + (monitor_size.height as i32 - window_size.height as i32) / 2;
    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

/// Reclasses the overlay's `NSWindow` into a non-activating `NSPanel`.
///
/// tao's window class (`TaoWindow`) subclasses `NSWindow` and adds an ivar, so
/// its instances are larger than `NSPanel`. We reclass through the raw runtime
/// call rather than objc2's `AnyObject::set_class`, whose debug-only size
/// assertion would fire. `NSPanel` is a behavioral subclass of `NSWindow` with
/// no extra storage, so the orphaned trailing ivar is harmless; tao's
/// `canBecomeKeyWindow`/`sendEvent:` overrides simply stop being dispatched,
/// which is fine — the panel never needs key focus, only mouse clicks.
#[cfg(target_os = "macos")]
fn make_nonactivating_panel(window: &WebviewWindow) {
    use objc2::ffi::object_setClass;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_app_kit::{NSPanel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};

    let ptr = match window.ns_window() {
        Ok(ptr) => ptr,
        Err(err) => {
            log::warn!("could not reach changes window for panel conversion: {err}");
            return;
        }
    };

    // SAFETY: the changes window is created at startup and never closed, so its
    // `NSWindow` outlives this call. `init` runs on the main thread (Tauri
    // `setup`), where mutating an AppKit window is valid.
    unsafe {
        let object = ptr.cast::<AnyObject>();
        let panel_class: *const AnyClass = objc2::class!(NSPanel);
        let _ = object_setClass(object, panel_class);

        let panel = &*ptr.cast::<NSPanel>();
        let win = &*ptr.cast::<NSWindow>();
        win.setStyleMask(win.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        // Only grab keyboard if a control actually needs it (none here), so the
        // app the user dictated into keeps key focus and its ⌘Z undo.
        panel.setBecomesKeyOnlyIfNeeded(true);
        // Float above ordinary windows and survive our own app deactivating.
        panel.setFloatingPanel(true);
        win.setHidesOnDeactivate(false);
        // Render over other apps' native full-screen Spaces too (see hud.rs).
        win.setCollectionBehavior(
            win.collectionBehavior() | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn make_nonactivating_panel(_window: &WebviewWindow) {}
