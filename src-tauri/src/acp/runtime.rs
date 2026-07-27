use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    SessionConfigKind, SessionConfigOption, SessionConfigSelectOptions, SessionNotification,
    SetSessionConfigOptionRequest, TextContent,
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

    agent_client_protocol::Client
        .builder()
        .name("circulo")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                let payload = serde_json::to_value(&notification).unwrap_or(Value::Null);
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
                    if let Some(agent) = guard.agent_for_path_mut(&project_path) {
                        agent.agent_capabilities = agent_capabilities.clone();
                        agent.connected = true;
                    }
                }

                let _ = app.emit(
                    "agent:ready",
                    serde_json::json!({
                        "projectPath": project_path.display().to_string(),
                        "capabilities": agent_capabilities,
                    }),
                );

                // Create the first session immediately (no reserve / pre-warm).
                // Only notify open_project waiters after session is ready so the UI
                // does not enable New Chat mid-create (which caused a second session/new).
                let active_session_id = Arc::new(Mutex::new(String::new()));
                match create_session_on_connection(
                    &connection,
                    &app,
                    &state,
                    &project_path,
                    &active_session_id,
                )
                .await
                {
                    Ok(()) => {
                        let mut guard = state.lock().await;
                        if let Some(agent) = guard.agent_for_path_mut(&project_path) {
                            agent.agent_done.notify_waiters();
                        }
                    }
                    Err(err) => {
                        error!(?err, "Initial session/new failed");
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({ "message": err }),
                        );
                        let mut guard = state.lock().await;
                        if let Some(agent) = guard.agent_for_path_mut(&project_path) {
                            agent.agent_done.notify_waiters();
                        }
                    }
                }

                while let Some(command) = cmd_rx.recv().await {
                    match command {
                        AgentCommand::SendPrompt {
                            text,
                            context_files,
                        } => {
                            let session_id = active_session_id.lock().await.clone();
                            if session_id.is_empty() {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "No hay sesión ACP activa"
                                    }),
                                );
                                continue;
                            }

                            let blocks = build_prompt_blocks(&text, &context_files);
                            let prompt_connection = connection.clone();
                            let app = app.clone();
                            info!(
                                session_id = %session_id,
                                chars = text.len(),
                                "Sending session/prompt"
                            );
                            let started = Instant::now();
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
                            let session_id = active_session_id.lock().await.clone();
                            if session_id.is_empty() {
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
                                    if let Some(agent) =
                                        state.lock().await.agent_for_path_mut(&project_path)
                                    {
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
                            let result = create_session_on_connection(
                                &connection,
                                &app,
                                &state,
                                &project_path,
                                &active_session_id,
                            )
                            .await;
                            let _ = done.send(result);
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

async fn create_session_on_connection(
    connection: &ConnectionTo<Agent>,
    app: &AppHandle,
    state: &SharedState,
    project_path: &PathBuf,
    active_session_id: &Arc<Mutex<String>>,
) -> Result<(), String> {
    info!(path = %project_path.display(), "Creating ACP session (session/new)");
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
        "ACP session/new completed"
    );

    *active_session_id.lock().await = session_id.clone();
    {
        let mut guard = state.lock().await;
        if let Some(agent) = guard.agent_for_path_mut(project_path) {
            agent.session_id = session_id.clone();
            agent.config_options = config_options.clone();
        }
    }

    let _ = app.emit(
        "acp:session_ready",
        serde_json::json!({
            "sessionId": session_id,
            "projectPath": project_path.display().to_string(),
            "configOptions": config_options,
        }),
    );

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
