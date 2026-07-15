mod acp;
mod agents;
mod commands;
mod session_store;
mod state;

use std::sync::Arc;

use state::{ForgeState, SharedState};
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let shared_state: SharedState = Arc::new(tokio::sync::Mutex::new(ForgeState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_project_status,
            commands::open_project,
            commands::close_project,
            commands::send_prompt,
            commands::respond_permission,
            commands::set_config_option,
            commands::list_sessions,
            commands::create_session,
            commands::load_session,
            commands::close_session,
            commands::search_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}