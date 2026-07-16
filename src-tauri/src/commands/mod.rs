use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Emitter, Manager, State};
use tokio::sync::mpsc;

use crate::acp::{read_context_file, search_project_files, start_agent_connection};
use crate::opencode_config::{
    list_opencode_commands as read_opencode_commands,
    list_opencode_mcp_servers as read_opencode_mcp_servers,
    list_opencode_skills as read_opencode_skills,
    set_opencode_mcp_enabled as write_opencode_mcp_enabled, CommandEntryDto, McpServerEntryDto,
    SkillEntryDto,
};
use crate::session_store::{store_path_for, ProjectSessionStore};
use crate::state::{
    ActiveProject, AgentCapabilitiesDto, AgentCommand, ContextFile, CredentialResponseDto,
    ProjectStatus, SharedState,
};

#[tauri::command]
pub async fn get_project_status(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn open_project(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    path: String,
) -> Result<ProjectStatus, String> {
    let project_path = PathBuf::from(&path);
    if !project_path.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    {
        let mut guard = state.lock().await;
        if let Some(existing) = guard.project.take() {
            let _ = existing.cmd_tx.send(AgentCommand::Shutdown).await;
        }
        guard.permission_waiters.clear();
        guard.credential_waiters.clear();
    }

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let session_placeholder = "pending".to_string();

    {
        let mut guard = state.lock().await;
        guard.project = Some(ActiveProject {
            project_path: project_path.clone(),
            session_id: session_placeholder,
            cmd_tx: cmd_tx.clone(),
            config_options: Vec::new(),
            sessions: Vec::new(),
            agent_capabilities: AgentCapabilitiesDto {
                load_session: false,
                list_sessions: false,
                resume_session: false,
                close_session: false,
            },
            list_cursor: None,
        });
    }

    let shared: SharedState = Arc::clone(state.inner());
    let app_clone = app.clone();
    let path_clone = project_path.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(err) = start_agent_connection(app_clone.clone(), shared.clone(), path_clone, cmd_rx).await
        {
            let _ = app_clone.emit("acp:error", serde_json::json!({ "message": err }));
            let mut guard = shared.lock().await;
            guard.project = None;
            let _ = app_clone.emit("agent:disconnected", ());
        }
    });

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn close_project(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    let mut guard = state.lock().await;
    if let Some(project) = guard.project.take() {
        let _ = project.cmd_tx.send(AgentCommand::Shutdown).await;
    }
    guard.permission_waiters.clear();
    guard.credential_waiters.clear();
    Ok(guard.status())
}

#[tauri::command]
pub async fn send_prompt(
    state: State<'_, SharedState>,
    text: String,
    context_paths: Vec<String>,
) -> Result<(), String> {
    let (cmd_tx, project_path) = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        (project.cmd_tx.clone(), project.project_path.clone())
    };

    let mut context_files = Vec::new();
    for path in context_paths {
        let content = read_context_file(&project_path, &path)?;
        context_files.push(ContextFile { path, content });
    }

    cmd_tx
        .send(AgentCommand::SendPrompt {
            text,
            context_files,
        })
        .await
        .map_err(|err| format!("Failed to queue prompt: {err}"))
}

#[tauri::command]
pub async fn respond_permission(
    state: State<'_, SharedState>,
    request_id: String,
    option_id: String,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    if let Some(tx) = guard.permission_waiters.remove(&request_id) {
        tx.send(option_id)
            .map_err(|_| "Permission waiter dropped".to_string())?;
        Ok(())
    } else {
        Err("Permission request not found".to_string())
    }
}

#[tauri::command]
pub async fn respond_credential(
    state: State<'_, SharedState>,
    request_id: String,
    response: CredentialResponseDto,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    if let Some(tx) = guard.credential_waiters.remove(&request_id) {
        tx.send(response)
            .map_err(|_| "Credential waiter dropped".to_string())?;
        Ok(())
    } else {
        // UI-only phase: accept response even when no agent waiter is registered yet.
        Ok(())
    }
}

#[tauri::command]
pub async fn set_config_option(
    state: State<'_, SharedState>,
    config_id: String,
    value: String,
) -> Result<(), String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::SetConfigOption { config_id, value })
        .await
        .map_err(|err| format!("Failed to set config option: {err}"))
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::ListSessions)
        .await
        .map_err(|err| format!("Failed to list sessions: {err}"))?;

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn create_session(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::CreateSession)
        .await
        .map_err(|err| format!("Failed to create session: {err}"))?;

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn load_session(
    state: State<'_, SharedState>,
    id: String,
) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        if !project.supports_load_session() {
            return Err("Agent does not support session/load".to_string());
        }
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::LoadSession { id })
        .await
        .map_err(|err| format!("Failed to load session: {err}"))?;

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn close_session(
    state: State<'_, SharedState>,
    id: String,
) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .project
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        if !project.supports_close_session() {
            return Err("Agent does not support session/close".to_string());
        }
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::CloseSession { id })
        .await
        .map_err(|err| format!("Failed to close session: {err}"))?;

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn rename_session(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    id: String,
    title: String,
) -> Result<ProjectStatus, String> {
    let trimmed = title.trim().to_string();
    if trimmed.is_empty() {
        return Err("El nombre no puede estar vacío".to_string());
    }
    if trimmed.chars().count() > 120 {
        return Err("El nombre es demasiado largo".to_string());
    }

    let (project_path, updated_session) = {
        let mut guard = state.lock().await;
        let project = guard
            .project
            .as_mut()
            .ok_or_else(|| "No project open".to_string())?;
        let session = project
            .sessions
            .iter_mut()
            .find(|entry| entry.session_id == id)
            .ok_or_else(|| "Session not found".to_string())?;
        session.title = Some(trimmed.clone());
        (project.project_path.clone(), session.clone())
    };

    if let Ok(dir) = app.path().app_data_dir() {
        let store_path = store_path_for(&dir, &project_path);
        let mut store = ProjectSessionStore::load(&store_path);
        if !store.update_metadata(&updated_session) {
            store.register(&updated_session);
        }
        store.save(&store_path)?;
    }

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn search_files(
    state: State<'_, SharedState>,
    query: String,
) -> Result<Vec<String>, String> {
    let guard = state.lock().await;
    let project = guard
        .project
        .as_ref()
        .ok_or_else(|| "No project open".to_string())?;
    Ok(search_project_files(&project.project_path, &query, 50))
}

#[tauri::command]
pub async fn list_opencode_commands(
    state: State<'_, SharedState>,
    project_path: Option<String>,
) -> Result<Vec<CommandEntryDto>, String> {
    let fallback = state
        .lock()
        .await
        .project
        .as_ref()
        .map(|project| project.project_path.display().to_string());
    Ok(read_opencode_commands(project_path.or(fallback)))
}

#[tauri::command]
pub async fn list_opencode_skills(
    state: State<'_, SharedState>,
    project_path: Option<String>,
) -> Result<Vec<SkillEntryDto>, String> {
    let fallback = state
        .lock()
        .await
        .project
        .as_ref()
        .map(|project| project.project_path.display().to_string());
    Ok(read_opencode_skills(project_path.or(fallback)))
}

#[tauri::command]
pub async fn list_opencode_mcp_servers(
    state: State<'_, SharedState>,
    project_path: Option<String>,
) -> Result<Vec<McpServerEntryDto>, String> {
    let fallback = state
        .lock()
        .await
        .project
        .as_ref()
        .map(|project| project.project_path.display().to_string());
    Ok(read_opencode_mcp_servers(project_path.or(fallback)))
}

#[tauri::command]
pub async fn set_opencode_mcp_enabled(
    name: String,
    scope: String,
    enabled: bool,
    project_path: Option<String>,
    config_path: Option<String>,
) -> Result<(), String> {
    write_opencode_mcp_enabled(name, scope, enabled, project_path, config_path)
}