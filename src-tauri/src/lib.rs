mod commands;

use commands::sidecar::{init_state, SIDECAR_URL};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .manage(init_state())
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::window::window_minimize,
            commands::window::window_maximize,
            commands::window::window_unmaximize,
            commands::window::window_close,
            commands::window::window_is_maximized,
            commands::window::window_toggle_fullscreen,
            commands::window::window_is_fullscreen,
            commands::window::window_set_title,
            commands::sidecar::sidecar_start,
            commands::sidecar::sidecar_stop,
            commands::sidecar::sidecar_get_status,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<commands::sidecar::SidecarState>();
                let app_handle = handle.clone();
                let _ = commands::sidecar::sidecar_start(app_handle, state).await;
            });
            println!("[circulo] sidecar tRPC esperado en {}", SIDECAR_URL);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al ejecutar Círculo");
}