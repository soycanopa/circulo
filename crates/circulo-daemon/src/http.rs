use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use circulo_adapter::{AdapterHealth, AgentAdapter};
use circulo_core::{
    AgentType, OffsetDateTime, Project, ProjectStatus, Session, SessionStatus, SidebarView, Uuid,
};
use circulo_persist::{PersistError, Store};
use circulo_protocol::{
    ApiError, CreateMessageRequest, CreateProjectRequest, CreateSessionRequest, ErrorCode,
    HealthResponse, PatchProjectRequest, PatchSessionRequest, PreferencesBody, ProtocolEvent,
    API_VERSION,
};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::generate::run_turn;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub adapter: Arc<dyn AgentAdapter>,
    pub events: broadcast::Sender<ProtocolEvent>,
}

impl AppState {
    pub fn new(store: Store, adapter: Arc<dyn AgentAdapter>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            store: Arc::new(Mutex::new(store)),
            adapter,
            events,
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
        .route(
            "/v1/sessions/{id}",
            get(get_session).patch(patch_session),
        )
        .route("/v1/sessions/{id}/messages", get(list_messages).post(post_message))
        .route("/v1/sessions/{id}/events", get(session_events))
        .route("/v1/preferences", get(get_preferences).put(put_preferences))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    // probe() may spawn an OpenCode server; keep it off the async runtime.
    let adapter = Arc::clone(&state.adapter);
    let (adapter_state, adapter_message) = tokio::task::spawn_blocking(move || match adapter.probe()
    {
        AdapterHealth::Available => ("available".to_string(), None),
        AdapterHealth::Missing => ("missing".to_string(), None),
        AdapterHealth::Error { message } => ("error".to_string(), Some(message)),
    })
    .await
    .unwrap_or_else(|_| {
        (
            "error".to_string(),
            Some("The agent check did not answer.".into()),
        )
    });
    Json(HealthResponse {
        api_version: API_VERSION,
        daemon: "ok".into(),
        adapter: adapter_state,
        adapter_message,
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
    if body.archive == Some(true) {
        session.status = SessionStatus::Archived;
    }
    session.updated_at = OffsetDateTime::now_utc();
    store.update_session(&session)?;
    Ok(Json(session))
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
    state.store()?.get_session(id)?.ok_or_else(|| {
        HttpError::from(ApiError::not_found("Session not found."))
    })?;
    // A real adapter blocks on network IO and SSE reads for the whole turn.
    let store = Arc::clone(&state.store);
    let adapter = Arc::clone(&state.adapter);
    let events = state.events.clone();
    let assistant = tokio::task::spawn_blocking(move || {
        let store = store.lock().map_err(|_| ApiError::internal())?;
        run_turn(&store, adapter.as_ref(), id, &body.content, &mut |event| {
            let _ = events.send(event);
        })
    })
    .await
    .map_err(|_| HttpError::from(ApiError::internal()))??;
    Ok(Json(assistant))
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
    Ok(Json(PreferencesBody {
        sidebar_view: state.store()?.sidebar_view()?,
    }))
}

async fn put_preferences(
    State(state): State<AppState>,
    Json(body): Json<PreferencesBody>,
) -> Result<Json<PreferencesBody>, HttpError> {
    let view = match body.sidebar_view {
        SidebarView::Sessions | SidebarView::Groups => body.sidebar_view,
    };
    state.store()?.set_sidebar_view(view)?;
    Ok(Json(PreferencesBody { sidebar_view: view }))
}
