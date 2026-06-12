// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod audio;
mod changes;
mod cloud_stt;
mod commands;
mod db;
mod error;
mod history;
mod hud;
mod llm;
mod models;
mod modes;
mod output;
mod permissions;
mod pipeline;
mod profiles;
mod resample;
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
            tray::show_settings_window(app);
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
                tray::show_settings_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window hides it; the app lives in the
            // menu bar until quit from the tray.
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    // Drop the Dock icon on close — unless the user pinned it.
                    let app = window.app_handle();
                    let show_in_dock = app.state::<AppState>().settings.get().show_in_dock;
                    commands::apply_dock_policy(app, show_in_dock);
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
            commands::start_polish_selection,
            commands::get_last_result,
            commands::get_last_dictation_app,
            commands::get_history,
            commands::clear_history,
            commands::delete_history_entry,
            commands::reprocess_history,
            commands::get_insights,
            commands::clear_insights,
            commands::list_dictionary_suggestions,
            commands::dismiss_dictionary_suggestion,
            commands::copy_text,
            commands::set_changes_interactive,
            commands::test_llm,
            commands::test_mode,
            commands::list_llm_profiles,
            commands::save_llm_profile,
            commands::delete_llm_profile,
            commands::reveal_llm_profiles,
            commands::list_stt_profiles,
            commands::save_stt_profile,
            commands::delete_stt_profile,
            commands::reveal_stt_profiles,
            commands::export_mode,
            commands::export_dictionary,
            commands::list_ollama_models,
            commands::check_permissions,
            commands::request_microphone_permission,
            commands::prompt_accessibility_permission,
            commands::open_accessibility_settings,
            commands::open_microphone_settings,
            commands::get_app_info,
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
