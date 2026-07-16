use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tauri::{Emitter, Manager, State};
use tokio::sync::mpsc;

use crate::acp::{read_context_file, search_project_files, start_agent_connection};
use crate::orchestrator::{
    activate_path, ensure_pool_capacity, insert_spawning_agent, pool_key, shutdown_active_agent,
};
use crate::agent_versions::list_agent_provider_versions as read_agent_provider_versions;
use crate::agent_versions::AgentVersionInfo;
use crate::opencode_config::{
    list_opencode_commands as read_opencode_commands,
    list_opencode_mcp_servers as read_opencode_mcp_servers,
    list_opencode_skills as read_opencode_skills,
    set_opencode_mcp_enabled as write_opencode_mcp_enabled, CommandEntryDto, McpServerEntryDto,
    SkillEntryDto,
};
use crate::skills_cli::{
    install_skills_package, search_skills_sh as fetch_skills_sh, SkillsShSearchResultDto,
};
use crate::session_store::{store_path_for, ProjectSessionStore};
use serde::Serialize;

use crate::state::{
    AgentCommand, ContextFile, CredentialResponseDto, ProjectStatus, SessionInfoDto, SharedState,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseSessionResult {
    pub status: ProjectStatus,
    pub project_path: String,
    pub sessions: Vec<SessionInfoDto>,
}

fn emit_sessions_updated(
    app: &tauri::AppHandle,
    sessions: &[SessionInfoDto],
    active_session_id: Option<&str>,
) {
    let _ = app.emit(
        "acp:sessions_updated",
        serde_json::json!({
            "sessions": sessions,
            "activeSessionId": active_session_id,
            "nextCursor": null,
        }),
    );
}

fn remove_session_from_store(app: &tauri::AppHandle, project_path: &PathBuf, id: &str) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?;
    let store_path = store_path_for(&dir, project_path);
    let mut store = ProjectSessionStore::load(&store_path);
    store.remove(id);
    store.save(&store_path)
}

fn stored_sessions_for(app: &tauri::AppHandle, project_path: &PathBuf) -> Vec<SessionInfoDto> {
    let dir = app.path().app_data_dir().ok();
    let store = dir
        .map(|dir| ProjectSessionStore::load(&store_path_for(&dir, project_path)))
        .unwrap_or_default();
    store.as_session_dtos(project_path)
}

#[tauri::command]
pub async fn get_project_status(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    Ok(state.lock().await.status())
}

fn emit_agent_ready_from_pool(app: &tauri::AppHandle, status: &ProjectStatus) {
    if !status.connected {
        return;
    }
    let _ = app.emit(
        "agent:ready",
        serde_json::json!({
            "projectPath": status.project_path,
            "capabilities": status.capabilities,
        }),
    );
}

#[tauri::command]
pub async fn open_project(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    path: String,
    agent_id: Option<String>,
    defer_session_bootstrap: Option<bool>,
) -> Result<ProjectStatus, String> {
    let project_path = PathBuf::from(&path);
    if !project_path.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let key = pool_key(&project_path);
    let resolved_agent_id =
        crate::agents::normalize_agent_id(agent_id.as_deref()).to_string();

    {
        let mut guard = state.lock().await;
        if guard.agents.contains_key(&key) {
            activate_path(&mut guard, key.clone());
            let status = guard.status();
            drop(guard);
            if status.connected {
                emit_agent_ready_from_pool(&app, &status);
                emit_sessions_updated(
                    &app,
                    &status.sessions,
                    status.active_session_id.as_deref(),
                );
            }
            return Ok(status);
        }
    }

    ensure_pool_capacity(state.inner(), &key).await;

    let (cmd_tx, cmd_rx) = mpsc::channel(32);

    {
        let mut guard = state.lock().await;
        insert_spawning_agent(
            &mut guard,
            project_path.clone(),
            resolved_agent_id,
            cmd_tx.clone(),
        );
    }

    let shared: SharedState = Arc::clone(state.inner());
    let app_clone = app.clone();
    let path_clone = project_path.clone();
    let defer_session_bootstrap = defer_session_bootstrap.unwrap_or(false);

    tauri::async_runtime::spawn(async move {
        let result = start_agent_connection(
            app_clone.clone(),
            shared.clone(),
            path_clone.clone(),
            cmd_rx,
            defer_session_bootstrap,
        )
        .await;

        if let Err(err) = result {
            let _ = app_clone.emit("acp:error", serde_json::json!({ "message": err }));
            let mut guard = shared.lock().await;
            crate::orchestrator::mark_agent_disconnected(&mut guard, &path_clone);
            let _ = app_clone.emit("agent:disconnected", ());
        }
    });

    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn close_project(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    shutdown_active_agent(state.inner()).await;
    Ok(state.lock().await.status())
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
            .active_agent()
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
            .active_agent()
            .ok_or_else(|| "No project open".to_string())?;
        project.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::SetConfigOption { config_id, value })
        .await
        .map_err(|err| format!("Failed to set config option: {err}"))
}

#[tauri::command]
pub async fn list_stored_sessions(
    app: tauri::AppHandle,
    project_path: String,
) -> Result<Vec<SessionInfoDto>, String> {
    let path = PathBuf::from(&project_path);
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?;
    let store = ProjectSessionStore::load(&store_path_for(&dir, &path));
    Ok(store.as_session_dtos(&path))
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .active_agent()
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
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();

    let cmd_tx = {
        let mut guard = state.lock().await;
        if guard.session_create_waiter.is_some() {
            return Err("Ya hay una sesión en creación".to_string());
        }
        let cmd_tx = guard
            .active_agent()
            .ok_or_else(|| "No project open".to_string())?
            .cmd_tx
            .clone();
        guard.session_create_waiter = Some(done_tx);
        cmd_tx
    };

    if let Err(err) = cmd_tx.send(AgentCommand::CreateSession).await {
        let mut guard = state.lock().await;
        guard.session_create_waiter = None;
        return Err(format!("Failed to create session: {err}"));
    }

    match tokio::time::timeout(Duration::from_secs(45), done_rx).await {
        Ok(Ok(Ok(()))) => Ok(state.lock().await.status()),
        Ok(Ok(Err(message))) => Err(message),
        Ok(Err(_)) => {
            state.lock().await.session_create_waiter = None;
            Err("La creación de sesión se canceló".to_string())
        }
        Err(_) => {
            state.lock().await.session_create_waiter = None;
            Err("Tiempo de espera agotado al crear la sesión".to_string())
        }
    }
}

#[tauri::command]
pub async fn load_session(
    state: State<'_, SharedState>,
    id: String,
) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let project = guard
            .active_agent()
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
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    id: String,
    project_path: Option<String>,
) -> Result<CloseSessionResult, String> {
    let target_path = match project_path {
        Some(path) => PathBuf::from(path),
        None => {
            let guard = state.lock().await;
            guard
                .active_agent()
                .map(|project| project.project_path.clone())
                .ok_or_else(|| "No project open".to_string())?
        }
    };

    remove_session_from_store(&app, &target_path, &id)?;

    let (cmd_tx, supports_close) = {
        let mut guard = state.lock().await;
        let Some(project) = guard.active_agent_mut() else {
            let sessions = stored_sessions_for(&app, &target_path);
            return Ok(CloseSessionResult {
                status: guard.status(),
                project_path: target_path.display().to_string(),
                sessions,
            });
        };

        if project.project_path != target_path {
            return Ok(CloseSessionResult {
                status: guard.status(),
                project_path: target_path.display().to_string(),
                sessions: stored_sessions_for(&app, &target_path),
            });
        }

        project.sessions.retain(|entry| entry.session_id != id);
        if project.session_id == id {
            project.session_id = project
                .sessions
                .first()
                .map(|entry| entry.session_id.clone())
                .unwrap_or_else(|| "pending".to_string());
        }

        let sessions = project.sessions.clone();
        let active_id = project.session_id.clone();
        let cmd_tx = project.cmd_tx.clone();
        let supports_close = project.supports_close_session();

        emit_sessions_updated(
            &app,
            &sessions,
            if active_id == "pending" {
                None
            } else {
                Some(active_id.as_str())
            },
        );

        (cmd_tx, supports_close)
    };

    if supports_close {
        let _ = cmd_tx.send(AgentCommand::CloseSession { id }).await;
    }

    let sessions = stored_sessions_for(&app, &target_path);
    let status = state.lock().await.status();

    Ok(CloseSessionResult {
        status,
        project_path: target_path.display().to_string(),
        sessions,
    })
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
            .active_agent_mut()
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
        .active_agent()
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
        .active_agent()
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
        .active_agent()
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
        .active_agent()
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

#[tauri::command]
pub async fn list_agent_provider_versions() -> Result<Vec<AgentVersionInfo>, String> {
    Ok(read_agent_provider_versions())
}

#[tauri::command]
pub async fn search_skills_sh(query: String) -> Result<Vec<SkillsShSearchResultDto>, String> {
    fetch_skills_sh(&query).await
}

#[tauri::command]
pub async fn install_skills_sh_skill(
    package: String,
    scope: String,
    project_path: Option<String>,
) -> Result<String, String> {
    let project = project_path.map(PathBuf::from);
    tokio::task::spawn_blocking(move || {
        install_skills_package(&package, &scope, project.as_deref())
    })
    .await
    .map_err(|err| err.to_string())?
}