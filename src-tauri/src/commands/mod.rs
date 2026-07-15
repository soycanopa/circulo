use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Emitter, State};
use tokio::sync::mpsc;

use crate::acp::{read_context_file, search_project_files, start_agent_connection};
use crate::state::{
    ActiveProject, AgentCommand, ContextFile, ProjectStatus, SharedState,
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