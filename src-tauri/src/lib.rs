mod acp;
mod agents;
mod cli_resolve;
mod commands;
mod state;

use std::sync::Arc;

use state::{CirculoState, SharedState};
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("circulo_lib=info".parse().unwrap()))
        .init();

    let shared_state: SharedState = Arc::new(tokio::sync::Mutex::new(CirculoState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_project_status,
            commands::open_project,
            commands::close_project,
            commands::create_session,
            commands::send_prompt,
            commands::respond_permission,
            commands::set_config_option,
            commands::search_files,
            commands::pick_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
