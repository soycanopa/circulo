//! Scripted OpenCode server for integration tests (feature `test-support`).
//!
//! Serves the subset of the real API Circulo talks to (see
//! `tests/fixtures/EVENTS.md`): `GET /doc`, `GET /global/health`, `POST /session`,
//! `POST /session/{id}/prompt_async`, `GET /session/{id}/todo`, `DELETE /session/{id}`, and `GET /event`.
//! Each prompt replays the configured script on the SSE stream.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone)]
pub enum ScriptStep {
    /// Raw event envelope; `properties.sessionID` is filled with the prompted
    /// session id when absent.
    Event(Value),
    /// Closes the SSE stream mid-turn.
    Drop,
    SleepMs(u64),
    /// Like `SleepMs`, but emits `server.heartbeat` frames during the wait.
    SleepWithHeartbeatsMs(u64),
}

struct Shared {
    script: Mutex<Vec<ScriptStep>>,
    require_auth: AtomicBool,
    sessions_created: AtomicUsize,
    last_prompt: Mutex<Option<(String, String, Option<String>)>>,
    last_event_directory: Mutex<Option<String>>,
    last_permission_reply: Mutex<Option<(String, String, bool)>>,
    deleted_sessions: Mutex<Vec<String>>,
    event_tx: Mutex<Option<mpsc::Sender<String>>>,
    todo_snapshot: Mutex<Vec<Value>>,
}

#[derive(Clone)]
struct AppState {
    shared: Arc<Shared>,
}

pub struct FakeOpenCodeServer {
    pub port: u16,
    shared: Arc<Shared>,
    shutdown: watch::Sender<bool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeOpenCodeServer {
    pub fn spawn() -> Self {
        let shared = Arc::new(Shared {
            script: Mutex::new(Vec::new()),
            require_auth: AtomicBool::new(false),
            sessions_created: AtomicUsize::new(0),
            last_prompt: Mutex::new(None),
            last_event_directory: Mutex::new(None),
            last_permission_reply: Mutex::new(None),
            deleted_sessions: Mutex::new(Vec::new()),
            event_tx: Mutex::new(None),
            todo_snapshot: Mutex::new(Vec::new()),
        });
        let (shutdown, mut shutdown_rx) = watch::channel(false);
        let (port_tx, port_rx) = std::sync::mpsc::channel();
        let state = AppState {
            shared: Arc::clone(&shared),
        };
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("fake server runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                    .await
                    .expect("fake server bind");
                let port = listener.local_addr().expect("fake server addr").port();
                port_tx.send(port).expect("report fake server port");
                let app = Router::new()
                    .route("/doc", get(doc))
                    .route("/global/health", get(global_health))
                    .route("/session", post(create_session))
                    .route("/session/{id}", delete(delete_session))
                    .route("/session/{id}/prompt_async", post(prompt_async))
                    .route("/session/{id}/todo", get(list_session_todos))
                    .route("/session/{id}/abort", post(abort_session))
                    .route(
                        "/session/{id}/permissions/{permission_id}",
                        post(reply_permission),
                    )
                    .route("/event", get(event_stream))
                    .with_state(state);
                let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                    let _ = shutdown_rx.wait_for(|down| *down).await;
                });
                let _ = server.await;
            });
        });
        let port = port_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("fake server started");
        Self {
            port,
            shared,
            shutdown,
            thread: Some(thread),
        }
    }

    pub fn set_script(&self, steps: Vec<ScriptStep>) {
        *self.shared.script.lock().expect("script lock") = steps;
    }

    pub fn set_todo_snapshot(&self, todos: &[(&str, &str)]) {
        let items: Vec<Value> = todos
            .iter()
            .map(|(content, status)| {
                json!({ "content": content, "status": status, "priority": "medium" })
            })
            .collect();
        *self
            .shared
            .todo_snapshot
            .lock()
            .expect("todo snapshot lock") = items;
    }

    pub fn require_auth(&self, required: bool) {
        self.shared.require_auth.store(required, Ordering::SeqCst);
    }

    pub fn last_prompt(&self) -> Option<(String, String, Option<String>)> {
        self.shared.last_prompt.lock().expect("prompt lock").clone()
    }

    pub fn last_event_directory(&self) -> Option<String> {
        self.shared
            .last_event_directory
            .lock()
            .expect("event directory lock")
            .clone()
    }

    pub fn last_permission_reply(&self) -> Option<(String, String, bool)> {
        self.shared
            .last_permission_reply
            .lock()
            .expect("permission reply lock")
            .clone()
    }

    pub fn deleted_sessions(&self) -> Vec<String> {
        self.shared
            .deleted_sessions
            .lock()
            .expect("deleted sessions lock")
            .clone()
    }

    pub fn sessions_created(&self) -> usize {
        self.shared.sessions_created.load(Ordering::SeqCst)
    }
}

impl Drop for FakeOpenCodeServer {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn doc() -> Json<Value> {
    Json(json!({ "openapi": "3.1.0", "info": { "title": "fake opencode" } }))
}

async fn global_health() -> Json<Value> {
    Json(json!({ "healthy": true, "version": "0.0.0-test" }))
}

async fn list_session_todos(
    State(state): State<AppState>,
    Path(_session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let todos = state
        .shared
        .todo_snapshot
        .lock()
        .expect("todo snapshot lock")
        .clone();
    Ok(Json(Value::Array(todos)))
}

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DirectoryQuery>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let index = state.shared.sessions_created.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = query.directory;
    Ok((
        StatusCode::OK,
        Json(json!({ "id": format!("ses_fake_{index}") })),
    ))
}

#[derive(Debug, Deserialize)]
struct DirectoryQuery {
    directory: Option<String>,
}

async fn prompt_async(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<DirectoryQuery>,
    body: Option<Json<Value>>,
) -> StatusCode {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return StatusCode::UNAUTHORIZED;
    }
    let user_text = body
        .and_then(|Json(value)| {
            value
                .get("parts")?
                .get(0)?
                .get("text")?
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or_default();
    *state.shared.last_prompt.lock().expect("prompt lock") =
        Some((session_id.clone(), user_text, query.directory));

    let sender = state.shared.event_tx.lock().expect("event lock").clone();
    let Some(sender) = sender else {
        return StatusCode::NO_CONTENT;
    };
    let steps = state.shared.script.lock().expect("script lock").clone();
    let shared = Arc::clone(&state.shared);
    tokio::spawn(async move {
        for step in steps {
            match step {
                ScriptStep::Event(mut envelope) => {
                    inject_session(&mut envelope, &session_id);
                    if sender.send(envelope.to_string()).await.is_err() {
                        return;
                    }
                }
                ScriptStep::Drop => {
                    *shared.event_tx.lock().expect("event lock") = None;
                }
                ScriptStep::SleepMs(ms) => {
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                }
                ScriptStep::SleepWithHeartbeatsMs(ms) => {
                    let sender = sender.clone();
                    let started = std::time::Instant::now();
                    while started.elapsed() < Duration::from_millis(ms) {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        let heartbeat = json!({
                            "id": "evt_hb",
                            "type": "server.heartbeat",
                            "properties": {}
                        });
                        if sender.send(heartbeat.to_string()).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }
    });
    StatusCode::NO_CONTENT
}

fn inject_session(envelope: &mut Value, session_id: &str) {
    if let Some(properties) = envelope.get_mut("properties") {
        if properties.get("sessionID").is_none() {
            properties["sessionID"] = json!(session_id);
        }
    }
}

async fn abort_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let sender = state.shared.event_tx.lock().expect("event lock").clone();
    if let Some(sender) = sender {
        let mut envelope = json!({ "id": "evt_idle_abort", "type": "session.idle", "properties": {} });
        inject_session(&mut envelope, &session_id);
        let _ = sender.try_send(envelope.to_string());
    }
    Ok(Json(json!(true)))
}

async fn reply_permission(
    State(state): State<AppState>,
    Path((session_id, permission_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let allow = body
        .get("response")
        .and_then(Value::as_str)
        .is_some_and(|response| response == "once" || response == "always");
    *state
        .shared
        .last_permission_reply
        .lock()
        .expect("permission reply lock") = Some((session_id, permission_id, allow));
    Ok(Json(json!(true)))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<DirectoryQuery>,
) -> Result<Json<Value>, StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst) && !headers.contains_key("authorization") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let _ = query.directory;
    state
        .shared
        .deleted_sessions
        .lock()
        .expect("deleted sessions lock")
        .push(session_id);
    Ok(Json(json!(true)))
}

async fn event_stream(
    State(state): State<AppState>,
    Query(query): Query<DirectoryQuery>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    *state
        .shared
        .last_event_directory
        .lock()
        .expect("event directory lock") = query.directory.clone();
    let (tx, rx) = mpsc::channel::<String>(64);
    *state.shared.event_tx.lock().expect("event lock") = Some(tx);
    let first = futures_util::stream::once(async {
        Ok::<_, Infallible>(
            Event::default().data(r#"{"id":"evt_0","type":"server.connected","properties":{}}"#),
        )
    });
    let rest = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|data| Ok::<_, Infallible>(Event::default().data(data)));
    Sse::new(first.chain(rest))
}

// Script helpers: shapes mirror the fixtures captured from the real server.

pub fn text_delta(part_id: &str, delta: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_delta", "type": "message.part.delta",
        "properties": { "messageID": "msg_a", "partID": part_id, "field": "text", "delta": delta }
    }))
}

pub fn text_snapshot(part_id: &str, text: &str) -> ScriptStep {
    part_snapshot(part_id, "text", text)
}

pub fn reasoning_snapshot(part_id: &str, text: &str) -> ScriptStep {
    part_snapshot(part_id, "reasoning", text)
}

pub fn reasoning_opaque_snapshot(part_id: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_reason_opaque",
        "type": "message.part.updated",
        "properties": {
            "part": {
                "id": part_id,
                "type": "reasoning",
                "text": "",
                "metadata": { "anthropic": { "signature": "sig_test" } },
                "messageID": "msg_a",
                "sessionID": "__fill__",
                "time": { "start": 1786918535912_i64, "end": 1786918536912_i64 }
            }
        }
    }))
}

fn part_snapshot(part_id: &str, part_type: &str, text: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_text", "type": "message.part.updated",
        "properties": { "part": {
            "id": part_id, "type": part_type, "text": text, "messageID": "msg_a", "sessionID": "__fill__"
        }, "time": 1786918535912_u64 }
    }))
}

pub fn tool_state(
    part_id: &str,
    call_id: &str,
    tool: &str,
    status: &str,
    output: Option<&str>,
) -> ScriptStep {
    let state = match (status, output) {
        ("completed", Some(output)) => {
            json!({ "status": status, "input": {"path": "notes.md"}, "output": output, "time": {"start": 1786918540000_i64, "end": 1786918541000_i64 } })
        }
        ("error", Some(message)) => {
            json!({ "status": status, "input": {"path": "notes.md"}, "error": message, "time": {"start": 1786918540000_i64, "end": 1786918541000_i64 } })
        }
        _ => json!({ "status": status, "input": {"path": "notes.md"} }),
    };
    ScriptStep::Event(json!({
        "id": "evt_tool", "type": "message.part.updated",
        "properties": { "part": {
            "id": part_id, "type": "tool", "callID": call_id, "tool": tool,
            "state": state, "messageID": "msg_a", "sessionID": "__fill__"
        }, "time": 1786918542951_u64 }
    }))
}

pub fn todo_list(todos: &[(&str, &str)]) -> ScriptStep {
    let items: Vec<Value> = todos
        .iter()
        .map(|(content, status)| json!({ "content": content, "status": status, "priority": "medium" }))
        .collect();
    ScriptStep::Event(json!({
        "id": "evt_todo", "type": "todo.updated",
        "properties": { "todos": items }
    }))
}

pub fn idle() -> ScriptStep {
    ScriptStep::Event(json!({ "id": "evt_idle", "type": "session.idle", "properties": {} }))
}

pub fn session_title_updated(title: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_title",
        "type": "session.updated",
        "properties": {
            "info": { "title": title }
        }
    }))
}

pub fn permission_asked(id: &str, permission: &str, pattern: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_perm",
        "type": "permission.asked",
        "properties": {
            "id": id,
            "permission": permission,
            "patterns": [pattern],
            "metadata": {},
            "always": []
        }
    }))
}

pub fn session_error(name: &str, message: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_err", "type": "session.error",
        "properties": { "error": { "name": name, "message": message } }
    }))
}

pub fn message_with_error(name: &str, message: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_msg_err", "type": "message.updated",
        "properties": { "info": { "role": "assistant", "error": { "name": name, "message": message } } }
    }))
}

pub fn unknown_event() -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_future", "type": "session.next.reasoning.delta",
        "properties": { "delta": "thinking…" }
    }))
}

pub fn drop_stream() -> ScriptStep {
    ScriptStep::Drop
}

pub fn sleep_ms(ms: u64) -> ScriptStep {
    ScriptStep::SleepMs(ms)
}

pub fn sleep_with_heartbeats(ms: u64) -> ScriptStep {
    ScriptStep::SleepWithHeartbeatsMs(ms)
}

pub fn heartbeat() -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_hb",
        "type": "server.heartbeat",
        "properties": {}
    }))
}
