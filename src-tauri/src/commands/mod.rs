use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const AGENT_WARM_TIMEOUT: Duration = Duration::from_secs(60);
const SESSION_OPERATION_TIMEOUT: Duration = Duration::from_secs(60);
const SESSION_CLOSE_TIMEOUT: Duration = Duration::from_secs(15);
const SHUTDOWN_ACK_TIMEOUT: Duration = Duration::from_secs(3);

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

pub mod persistence;

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

/// General chats cwd for the **active** workspace (legacy default space → ~/.circulo/chats).
pub fn default_chats_path() -> Result<String, String> {
    crate::persistence::active_workspace_chats_path()
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
        std::fs::create_dir_all(&project_path)
            .map_err(|err| format!("Not a directory and could not create {path}: {err}"))?;
    }

    let resolved_agent_id = crate::agents::normalize_agent_id(agent_id.as_deref()).to_string();

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
    shutdown_agent(&app, state).await;

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let agent_done = Arc::new(tokio::sync::Notify::new());

    let generation = {
        let mut guard = state.lock().await;
        guard.next_generation = guard.next_generation.saturating_add(1);
        let generation = guard.next_generation;
        guard.agent = Some(ActiveAgent {
            generation,
            project_path: project_path.clone(),
            agent_id: resolved_agent_id,
            agent_capabilities: AgentCapabilitiesDto::empty(),
            cmd_tx: cmd_tx.clone(),
            agent_done: agent_done.clone(),
            connected: false,
            sessions: HashMap::new(),
            visible_session_id: None,
        });
        generation
    };

    let shared: SharedState = Arc::clone(state);
    let app_clone = app.clone();
    let path_clone = project_path.clone();

    info!(path = %project_path.display(), "Spawning single OpenCode ACP process (opencode acp)");
    tauri::async_runtime::spawn(async move {
        let result = start_agent_connection(
            app_clone.clone(),
            shared.clone(),
            path_clone,
            generation,
            cmd_rx,
        )
        .await;

        if let Err(err) = result {
            let is_current = shared.lock().await.is_current_generation(generation);
            if !is_current {
                return;
            }
            let _ = app_clone.emit(
                "acp:error",
                serde_json::json!({
                    "message": err,
                    "connectionGeneration": generation,
                }),
            );
            let mut guard = shared.lock().await;
            if let Some(agent) = guard.agent_for_generation_mut(generation) {
                agent.connected = false;
                agent.agent_done.notify_waiters();
            }
            drop(guard);
            let _ = app_clone.emit(
                "agent:disconnected",
                serde_json::json!({ "connectionGeneration": generation }),
            );
        }
    });

    // Non-blocking: UI must not sit on "Agent starting" for OpenCode cold start.
    // `agent:ready` / `create_session` wait when the user actually needs the agent.
    let _ = crate::persistence::touch_recent_project(&project_path);

    Ok(state.lock().await.status())
}

/// Wait until ACP initialize finished (connected), or error/timeout.
async fn wait_until_agent_connected(state: &SharedState) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + AGENT_WARM_TIMEOUT;
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
pub async fn close_project(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<ProjectStatus, String> {
    shutdown_agent(&app, state.inner()).await;
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

    match tokio::time::timeout(SESSION_OPERATION_TIMEOUT, done_rx).await {
        Ok(Ok(Ok(()))) => Ok(state.lock().await.status()),
        Ok(Ok(Err(message))) => Err(message),
        Ok(Err(_)) => Err("Session creation was cancelled".to_string()),
        Err(_) => Err("Timed out while creating session".to_string()),
    }
}

/// Switch the visible session without closing the previous one. Background sessions
/// keep running their in-flight prompts. Pass `None` to clear the visible session.
#[tauri::command]
pub async fn set_visible_session(
    state: State<'_, SharedState>,
    session_id: Option<String>,
) -> Result<ProjectStatus, String> {
    let cmd_tx = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .ok_or_else(|| "No agent process".to_string())?
            .cmd_tx
            .clone()
    };
    cmd_tx
        .send(AgentCommand::SetVisibleSession { session_id })
        .await
        .map_err(|err| format!("Failed to swap visible session: {err}"))?;
    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn load_session(
    state: State<'_, SharedState>,
    session_id: String,
) -> Result<ProjectStatus, String> {
    if session_id.is_empty() {
        return Err("Session id is required".to_string());
    }

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
        .send(AgentCommand::LoadSession {
            session_id,
            done: done_tx,
        })
        .await
        .map_err(|err| format!("Failed to load session: {err}"))?;

    match tokio::time::timeout(SESSION_OPERATION_TIMEOUT, done_rx).await {
        Ok(Ok(Ok(()))) => Ok(state.lock().await.status()),
        Ok(Ok(Err(message))) => Err(message),
        Ok(Err(_)) => Err("Session load was cancelled".to_string()),
        Err(_) => Err("Timed out while loading session".to_string()),
    }
}

#[tauri::command]
pub async fn close_session_cmd(
    state: State<'_, SharedState>,
    session_id: String,
) -> Result<ProjectStatus, String> {
    if session_id.is_empty() {
        return Err("Session id is required".to_string());
    }

    let (done_tx, done_rx) = oneshot::channel();
    let cmd_tx = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        agent.cmd_tx.clone()
    };

    cmd_tx
        .send(AgentCommand::CloseSession {
            session_id,
            done: done_tx,
        })
        .await
        .map_err(|err| format!("Failed to close session: {err}"))?;

    match tokio::time::timeout(SESSION_CLOSE_TIMEOUT, done_rx).await {
        Ok(Ok(Ok(()))) => Ok(state.lock().await.status()),
        Ok(Ok(Err(message))) => Err(message),
        Ok(Err(_)) => Err("Session close was cancelled".to_string()),
        Err(_) => Err("Timed out while closing session".to_string()),
    }
}

#[tauri::command]
pub async fn send_prompt(
    state: State<'_, SharedState>,
    text: String,
    context_paths: Vec<String>,
) -> Result<(), String> {
    let (cmd_tx, project_path, session_id) = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        let visible = agent
            .visible_session_id
            .clone()
            .ok_or_else(|| "No active session — use New Chat first".to_string())?;
        let ready = agent
            .sessions
            .get(&visible)
            .is_some_and(|s| s.session_ready_for_ui);
        if !ready {
            return Err("No active session — use New Chat first".to_string());
        }
        if agent
            .sessions
            .get(&visible)
            .is_some_and(|s| s.prompt_in_flight)
        {
            return Err("Prompt already in flight".to_string());
        }
        (agent.cmd_tx.clone(), agent.project_path.clone(), visible)
    };

    let mut context_files = Vec::new();
    for path in context_paths {
        let content = read_context_file(&project_path, &path)?;
        context_files.push(ContextFile { path, content });
    }

    cmd_tx
        .send(AgentCommand::SendPrompt {
            session_id,
            text,
            context_files,
        })
        .await
        .map_err(|err| format!("Failed to queue prompt: {err}"))
}

#[tauri::command]
pub async fn cancel_prompt(state: State<'_, SharedState>) -> Result<(), String> {
    let (cmd_tx, session_id) = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        let visible = agent
            .visible_session_id
            .clone()
            .ok_or_else(|| "No active session".to_string())?;
        (agent.cmd_tx.clone(), visible)
    };

    cmd_tx
        .send(AgentCommand::CancelPrompt { session_id })
        .await
        .map_err(|err| format!("Failed to cancel prompt: {err}"))
}

#[tauri::command]
pub async fn respond_permission(
    state: State<'_, SharedState>,
    request_id: String,
    option_id: String,
    session_id: String,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    let waiter = guard
        .permission_waiters
        .remove(&request_id)
        .ok_or_else(|| "Permission request not found".to_string())?;
    validate_permission_response(waiter, &option_id, &request_id, &session_id)?;
    Ok(())
}

/// Pure validation helper exposed for unit testing without spinning up Tauri State.
/// Sends the validated option_id through the waiter; returns an error for unknown / mismatched options.
fn validate_permission_response(
    waiter: crate::state::PermissionWaiter,
    option_id: &str,
    request_id: &str,
    session_id: &str,
) -> Result<(), String> {
    if option_id.is_empty() {
        return Err("optionId must not be empty".to_string());
    }
    if waiter.session_id != session_id {
        return Err("sessionId does not match the permission request".to_string());
    }
    if !waiter
        .allowed_option_ids
        .iter()
        .any(|o| o.0.as_ref() == option_id)
    {
        return Err(format!(
            "Unknown optionId '{option_id}' for request '{request_id}'"
        ));
    }
    waiter
        .tx
        .send(option_id.to_string())
        .map_err(|_| "Permission waiter dropped".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_config_option(
    state: State<'_, SharedState>,
    config_id: String,
    value: String,
) -> Result<(), String> {
    let (cmd_tx, session_id) = {
        let guard = state.lock().await;
        let agent = guard
            .agent
            .as_ref()
            .ok_or_else(|| "No project open".to_string())?;
        let visible = agent
            .visible_session_id
            .clone()
            .ok_or_else(|| "No active session".to_string())?;
        (agent.cmd_tx.clone(), visible)
    };

    cmd_tx
        .send(AgentCommand::SetConfigOption {
            session_id,
            config_id,
            value,
        })
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

/// One directory match for path autocomplete in the Open Project palette.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryCompletion {
    /// Absolute path (no trailing slash, except filesystem root).
    pub path: String,
    /// Last path component (or `/` for root).
    pub name: String,
}

/// List directories that match a typed path prefix (e.g. `/Vol` → `/Volumes`).
/// Only directories; used for Open Project autocomplete (user-initiated).
#[tauri::command]
pub async fn complete_directory_path(partial: String) -> Result<Vec<DirectoryCompletion>, String> {
    tauri::async_runtime::spawn_blocking(move || complete_directory_path_sync(&partial))
        .await
        .map_err(|err| format!("Path completion task failed: {err}"))?
}

fn expand_user_path(partial: &str) -> Result<String, String> {
    let trimmed = partial.trim();
    if trimmed == "~" || trimmed.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        if trimmed == "~" {
            return Ok(home);
        }
        return Ok(format!("{home}{}", &trimmed[1..]));
    }
    Ok(trimmed.to_string())
}

fn complete_directory_path_sync(partial: &str) -> Result<Vec<DirectoryCompletion>, String> {
    let expanded = expand_user_path(partial)?;
    if expanded.is_empty() {
        return Ok(Vec::new());
    }

    // Absolute / ~/path: complete path segments.
    if expanded.starts_with('/') {
        return complete_absolute_path_prefix(&expanded);
    }

    // Free text: "Volumes", "Desktop", "circulo" — search common locations by name.
    search_directories_by_name(&expanded)
}

fn path_display(path: &std::path::Path) -> String {
    if path == std::path::Path::new("/") {
        "/".to_string()
    } else {
        path.to_string_lossy().trim_end_matches('/').to_string()
    }
}

fn complete_absolute_path_prefix(expanded: &str) -> Result<Vec<DirectoryCompletion>, String> {
    let ends_with_sep = expanded.ends_with('/');
    let (parent, prefix) = if ends_with_sep {
        (expanded.trim_end_matches('/').to_string(), String::new())
    } else {
        let path = std::path::Path::new(expanded);
        match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (parent.to_string_lossy().into_owned(), name)
            }
            // e.g. "/Vol" → parent "/", prefix "Vol"
            _ => {
                let name = expanded.trim_start_matches('/').to_string();
                ("/".to_string(), name)
            }
        }
    };

    let parent_path = if parent.is_empty() {
        PathBuf::from("/")
    } else {
        PathBuf::from(&parent)
    };

    if !parent_path.is_dir() {
        return Ok(Vec::new());
    }

    let prefix_lower = prefix.to_ascii_lowercase();
    let show_hidden = prefix.starts_with('.');

    let mut matches: Vec<DirectoryCompletion> = Vec::new();
    let read = match std::fs::read_dir(&parent_path) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    for entry in read.flatten() {
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if !prefix.is_empty() && !name.to_ascii_lowercase().starts_with(&prefix_lower) {
            continue;
        }

        matches.push(DirectoryCompletion {
            path: path_display(&entry.path()),
            name,
        });
    }

    matches.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    matches.truncate(40);
    Ok(matches)
}

fn name_match_rank(name: &str, query_lower: &str) -> Option<i32> {
    let name_lower = name.to_ascii_lowercase();
    if name_lower == query_lower {
        Some(0)
    } else if name_lower.starts_with(query_lower) {
        Some(1)
    } else if name_lower.contains(query_lower) {
        Some(2)
    } else {
        None
    }
}

fn should_skip_dir_name(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".git"
            | ".svn"
            | ".hg"
            | "Library"
            | "Applications"
            | "Caches"
            | "__pycache__"
            | ".Trash"
            | "System"
            | "private"
    )
}

/// Search folders by name without requiring a leading `/`.
/// Scans common roots + a shallow walk under $HOME.
fn search_directories_by_name(query: &str) -> Result<Vec<DirectoryCompletion>, String> {
    let q = query.trim();
    if q.is_empty() || q.len() > 120 {
        return Ok(Vec::new());
    }
    // Avoid scanning the whole disk for single characters like "a".
    if q.len() < 2 {
        return search_directories_by_name_shallow(q);
    }

    let query_lower = q.to_ascii_lowercase();
    let home = std::env::var("HOME").unwrap_or_default();
    let mut scored: Vec<(i32, DirectoryCompletion)> = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    let mut push_path = |path: PathBuf| {
        if !path.is_dir() {
            return;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_display(&path));
        let Some(rank) = name_match_rank(&name, &query_lower) else {
            return;
        };
        let path_str = path_display(&path);
        if !seen.insert(path_str.clone()) {
            return;
        }
        scored.push((
            rank,
            DirectoryCompletion {
                path: path_str,
                name,
            },
        ));
    };

    // High-signal roots: match the root itself and its immediate children.
    let mut scan_parents: Vec<PathBuf> = vec![
        PathBuf::from("/"),
        PathBuf::from("/Volumes"),
        PathBuf::from("/Users"),
    ];
    if !home.is_empty() {
        let home_path = PathBuf::from(&home);
        scan_parents.push(home_path.clone());
        for child in [
            "Desktop",
            "Documents",
            "Downloads",
            "Developer",
            "Projects",
            "repos",
            "code",
            "src",
            "dev",
            "work",
        ] {
            scan_parents.push(home_path.join(child));
        }
    }

    for parent in &scan_parents {
        // Match the parent folder name itself (e.g. query "Volumes" → /Volumes).
        push_path(parent.clone());

        let Ok(read) = std::fs::read_dir(parent) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || should_skip_dir_name(&name) {
                continue;
            }
            if name_match_rank(&name, &query_lower).is_some() {
                push_path(entry.path());
            }
        }
    }

    // Shallow walk under home for project folders (depth ≤ 3).
    if !home.is_empty() {
        let home_path = PathBuf::from(&home);
        let walker = walkdir::WalkDir::new(&home_path)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.depth() == 0 {
                    return true;
                }
                if name.starts_with('.') {
                    return false;
                }
                !should_skip_dir_name(&name)
            });

        for entry in walker.flatten() {
            if entry.depth() == 0 {
                continue;
            }
            if !entry.file_type().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name_match_rank(&name, &query_lower).is_some() {
                push_path(entry.path().to_path_buf());
            }
        }
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| {
                a.1.name
                    .to_ascii_lowercase()
                    .cmp(&b.1.name.to_ascii_lowercase())
            })
            .then_with(|| a.1.path.cmp(&b.1.path))
    });
    scored.truncate(40);
    Ok(scored.into_iter().map(|(_, item)| item).collect())
}

/// Single-character free-text search: only root-level / high-signal folders.
fn search_directories_by_name_shallow(query: &str) -> Result<Vec<DirectoryCompletion>, String> {
    let query_lower = query.to_ascii_lowercase();
    let home = std::env::var("HOME").unwrap_or_default();
    let mut scored: Vec<(i32, DirectoryCompletion)> = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    let parents = [
        PathBuf::from("/"),
        PathBuf::from("/Volumes"),
        if home.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(&home)
        },
    ];

    for parent in parents {
        if !parent.is_dir() {
            continue;
        }
        // Root special-case: "/" has no useful file_name for matching.
        if let Some(name) = parent.file_name() {
            let name = name.to_string_lossy().into_owned();
            if let Some(rank) = name_match_rank(&name, &query_lower) {
                let path_str = path_display(&parent);
                if seen.insert(path_str.clone()) {
                    scored.push((
                        rank,
                        DirectoryCompletion {
                            path: path_str,
                            name,
                        },
                    ));
                }
            }
        }

        let Ok(read) = std::fs::read_dir(&parent) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || should_skip_dir_name(&name) {
                continue;
            }
            let Some(rank) = name_match_rank(&name, &query_lower) else {
                continue;
            };
            let path_str = path_display(&entry.path());
            if !seen.insert(path_str.clone()) {
                continue;
            }
            scored.push((
                rank,
                DirectoryCompletion {
                    path: path_str,
                    name,
                },
            ));
        }
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| {
            a.1.name
                .to_ascii_lowercase()
                .cmp(&b.1.name.to_ascii_lowercase())
        })
    });
    scored.truncate(40);
    Ok(scored.into_iter().map(|(_, item)| item).collect())
}

#[tauri::command]
pub async fn export_transcript_cmd(
    app: AppHandle,
    filename: String,
    content: String,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .set_title("Export transcript")
        .set_file_name(&filename)
        .add_filter("Markdown", &["md"])
        .blocking_save_file();

    match path {
        Some(p) => {
            let file_path = p
                .into_path()
                .map_err(|err| format!("Invalid save path: {err}"))?;
            std::fs::write(&file_path, content.as_bytes())
                .map_err(|err| format!("Could not write file: {err}"))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

async fn shutdown_agent(app: &AppHandle, state: &SharedState) {
    let (cmd_tx, generation) = {
        let mut guard = state.lock().await;
        guard.permission_waiters.clear();
        let agent = guard.agent.as_ref();
        (agent.map(|a| a.cmd_tx.clone()), agent.map(|a| a.generation))
    };

    if let (Some(tx), Some(generation)) = (cmd_tx, generation) {
        let (ack_tx, ack_rx) = oneshot::channel::<()>();
        if tx.send(AgentCommand::Shutdown { ack: ack_tx }).await.is_ok() {
            // Deterministic handshake — bound the wait so a stuck agent cannot hang open_project.
            let _ = tokio::time::timeout(SHUTDOWN_ACK_TIMEOUT, ack_rx).await;
        }
        let _ = app.emit(
            "agent:disconnected",
            serde_json::json!({ "connectionGeneration": generation }),
        );
    }

    let mut guard = state.lock().await;
    guard.agent = None;
}

#[cfg(test)]
mod tests {
    use super::validate_permission_response;
    use crate::state::{PermissionOptionId, PermissionWaiter};
    use std::sync::Arc;
    use tokio::sync::oneshot;

    fn make_waiter(session_id: &str, options: &[&str]) -> (PermissionWaiter, oneshot::Receiver<String>) {
        let (tx, rx) = oneshot::channel();
        let waiter = PermissionWaiter {
            tx,
            allowed_option_ids: options
                .iter()
                .map(|o| PermissionOptionId::new(Arc::from(*o)))
                .collect(),
            session_id: session_id.to_string(),
        };
        (waiter, rx)
    }

    #[test]
    fn rejects_empty_option_id() {
        let (waiter, _rx) = make_waiter("s1", &["allow"]);
        let err = validate_permission_response(waiter, "", "req-1", "s1").unwrap_err();
        assert_eq!(err, "optionId must not be empty");
    }

    #[test]
    fn rejects_session_mismatch() {
        let (waiter, _rx) = make_waiter("s1", &["allow"]);
        let err =
            validate_permission_response(waiter, "allow", "req-1", "other").unwrap_err();
        assert_eq!(err, "sessionId does not match the permission request");
    }

    #[test]
    fn rejects_unknown_option() {
        let (waiter, _rx) = make_waiter("s1", &["allow"]);
        let err =
            validate_permission_response(waiter, "deny", "req-1", "s1").unwrap_err();
        assert!(err.contains("Unknown optionId 'deny' for request 'req-1'"));
    }

    #[test]
    fn accepts_known_option() {
        let (waiter, rx) = make_waiter("s1", &["allow", "deny"]);
        validate_permission_response(waiter, "allow", "req-1", "s1").unwrap();
        assert_eq!(rx.blocking_recv().unwrap(), "allow");
    }
}
