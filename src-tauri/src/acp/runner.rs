use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CloseSessionRequest, ContentBlock, InitializeRequest, ListSessionsRequest,
    LoadSessionRequest, NewSessionRequest, PromptRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, ResumeSessionRequest,
    SelectedPermissionOutcome, SessionConfigKind, SessionConfigOption,
    SessionConfigSelectOptions, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, TextContent,
};
use agent_client_protocol::{Agent, ConnectionTo};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info};

use crate::agents::build_opencode_agent;
use crate::session_store::{store_path_for, ProjectSessionStore};
use crate::state::{
    AgentCapabilitiesDto, AgentCommand, ConfigOptionDto, ConfigOptionValueDto, ContextFile,
    SessionInfoDto, SharedState,
};

fn load_project_store(app: &AppHandle, project_path: &Path) -> ProjectSessionStore {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| ProjectSessionStore::load(&store_path_for(&dir, project_path)))
        .unwrap_or_default()
}

fn save_project_store(app: &AppHandle, project_path: &Path, store: &ProjectSessionStore) {
    if let Ok(dir) = app.path().app_data_dir() {
        let path = store_path_for(&dir, project_path);
        let _ = store.save(&path);
    }
}

fn sessions_from_store(store: &ProjectSessionStore, project_path: &Path) -> Vec<SessionInfoDto> {
    let tracked_ids = store.ids();
    store
        .sessions
        .iter()
        .filter(|stored| tracked_ids.contains(&stored.session_id))
        .map(|stored| {
            session_info_from_parts(
                stored.session_id.clone(),
                project_path.to_path_buf(),
                stored.title.clone(),
                stored.updated_at.clone(),
            )
        })
        .collect()
}

fn resolve_circulo_sessions(
    store: &ProjectSessionStore,
    agent_sessions: &[SessionInfoDto],
    project_path: &Path,
) -> Vec<SessionInfoDto> {
    if store.is_empty() {
        return Vec::new();
    }

    let filtered = store.filter_agent_sessions(agent_sessions);
    if !filtered.is_empty() {
        return filtered;
    }

    sessions_from_store(store, project_path)
}

pub async fn start_agent_connection(
    app: AppHandle,
    state: SharedState,
    project_path: PathBuf,
    cmd_rx: mpsc::Receiver<AgentCommand>,
) -> Result<(), String> {
    let agent = build_opencode_agent(&project_path)?;

    let app_for_notifications = app.clone();
    let state_for_notifications = state.clone();
    let state_for_permissions = state.clone();
    let app_for_permissions = app.clone();

    let mut cmd_rx = cmd_rx;

    agent_client_protocol::Client
        .builder()
        .name("circulo")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                let payload = serde_json::to_value(&notification).unwrap_or(Value::Null);
                let _ = app_for_notifications.emit("acp:session_update", payload);

                if let SessionUpdate::SessionInfoUpdate(update) = &notification.update {
                    let (sessions, active_session_id) = {
                        let mut guard = state_for_notifications.lock().await;
                        if let Some(project) = guard.project.as_mut() {
                            let session_id = notification.session_id.to_string();
                            if let Some(session) = project
                                .sessions
                                .iter_mut()
                                .find(|entry| entry.session_id == session_id)
                            {
                                match update.title.as_opt_ref() {
                                    Some(Some(title)) => session.title = Some(title.clone()),
                                    Some(None) => session.title = None,
                                    None => {}
                                }
                                match update.updated_at.as_opt_ref() {
                                    Some(Some(updated_at)) => {
                                        session.updated_at = Some(updated_at.clone())
                                    }
                                    Some(None) => session.updated_at = None,
                                    None => {}
                                }
                            }
                            (project.sessions.clone(), project.session_id.clone())
                        } else {
                            return Ok(());
                        }
                    };

                    emit_sessions_updated(
                        &app_for_notifications,
                        &sessions,
                        Some(&active_session_id),
                        None,
                    );

                    let project_path = {
                        let guard = state_for_notifications.lock().await;
                        guard
                            .project
                            .as_ref()
                            .map(|project| project.project_path.clone())
                    };
                    if let Some(project_path) = project_path {
                        let session_id = notification.session_id.to_string();
                        let mut store = load_project_store(&app_for_notifications, &project_path);
                        if let Some(session) = sessions
                            .iter()
                            .find(|entry| entry.session_id == session_id)
                        {
                            if store.update_metadata(session) {
                                save_project_store(&app_for_notifications, &project_path, &store);
                            }
                        }
                    }
                }

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
                let init_response = match connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await
                {
                    Ok(response) => response,
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

                {
                    let mut guard = state.lock().await;
                    if let Some(project) = guard.project.as_mut() {
                        project.agent_capabilities = agent_capabilities.clone();
                    }
                }

                let mut session_store = load_project_store(&app, &project_path);

                let agent_sessions = if agent_capabilities.list_sessions {
                    fetch_agent_sessions(&connection, &project_path)
                        .await
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                if !agent_sessions.is_empty() {
                    session_store.merge_agent_metadata(&agent_sessions);
                    save_project_store(&app, &project_path, &session_store);
                }

                let app_sessions =
                    resolve_circulo_sessions(&session_store, &agent_sessions, &project_path);

                {
                    let mut guard = state.lock().await;
                    if let Some(project) = guard.project.as_mut() {
                        project.sessions = app_sessions.clone();
                        project.list_cursor = None;
                    }
                }

                let target_session_id =
                    session_store.preferred_active_id(&app_sessions);

                let bootstrap = async {
                    if let Some(id) = target_session_id.clone().filter(|_| !app_sessions.is_empty())
                    {
                        let session_entry = app_sessions
                            .iter()
                            .find(|entry| entry.session_id == id)
                            .cloned()
                            .unwrap_or_else(|| {
                                session_info_from_parts(
                                    id.clone(),
                                    project_path.clone(),
                                    None,
                                    None,
                                )
                            });

                        if agent_capabilities.load_session {
                            let request =
                                LoadSessionRequest::new(id.clone(), project_path.clone());
                            if let Ok(response) =
                                connection.send_request(request).block_task().await
                            {
                                let config_options =
                                    map_config_options(response.config_options.as_deref());
                                return Ok::<_, String>((id, config_options, session_entry));
                            }
                        } else {
                            let config_options = {
                                let guard = state.lock().await;
                                guard
                                    .project
                                    .as_ref()
                                    .map(|project| project.config_options.clone())
                                    .unwrap_or_default()
                            };
                            return Ok::<_, String>((id, config_options, session_entry));
                        }
                    }

                    let session_response = connection
                        .send_request(NewSessionRequest::new(project_path.clone()))
                        .block_task()
                        .await
                        .map_err(|err| format!("ACP session/new failed: {err}"))?;
                    let session_id = session_response.session_id.to_string();
                    let config_options =
                        map_config_options(session_response.config_options.as_deref());
                    let session_entry = session_info_from_parts(
                        session_id.clone(),
                        project_path.clone(),
                        None,
                        None,
                    );
                    Ok::<_, String>((session_id, config_options, session_entry))
                };

                let (session_id, config_options, session_entry) = match bootstrap.await {
                    Ok(result) => result,
                    Err(err) => {
                        let _ = app.emit(
                            "acp:error",
                            serde_json::json!({ "message": err }),
                        );
                        return Ok(());
                    }
                };

                session_store.register(&session_entry);
                session_store.set_active(&session_id);
                save_project_store(&app, &project_path, &session_store);

                {
                    let mut guard = state.lock().await;
                    if let Some(project) = guard.project.as_mut() {
                        project.session_id = session_id.clone();
                        project.config_options = config_options.clone();
                        if !project
                            .sessions
                            .iter()
                            .any(|entry| entry.session_id == session_id)
                        {
                            project.sessions.push(session_entry);
                        }
                    }
                }

                let active_session_id = Arc::new(Mutex::new(session_id.clone()));

                emit_session_ready(
                    &app,
                    session_id.to_string(),
                    &project_path,
                    &config_options,
                );
                info!(session_id = %session_id, "ACP session ready");

                {
                    let sessions = state
                        .lock()
                        .await
                        .project
                        .as_ref()
                        .map(|project| project.sessions.clone())
                        .unwrap_or_default();
                    emit_sessions_updated(
                        &app,
                        &sessions,
                        Some(&session_id.to_string()),
                        None,
                    );
                }

                while let Some(command) = cmd_rx.recv().await {
                    match command {
                        AgentCommand::SendPrompt {
                            text,
                            context_files,
                        } => {
                            let active_id = active_session_id.lock().await.clone();
                            let blocks = build_prompt_blocks(&text, &context_files);
                            if let Err(err) = connection
                                .send_request(PromptRequest::new(active_id, blocks))
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
                            let active_id = active_session_id.lock().await.clone();
                            let request = SetSessionConfigOptionRequest::new(
                                active_id,
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
                        AgentCommand::ListSessions => {
                            let (sessions, active_session_id) = {
                                let guard = state.lock().await;
                                match guard.project.as_ref() {
                                    Some(project) => (
                                        project.sessions.clone(),
                                        project.session_id.clone(),
                                    ),
                                    None => (Vec::new(), String::new()),
                                }
                            };
                            emit_sessions_updated(
                                &app,
                                &sessions,
                                Some(&active_session_id),
                                None,
                            );
                        }
                        AgentCommand::CreateSession => {
                            match connection
                                .send_request(NewSessionRequest::new(project_path.clone()))
                                .block_task()
                                .await
                            {
                                Ok(response) => {
                                    let new_session_id = response.session_id.to_string();
                                    let config_options = map_config_options(
                                        response.config_options.as_deref(),
                                    );
                                    let session_entry = session_info_from_parts(
                                        new_session_id.clone(),
                                        project_path.clone(),
                                        None,
                                        None,
                                    );

                                    {
                                        let mut guard = state.lock().await;
                                        if let Some(project) = guard.project.as_mut() {
                                            if !project
                                                .sessions
                                                .iter()
                                                .any(|entry| entry.session_id == new_session_id)
                                            {
                                                project.sessions.push(session_entry);
                                            }
                                            project.session_id = new_session_id.clone();
                                            project.config_options = config_options.clone();
                                        }
                                    }

                                    *active_session_id.lock().await = new_session_id.clone();
                                    emit_session_ready(
                                        &app,
                                        new_session_id.clone(),
                                        &project_path,
                                        &config_options,
                                    );

                                    let sessions = state
                                        .lock()
                                        .await
                                        .project
                                        .as_ref()
                                        .map(|project| project.sessions.clone())
                                        .unwrap_or_default();
                                    emit_sessions_updated(
                                        &app,
                                        &sessions,
                                        Some(&new_session_id),
                                        None,
                                    );

                                    let mut store = load_project_store(&app, &project_path);
                                    if let Some(session) = sessions
                                        .iter()
                                        .find(|entry| entry.session_id == new_session_id)
                                    {
                                        store.register(session);
                                    }
                                    store.set_active(&new_session_id);
                                    save_project_store(&app, &project_path, &store);
                                }
                                Err(err) => {
                                    let _ = app.emit(
                                        "acp:error",
                                        serde_json::json!({ "message": err.to_string() }),
                                    );
                                }
                            }
                        }
                        AgentCommand::LoadSession { id } => {
                            let (supports_load, is_circulo_session) = {
                                let guard = state.lock().await;
                                let project = match guard.project.as_ref() {
                                    Some(p) => p,
                                    None => continue,
                                };
                                (
                                    project.supports_load_session(),
                                    project.sessions.iter().any(|s| s.session_id == id),
                                )
                            };

                            if !supports_load {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "Agent does not support session/load"
                                    }),
                                );
                                continue;
                            }

                            if !is_circulo_session {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "Session was not started from Circulo"
                                    }),
                                );
                                continue;
                            }

                            let request =
                                LoadSessionRequest::new(id.clone(), project_path.clone());
                            match connection.send_request(request).block_task().await {
                                Ok(response) => {
                                    let config_options = map_config_options(
                                        response.config_options.as_deref(),
                                    );
                                    switch_active_session(
                                        &app,
                                        &state,
                                        &active_session_id,
                                        id,
                                        config_options,
                                        &project_path,
                                    )
                                    .await;
                                }
                                Err(err) => {
                                    let _ = app.emit(
                                        "acp:error",
                                        serde_json::json!({ "message": err.to_string() }),
                                    );
                                }
                            }
                        }
                        AgentCommand::ResumeSession { id } => {
                            if !state
                                .lock()
                                .await
                                .project
                                .as_ref()
                                .is_some_and(|project| project.supports_resume_session())
                            {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "Agent does not support session/resume"
                                    }),
                                );
                                continue;
                            }

                            let request =
                                ResumeSessionRequest::new(id.clone(), project_path.clone());
                            match connection.send_request(request).block_task().await {
                                Ok(response) => {
                                    let config_options = map_config_options(
                                        response.config_options.as_deref(),
                                    );
                                    switch_active_session(
                                        &app,
                                        &state,
                                        &active_session_id,
                                        id,
                                        config_options,
                                        &project_path,
                                    )
                                    .await;
                                }
                                Err(err) => {
                                    let _ = app.emit(
                                        "acp:error",
                                        serde_json::json!({ "message": err.to_string() }),
                                    );
                                }
                            }
                        }
                        AgentCommand::CloseSession { id } => {
                            if !state
                                .lock()
                                .await
                                .project
                                .as_ref()
                                .is_some_and(|project| project.supports_close_session())
                            {
                                let _ = app.emit(
                                    "acp:error",
                                    serde_json::json!({
                                        "message": "Agent does not support session/close"
                                    }),
                                );
                                continue;
                            }

                            let request = CloseSessionRequest::new(id.clone());
                            match connection.send_request(request).block_task().await {
                                Ok(_) => {
                                    let (sessions, active_session_id_value) = {
                                        let mut guard = state.lock().await;
                                        let Some(project) = guard.project.as_mut() else {
                                            continue;
                                        };

                                        project
                                            .sessions
                                            .retain(|entry| entry.session_id != id);

                                        project.session_id = project
                                            .sessions
                                            .first()
                                            .map(|entry| entry.session_id.clone())
                                            .unwrap_or_default();

                                        (
                                            project.sessions.clone(),
                                            project.session_id.clone(),
                                        )
                                    };

                                    *active_session_id.lock().await =
                                        active_session_id_value.clone();
                                    emit_sessions_updated(
                                        &app,
                                        &sessions,
                                        Some(&active_session_id_value),
                                        None,
                                    );

                                    let mut store = load_project_store(&app, &project_path);
                                    store.remove(&id);
                                    save_project_store(&app, &project_path, &store);
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

async fn fetch_agent_sessions(
    connection: &ConnectionTo<Agent>,
    project_path: &PathBuf,
) -> Result<Vec<SessionInfoDto>, String> {
    let mut all_sessions = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut request = ListSessionsRequest::new().cwd(project_path.clone());
        if let Some(next) = cursor.clone() {
            request = request.cursor(next);
        }

        let response = connection
            .send_request(request)
            .block_task()
            .await
            .map_err(|err| err.to_string())?;

        all_sessions.extend(
            response
                .sessions
                .iter()
                .map(SessionInfoDto::from)
                .collect::<Vec<_>>(),
        );

        cursor = response.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_sessions)
}

async fn switch_active_session(
    app: &AppHandle,
    state: &SharedState,
    active_session_id: &Arc<Mutex<String>>,
    session_id: String,
    config_options: Vec<ConfigOptionDto>,
    project_path: &PathBuf,
) {
    *active_session_id.lock().await = session_id.clone();
    {
        let mut guard = state.lock().await;
        if let Some(project) = guard.project.as_mut() {
            project.session_id = session_id.clone();
            project.config_options = config_options.clone();
        }
    }

    let mut store = load_project_store(app, project_path);
    store.set_active(&session_id);
    save_project_store(app, project_path, &store);

    emit_session_ready(app, session_id, project_path, &config_options);
}

fn emit_session_ready(
    app: &AppHandle,
    session_id: String,
    project_path: &PathBuf,
    config_options: &[ConfigOptionDto],
) {
    let _ = app.emit(
        "acp:session_ready",
        serde_json::json!({
            "sessionId": session_id,
            "projectPath": project_path.display().to_string(),
            "configOptions": config_options,
        }),
    );
}

fn emit_sessions_updated(
    app: &AppHandle,
    sessions: &[SessionInfoDto],
    active_session_id: Option<&str>,
    next_cursor: Option<String>,
) {
    let _ = app.emit(
        "acp:sessions_updated",
        serde_json::json!({
            "sessions": sessions,
            "activeSessionId": active_session_id,
            "nextCursor": next_cursor,
        }),
    );
}

fn session_info_from_parts(
    session_id: String,
    cwd: PathBuf,
    title: Option<String>,
    updated_at: Option<String>,
) -> SessionInfoDto {
    SessionInfoDto {
        session_id,
        cwd: cwd.display().to_string(),
        additional_directories: Vec::new(),
        title,
        updated_at,
    }
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