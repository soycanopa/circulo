use std::path::PathBuf;

use tauri::{AppHandle, State};
use tracing::info;

use crate::agent_pool::{
    shutdown_pooled_agents_for_agent, warm_agent_in_pool,
};
use crate::state::SharedState;
use crate::persistence::{
    create_workspace, delete_automation, delete_chat_transcript, delete_workspace, list_automations,
    list_chat_sessions, load_chat_transcript, load_settings, remove_project_from_workspace,
    rename_chat_transcript, save_automation, save_chat_transcript, save_settings,
    seed_chat_transcript, set_active_workspace, workspace_chats_dir, workspace_entry_path,
    AppSettings, Automation, ChatSessionSummary, StoredChatMessage, StoredTranscript,
};

#[tauri::command]
pub fn set_preferred_agent_cmd(agent_id: String) -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    if !settings.enabled_agent_ids.iter().any(|id| id == &agent_id) {
        return Err(format!("Agent '{agent_id}' is not enabled"));
    }
    crate::agents::ensure_agent_available(&agent_id)?;
    settings.preferred_agent_id = Some(agent_id);
    save_settings(&settings)?;
    load_settings()
}

#[tauri::command]
pub async fn set_enabled_agents_cmd(
    app: AppHandle,
    state: State<'_, SharedState>,
    ids: Vec<String>,
) -> Result<AppSettings, String> {
    if ids.is_empty() {
        return Err("At least one agent must be enabled".to_string());
    }

    for id in &ids {
        if !crate::agents::is_known_agent_id(id) {
            return Err(format!("Unknown agent: {id}"));
        }
    }

    let mut settings = load_settings()?;
    let previous_enabled = settings.enabled_agent_ids.clone();
    settings.enabled_agent_ids = ids;

    let preferred = settings.preferred_agent_id.as_deref();
    let resolved = crate::agents::resolve_enabled_agent_id(
        preferred,
        &settings.enabled_agent_ids,
    )?;
    settings.preferred_agent_id = Some(resolved);

    save_settings(&settings)?;
    let saved = load_settings()?;

    reconcile_pool_for_enabled(&app, state.inner(), &previous_enabled, &saved).await;

    Ok(saved)
}

/// Warm agents added to the enabled set (same cwd as startup warm) and tear down
/// pooled processes for agents that were disabled — so enabling a provider makes
/// switching to it instant instead of a cold start.
async fn reconcile_pool_for_enabled(
    app: &AppHandle,
    state: &SharedState,
    previous_enabled: &[String],
    settings: &AppSettings,
) {
    let newly_enabled: Vec<String> = settings
        .enabled_agent_ids
        .iter()
        .filter(|id| !previous_enabled.contains(id))
        .cloned()
        .collect();
    let now_disabled: Vec<String> = previous_enabled
        .iter()
        .filter(|id| !settings.enabled_agent_ids.contains(id))
        .cloned()
        .collect();

    for agent_id in &now_disabled {
        shutdown_pooled_agents_for_agent(app, state, agent_id).await;
    }
    if newly_enabled.is_empty() {
        return;
    }

    // Warm where the user actually switches agents: the open project and the
    // general-chats folder (they coincide on first launch).
    let open_path = {
        let guard = state.lock().await;
        guard.agent.as_ref().map(|agent| agent.project_path.clone())
    };
    let chats_path = crate::commands::default_chats_path().ok().map(PathBuf::from);

    let mut paths = Vec::new();
    if let Some(path) = open_path {
        paths.push(path);
    }
    if let Some(path) = chats_path {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }

    for path in paths {
        for agent_id in &newly_enabled {
            if crate::agents::ensure_agent_available(agent_id).is_err() {
                tracing::info!(agent_id = %agent_id, "Skipping pool warm — agent not available");
                continue;
            }
            let app = app.clone();
            let state = state.clone();
            let path = path.clone();
            let agent_id = agent_id.clone();
            tauri::async_runtime::spawn(async move {
                match warm_agent_in_pool(&app, &state, path, agent_id.clone()).await {
                    Ok(()) => info!(agent_id = %agent_id, "Warm: newly enabled agent queued"),
                    Err(err) => {
                        tracing::warn!(agent_id = %agent_id, %err, "Warm: newly enabled agent failed")
                    }
                }
            });
        }
    }
}

#[tauri::command]
pub fn set_favorite_model_cmd(
    model_id: String,
    favorite: bool,
) -> Result<AppSettings, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("Model id must not be empty".to_string());
    }

    let mut settings = load_settings()?;
    if favorite {
        if !settings.favorite_model_ids.iter().any(|id| id == model_id) {
            settings.favorite_model_ids.push(model_id.to_string());
        }
    } else {
        settings
            .favorite_model_ids
            .retain(|id| id != model_id);
    }
    save_settings(&settings)?;
    load_settings()
}

const MAX_RECENT_MODELS: usize = 5;

/// Remember a model the user selected, most-recent-first, capped at 5.
#[tauri::command]
pub fn mark_model_used_cmd(model_id: String) -> Result<AppSettings, String> {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err("Model id must not be empty".to_string());
    }

    let mut settings = load_settings()?;
    settings.recent_model_ids.retain(|id| id != model_id);
    settings.recent_model_ids.insert(0, model_id.to_string());
    settings.recent_model_ids.truncate(MAX_RECENT_MODELS);
    save_settings(&settings)?;
    load_settings()
}

#[tauri::command]
pub fn set_auto_approve_cmd(enabled: bool) -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    settings.auto_approve_enabled = enabled;
    save_settings(&settings)?;
    load_settings()
}

/// Add or remove a remembered "allow always" tool pattern. Exact or glob
/// (`*`) matches against the tool call name/title when a permission arrives.
#[tauri::command]
pub fn set_allowed_tool_cmd(pattern: String, enabled: bool) -> Result<AppSettings, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("Tool pattern must not be empty".to_string());
    }
    let mut settings = load_settings()?;
    if enabled {
        if !settings.allowed_tool_patterns.iter().any(|p| p == pattern) {
            settings.allowed_tool_patterns.push(pattern.to_string());
        }
    } else {
        settings.allowed_tool_patterns.retain(|p| p != pattern);
    }
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

#[tauri::command]
pub fn remove_project_from_workspace_cmd(
    project_path: String,
) -> Result<AppSettings, String> {
    remove_project_from_workspace(&project_path)
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
