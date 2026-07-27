use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

use crate::acp::{read_context_file, search_project_files, start_agent_connection};
use crate::state::{
    ActiveAgent, AgentCapabilitiesDto, AgentCommand, ContextFile, ProjectStatus, SharedState,
};

#[tauri::command]
pub async fn get_project_status(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    Ok(state.lock().await.status())
}

/// Small dedicated workspace for general chats (never $HOME).
#[tauri::command]
pub async fn get_default_chats_path() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let path = PathBuf::from(home).join(".circulo").join("chats");
    std::fs::create_dir_all(&path)
        .map_err(|err| format!("Could not create chats dir: {err}"))?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn open_project(
    app: AppHandle,
    state: State<'_, SharedState>,
    path: String,
    agent_id: Option<String>,
) -> Result<ProjectStatus, String> {
    let project_path = PathBuf::from(&path);
    if !project_path.is_dir() {
        std::fs::create_dir_all(&project_path).map_err(|err| {
            format!("Not a directory and could not create {path}: {err}")
        })?;
    }

    let resolved_agent_id =
        crate::agents::normalize_agent_id(agent_id.as_deref()).to_string();

    // Reuse if already open on the same path.
    {
        let guard = state.lock().await;
        if let Some(agent) = &guard.agent {
            if agent.project_path == project_path && agent.connected {
                return Ok(guard.status());
            }
        }
    }

    // Shutdown previous agent if any.
    shutdown_agent(state.inner()).await;

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let agent_done = Arc::new(tokio::sync::Notify::new());

    {
        let mut guard = state.lock().await;
        guard.agent = Some(ActiveAgent {
            project_path: project_path.clone(),
            agent_id: resolved_agent_id,
            session_id: String::new(),
            cmd_tx: cmd_tx.clone(),
            config_options: Vec::new(),
            agent_capabilities: AgentCapabilitiesDto::empty(),
            agent_done: agent_done.clone(),
            connected: false,
        });
    }

    let shared: SharedState = Arc::clone(state.inner());
    let app_clone = app.clone();
    let path_clone = project_path.clone();

    tauri::async_runtime::spawn(async move {
        let result = start_agent_connection(app_clone.clone(), shared.clone(), path_clone, cmd_rx)
            .await;

        if let Err(err) = result {
            let _ = app_clone.emit("acp:error", serde_json::json!({ "message": err }));
            let mut guard = shared.lock().await;
            if let Some(agent) = &mut guard.agent {
                agent.connected = false;
            }
            let _ = app_clone.emit("agent:disconnected", ());
        }
    });

    // Wait until the first session exists (initialize + session/new), not merely connected.
    // Returning early after initialize allowed New Chat to queue a second session/new.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        {
            let guard = state.lock().await;
            if guard
                .agent
                .as_ref()
                .is_some_and(|a| a.connected && !a.session_id.is_empty())
            {
                break;
            }
            if guard.agent.is_none() {
                break;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::select! {
            _ = agent_done.notified() => {}
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    }

    let status = state.lock().await.status();
    if status.session_id.is_none() && status.connected {
        return Err(
            "El agente conectó pero la sesión tarda demasiado. Usa un proyecto más pequeño (no Desktop/Home)."
                .to_string(),
        );
    }
    if !status.connected {
        return Err(
            "No se pudo conectar al agente (OpenCode). ¿Está instalado? `opencode --version`"
                .to_string(),
        );
    }

    Ok(status)
}

#[tauri::command]
pub async fn close_project(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    shutdown_agent(state.inner()).await;
    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn create_session(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    let (done_tx, done_rx) = oneshot::channel();

    let cmd_tx = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        if !agent.connected {
            return Err("Agent not connected".to_string());
        }
        agent.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::CreateSession { done: done_tx })
        .await
        .map_err(|err| format!("Failed to create session: {err}"))?;

    match tokio::time::timeout(Duration::from_secs(45), done_rx).await {
        Ok(Ok(Ok(()))) => Ok(state.lock().await.status()),
        Ok(Ok(Err(message))) => Err(message),
        Ok(Err(_)) => Err("La creación de sesión se canceló".to_string()),
        Err(_) => Err("Tiempo de espera agotado al crear la sesión".to_string()),
    }
}

#[tauri::command]
pub async fn send_prompt(
    state: State<'_, SharedState>,
    text: String,
    context_paths: Vec<String>,
) -> Result<(), String> {
    let (cmd_tx, project_path) = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        if agent.session_id.is_empty() {
            return Err("No active session".to_string());
        }
        (agent.cmd_tx.clone(), agent.project_path.clone())
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
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        agent.cmd_tx.clone()
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
    let project_path = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.project_path.clone())
            .ok_or_else(|| "No project open".to_string())?
    };
    Ok(search_project_files(&project_path, &query, 40))
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let folder = app
        .dialog()
        .file()
        .set_title("Open project")
        .blocking_pick_folder();
    Ok(folder.map(|p| p.to_string()))
}

async fn shutdown_agent(state: &SharedState) {
    let cmd_tx = {
        let mut guard = state.lock().await;
        guard.permission_waiters.clear();
        guard.agent.as_ref().map(|a| a.cmd_tx.clone())
    };

    if let Some(tx) = cmd_tx {
        let _ = tx.send(AgentCommand::Shutdown).await;
    }

    let mut guard = state.lock().await;
    guard.agent = None;
}
