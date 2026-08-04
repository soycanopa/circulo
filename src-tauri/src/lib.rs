mod acp;
mod agents;
mod cli_resolve;
mod commands;
mod persistence;
mod state;

use std::sync::Arc;

use state::{CirculoState, SharedState};
use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("circulo_lib=info".parse().unwrap()),
        )
        .init();

    let shared_state: SharedState = Arc::new(tokio::sync::Mutex::new(CirculoState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .setup(|app| {
            // Native blur for frost depth; CSS applies a strong neutral gray veil so
            // wallpaper hue barely tints sidebars (gray plate + some transparency).
            if let Some(window) = app.get_webview_window("main") {
                use tauri::window::{Effect, EffectState, EffectsBuilder};
                #[allow(deprecated)]
                let effect = Effect::Dark;
                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(effect)
                        .state(EffectState::Active)
                        .build(),
                );
            }

            // Start OpenCode as early as possible (in parallel with webview load)
            // so the user does not wait a full cold start after the UI appears.
            commands::spawn_eager_agent_warm(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_project_status,
            commands::get_default_chats_path,
            commands::get_home_path,
            commands::check_opencode,
            commands::open_project,
            commands::close_project,
            commands::create_session,
            commands::load_session,
            commands::close_session_cmd,
            commands::send_prompt,
            commands::cancel_prompt,
            commands::respond_permission,
            commands::set_config_option,
            commands::set_visible_session,
            commands::search_files,
            commands::pick_directory,
            commands::complete_directory_path,
            commands::export_transcript_cmd,
            commands::persistence::list_automations_cmd,
            commands::persistence::save_automation_cmd,
            commands::persistence::delete_automation_cmd,
            commands::persistence::get_app_settings,
            commands::persistence::set_app_settings,
            commands::persistence::create_workspace_cmd,
            commands::persistence::set_active_workspace_cmd,
            commands::persistence::delete_workspace_cmd,
            commands::persistence::get_workspace_paths_cmd,
            commands::persistence::list_chat_sessions_cmd,
            commands::persistence::load_chat_transcript_cmd,
            commands::persistence::save_chat_transcript_cmd,
            commands::persistence::seed_chat_transcript_cmd,
            commands::persistence::delete_chat_transcript_cmd,
            commands::persistence::rename_chat_transcript_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
