//! Menu-bar (tray) icon and menu.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::settings::InsertMethod;
use crate::state::AppState;

pub const TRAY_ID: &str = "openflow-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .tooltip("OpenFlow")
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

/// Rebuilds the menu after settings changes (modes added/renamed/activated).
pub fn rebuild_menu(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_menu(app)?))?;
    }
    Ok(())
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<AppState>();
    let settings = state.settings.get();

    let menu = Menu::new(app)?;
    let header = MenuItem::with_id(app, "header", "Mode", false, None::<&str>)?;
    menu.append(&header)?;
    for mode in &settings.modes {
        let item = CheckMenuItem::with_id(
            app,
            format!("mode:{}", mode.id),
            &mode.name,
            true,
            mode.id == settings.active_mode_id,
            None::<&str>,
        )?;
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "copy-last",
        "Copy Last Result",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "settings",
        "Settings…",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quit OpenFlow",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "quit" => app.exit(0),
        "settings" => show_settings_window(app),
        "copy-last" => {
            let state = app.state::<AppState>();
            if let Some(result) = state.pipeline.last_result() {
                if let Err(err) = state
                    .output
                    .insert(result.text, InsertMethod::Clipboard, false)
                {
                    log::warn!("copy last result failed: {err}");
                }
            }
        }
        other => {
            if let Some(mode_id) = other.strip_prefix("mode:") {
                set_active_mode(app, mode_id);
            }
        }
    }
}

fn set_active_mode(app: &AppHandle, mode_id: &str) {
    let state = app.state::<AppState>();
    let mut settings = state.settings.get();
    settings.active_mode_id = mode_id.to_string();
    match state.settings.set(settings) {
        Ok(saved) => {
            let _ = tauri::Emitter::emit(app, "settings-changed", &saved);
            if let Err(err) = rebuild_menu(app) {
                log::warn!("tray rebuild failed: {err}");
            }
        }
        Err(err) => log::warn!("could not switch mode: {err}"),
    }
}

/// Shows the settings window, bringing the app out of Accessory mode so it
/// appears in the Dock and can take focus while open.
pub fn show_settings_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
