mod acp;
mod agents;
mod commands;
mod opencode_config;
mod skills_cli;
mod session_store;
mod state;

use std::sync::Arc;

use state::{CirculoState, SharedState};
use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let shared_state: SharedState = Arc::new(tokio::sync::Mutex::new(CirculoState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_pty::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window should exist");

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
                    .expect("apply_vibrancy is only supported on macOS");
            }

            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::{apply_blur, apply_mica};
                if apply_mica(&window, None).is_err() {
                    apply_blur(&window, Some((22, 22, 22, 200)))
                        .expect("apply_blur is only supported on Windows");
                }
            }

            Ok(())
        })
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_project_status,
            commands::open_project,
            commands::close_project,
            commands::send_prompt,
            commands::respond_permission,
            commands::respond_credential,
            commands::set_config_option,
            commands::list_sessions,
            commands::create_session,
            commands::load_session,
            commands::close_session,
            commands::rename_session,
            commands::search_files,
            commands::list_opencode_commands,
            commands::list_opencode_skills,
            commands::list_opencode_mcp_servers,
            commands::set_opencode_mcp_enabled,
            commands::search_skills_sh,
            commands::install_skills_sh_skill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}