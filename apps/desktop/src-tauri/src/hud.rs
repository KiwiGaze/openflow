//! The recording HUD overlay.
//!
//! Tauri windows on macOS can steal focus when shown even with
//! `focusable: false` (tauri#14102), which would defocus the app the user is
//! dictating into. To sidestep that entirely, the HUD window is shown once at
//! startup and stays visible forever: transparent, click-through, on every
//! workspace. The webview content fades in/out based on pipeline events, so
//! "showing" the HUD never touches window ordering or focus.

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
