use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use agent_client_protocol::schema::v1::{
    CancelNotification, ClientCapabilities, CloseSessionRequest, ContentBlock,
    CreateTerminalRequest, InitializeRequest, KillTerminalRequest,
    LoadSessionRequest, NewSessionRequest, PromptRequest,
    ReleaseTerminalRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    SessionNotification,
    SetSessionConfigOptionRequest, TerminalOutputRequest, TextContent,
    WaitForTerminalExitRequest,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, ConnectionTo, Error as AcpError};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info};

use crate::agents::build_agent;
use crate::state::{
    AgentCapabilitiesDto, AgentCommand, ContextFile,
    PermissionOptionId, SessionHandle, SharedState,
};
use crate::acp::config_bridge::{
    bridge_session_config_sync, map_config_options, merge_config_options,
    should_auto_refresh_config, refresh_session_config,
};
use crate::acp::terminal::TerminalManager;

async fn emit_if_active(
    app: &AppHandle,
    state: &SharedState,
    generation: u64,
    event: &str,
    payload: serde_json::Value,
) {
    if state.lock().await.is_current_generation(generation) {
        let _ = app.emit(event, payload);
    }
}

pub async fn start_agent_connection(
    app: AppHandle,
    state: SharedState,
    project_path: PathBuf,
    generation: u64,
    cmd_rx: mpsc::Receiver<AgentCommand>,
) -> Result<(), String> {
    let agent_id = {
        let guard = state.lock().await;
        guard
            .agent_for_generation(generation)
            .map(|a| a.agent_id.clone())
            .unwrap_or_else(|| crate::agents::DEFAULT_AGENT_ID.to_string())
    };

    let agent = build_agent(&agent_id, &project_path)?;
    let agent_label = crate::agents::agent_progress_label(&agent_id);

    let terminals = Arc::new(Mutex::new(TerminalManager::new(
        project_path.clone(),
        app.clone(),
        generation,
    )));

    let state_for_notifications = state.clone();
    let app_for_notifications = app.clone();
    let state_for_permissions = state.clone();
    let app_for_permissions = app.clone();
    // Time-to-first-token for the current prompt (set when prompt is sent).
    let prompt_started_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let first_chunk_logged: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let prompt_started_for_notify = prompt_started_at.clone();
    let first_chunk_for_notify = first_chunk_logged.clone();

    let terminals_for_create = terminals.clone();
    let terminals_for_output = terminals.clone();
    let terminals_for_wait = terminals.clone();
    let terminals_for_kill = terminals.clone();
    let terminals_for_release = terminals.clone();
    let state_for_terminals_create = state.clone();
    let state_for_terminals_output = state.clone();
    let state_for_terminals_wait = state.clone();
    let state_for_terminals_kill = state.clone();
    let state_for_terminals_release = state.clone();

    agent_client_protocol::Client
        .builder()
        .name("circulo")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                // Forward immediately — UI must stream agent_message_chunk before
                // session/prompt RPC resolves (ACP prompt-turn lifecycle).
                if !state_for_notifications
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    return Ok(());
                }

                let mut payload = serde_json::to_value(&notification).unwrap_or(Value::Null);
                if let Some(object) = payload.as_object_mut() {
                    object.insert(
                        "connectionGeneration".to_string(),
                        serde_json::json!(generation),
                    );
                }

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
                if !state_for_permissions
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                    return Ok(());
                }

                let request_id = uuid::Uuid::new_v4().to_string();
                let (tx, rx) = oneshot::channel::<String>();
                let allowed: Vec<PermissionOptionId> = request
                    .options
                    .iter()
                    .map(|o| o.option_id.clone())
                    .collect();
                let session_id = request.session_id.to_string();

                {
                    let mut guard = state_for_permissions.lock().await;
                    guard.permission_waiters.insert(
                        request_id.clone(),
                        crate::state::PermissionWaiter {
                            tx,
                            allowed_option_ids: allowed,
                            session_id,
                        },
                    );
                }

                let payload = serde_json::json!({
                    "requestId": request_id,
                    "sessionId": request.session_id,
                    "toolCall": request.tool_call,
                    "options": request.options,
                    "connectionGeneration": generation,
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
        .on_receive_request(
            async move |request: CreateTerminalRequest, responder, _connection| {
                if !state_for_terminals_create
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond_with_error(AcpError::internal_error());
                    return Ok(());
                }
                let response = terminals_for_create.lock().await.create(request).await;
                let _ = responder.respond_with_result(
                    response.map_err(|err| AcpError::invalid_params().data(err)),
                );
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: TerminalOutputRequest, responder, _connection| {
                if !state_for_terminals_output
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond_with_error(AcpError::internal_error());
                    return Ok(());
                }
                let response = terminals_for_output.lock().await.output(request).await;
                let _ = responder.respond_with_result(
                    response.map_err(|err| AcpError::invalid_params().data(err)),
                );
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: WaitForTerminalExitRequest, responder, _connection| {
                if !state_for_terminals_wait
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond_with_error(AcpError::internal_error());
                    return Ok(());
                }
                let response = terminals_for_wait.lock().await.wait_for_exit(request).await;
                let _ = responder.respond_with_result(
                    response.map_err(|err| AcpError::invalid_params().data(err)),
                );
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: KillTerminalRequest, responder, _connection| {
                if !state_for_terminals_kill
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond_with_error(AcpError::internal_error());
                    return Ok(());
                }
                let response = terminals_for_kill.lock().await.kill(request).await;
                let _ = responder.respond_with_result(
                    response.map_err(|err| AcpError::invalid_params().data(err)),
                );
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ReleaseTerminalRequest, responder, _connection| {
                if !state_for_terminals_release
                    .lock()
                    .await
                    .is_current_generation(generation)
                {
                    let _ = responder.respond_with_error(AcpError::internal_error());
                    return Ok(());
                }
                let response = terminals_for_release.lock().await.release(request).await;
                let _ = responder.respond_with_result(
                    response.map_err(|err| AcpError::invalid_params().data(err)),
                );
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, move |connection: ConnectionTo<Agent>| {
            let app = app.clone();
            let state = state.clone();
            let project_path = project_path.clone();
            let agent_label = agent_label.clone();

            async move {
                info!(path = %project_path.display(), generation, "Initializing ACP agent");
                if !state.lock().await.is_known_generation(generation) {
                    return Ok(());
                }
                emit_if_active(
                    &app,
                    &state,
                    generation,
                    "agent:progress",
                    serde_json::json!({
                        "phase": "initialize",
                        "message": format!("Connecting to {agent_label}…"),
                        "connectionGeneration": generation,
                    }),
                )
                .await;
                let init_started = Instant::now();
                let init_response = match connection
                    .send_request(
                        InitializeRequest::new(ProtocolVersion::V1)
                            .client_capabilities(ClientCapabilities::new().terminal(true)),
                    )
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
                        emit_if_active(
                            &app,
                            &state,
                            generation,
                            "acp:error",
                            serde_json::json!({
                                "message": format!("ACP initialize failed: {err}"),
                                "connectionGeneration": generation,
                            }),
                        )
                        .await;
                        // Unblock open_project waiters.
                        let mut guard = state.lock().await;
                        if let Some(agent) = guard.agent_for_generation_mut(generation) {
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
                    if let Some(agent) = guard.agent_for_generation_mut(generation) {
                        agent.agent_capabilities = agent_capabilities.clone();
                        agent.connected = true;
                        agent.project_path = project_path.clone();
                    }
                }

                emit_if_active(
                    &app,
                    &state,
                    generation,
                    "agent:progress",
                    serde_json::json!({
                        "phase": "initialize",
                        "message": "Agent connected",
                        "elapsedMs": init_started.elapsed().as_millis() as u64,
                        "connectionGeneration": generation,
                    }),
                )
                .await;

                emit_if_active(
                    &app,
                    &state,
                    generation,
                    "agent:ready",
                    serde_json::json!({
                        "projectPath": project_path.display().to_string(),
                        "capabilities": agent_capabilities,
                        "connectionGeneration": generation,
                    }),
                )
                .await;
                emit_if_active(
                    &app,
                    &state,
                    generation,
                    "agent:progress",
                    serde_json::json!({
                        "phase": "ready",
                        "message": format!("{agent_label} ready — preparing session…"),
                        "connectionGeneration": generation,
                    }),
                )
                .await;

                // Unblock open_project waiters after initialize only (ACP: agent ready, no UI session yet).
                {
                    let mut guard = state.lock().await;
                    if let Some(agent) = guard.agent_for_generation_mut(generation) {
                        agent.agent_done.notify_waiters();
                    }
                }

                // The command run-loop is spawned as a separate task (see below) so
                // the receiver is ready when the prewarm below sends its RPC.
                let session_ops = Arc::new(Mutex::new(()));

                // Spawn the command run-loop BEFORE the prewarm so the receiver is
                // ready when the prewarm sends its `oneshot`-backed CreateSession.
                // Spawning the run-loop inline used to race with the prewarm's
                // session/new RPC and trigger `oneshot canceled` on startup.
                let run_state = state.clone();
                let run_app = app.clone();
                let run_connection = connection.clone();
                let run_session_ops = session_ops.clone();
                let run_project_path = project_path.clone();
                let run_prompt_started_at = prompt_started_at.clone();
                let run_first_chunk_logged = first_chunk_logged.clone();
                let run_generation = generation;
                let run_terminals = terminals.clone();
                let run_loop = tokio::spawn(async move {
                    run_command_loop(
                        run_state,
                        run_app,
                        run_connection,
                        cmd_rx,
                        run_session_ops,
                        run_project_path,
                        run_prompt_started_at,
                        run_first_chunk_logged,
                        run_generation,
                        run_terminals,
                    )
                    .await;
                });

                // Background session/new prewarm (ACP session-setup). OpenCode pays ~6–10s on
                // the first session/new per process (context + configured MCP). We publish
                // to the UI immediately so mode/model selectors and compose-first send work.
                let prewarm_connection = connection.clone();
                let prewarm_app = app.clone();
                let prewarm_state = state.clone();
                let prewarm_project_path = project_path.clone();
                let prewarm_session_ops = session_ops.clone();
                let _prewarm = tokio::spawn(async move {
                    let _ops = prewarm_session_ops.lock().await;
                    if let Some(agent) =
                        prewarm_state.lock().await.agent_for_generation_mut(generation)
                    {
                        if !agent.session_id().is_empty() {
                            return;
                        }
                    }
                    emit_if_active(
                        &prewarm_app,
                        &prewarm_state,
                        generation,
                        "agent:progress",
                        serde_json::json!({
                            "phase": "session_prewarm",
                            "message": "Preparando sesión…",
                            "connectionGeneration": generation,
                        }),
                    )
                    .await;
                    let prewarm_started = Instant::now();
                    match create_session_on_connection(
                        &prewarm_connection,
                        &prewarm_app,
                        &prewarm_state,
                        &prewarm_project_path,
                        generation,
                        /* publish_to_ui */ false,
                        /* resume */ false,
                    )
                    .await
                    {
                        Ok(()) => {
                            info!("Session prewarm complete (hidden until first message)");
                            emit_if_active(
                                &prewarm_app,
                                &prewarm_state,
                                generation,
                                "agent:progress",
                                serde_json::json!({
                                    "phase": "session_prewarm",
                                    "message": "Sesión lista",
                                    "elapsedMs": prewarm_started.elapsed().as_millis() as u64,
                                    "connectionGeneration": generation,
                                }),
                            )
                            .await;
                            emit_if_active(
                                &prewarm_app,
                                &prewarm_state,
                                generation,
                                "agent:progress",
                                serde_json::json!({
                                    "phase": "ready",
                                    "message": "Ready",
                                    "connectionGeneration": generation,
                                }),
                            )
                            .await;
                        }
                        Err(err) => {
                            error!(%err, "Session prewarm failed (New Chat will create on demand)");
                        }
                    }
                });

                // Keep the connect handler alive until the command run-loop exits
                // (Shutdown). Prewarm is fire-and-forget — awaiting it in a select!
                // used to end this closure as soon as session/new finished, which
                // dropped the ACP connection and broke session/prompt.
                let _ = run_loop.await;

                Ok(())
            }
        })
        .await
        .map_err(|err| format!("ACP connection failed: {err}"))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_command_loop(
    state: SharedState,
    app: AppHandle,
    connection: ConnectionTo<Agent>,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    session_ops: Arc<Mutex<()>>,
    project_path: PathBuf,
    prompt_started_at: Arc<Mutex<Option<Instant>>>,
    first_chunk_logged: Arc<Mutex<bool>>,
    generation: u64,
    terminals: Arc<Mutex<TerminalManager>>,
) {
    while let Some(command) = cmd_rx.recv().await {
        match command {
            AgentCommand::SendPrompt {
                session_id,
                text,
                context_files,
            } => {
                // Prompts target a specific session_id; the runtime no longer
                // assumes a single visible session.
                let ready = {
                    let mut guard = state.lock().await;
                    guard
                        .agent_for_generation_mut(generation)
                        .and_then(|a| a.sessions.get(&session_id))
                        .is_some_and(|s| s.session_ready_for_ui)
                };
                if !ready {
                    let _ = app.emit(
                        "acp:error",
                        serde_json::json!({
                            "message": "No active ACP session — wait for the agent to finish starting"
                        }),
                    );
                    continue;
                }

                // Mark in-flight for the targeted session only.
                {
                    let mut guard = state.lock().await;
                    if let Some(agent) = guard.agent_for_generation_mut(generation) {
                        if let Some(handle) = agent.sessions.get_mut(&session_id) {
                            if handle.prompt_in_flight {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "Prompt already in flight"
                                    }),
                                );
                                continue;
                            }
                            handle.prompt_in_flight = true;
                            handle.user_prompt_sent = true;
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
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
                let _ = app.emit(
                    "acp:prompt_started",
                    serde_json::json!({
                        "sessionId": session_id,
                        "chars": text.len(),
                    }),
                );
                tokio::spawn({
                    let state = state.clone();
                    let app = app.clone();
                    let session_id = session_id.clone();
                    async move {
                        let result = prompt_connection
                            .send_request(PromptRequest::new(session_id.clone(), blocks))
                            .block_task()
                            .await;
                        {
                            let mut guard = state.lock().await;
                            if let Some(agent) = guard.agent_for_generation_mut(generation) {
                                if let Some(handle) = agent.sessions.get_mut(&session_id) {
                                    handle.prompt_in_flight = false;
                                }
                            }
                        }
                        match result {
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
                                        "connectionGeneration": generation,
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
                                        "connectionGeneration": generation,
                                    }),
                                );
                            }
                        }
                    }
                });
            }
            AgentCommand::SetConfigOption {
                session_id,
                config_id,
                value,
            } => {
                let _ops = session_ops.lock().await;
                let known = {
                    let mut guard = state.lock().await;
                    guard
                        .agent_for_generation_mut(generation)
                        .map(|a| a.sessions.contains_key(&session_id))
                        .unwrap_or(false)
                };
                if !known {
                    continue;
                }
                let request = SetSessionConfigOptionRequest::new(
                    session_id.clone(),
                    config_id,
                    value.as_str(),
                );
                match connection.send_request(request).block_task().await {
                    Ok(response) => {
                        let mapped = map_config_options(Some(&response.config_options));
                        if let Some(agent) =
                            state.lock().await.agent_for_generation_mut(generation)
                        {
                            if let Some(handle) = agent.sessions.get_mut(&session_id) {
                                handle.config_options = mapped.clone();
                            }
                        }
                        let _ = app.emit(
                            "acp:config_options",
                            serde_json::json!({
                                "configOptions": mapped,
                                "sessionId": session_id,
                                "connectionGeneration": generation,
                            }),
                        );
                    }
                    Err(err) => {
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({
                                "message": err.to_string(),
                                "connectionGeneration": generation,
                            }),
                        );
                    }
                }
            }
            AgentCommand::CreateSession { done } => {
                let _ops = session_ops.lock().await;
                let result =
                    publish_or_create_session(
                        &connection,
                        &app,
                        &state,
                        &project_path,
                        generation,
                        &terminals,
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
                    generation,
                    &session_id,
                    &terminals,
                )
                .await;
                let _ = done.send(result);
            }
            AgentCommand::CloseSession { session_id, done } => {
                let _ops = session_ops.lock().await;
                let result = close_session_on_connection(
                    &connection,
                    &state,
                    generation,
                    &session_id,
                    &terminals,
                )
                .await;
                let _ = done.send(result);
            }
            AgentCommand::CancelPrompt { session_id } => {
                if session_id.is_empty() {
                    continue;
                }
                match connection.send_notification(CancelNotification::new(session_id.clone())) {
                    Ok(()) => {
                        info!(session_id = %session_id, "Sent session/cancel");
                    }
                    Err(err) => {
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({
                                "message": err.to_string(),
                                "connectionGeneration": generation,
                            }),
                        );
                    }
                }
            }
            AgentCommand::SetVisibleSession { session_id, done } => {
                let result = set_visible_session_on_connection(
                    &app,
                    &state,
                    generation,
                    session_id,
                )
                .await;
                let _ = done.send(result);
            }
            AgentCommand::Shutdown { ack } => {
                let _ = ack.send(());
                break;
            }
        }
    }
}

/// Promote the first hidden prewarmed session to the visible UI session.
async fn publish_prewarmed_session(
    app: &AppHandle,
    state: &SharedState,
    generation: u64,
    project_path: &Path,
) -> Result<Option<String>, String> {
    let (session_id, config_options) = {
        let mut guard = state.lock().await;
        let agent = guard
            .agent_for_generation_mut(generation)
            .ok_or_else(|| "No agent process".to_string())?;
        let prewarmed = agent
            .sessions
            .iter()
            .find(|(_, handle)| !handle.session_ready_for_ui)
            .map(|(sid, _)| sid.clone());
        let Some(session_id) = prewarmed else {
            return Ok(None);
        };
        let config_options = agent
            .sessions
            .get(&session_id)
            .map(|h| h.config_options.clone())
            .unwrap_or_default();
        if let Some(handle) = agent.sessions.get_mut(&session_id) {
            handle.session_ready_for_ui = true;
        }
        agent.visible_session_id = Some(session_id.clone());
        (session_id, config_options)
    };

    info!(session_id = %session_id, "Publishing prewarmed ACP session to UI");
    let _ = app.emit(
        "acp:session_ready",
        serde_json::json!({
            "sessionId": session_id,
            "projectPath": project_path.display().to_string(),
            "configOptions": config_options,
            "resume": false,
            "connectionGeneration": generation,
        }),
    );
    Ok(Some(session_id))
}

/// New Chat: promote a prewarmed session (no extra RPC) or call session/new.
async fn publish_or_create_session(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    generation: u64,
    terminals: &Arc<Mutex<TerminalManager>>,
) -> Result<(), String> {
    let concurrent_sessions = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .map(|a| a.agent_capabilities.concurrent_sessions)
            .unwrap_or(true)
    };

    // If prewarm finished and UI has not claimed it, publish without another session/new.
    if publish_prewarmed_session(app, state, generation, project_path)
        .await?
        .is_some()
    {
        return Ok(());
    }

    // When the agent supports concurrent sessions, keep a used previous session
    // alive (its in-flight prompt continues in the background). Pristine sessions
    // (compose-first auto-publish with no user prompt yet) are closed so New Chat
    // does not stack empty chats. Serial agents always close the previous session.
    let previous_session_id = {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .filter(|a| a.session_ready_for_ui() && !a.session_id().is_empty())
            .map(|a| a.session_id().to_string())
            .unwrap_or_default()
    };
    let close_previous = if previous_session_id.is_empty() {
        false
    } else if !concurrent_sessions {
        true
    } else {
        let guard = state.lock().await;
        guard
            .agent
            .as_ref()
            .and_then(|agent| {
                agent
                    .visible_session_id
                    .as_ref()
                    .and_then(|sid| agent.sessions.get(sid))
            })
            .is_some_and(|handle| !handle.user_prompt_sent && !handle.prompt_in_flight)
    };
    if close_previous {
        info!(
            session_id = %previous_session_id,
            "Closing previous ACP session before New Chat"
        );
        close_session_on_connection(
            connection,
            state,
            generation,
            &previous_session_id,
            terminals,
        )
        .await?;
    } else if !previous_session_id.is_empty() {
        info!(
            session_id = %previous_session_id,
            "Keeping previous ACP session alive (concurrent_sessions enabled)"
        );
    }

    // Fresh session/new (subsequent New Chat, or prewarm missed/failed).
    // Close any stale hidden sessions so a late prewarm cannot orphan state.
    {
        let stale: Vec<String> = {
            let guard = state.lock().await;
            guard
                .agent
                .as_ref()
                .map(|agent| {
                    agent
                        .sessions
                        .iter()
                        .filter(|(_, handle)| !handle.session_ready_for_ui)
                        .map(|(sid, _)| sid.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        for sid in stale {
            let _ = close_session_on_connection(connection, state, generation, &sid, terminals).await;
        }
    }

    create_session_on_connection(
        connection,
        app,
        state,
        project_path,
        generation,
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
    generation: u64,
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
    let modes = response.modes.clone();
    let meta = response.meta.clone();
    let mapped = map_config_options(response.config_options.as_deref());
    let config_options =
        bridge_session_config_sync(mapped, modes.as_ref(), meta.as_ref());

    info!(
        session_id = %session_id,
        elapsed_ms = started.elapsed().as_millis() as u64,
        publish_to_ui,
        "ACP session/new completed"
    );

    if !publish_to_ui {
        let already_visible = {
            let guard = state.lock().await;
            guard
                .agent
                .as_ref()
                .map(|a| a.session_ready_for_ui() && !a.session_id().is_empty())
                .unwrap_or(false)
        };
        if already_visible {
            info!(
                session_id = %session_id,
                "Discarding prewarm session because UI already has a visible session"
            );
            let _ = connection
                .send_request(CloseSessionRequest::new(session_id.clone()))
                .block_task()
                .await;
            return Ok(());
        }
    }

    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.agent_for_generation_mut(generation) {
            upsert_session(&mut agent.sessions, &session_id, project_path);
            let handle = agent.sessions.get_mut(&session_id).expect("upserted");
            handle.config_options = config_options.clone();
            handle.session_ready_for_ui = publish_to_ui;
            if publish_to_ui {
                agent.visible_session_id = Some(session_id.clone());
            }
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
                "connectionGeneration": generation,
            }),
        );
    } else {
        emit_if_active(
            app,
            state,
            generation,
            "acp:config_options",
            serde_json::json!({
                "configOptions": config_options,
                "sessionId": session_id,
                "connectionGeneration": generation,
            }),
        )
        .await;
    }

    if should_auto_refresh_config(
        &{
            let guard = state.lock().await;
            guard
                .agent_for_generation(generation)
                .map(|agent| agent.agent_id.clone())
                .unwrap_or_else(|| crate::agents::DEFAULT_AGENT_ID.to_string())
        },
        &config_options,
        modes.as_ref(),
        meta.as_ref(),
    ) {
        spawn_config_refresh(
            connection.clone(),
            app.clone(),
            state.clone(),
            session_id,
            generation,
            modes,
            meta,
        );
    }

    Ok(())
}

fn spawn_config_refresh(
    connection: ConnectionTo<Agent>,
    app: AppHandle,
    state: SharedState,
    session_id: String,
    generation: u64,
    modes: Option<agent_client_protocol::schema::v1::SessionModeState>,
    meta: Option<serde_json::Map<String, Value>>,
) {
    tokio::spawn(async move {
        let started = Instant::now();
        let partial = {
            let guard = state.lock().await;
            guard
                .agent
                .as_ref()
                .filter(|agent| agent.generation == generation)
                .and_then(|agent| agent.sessions.get(&session_id))
                .map(|handle| handle.config_options.clone())
                .unwrap_or_default()
        };
        match refresh_session_config(
            &connection,
            &session_id,
            modes.as_ref(),
            meta.as_ref(),
        )
        .await
        {
            Ok(refreshed) => {
                let merged = merge_config_options(partial, refreshed);
                let elapsed_ms = started.elapsed().as_millis() as u64;
                {
                    let mut guard = state.lock().await;
                    if let Some(agent) = guard.agent_for_generation_mut(generation) {
                        if let Some(handle) = agent.sessions.get_mut(&session_id) {
                            handle.config_options = merged.clone();
                        }
                    }
                }
                let _ = app.emit(
                    "acp:config_options",
                    serde_json::json!({
                        "configOptions": merged,
                        "sessionId": session_id,
                        "connectionGeneration": generation,
                    }),
                );
                let _ = app.emit(
                    "agent:progress",
                    serde_json::json!({
                        "phase": "config_refresh",
                        "elapsedMs": elapsed_ms,
                        "connectionGeneration": generation,
                    }),
                );
            }
            Err(err) => {
                tracing::debug!(%err, session_id = %session_id, "Config refresh skipped");
            }
        }
    });
}

/// ACP `session/load` — resume an existing agent session by id.
async fn load_session_on_connection(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    generation: u64,
    session_id: &str,
    terminals: &Arc<Mutex<TerminalManager>>,
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
            .map(|a| a.session_id().to_string())
            .unwrap_or_default()
    };
    if current_session_id == session_id {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.agent_for_generation_mut(generation) {
            let config_options = agent.config_options();
            if let Some(handle) = agent.sessions.get_mut(session_id) {
                handle.session_ready_for_ui = true;
            }
            agent.visible_session_id = Some(session_id.to_string());
            let _ = app.emit(
                "acp:session_ready",
                serde_json::json!({
                    "sessionId": session_id,
                    "projectPath": project_path.display().to_string(),
                    "configOptions": config_options,
                    "resume": true,
                    "connectionGeneration": generation,
                }),
            );
        }
        return Ok(());
    }

    if !current_session_id.is_empty() {
        close_session_on_connection(
            connection,
            state,
            generation,
            &current_session_id,
            terminals,
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

    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.agent_for_generation_mut(generation) {
            upsert_session(&mut agent.sessions, session_id, project_path);
            let handle = agent.sessions.get_mut(session_id).expect("upserted");
            handle.config_options = config_options.clone();
            handle.session_ready_for_ui = true;
            agent.visible_session_id = Some(session_id.to_string());
        }
    }

    let _ = app.emit(
        "acp:session_ready",
        serde_json::json!({
            "sessionId": session_id,
            "projectPath": project_path.display().to_string(),
            "configOptions": config_options,
            "resume": true,
            "connectionGeneration": generation,
        }),
    );

    Ok(())
}

async fn close_session_on_connection(
    connection: &ConnectionTo<Agent>,
    state: &SharedState,
    generation: u64,
    session_id: &str,
    terminals: &Arc<Mutex<TerminalManager>>,
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

    terminals.lock().await.release_session(session_id).await;

    let mut guard = state.lock().await;
    if let Some(agent) = guard.agent_for_generation_mut(generation) {
        if agent.session_id() == session_id {
            if let Some(handle) = agent.sessions.get_mut(session_id) {
                handle.session_ready_for_ui = false;
                handle.config_options = Vec::new();
            }
            agent.visible_session_id = None;
        }
    }

    Ok(())
}

async fn set_visible_session_on_connection(
    app: &AppHandle,
    state: &SharedState,
    generation: u64,
    session_id: Option<String>,
) -> Result<(), String> {
    let (config_options, emit_session_id) = {
        let mut guard = state.lock().await;
        let agent = guard
            .agent_for_generation_mut(generation)
            .ok_or_else(|| "No agent process".to_string())?;

        match session_id {
            Some(ref sid) if !sid.is_empty() => {
                let handle = agent
                    .sessions
                    .get(sid)
                    .ok_or_else(|| format!("Unknown session: {sid}"))?;
                if !handle.session_ready_for_ui {
                    return Err(format!("Session {sid} is not ready for UI"));
                }
                let config_options = handle.config_options.clone();
                agent.visible_session_id = Some(sid.clone());
                (config_options, Some(sid.clone()))
            }
            Some(_) => return Err("Session id is required".to_string()),
            None => {
                agent.visible_session_id = None;
                (Vec::new(), None)
            }
        }
    };

    if let Some(sid) = emit_session_id {
        let _ = app.emit(
            "acp:visible_session_changed",
            serde_json::json!({
                "sessionId": sid,
                "configOptions": config_options,
                "connectionGeneration": generation,
            }),
        );
    } else {
        let _ = app.emit(
            "acp:visible_session_changed",
            serde_json::json!({
                "sessionId": null,
                "configOptions": [],
                "connectionGeneration": generation,
            }),
        );
    }

    Ok(())
}

fn upsert_session(
    sessions: &mut std::collections::HashMap<String, SessionHandle>,
    session_id: &str,
    project_path: &PathBuf,
) {
    sessions
        .entry(session_id.to_string())
        .and_modify(|handle| {
            handle.session_id = session_id.to_string();
        })
        .or_insert_with(|| SessionHandle {
            session_id: session_id.to_string(),
            session_ready_for_ui: false,
            prompt_in_flight: false,
            user_prompt_sent: false,
            config_options: Vec::new(),
        });
    let _ = project_path; // reserved for future per-session metadata
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
    const MAX_SEARCH_DEPTH: usize = 12;
    const IGNORED_DIRECTORIES: [&str; 4] = ["node_modules", ".git", "target", ".build"];

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(project_root)
        .follow_links(false)
        .max_depth(MAX_SEARCH_DEPTH)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !IGNORED_DIRECTORIES.contains(&entry.file_name().to_string_lossy().as_ref())
        })
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

        if query.is_empty() || relative.to_lowercase().contains(&query_lower) {
            results.push(relative.to_string());
        }
    }

    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::search_project_files;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn search_prunes_ignored_directory_segments() {
        let root = tempdir().unwrap();
        fs::create_dir_all(root.path().join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(root.path().join("node_modules/pkg/index.ts"), "ignored").unwrap();
        fs::write(root.path().join("src/index.ts"), "included").unwrap();

        let results = search_project_files(root.path(), "index", 40);

        assert_eq!(results, vec!["src/index.ts"]);
    }

    #[test]
    fn search_keeps_directories_that_only_contain_ignored_names() {
        let root = tempdir().unwrap();
        fs::create_dir_all(root.path().join("custom-node_modules-cache")).unwrap();
        fs::write(
            root.path().join("custom-node_modules-cache/index.ts"),
            "included",
        )
        .unwrap();

        let results = search_project_files(root.path(), "index", 40);

        assert_eq!(results, vec!["custom-node_modules-cache/index.ts"]);
    }

    #[test]
    fn search_does_not_walk_beyond_depth_limit() {
        let root = tempdir().unwrap();
        let mut deep = root.path().to_path_buf();
        for segment in 0..12 {
            deep.push(format!("level-{segment}"));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("too-deep.ts"), "ignored").unwrap();
        fs::write(root.path().join("visible.ts"), "included").unwrap();

        let results = search_project_files(root.path(), "", 40);

        assert_eq!(results, vec!["visible.ts"]);
    }
}
