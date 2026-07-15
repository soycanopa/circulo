use std::path::{Path, PathBuf};
use std::str::FromStr;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionConfigKind, SessionConfigOption,
    SessionConfigSelectOptions, SessionNotification, SetSessionConfigOptionRequest,
    TextContent,
};
use agent_client_protocol::{AcpAgent, Agent, ConnectionTo};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::agents::DEFAULT_AGENT_COMMAND;
use crate::state::{
    AgentCommand, ConfigOptionDto, ConfigOptionValueDto, ContextFile, SharedState,
};

pub async fn start_agent_connection(
    app: AppHandle,
    state: SharedState,
    project_path: PathBuf,
    cmd_rx: mpsc::Receiver<AgentCommand>,
) -> Result<(), String> {
    let agent = AcpAgent::from_str(DEFAULT_AGENT_COMMAND)
        .map_err(|err| format!("Failed to parse agent command: {err}"))?;

    let app_for_notifications = app.clone();
    let state_for_permissions = state.clone();
    let app_for_permissions = app.clone();

    let mut cmd_rx = cmd_rx;

    agent_client_protocol::Client
        .builder()
        .name("forge")
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
                let app = app_for_permissions.clone();
                let state = state_for_permissions.clone();
                    let request_id = uuid::Uuid::new_v4().to_string();
                    let (tx, rx) = oneshot::channel::<String>();

                    {
                        let mut guard = state.lock().await;
                        guard.permission_waiters.insert(request_id.clone(), tx);
                    }

                    let payload = serde_json::json!({
                        "requestId": request_id,
                        "sessionId": request.session_id,
                        "toolCall": request.tool_call,
                        "options": request.options,
                    });

                    let _ = app.emit("acp:permission_request", payload);

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
                info!("Initializing OpenCode ACP agent");
                if let Err(err) = connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await
                {
                    let _ = app.emit(
                        "acp:error",
                        serde_json::json!({ "message": format!("ACP initialize failed: {err}") }),
                    );
                    return Ok(());
                }

                let session_response = match connection
                    .send_request(NewSessionRequest::new(project_path.clone()))
                    .block_task()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({ "message": format!("ACP session/new failed: {err}") }),
                        );
                        return Ok(());
                    }
                };

                let session_id = session_response.session_id.clone();
                let config_options =
                    map_config_options(session_response.config_options.as_deref());

                {
                    let mut guard = state.lock().await;
                    if let Some(project) = guard.project.as_mut() {
                        project.session_id = session_id.to_string();
                        project.config_options = config_options.clone();
                    }
                }

                let _ = app.emit(
                    "acp:session_ready",
                    serde_json::json!({
                        "sessionId": session_id.to_string(),
                        "projectPath": project_path.display().to_string(),
                        "configOptions": config_options,
                    }),
                );

                info!(session_id = %session_id, "ACP session ready");

                while let Some(command) = cmd_rx.recv().await {
                    match command {
                        AgentCommand::SendPrompt {
                            text,
                            context_files,
                        } => {
                            let blocks = build_prompt_blocks(&text, &context_files);
                            if let Err(err) = connection
                                .send_request(PromptRequest::new(session_id.clone(), blocks))
                                .block_task()
                                .await
                            {
                                error!(?err, "Prompt failed");
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({ "message": err.to_string() }),
                                );
                            } else {
                                let _ = app.emit("acp:prompt_complete", ());
                            }
                        }
                        AgentCommand::SetConfigOption { config_id, value } => {
                            let request = SetSessionConfigOptionRequest::new(
                                session_id.clone(),
                                config_id,
                                value.as_str(),
                            );
                            match connection.send_request(request).block_task().await {
                                Ok(response) => {
                                    let mapped = map_config_options(Some(&response.config_options));
                                    if let Some(project) = state.lock().await.project.as_mut() {
                                        project.config_options = mapped.clone();
                                    }
                                    let _ = app.emit(
                                        "acp:config_options",
                                        serde_json::json!({ "configOptions": mapped }),
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
                        AgentCommand::Shutdown => break,
                    }
                }

                Ok(())
            }
        })
        .await
        .map_err(|err| format!("ACP connection ended: {err}"))?;

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
                            })
                            .collect(),
                        SessionConfigSelectOptions::Grouped(groups) => groups
                            .iter()
                            .flat_map(|group| group.options.iter())
                            .map(|item| ConfigOptionValueDto {
                                value: item.value.to_string(),
                                name: item.name.clone(),
                                description: item.description.clone(),
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