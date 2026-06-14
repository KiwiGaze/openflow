// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod audio;
mod changes;
mod cloud_stt;
mod commands;
mod db;
mod error;
mod fn_gesture;
mod history;
mod hud;
mod llm;
mod models;
mod notes;
mod output;
mod permissions;
mod pipeline;
mod profiles;
mod prompts;
mod resample;
mod scratchpad;
mod settings;
mod shortcuts;
mod state;
mod stats;
mod stt;
mod stt_profiles;
mod suggestions;
mod text;
mod tray;

use std::sync::Arc;

use tauri::Manager;

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        // Must be first: a second launch focuses the existing instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // Menu-bar app: no Dock icon until a window is opened.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;

            let settings = Arc::new(settings::SettingsManager::load(&config_dir));
            let profiles = Arc::new(profiles::ProfileManager::new(data_dir.join("profiles")));
            // Moves a v1 inline LLM config into a profile file and repairs a
            // dangling active-profile pointer.
            profiles::reconcile(&settings, &profiles);
            let models = Arc::new(models::ModelManager::new(data_dir.join("models")));
            let db = Arc::new(db::Db::open(&data_dir)?);
            let history = Arc::new(history::HistoryStore::new(Arc::clone(&db)));
            // Enforce history retention once at startup so an entry that aged out
            // while the app was closed is gone before any view can read it. 0 =
            // keep forever (no purge); the per-append purge handles the rest.
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            if let Some(cutoff) =
                history::retention_cutoff_ms(now_ms, settings.get().history_retention_days)
            {
                match db.history_purge_older_than(cutoff) {
                    Ok(removed) if removed > 0 => {
                        log::info!("purged {removed} history entries past the retention window")
                    }
                    Ok(_) => {}
                    Err(err) => log::warn!("startup history retention purge failed: {err}"),
                }
            }
            let stt_profiles = Arc::new(stt_profiles::SttProfileManager::new(
                data_dir.join("stt-profiles"),
            ));
            let stt = Arc::new(stt::SttEngine::new());
            let llm = Arc::new(llm::LlmClient::new());
            let audio = Arc::new(audio::AudioSystem::spawn());
            let output = Arc::new(output::OutputSystem::spawn(handle.clone()));
            let pipeline = pipeline::Pipeline::new(
                handle.clone(),
                Arc::clone(&audio),
                Arc::clone(&stt),
                Arc::clone(&llm),
                Arc::clone(&output),
                Arc::clone(&settings),
                Arc::clone(&models),
                Arc::clone(&profiles),
                Arc::clone(&history),
                Arc::clone(&db),
                Arc::clone(&stt_profiles),
            );

            app.manage(AppState {
                settings: Arc::clone(&settings),
                profiles,
                models,
                stt,
                llm,
                output,
                pipeline,
                db,
                history,
                stt_profiles,
            });

            hud::init(&handle)?;
            changes::init(&handle)?;
            tray::build(&handle)?;

            if let Err(err) = shortcuts::apply(&handle, &settings.get()) {
                // Not fatal: the settings UI reports and lets the user rebind.
                log::warn!("hotkey registration failed: {err}");
            }

            // Apply the persistent Dock preference now that settings are loaded
            // (the default above is Accessory; this upgrades to Regular if set).
            commands::apply_dock_policy(&handle, settings.get().show_in_dock);
            commands::apply_appearance(&handle, settings.get().appearance);

            if !settings.get().onboarding_completed {
                tray::show_main_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing either the App or Settings window hides it; the app lives
            // in the menu bar until quit from the tray.
            let label = window.label();
            let other = match label {
                commands::MAIN_WINDOW_LABEL => Some(commands::SETTINGS_WINDOW_LABEL),
                commands::SETTINGS_WINDOW_LABEL => Some(commands::MAIN_WINDOW_LABEL),
                _ => None,
            };
            if let Some(other) = other {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    let app = window.app_handle();
                    // Keep the Dock icon while the other window is still open;
                    // only drop to Accessory once neither is visible (and the
                    // user has not pinned the Dock icon). Never upgrade to
                    // Regular here, so there is no Accessory→Regular flicker.
                    let other_visible = app
                        .get_webview_window(other)
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);
                    if !other_visible {
                        let show_in_dock = app.state::<AppState>().settings.get().show_in_dock;
                        commands::apply_dock_policy(app, show_in_dock);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_models,
            commands::download_model,
            commands::cancel_model_download,
            commands::delete_model,
            commands::get_pipeline_state,
            commands::start_dictation,
            commands::stop_dictation,
            commands::cancel_dictation,
            commands::get_last_result,
            commands::get_history,
            commands::clear_history,
            commands::delete_history_entry,
            commands::get_insights,
            commands::list_dictionary_suggestions,
            commands::dismiss_dictionary_suggestion,
            commands::copy_text,
            commands::set_changes_interactive,
            commands::set_hud_menu_open,
            commands::test_llm,
            commands::list_llm_profiles,
            commands::save_llm_profile,
            commands::delete_llm_profile,
            commands::reveal_llm_profiles,
            commands::list_stt_profiles,
            commands::save_stt_profile,
            commands::delete_stt_profile,
            commands::reveal_stt_profiles,
            commands::export_dictionary,
            commands::set_post_dictation_transform,
            commands::list_ollama_models,
            commands::list_input_devices,
            commands::check_permissions,
            commands::request_microphone_permission,
            commands::prompt_accessibility_permission,
            commands::open_accessibility_settings,
            commands::open_microphone_settings,
            commands::request_input_monitoring,
            commands::get_app_info,
            commands::list_notes,
            commands::get_note,
            commands::create_note,
            commands::update_note,
            commands::set_note_pinned,
            commands::delete_note,
            commands::list_note_versions,
            commands::restore_note_version,
            commands::transform_note_text,
            commands::open_scratchpad_window,
            commands::open_main_window,
            commands::open_settings_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Velata")
        .run(|app, event| {
            // Quit ends in libc exit() (⌘Q via -[NSApplication terminate:],
            // tray quit via app.exit(0) → process exit), which never drops
            // Rust state. ggml's Metal teardown then aborts on still-resident
            // whisper buffers, so free the context while drops still run.
            if let tauri::RunEvent::Exit = event {
                log::info!("unloading whisper context before exit");
                app.state::<AppState>().stt.unload();
            }
        });
}
