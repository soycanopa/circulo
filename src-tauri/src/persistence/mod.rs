mod automations;
mod config;
mod mcp;
mod transcripts;

pub use automations::{delete_automation, list_automations, save_automation, Automation};
pub use mcp::{
    auto_load_servers, delete_mcp_server, import_mcp, list_mcp_imports, load_mcp_servers,
    mcp_registry_path, save_mcp_servers, slugify_id, upsert_mcp_server, validate_mcp_server,
    ManagedMcpServer, McpEnvVar, McpImportCandidate, McpServerKind,
};
pub use config::{
    active_workspace_chats_path, create_workspace, delete_workspace, load_settings,
    remove_project_from_workspace, save_settings, set_active_workspace, touch_recent_project,
    workspace_chats_dir, workspace_entry_path, AppSettings, CustomSlashCommand,
    MAX_CUSTOM_SLASH_COMMANDS,
};
pub use transcripts::{
    delete_chat_transcript, list_chat_sessions, load_chat_transcript, rename_chat_transcript,
    save_chat_transcript, seed_chat_transcript, ChatSessionSummary, StoredChatMessage, StoredTranscript,
};

pub fn circulo_data_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let dir = std::path::PathBuf::from(home).join(".circulo");
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create ~/.circulo: {err}"))?;
    Ok(dir)
}
