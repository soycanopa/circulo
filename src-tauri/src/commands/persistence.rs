use crate::persistence::{
    create_workspace, delete_automation, delete_chat_transcript, delete_workspace, list_automations,
    list_chat_sessions, load_chat_transcript, load_settings, rename_chat_transcript,
    save_automation, save_chat_transcript, save_settings, seed_chat_transcript, set_active_workspace,
    workspace_chats_dir, workspace_entry_path, AppSettings, Automation, ChatSessionSummary,
    StoredChatMessage, StoredTranscript,
};

#[tauri::command]
pub fn set_preferred_agent_cmd(agent_id: String) -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    settings.preferred_agent_id = Some(agent_id);
    save_settings(&settings)?;
    load_settings()
}

#[tauri::command]
pub fn list_automations_cmd() -> Result<Vec<Automation>, String> {
    list_automations()
}

#[tauri::command]
pub fn save_automation_cmd(title: String, prompt: String) -> Result<Automation, String> {
    save_automation(title, prompt)
}

#[tauri::command]
pub fn delete_automation_cmd(id: String) -> Result<(), String> {
    delete_automation(&id)
}

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
pub fn create_workspace_cmd() -> Result<AppSettings, String> {
    create_workspace()
}

#[tauri::command]
pub fn set_active_workspace_cmd(workspace_id: String) -> Result<AppSettings, String> {
    set_active_workspace(workspace_id)
}

#[tauri::command]
pub fn delete_workspace_cmd(workspace_id: String) -> Result<AppSettings, String> {
    delete_workspace(workspace_id)
}

/// General chats path + preferred open path when entering a workspace.
#[tauri::command]
pub fn get_workspace_paths_cmd(
    workspace_id: String,
) -> Result<WorkspacePathsDto, String> {
    let settings = load_settings()?;
    let chats_path = workspace_chats_dir(&workspace_id)?.display().to_string();
    let entry_path = workspace_entry_path(&settings, &workspace_id)?;
    Ok(WorkspacePathsDto {
        chats_path,
        entry_path,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePathsDto {
    pub chats_path: String,
    pub entry_path: String,
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

/// Seed an empty transcript with a provisional title so the sidebar can render
/// the chat immediately after `session_ready`, before the first chunk arrives.
#[tauri::command]
pub fn seed_chat_transcript_cmd(
    project_path: String,
    session_id: String,
    title: String,
) -> Result<ChatSessionSummary, String> {
    seed_chat_transcript(&project_path, &session_id, &title)
}

#[tauri::command]
pub fn delete_chat_transcript_cmd(
    project_path: String,
    session_id: String,
) -> Result<(), String> {
    delete_chat_transcript(&project_path, &session_id)
}

#[tauri::command]
pub fn rename_chat_transcript_cmd(
    project_path: String,
    session_id: String,
    title: String,
) -> Result<ChatSessionSummary, String> {
    rename_chat_transcript(&project_path, &session_id, &title)
}
