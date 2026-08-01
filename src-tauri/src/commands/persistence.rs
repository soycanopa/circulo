use crate::persistence::{
    delete_chat_transcript, list_chat_sessions, load_chat_transcript, load_settings,
    save_chat_transcript, save_settings, touch_recent_project, AppSettings, ChatSessionSummary,
    StoredChatMessage, StoredTranscript,
};

#[tauri::command]
pub fn get_app_settings() -> Result<AppSettings, String> {
    load_settings()
}

#[tauri::command]
pub fn set_app_settings(settings: AppSettings) -> Result<AppSettings, String> {
    save_settings(&settings)?;
    load_settings()
}

#[tauri::command]
pub fn list_chat_sessions_cmd(project_path: String) -> Result<Vec<ChatSessionSummary>, String> {
    list_chat_sessions(&project_path)
}

#[tauri::command]
pub fn load_chat_transcript_cmd(
    project_path: String,
    session_id: String,
) -> Result<StoredTranscript, String> {
    load_chat_transcript(&project_path, &session_id)
}

#[tauri::command]
pub fn save_chat_transcript_cmd(
    project_path: String,
    session_id: String,
    messages: Vec<StoredChatMessage>,
) -> Result<ChatSessionSummary, String> {
    save_chat_transcript(&project_path, &session_id, messages)
}

#[tauri::command]
pub fn delete_chat_transcript_cmd(
    project_path: String,
    session_id: String,
) -> Result<(), String> {
    delete_chat_transcript(&project_path, &session_id)
}
