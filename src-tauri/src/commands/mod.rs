use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use serde::Serialize;

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
    default_chats_path()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpencodeStatus {
    pub available: bool,
    pub path: Option<String>,
    pub install_hint: String,
}

#[tauri::command]
pub fn check_opencode() -> OpencodeStatus {
    match crate::cli_resolve::resolve_opencode() {
        Ok(path) => OpencodeStatus {
            available: true,
            path: Some(path.display().to_string()),
            install_hint: String::new(),
        },
        Err(_) => OpencodeStatus {
            available: false,
            path: None,
            install_hint: "Install OpenCode from https://opencode.ai or set OPENCODE_BIN to the full binary path.".to_string(),
        },
    }
}

#[tauri::command]
pub async fn get_home_path() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME not set".to_string())
}

pub fn default_chats_path() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let path = PathBuf::from(home).join(".circulo").join("chats");
    std::fs::create_dir_all(&path)
        .map_err(|err| format!("Could not create chats dir: {err}"))?;
    Ok(path.display().to_string())
}

/// Start (or reuse) the agent subprocess. Does **not** call session/new.
/// Returns **immediately** after spawn is queued — does not wait for OpenCode
/// cold start (~15–20s). Listen for `agent:ready` for connected state.
/// Per ACP: initialize once, then session/new only when the user starts a chat.
#[tauri::command]
pub async fn open_project(
    app: AppHandle,
    state: State<'_, SharedState>,
    path: String,
    agent_id: Option<String>,
) -> Result<ProjectStatus, String> {
    open_project_inner(app, state.inner(), path, agent_id).await
}

pub async fn open_project_inner(
    app: AppHandle,
    state: &SharedState,
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

    crate::cli_resolve::resolve_opencode().map_err(|err| err)?;

    // --- Single-flight: never kill a warming agent for the same path ---
    {
        let guard = state.lock().await;
        if let Some(agent) = &guard.agent {
            if paths_equal(&agent.project_path, &project_path) {
                // Already spawning or ready — return now (no 20s wait).
                info!(
                    path = %project_path.display(),
                    connected = agent.connected,
                    "Reusing agent process (non-blocking)"
                );
                return Ok(guard.status());
            }
        }
    }

    // Different path (or no agent): shut down previous and start one process.
    shutdown_agent(state).await;

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let agent_done = Arc::new(tokio::sync::Notify::new());

    {
        let mut guard = state.lock().await;
        guard.agent = Some(ActiveAgent {
            project_path: project_path.clone(),
            agent_id: resolved_agent_id,
            session_id: String::new(),
            session_ready_for_ui: false,
            cmd_tx: cmd_tx.clone(),
            config_options: Vec::new(),
            agent_capabilities: AgentCapabilitiesDto::empty(),
            agent_done: agent_done.clone(),
            connected: false,
        });
    }

    let shared: SharedState = Arc::clone(state);
    let app_clone = app.clone();
    let path_clone = project_path.clone();

    info!(path = %project_path.display(), "Spawning single OpenCode ACP process (opencode acp)");
    tauri::async_runtime::spawn(async move {
        let result =
            start_agent_connection(app_clone.clone(), shared.clone(), path_clone, cmd_rx).await;

        if let Err(err) = result {
            let _ = app_clone.emit("acp:error", serde_json::json!({ "message": err }));
            let mut guard = shared.lock().await;
            if let Some(agent) = &mut guard.agent {
                agent.connected = false;
                agent.agent_done.notify_waiters();
            }
            let _ = app_clone.emit("agent:disconnected", ());
        }
    });

    // Non-blocking: UI must not sit on "Agent starting" for OpenCode cold start.
    // `agent:ready` / `create_session` wait when the user actually needs the agent.
    Ok(state.lock().await.status())
}

/// Wait until ACP initialize finished (connected), or error/timeout.
async fn wait_until_agent_connected(state: &SharedState) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let (connected, done, gone) = {
            let guard = state.lock().await;
            match &guard.agent {
                Some(a) => (a.connected, a.agent_done.clone(), false),
                None => (false, Arc::new(tokio::sync::Notify::new()), true),
            }
        };
        if connected {
            return Ok(());
        }
        if gone {
            return Err("No agent process".to_string());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "OpenCode still starting (cold start ~15–20s). Try again in a moment.".to_string(),
            );
        }
        tokio::select! {
            _ = done.notified() => {}
            _ = tokio::time::sleep(Duration::from_millis(40)) => {}
        }
    }
}

fn paths_equal(a: &std::path::Path, b: &std::path::Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Spawn default chats agent once at app start (overlaps with webview load).
/// Non-blocking: returns as soon as the process is queued.
pub fn spawn_eager_agent_warm(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if crate::cli_resolve::resolve_opencode().is_err() {
            tracing::warn!("OpenCode binary not found — skipping eager agent warm");
            return;
        }
        let Ok(path) = default_chats_path() else {
            return;
        };
        let state = app.state::<SharedState>();
        match open_project_inner(app.clone(), state.inner(), path, None).await {
            Ok(status) => info!(
                connected = status.connected,
                "Eager agent warm queued (non-blocking)"
            ),
            Err(err) => tracing::error!(%err, "Eager agent warm failed"),
        }
    });
}

#[tauri::command]
pub async fn close_project(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    shutdown_agent(state.inner()).await;
    Ok(state.lock().await.status())
}

/// ACP `session/new` with absolute cwd (session-setup). Only call when the user starts a chat.
/// Waits for in-flight warm if needed (so New Chat works while OpenCode is still booting).
#[tauri::command]
pub async fn create_session(state: State<'_, SharedState>) -> Result<ProjectStatus, String> {
    // Ensure a process exists; if not, caller should have opened a project first.
    {
        let guard = state.lock().await;
        if guard.agent.is_none() {
            return Err("No agent process — open a project or wait for warm".to_string());
        }
    }
    wait_until_agent_connected(state.inner()).await?;

    let (done_tx, done_rx) = oneshot::channel();

    let cmd_tx = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No agent process — wait for warm or open a project".to_string())?;
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
        if !agent.session_ready_for_ui || agent.session_id.is_empty() {
            return Err("No active session — usa New Chat primero".to_string());
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
pub async fn cancel_prompt(state: State<'_, SharedState>) -> Result<(), String> {
    let cmd_tx = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        if !agent.session_ready_for_ui || agent.session_id.is_empty() {
            return Err("No active session".to_string());
        }
        agent.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::CancelPrompt)
        .await
        .map_err(|err| format!("Failed to cancel prompt: {err}"))
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

    // Brief yield so the process can exit before we spawn another.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut guard = state.lock().await;
    guard.agent = None;
}
