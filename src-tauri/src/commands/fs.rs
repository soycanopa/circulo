use crate::project_fs::{read_directory, DirectoryEntry};
use std::path::PathBuf;

#[tauri::command]
pub async fn read_directory_cmd(dir_path: String) -> Result<Vec<DirectoryEntry>, String> {
    let dir = PathBuf::from(&dir_path);
    tauri::async_runtime::spawn_blocking(move || read_directory(&dir))
        .await
        .map_err(|err| format!("Directory read task failed: {err}"))?
}
