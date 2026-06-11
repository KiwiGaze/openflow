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

/// Shows a dot beside the menu-bar icon while the mic is live — a privacy/trust
/// signal that you can always see when OpenFlow is listening (P2-7).
pub fn set_recording(app: &AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(if recording { "●" } else { "" }));
    }
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
    let header = MenuItem::with_id(app, "header", "Writing style", false, None::<&str>)?;
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
    // Always-visible speech locality (08 §3.3): on-device by default, or the
    // active cloud engine with a warning glyph. Disabled — a status, not a control.
    let speech_label = match settings.stt_model_id.strip_prefix("cloud:") {
        Some(pid) => {
            let name = state
                .stt_profiles
                .get(pid)
                .map(|p| p.name)
                .unwrap_or_else(|| "cloud".into());
            format!("Speech: cloud — {name} ⚠")
        }
        None => "Speech: on-device".to_string(),
    };
    menu.append(&MenuItem::with_id(
        app,
        "speech-status",
        &speech_label,
        false,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        "refine-dictation",
        "Refine Dictation with AI",
        true,
        settings.refine_after_dictation,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
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
        "refine-dictation" => toggle_refine_after_dictation(app),
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
        other => {
            if let Some(mode_id) = other.strip_prefix("mode:") {
                set_active_mode(app, mode_id);
            }
        }
    }
}

fn toggle_refine_after_dictation(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut settings = state.settings.get();
    settings.refine_after_dictation = !settings.refine_after_dictation;
    match state.settings.set(settings) {
        Ok(saved) => {
            let _ = tauri::Emitter::emit(app, "settings-changed", &saved);
            if let Err(err) = rebuild_menu(app) {
                log::warn!("tray rebuild failed: {err}");
            }
        }
        Err(err) => log::warn!("could not toggle dictation refinement: {err}"),
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
