//! Menu-bar (tray) icon and menu.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::commands;
use crate::settings::InsertMethod;
use crate::state::AppState;

pub const TRAY_ID: &str = "velata-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .tooltip("Velata")
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

/// Shows a dot beside the menu-bar icon while the mic is live — a privacy/trust
/// signal that you can always see when Velata is listening (P2-7).
pub fn set_recording(app: &AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(if recording { "●" } else { "" }));
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "Open Velata",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "copy-last",
        "Copy last dictation",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quit Velata",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "open" => show_main_window(app),
        "quit" => app.exit(0),
        "copy-last" => {
            let state = app.state::<AppState>();
            match state.pipeline.last_result() {
                Some(result) => {
                    if let Err(err) =
                        state
                            .output
                            .insert(result.text, InsertMethod::Clipboard, false)
                    {
                        log::warn!("copy last result failed: {err}");
                    }
                }
                None => state.pipeline.flash_notice("No dictation yet.".into()),
            }
        }
        _ => {}
    }
}

/// Shows the App window from the tray and startup paths, bringing the app out
/// of Accessory mode so it appears in the Dock and can take focus while open.
pub fn show_main_window(app: &AppHandle) {
    commands::show_window(app, commands::MAIN_WINDOW_LABEL);
}
