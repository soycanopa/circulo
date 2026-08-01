use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, CloseSessionRequest, ContentBlock, InitializeRequest, LoadSessionRequest,
    NewSessionRequest, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionConfigKind, SessionConfigOption,
    SessionConfigSelectOptions, SessionNotification, SetSessionConfigOptionRequest, TextContent,
};
use agent_client_protocol::{Agent, ConnectionTo};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info};

use crate::agents::build_agent;
use crate::state::{
    AgentCapabilitiesDto, AgentCommand, ConfigOptionDto, ConfigOptionValueDto, ContextFile,
    SharedState,
};

pub async fn start_agent_connection(
    app: AppHandle,
    state: SharedState,
    project_path: PathBuf,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
) -> Result<(), String> {
    let agent_id = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.agent_id.clone())
            .unwrap_or_else(|| crate::agents::DEFAULT_AGENT_ID.to_string())
    };

    let agent = build_agent(&agent_id, &project_path)?;

    let app_for_notifications = app.clone();
    let state_for_permissions = state.clone();
    let app_for_permissions = app.clone();
    // Time-to-first-token for the current prompt (set when prompt is sent).
    let prompt_started_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let first_chunk_logged: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let prompt_started_for_notify = prompt_started_at.clone();
    let first_chunk_for_notify = first_chunk_logged.clone();

    agent_client_protocol::Client
        .builder()
        .name("circulo")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                // Forward immediately — UI must stream agent_message_chunk before
                // session/prompt RPC resolves (ACP prompt-turn lifecycle).
                let payload = serde_json::to_value(&notification).unwrap_or(Value::Null);

                // Log time-to-first agent_message_chunk (Palot-feel metric).
                if let Some(update) = payload.get("update") {
                    let kind = update
                        .get("sessionUpdate")
                        .or_else(|| update.get("session_update"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if kind == "agent_message_chunk" {
                        let mut logged = first_chunk_for_notify.lock().await;
                        if !*logged {
                            if let Some(started) = *prompt_started_for_notify.lock().await {
                                info!(
                                    elapsed_ms = started.elapsed().as_millis() as u64,
                                    "First agent_message_chunk (time-to-first-token)"
                                );
                            }
                            *logged = true;
                        }
                    }
                }

                let _ = app_for_notifications.emit("acp:session_update", payload);
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _connection| {
                let request_id = uuid::Uuid::new_v4().to_string();
                let (tx, rx) = oneshot::channel::<String>();

                {
                    let mut guard = state_for_permissions.lock().await;
                    guard.permission_waiters.insert(request_id.clone(), tx);
                }

                let payload = serde_json::json!({
                    "requestId": request_id,
                    "sessionId": request.session_id,
                    "toolCall": request.tool_call,
                    "options": request.options,
                });
                let _ = app_for_permissions.emit("acp:permission_request", payload);

                let outcome = match rx.await {
                    Ok(option_id) => {
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                            option_id,
                        ))
                    }
                    Err(_) => RequestPermissionOutcome::Cancelled,
                };

                let _ = responder.respond(RequestPermissionResponse::new(outcome));
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, move |connection: ConnectionTo<Agent>| {
            let app = app.clone();
            let state = state.clone();
            let project_path = project_path.clone();

            async move {
                info!(path = %project_path.display(), "Initializing ACP agent");
                let _ = app.emit(
                    "agent:progress",
                    serde_json::json!({
                        "phase": "initialize",
                        "message": "Connecting to OpenCode…",
                    }),
                );
                let init_started = Instant::now();
                let init_response = match connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await
                {
                    Ok(response) => {
                        info!(
                            elapsed_ms = init_started.elapsed().as_millis() as u64,
                            "ACP initialize RPC completed"
                        );
                        response
                    }
                    Err(err) => {
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({ "message": format!("ACP initialize failed: {err}") }),
                        );
                        // Unblock open_project waiters.
                        let mut guard = state.lock().await;
                        if let Some(agent) = guard.active_agent_mut() {
                            agent.connected = false;
                            agent.agent_done.notify_waiters();
                        }
                        return Ok(());
                    }
                };

                let agent_capabilities =
                    AgentCapabilitiesDto::from_capabilities(&init_response.agent_capabilities);

                info!(
                    path = %project_path.display(),
                    "ACP initialize completed"
                );

                {
                    let mut guard = state.lock().await;
                    if let Some(agent) = guard.active_agent_mut() {
                        agent.agent_capabilities = agent_capabilities.clone();
                        agent.connected = true;
                        agent.project_path = project_path.clone();
                    }
                }

                let _ = app.emit(
                    "agent:ready",
                    serde_json::json!({
                        "projectPath": project_path.display().to_string(),
                        "capabilities": agent_capabilities,
                    }),
                );
                let _ = app.emit(
                    "agent:progress",
                    serde_json::json!({
                        "phase": "ready",
                        "message": "Agent ready — preparing first session in background…",
                    }),
                );

                // Unblock open_project waiters after initialize only (ACP: agent ready, no UI session yet).
                {
                    let mut guard = state.lock().await;
                    if let Some(agent) = guard.active_agent_mut() {
                        agent.agent_done.notify_waiters();
                    }
                }

                let active_session_id = Arc::new(Mutex::new(String::new()));
                // Serializes session/new (prewarm + New Chat) on this agent process.
                let session_ops = Arc::new(Mutex::new(()));

                // Background session/new prewarm (ACP session-setup). OpenCode pays ~6–10s on
                // the first session/new per process (context + configured MCP). We do not emit
                // acp:session_ready until the user clicks New Chat — UI still shows "no session".
                {
                    let connection = connection.clone();
                    let app = app.clone();
                    let state = state.clone();
                    let project_path = project_path.clone();
                    let active_session_id = active_session_id.clone();
                    let session_ops = session_ops.clone();
                    tokio::spawn(async move {
                        let _ops = session_ops.lock().await;
                        // Skip if New Chat already created a session.
                        if !active_session_id.lock().await.is_empty() {
                            return;
                        }
                        match create_session_on_connection(
                            &connection,
                            &app,
                            &state,
                            &project_path,
                            &active_session_id,
                            /* publish_to_ui */ false,
                            /* resume */ false,
                        )
                        .await
                        {
                            Ok(()) => {
                                info!("Session prewarm complete (not published to UI yet)");
                                let _ = app.emit(
                                    "agent:progress",
                                    serde_json::json!({
                                        "phase": "ready",
                                        "message": "Agent ready — New Chat to start",
                                    }),
                                );
                            }
                            Err(err) => {
                                error!(%err, "Session prewarm failed (New Chat will create on demand)");
                            }
                        }
                    });
                }

                while let Some(command) = cmd_rx.recv().await {
                    match command {
                        AgentCommand::SendPrompt {
                            text,
                            context_files,
                        } => {
                            // Prompts require a UI-published session.
                            let (session_id, ready) = {
                                let guard = state.lock().await;
                                match &guard.agent {
                                    Some(a) => (a.session_id.clone(), a.session_ready_for_ui),
                                    None => (String::new(), false),
                                }
                            };
                            if !ready || session_id.is_empty() {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "No active ACP session — use New Chat first"
                                    }),
                                );
                                continue;
                            }

                            let blocks = build_prompt_blocks(&text, &context_files);
                            let prompt_connection = connection.clone();
                            let app = app.clone();
                            let prompt_started_at = prompt_started_at.clone();
                            let first_chunk_logged = first_chunk_logged.clone();
                            info!(
                                session_id = %session_id,
                                chars = text.len(),
                                "Sending session/prompt"
                            );
                            let started = Instant::now();
                            *prompt_started_at.lock().await = Some(started);
                            *first_chunk_logged.lock().await = false;
                            // Tell UI the prompt left the client immediately (before LLM tokens).
                            let _ = app.emit(
                                "acp:prompt_started",
                                serde_json::json!({
                                    "sessionId": session_id,
                                    "chars": text.len(),
                                }),
                            );
                            tokio::spawn(async move {
                                match prompt_connection
                                    .send_request(PromptRequest::new(session_id.clone(), blocks))
                                    .block_task()
                                    .await
                                {
                                    Ok(_) => {
                                        info!(
                                            session_id = %session_id,
                                            elapsed_ms = started.elapsed().as_millis() as u64,
                                            "session/prompt completed"
                                        );
                                        let _ = app.emit(
                                            "acp:prompt_complete",
                                            serde_json::json!({
                                                "sessionId": session_id,
                                                "elapsedMs": started.elapsed().as_millis() as u64,
                                            }),
                                        );
                                    }
                                    Err(err) => {
                                        error!(
                                            ?err,
                                            elapsed_ms = started.elapsed().as_millis() as u64,
                                            "Prompt failed"
                                        );
                                        let _ = app.emit(
                                            "acp:error",
                                            serde_json::json!({
                                                "message": err.to_string(),
                                                "sessionId": session_id,
                                            }),
                                        );
                                    }
                                }
                            });
                        }
                        AgentCommand::SetConfigOption { config_id, value } => {
                            let (session_id, ready) = {
                                let guard = state.lock().await;
                                match &guard.agent {
                                    Some(a) => (a.session_id.clone(), a.session_ready_for_ui),
                                    None => (String::new(), false),
                                }
                            };
                            if !ready || session_id.is_empty() {
                                continue;
                            }
                            let request = SetSessionConfigOptionRequest::new(
                                session_id.clone(),
                                config_id,
                                value.as_str(),
                            );
                            match connection.send_request(request).block_task().await {
                                Ok(response) => {
                                    let mapped =
                                        map_config_options(Some(&response.config_options));
                                    if let Some(agent) = state.lock().await.active_agent_mut() {
                                        agent.config_options = mapped.clone();
                                    }
                                    let _ = app.emit(
                                        "acp:config_options",
                                        serde_json::json!({
                                            "configOptions": mapped,
                                            "sessionId": session_id,
                                        }),
                                    );
                                }
                                Err(err) => {
                                    let _ = app.emit(
                                        "acp:error",
                                        serde_json::json!({ "message": err.to_string() }),
                                    );
                                }
                            }
                        }
                        AgentCommand::CreateSession { done } => {
                            let _ops = session_ops.lock().await;
                            let result = publish_or_create_session(
                                &connection,
                                &app,
                                &state,
                                &project_path,
                                &active_session_id,
                            )
                            .await;
                            let _ = done.send(result);
                        }
                        AgentCommand::LoadSession { session_id, done } => {
                            let _ops = session_ops.lock().await;
                            let result = load_session_on_connection(
                                &connection,
                                &app,
                                &state,
                                &project_path,
                                &active_session_id,
                                &session_id,
                            )
                            .await;
                            let _ = done.send(result);
                        }
                        AgentCommand::CloseSession { session_id, done } => {
                            let result = close_session_on_connection(
                                &connection,
                                &state,
                                &active_session_id,
                                &session_id,
                            )
                            .await;
                            let _ = done.send(result);
                        }
                        AgentCommand::CancelPrompt => {
                            let session_id = {
                                let guard = state.lock().await;
                                match &guard.agent {
                                    Some(a) if a.session_ready_for_ui => {
                                        a.session_id.clone()
                                    }
                                    _ => String::new(),
                                }
                            };
                            if session_id.is_empty() {
                                continue;
                            }
                            match connection
                                .send_notification(CancelNotification::new(session_id.clone()))
                            {
                                Ok(()) => {
                                    info!(session_id = %session_id, "Sent session/cancel");
                                }
                                Err(err) => {
                                    let _ = app.emit(
                                        "acp:error",
                                        serde_json::json!({ "message": err.to_string() }),
                                    );
                                }
                            }
                        }
                        AgentCommand::Shutdown => break,
                    }
                }

                Ok(())
            }
        })
        .await
        .map_err(|err| format!("ACP connection failed: {err}"))?;

    Ok(())
}

/// New Chat: promote a prewarmed session (no extra RPC) or call session/new.
async fn publish_or_create_session(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    active_session_id: &Arc<Mutex<String>>,
) -> Result<(), String> {
    // If prewarm finished and UI has not claimed it, publish without another session/new.
    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.active_agent_mut() {
            if !agent.session_id.is_empty() && !agent.session_ready_for_ui {
                agent.session_ready_for_ui = true;
                let session_id = agent.session_id.clone();
                let config_options = agent.config_options.clone();
                info!(session_id = %session_id, "Publishing prewarmed ACP session to UI");
                let _ = app.emit(
                    "acp:session_ready",
                    serde_json::json!({
                        "sessionId": session_id,
                        "projectPath": project_path.display().to_string(),
                        "configOptions": config_options,
                        "resume": false,
                    }),
                );
                return Ok(());
            }
        }
    }

    // Close any UI-bound session before creating a fresh one.
    let previous_session_id = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .filter(|a| a.session_ready_for_ui && !a.session_id.is_empty())
            .map(|a| a.session_id.clone())
            .unwrap_or_default()
    };
    if !previous_session_id.is_empty() {
        close_session_on_connection(
            connection,
            state,
            active_session_id,
            &previous_session_id,
        )
        .await?;
    }

    // Fresh session/new (subsequent New Chat, or prewarm missed/failed).
    create_session_on_connection(
        connection,
        app,
        state,
        project_path,
        active_session_id,
        /* publish_to_ui */ true,
        /* resume */ false,
    )
    .await
}

/// ACP `session/new` with absolute cwd (session-setup).
/// When `publish_to_ui` is false, the session is held for New Chat (prewarm).
async fn create_session_on_connection(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    active_session_id: &Arc<Mutex<String>>,
    publish_to_ui: bool,
    resume: bool,
) -> Result<(), String> {
    info!(
        path = %project_path.display(),
        publish_to_ui,
        "Creating ACP session (session/new)"
    );
    let started = Instant::now();

    let response = connection
        .send_request(NewSessionRequest::new(project_path.clone()))
        .block_task()
        .await
        .map_err(|err| format!("session/new failed: {err}"))?;

    let session_id = response.session_id.to_string();
    let config_options = map_config_options(response.config_options.as_deref());

    info!(
        session_id = %session_id,
        elapsed_ms = started.elapsed().as_millis() as u64,
        publish_to_ui,
        "ACP session/new completed"
    );

    *active_session_id.lock().await = session_id.clone();
    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.active_agent_mut() {
            agent.session_id = session_id.clone();
            agent.config_options = config_options.clone();
            agent.project_path = project_path.clone();
            agent.session_ready_for_ui = publish_to_ui;
        }
    }

    if publish_to_ui {
        let _ = app.emit(
            "acp:session_ready",
            serde_json::json!({
                "sessionId": session_id,
                "projectPath": project_path.display().to_string(),
                "configOptions": config_options,
                "resume": resume,
            }),
        );
    }

    Ok(())
}

/// ACP `session/load` — resume an existing agent session by id.
async fn load_session_on_connection(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    active_session_id: &Arc<Mutex<String>>,
    session_id: &str,
) -> Result<(), String> {
    let capabilities = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.agent_capabilities.clone())
            .ok_or_else(|| "No agent process".to_string())?
    };
    if !capabilities.load_session {
        return Err("Agent does not support session/load".to_string());
    }

    let current_session_id = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.session_id.clone())
            .unwrap_or_default()
    };
    if current_session_id == session_id {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.active_agent_mut() {
            agent.session_ready_for_ui = true;
            let config_options = agent.config_options.clone();
            let _ = app.emit(
                "acp:session_ready",
                serde_json::json!({
                    "sessionId": session_id,
                    "projectPath": project_path.display().to_string(),
                    "configOptions": config_options,
                    "resume": true,
                }),
            );
        }
        return Ok(());
    }

    if !current_session_id.is_empty() {
        close_session_on_connection(
            connection,
            state,
            active_session_id,
            &current_session_id,
        )
        .await?;
    }

    info!(
        session_id = %session_id,
        path = %project_path.display(),
        "Loading ACP session (session/load)"
    );
    let started = Instant::now();

    let response = connection
        .send_request(LoadSessionRequest::new(
            session_id.to_string(),
            project_path.clone(),
        ))
        .block_task()
        .await
        .map_err(|err| format!("session/load failed: {err}"))?;

    let config_options = map_config_options(response.config_options.as_deref());

    info!(
        session_id = %session_id,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "ACP session/load completed"
    );

    *active_session_id.lock().await = session_id.to_string();
    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.active_agent_mut() {
            agent.session_id = session_id.to_string();
            agent.config_options = config_options.clone();
            agent.project_path = project_path.clone();
            agent.session_ready_for_ui = true;
        }
    }

    let _ = app.emit(
        "acp:session_ready",
        serde_json::json!({
            "sessionId": session_id,
            "projectPath": project_path.display().to_string(),
            "configOptions": config_options,
            "resume": true,
        }),
    );

    Ok(())
}

async fn close_session_on_connection(
    connection: &ConnectionTo<Agent>,
    state: &SharedState,
    active_session_id: &Arc<Mutex<String>>,
    session_id: &str,
) -> Result<(), String> {
    if session_id.is_empty() {
        return Ok(());
    }

    let close_supported = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.agent_capabilities.close_session)
            .unwrap_or(false)
    };

    if close_supported {
        connection
            .send_request(CloseSessionRequest::new(session_id.to_string()))
            .block_task()
            .await
            .map_err(|err| format!("session/close failed: {err}"))?;
        info!(session_id = %session_id, "ACP session/close completed");
    }

    *active_session_id.lock().await = String::new();
    let mut guard = state.lock().await;
    if let Some(agent) = guard.active_agent_mut() {
        if agent.session_id == session_id {
            agent.session_id = String::new();
            agent.session_ready_for_ui = false;
            agent.config_options = Vec::new();
        }
    }

    Ok(())
}

fn build_prompt_blocks(text: &str, context_files: &[ContextFile]) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    for file in context_files {
        let context = format!(
            "Context file `{}`:\n\n```\n{}\n```",
            file.path, file.content
        );
        blocks.push(ContentBlock::Text(TextContent::new(context)));
    }

    blocks.push(ContentBlock::Text(TextContent::new(text.to_string())));
    blocks
}

fn map_config_options(options: Option<&[SessionConfigOption]>) -> Vec<ConfigOptionDto> {
    options
        .unwrap_or_default()
        .iter()
        .map(|option| {
            let (current_value, values) = match &option.kind {
                SessionConfigKind::Select(select) => {
                    let values = match &select.options {
                        SessionConfigSelectOptions::Ungrouped(items) => items
                            .iter()
                            .map(|item| ConfigOptionValueDto {
                                value: item.value.to_string(),
                                name: item.name.clone(),
                                description: item.description.clone(),
                                group: None,
                            })
                            .collect(),
                        SessionConfigSelectOptions::Grouped(groups) => groups
                            .iter()
                            .flat_map(|group| {
                                group.options.iter().map(|item| ConfigOptionValueDto {
                                    value: item.value.to_string(),
                                    name: item.name.clone(),
                                    description: item.description.clone(),
                                    group: Some(group.name.clone()),
                                })
                            })
                            .collect(),
                        _ => Vec::new(),
                    };

                    (select.current_value.to_string(), values)
                }
                SessionConfigKind::Boolean(boolean) => {
                    (boolean.current_value.to_string(), Vec::new())
                }
                _ => (String::new(), Vec::new()),
            };

            ConfigOptionDto {
                id: option.id.to_string(),
                name: option.name.clone(),
                category: option.category.as_ref().map(|c| format!("{c:?}")),
                current_value,
                options: values,
            }
        })
        .collect()
}

pub fn read_context_file(project_root: &Path, relative_path: &str) -> Result<String, String> {
    let candidate = project_root.join(relative_path);
    let canonical_root = project_root
        .canonicalize()
        .map_err(|err| format!("Invalid project path: {err}"))?;
    let canonical_file = candidate
        .canonicalize()
        .map_err(|err| format!("File not found: {err}"))?;

    if !canonical_file.starts_with(&canonical_root) {
        return Err("Path escapes project root".to_string());
    }

    std::fs::read_to_string(&canonical_file).map_err(|err| err.to_string())
}

pub fn search_project_files(project_root: &Path, query: &str, limit: usize) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if results.len() >= limit {
            break;
        }

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let relative = path
            .strip_prefix(project_root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or_default();

        if relative.is_empty() {
            continue;
        }

        if relative.contains("node_modules")
            || relative.contains(".git/")
            || relative.contains("target/")
            || relative.contains(".build/")
        {
            continue;
        }

        if query.is_empty() || relative.to_lowercase().contains(&query_lower) {
            results.push(relative.to_string());
        }
    }

    results.sort();
    results
}
