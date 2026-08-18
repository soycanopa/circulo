use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::model_catalog_cache::{DEFAULT_MODEL_CATALOG_TTL, ModelCatalogCache};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use circulo_adapter::{AdapterError, AdapterHealth, AgentAdapter, AgentSessionSettings, ErrorReason};
use circulo_core::{
    AgentType, ModelCatalogEntry, OffsetDateTime, Project, ProjectStatus, Session, SessionStatus,
    Uuid,
};
use circulo_persist::{PersistError, Store};
use circulo_protocol::{
    ApiError, CreateMessageRequest, CreateProjectRequest, CreateSessionRequest, ErrorCode,
    HealthResponse, OpenCodeHealthBody, PatchProjectRequest, PatchSessionRequest,
    PermissionReplyRequest, PreferencesBody, ProtocolEvent, API_VERSION,
};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::generate::{resolve_working_directory, run_turn};
use crate::permission_waiter::PermissionWaiter;
use crate::turn_registry::TurnRegistry;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub adapter: Arc<dyn AgentAdapter>,
    pub events: broadcast::Sender<ProtocolEvent>,
    pub turns: Arc<TurnRegistry>,
    pub permissions: Arc<PermissionWaiter>,
    model_catalog_cache: Arc<Mutex<ModelCatalogCache>>,
}

impl AppState {
    pub fn new(store: Store, adapter: Arc<dyn AgentAdapter>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            store: Arc::new(Mutex::new(store)),
            adapter,
            events,
            turns: Arc::new(TurnRegistry::new()),
            permissions: Arc::new(PermissionWaiter::new()),
            model_catalog_cache: Arc::new(Mutex::new(
                ModelCatalogCache::new(DEFAULT_MODEL_CATALOG_TTL),
            )),
        }
    }

    fn store(&self) -> Result<std::sync::MutexGuard<'_, Store>, ApiError> {
        self.store.lock().map_err(|_| ApiError::internal())
    }
}

struct HttpError(StatusCode, ApiError);

impl From<ApiError> for HttpError {
    fn from(value: ApiError) -> Self {
        let status = match value.code {
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ErrorCode::ProjectAssignmentLocked => StatusCode::CONFLICT,
            ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self(status, value)
    }
}

impl From<PersistError> for HttpError {
    fn from(value: PersistError) -> Self {
        match value {
            PersistError::NotFound => HttpError::from(ApiError::not_found("Not found.")),
            PersistError::Domain(_) => HttpError::from(ApiError::project_assignment_locked()),
            _ => HttpError::from(ApiError::internal()),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/projects", get(list_projects).post(create_project))
        .route(
            "/v1/projects/{id}",
            get(get_project).patch(patch_project).delete(delete_project),
        )
        .route("/v1/projects/{id}/archive", post(archive_project))
        .route("/v1/projects/{id}/restore", post(restore_project))
        .route("/v1/projects/{id}/sessions", get(list_project_sessions))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}", get(get_session).patch(patch_session).delete(delete_session))
        .route(
            "/v1/sessions/{id}/messages",
            get(list_messages).post(post_message),
        )
        .route("/v1/sessions/{id}/abort", post(abort_session))
        .route(
            "/v1/sessions/{id}/permissions/{permission_id}/reply",
            post(reply_permission),
        )
        .route("/v1/sessions/{id}/events", get(session_events))
        .route("/v1/models", get(list_models))
        .route("/v1/preferences", get(get_preferences).put(put_preferences))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    // probe() may spawn an OpenCode server; keep it off the async runtime.
    let adapter = Arc::clone(&state.adapter);
    let (adapter_state, adapter_message, opencode) =
        tokio::task::spawn_blocking(move || {
            let (adapter_state, adapter_message) = match adapter.probe() {
                AdapterHealth::Available => ("available".to_string(), None),
                AdapterHealth::Missing => ("missing".to_string(), None),
                AdapterHealth::Error { message } => ("error".to_string(), Some(message)),
            };
            let opencode = adapter.opencode_health().map(|health| OpenCodeHealthBody {
                available: health.available,
                version: health.version,
            });
            (adapter_state, adapter_message, opencode)
        })
        .await
        .unwrap_or_else(|_| {
            (
                "error".to_string(),
                Some("The agent check did not answer.".into()),
                None,
            )
        });
    Json(HealthResponse {
        api_version: API_VERSION,
        daemon: "ok".into(),
        adapter: adapter_state,
        adapter_message,
        opencode,
    })
}

#[derive(Debug, Deserialize)]
struct ProjectQuery {
    status: Option<String>,
}

async fn list_projects(
    State(state): State<AppState>,
    Query(query): Query<ProjectQuery>,
) -> Result<Json<Vec<Project>>, HttpError> {
    let store = state.store()?;
    let list = match query.status.as_deref() {
        Some("archived") => store.list_archived_projects()?,
        _ => store.list_active_projects()?,
    };
    Ok(Json(list))
}

async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), HttpError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::invalid_request("Project name is required.").into());
    }
    let now = OffsetDateTime::now_utc();
    let project = Project {
        id: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        color: body.color,
        folder_path: body.folder_path,
        status: ProjectStatus::Active,
        created_at: now,
        updated_at: now,
    };
    state.store()?.create_project(&project)?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, HttpError> {
    state
        .store()?
        .get_project(id)?
        .map(Json)
        .ok_or_else(|| HttpError::from(ApiError::not_found("Project not found.")))
}

async fn patch_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchProjectRequest>,
) -> Result<Json<Project>, HttpError> {
    let store = state.store()?;
    let mut project = store
        .get_project(id)?
        .ok_or_else(|| ApiError::not_found("Project not found."))?;
    if let Some(name) = body.name {
        project.name = name;
    }
    if let Some(description) = body.description {
        project.description = Some(description);
    }
    if let Some(color) = body.color {
        project.color = Some(color);
    }
    if let Some(folder_path) = body.folder_path {
        project.folder_path = Some(folder_path);
    }
    project.updated_at = OffsetDateTime::now_utc();
    store.update_project(&project)?;
    Ok(Json(project))
}

async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    state.store()?.delete_project(id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn archive_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    state.store()?.archive_project(id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn restore_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    state.store()?.restore_project(id)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    q: Option<String>,
    unassigned: Option<bool>,
}

async fn list_sessions(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Vec<Session>>, HttpError> {
    let store = state.store()?;
    let list = if query.unassigned.unwrap_or(false) {
        store.list_unassigned_sessions()?
    } else if let Some(q) = query.q.as_deref() {
        store.search_sessions(q)?
    } else {
        store.list_visible_sessions()?
    };
    Ok(Json(list))
}

async fn list_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Session>>, HttpError> {
    Ok(Json(state.store()?.list_sessions_for_project(id)?))
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<Session>), HttpError> {
    let now = OffsetDateTime::now_utc();
    let session = Session {
        id: Uuid::new_v4(),
        project_id: body.project_id,
        title: body
            .title
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "New session".into()),
        agent: AgentType::OpenCode,
        status: SessionStatus::Active,
        created_at: now,
        updated_at: now,
        last_message_at: None,
        first_send_at: None,
        composer_model_id: None,
        composer_model_variant: None,
        composer_permission_mode: None,
        composer_interaction_mode: None,
    };
    state.store()?.create_session(&session)?;
    Ok((StatusCode::CREATED, Json(session)))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Session>, HttpError> {
    state
        .store()?
        .get_session(id)?
        .map(Json)
        .ok_or_else(|| HttpError::from(ApiError::not_found("Session not found.")))
}

async fn patch_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchSessionRequest>,
) -> Result<Json<Session>, HttpError> {
    let (session, agent_session_id) = {
        let store = state.store()?;
        if let Some(project_id) = body.project_id {
            store.assign_session_project(id, project_id)?;
        }
        let mut session = store
            .get_session(id)?
            .ok_or_else(|| ApiError::not_found("Session not found."))?;
        if let Some(title) = body.title {
            session.title = title;
        }
        if let Some(model_id) = body.composer_model_id {
            session.composer_model_id = Some(model_id);
        }
        if let Some(variant) = body.composer_model_variant {
            session.composer_model_variant = Some(variant);
        }
        if let Some(mode) = body.composer_permission_mode {
            session.composer_permission_mode = Some(mode);
        }
        if let Some(mode) = body.composer_interaction_mode {
            session.composer_interaction_mode = Some(mode);
        }
        if body.archive == Some(true) {
            session.status = SessionStatus::Archived;
        }
        session.updated_at = OffsetDateTime::now_utc();
        store.update_session(&session)?;
        let agent_session_id = if body.composer_permission_mode.is_some() {
            store.opencode_session_id(id)?
        } else {
            None
        };
        (session, agent_session_id)
    };

    if let Some(agent_session_id) = agent_session_id {
        let adapter = Arc::clone(&state.adapter);
        let settings = AgentSessionSettings {
            composer_permission_mode: session.composer_permission_mode,
        };
        let sync_result = tokio::task::spawn_blocking(move || {
            adapter.sync_session_settings(&agent_session_id, &settings)
        })
        .await
        .map_err(|_| HttpError::from(ApiError::internal()))?;
        if let Err(err) = sync_result {
            eprintln!("circulo-daemon: opencode permission sync failed: {}", err.message());
        }
    }

    Ok(Json(session))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    let (agent_session_id, working_directory) = {
        let store = state.store()?;
        let session = store
            .get_session(id)?
            .ok_or_else(|| HttpError::from(ApiError::not_found("Session not found.")))?;
        let agent_session_id = store.opencode_session_id(id)?;
        let working_directory = resolve_working_directory(&store, &session);
        (agent_session_id, Some(working_directory))
    };

    if let Some(agent_session_id) = agent_session_id {
        let adapter = Arc::clone(&state.adapter);
        let delete_result = tokio::task::spawn_blocking(move || {
            adapter.delete_agent_session(&agent_session_id, working_directory.as_deref())
        })
        .await
        .map_err(|_| HttpError::from(ApiError::internal()))?;
        if let Err(err) = delete_result {
            eprintln!(
                "circulo-daemon: opencode session delete failed: {}",
                err.message()
            );
        }
    }

    state.store()?.delete_session(id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_messages(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<circulo_core::Message>>, HttpError> {
    Ok(Json(state.store()?.list_messages(id)?))
}

async fn post_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateMessageRequest>,
) -> Result<Json<circulo_core::Message>, HttpError> {
    if body.content.trim().is_empty() {
        return Err(ApiError::invalid_request("Message content is required.").into());
    }
    let working_directory = {
        let store_guard = state.store()?;
        let session = store_guard
            .get_session(id)?
            .ok_or_else(|| HttpError::from(ApiError::not_found("Session not found.")))?;
        resolve_working_directory(&store_guard, &session)
    };

    let content = body.content;
    let store = Arc::clone(&state.store);
    let adapter = Arc::clone(&state.adapter);
    let events = state.events.clone();
    let turns = Arc::clone(&state.turns);
    let permissions = Arc::clone(&state.permissions);
    let cancel = turns.begin(id, Some(working_directory));
    let assistant = tokio::task::spawn_blocking(move || {
        let store = store.lock().map_err(|_| ApiError::internal())?;
        let turns_ref = turns.as_ref();
        let result = run_turn(
            &store,
            adapter.as_ref(),
            id,
            &content,
            Some(cancel),
            Some((turns_ref, id)),
            Some(permissions.as_ref()),
            Some(&events),
            &mut |event| {
                let _ = events.send(event);
            },
        );
        turns.finish(id);
        result
    })
    .await
    .map_err(|_| HttpError::from(ApiError::internal()))??;
    Ok(Json(assistant))
}

async fn abort_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    state
        .store()?
        .get_session(id)?
        .ok_or_else(|| HttpError::from(ApiError::not_found("Session not found.")))?;
    let adapter = Arc::clone(&state.adapter);
    let turns = Arc::clone(&state.turns);
    tokio::task::spawn_blocking(move || turns.abort(id, adapter.as_ref()))
        .await
        .map_err(|_| HttpError::from(ApiError::internal()))??;
    Ok(StatusCode::NO_CONTENT)
}

async fn reply_permission(
    State(state): State<AppState>,
    Path((session_id, permission_id)): Path<(Uuid, String)>,
    Json(body): Json<PermissionReplyRequest>,
) -> Result<StatusCode, HttpError> {
    state
        .store()?
        .get_session(session_id)?
        .ok_or_else(|| HttpError::from(ApiError::not_found("Session not found.")))?;
    state
        .permissions
        .reply(session_id, &permission_id, body.allow)
        .map_err(HttpError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn session_events(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, HttpError> {
    state
        .store()?
        .get_session(id)?
        .ok_or_else(|| ApiError::not_found("Session not found."))?;
    let rx = state.events.subscribe();
    let connected = ProtocolEvent::server_connected();
    let first = Event::default()
        .event("server.connected")
        .data(serde_json::to_string(&connected).unwrap_or_else(|_| "{}".into()));
    let rest = BroadcastStream::new(rx).filter_map(move |item| {
        let mapped = (|| {
            let event = item.ok()?;
            if event.session_id() != Some(id) {
                return None;
            }
            let name = match &event {
                ProtocolEvent::ServerConnected { .. } => "server.connected",
                ProtocolEvent::SessionMessageCreated { .. } => "session.message.created",
                ProtocolEvent::SessionMessageUpdated { .. } => "session.message.updated",
                ProtocolEvent::SessionPartAppended { .. } => "session.part.appended",
                ProtocolEvent::SessionPartUpdated { .. } => "session.part.updated",
                ProtocolEvent::SessionToolCallUpdated { .. } => "session.tool_call.updated",
                ProtocolEvent::SessionMessageCompleted { .. } => "session.message.completed",
                ProtocolEvent::SessionMessageFailed { .. } => "session.message.failed",
                ProtocolEvent::SessionPermissionRequested { .. } => {
                    "session.permission.requested"
                }
                ProtocolEvent::SessionTitleUpdated { .. } => "session.title.updated",
            };
            let data = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().event(name).data(data)))
        })();
        std::future::ready(mapped)
    });
    let stream = stream::once(async move { Ok(first) }).chain(rest);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

async fn get_preferences(State(state): State<AppState>) -> Result<Json<PreferencesBody>, HttpError> {
    let prefs = state.store()?.get_preferences().map_err(HttpError::from)?;
    Ok(Json(PreferencesBody::from(prefs)))
}

async fn list_models(State(state): State<AppState>) -> Result<Json<Vec<ModelCatalogEntry>>, HttpError> {
    let adapter = Arc::clone(&state.adapter);
    let cache = Arc::clone(&state.model_catalog_cache);
    let models = tokio::task::spawn_blocking(move || {
        let mut cache = cache
            .lock()
            .map_err(|_| {
                AdapterError::failed(
                    ErrorReason::Internal,
                    "Model catalog cache lock poisoned.",
                )
            })?;
        cache.get_or_load(adapter.as_ref())
    })
        .await
        .map_err(|_| HttpError::from(ApiError::internal()))?
        .map_err(|err| {
            HttpError::from(ApiError::unavailable(format!(
                "Could not load models from the agent adapter: {}",
                err.message()
            )))
        })?;
    Ok(Json(models))
}

async fn put_preferences(
    State(state): State<AppState>,
    Json(body): Json<PreferencesBody>,
) -> Result<Json<PreferencesBody>, HttpError> {
    let prefs = circulo_core::UserPreferences::from(body);
    state.store()?.set_preferences(&prefs).map_err(HttpError::from)?;
    Ok(Json(PreferencesBody::from(prefs)))
}
