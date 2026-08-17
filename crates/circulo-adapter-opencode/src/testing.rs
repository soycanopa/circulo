//! Scripted OpenCode server for integration tests (feature `test-support`).
//!
//! Serves the subset of the real API Circulo talks to (see
//! `tests/fixtures/EVENTS.md`): `GET /doc`, `POST /session`,
//! `POST /session/{id}/prompt_async`, and `GET /event`. Each prompt replays the
//! configured script on the SSE stream.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
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
}

struct Shared {
    script: Mutex<Vec<ScriptStep>>,
    require_auth: AtomicBool,
    sessions_created: AtomicUsize,
    last_prompt: Mutex<Option<(String, String)>>,
    event_tx: Mutex<Option<mpsc::Sender<String>>>,
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
            event_tx: Mutex::new(None),
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
                    .route("/session", post(create_session))
                    .route("/session/{id}/prompt_async", post(prompt_async))
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

    pub fn require_auth(&self, required: bool) {
        self.shared.require_auth.store(required, Ordering::SeqCst);
    }

    pub fn last_prompt(&self) -> Option<(String, String)> {
        self.shared
            .last_prompt
            .lock()
            .expect("prompt lock")
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

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    if state.shared.require_auth.load(Ordering::SeqCst)
        && !headers.contains_key("authorization")
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let index = state
        .shared
        .sessions_created
        .fetch_add(1, Ordering::SeqCst)
        + 1;
    Ok((
        StatusCode::OK,
        Json(json!({ "id": format!("ses_fake_{index}") })),
    ))
}

async fn prompt_async(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> StatusCode {
    if state.shared.require_auth.load(Ordering::SeqCst)
        && !headers.contains_key("authorization")
    {
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
    *state.shared.last_prompt.lock().expect("prompt lock") = Some((session_id.clone(), user_text));

    let sender = state
        .shared
        .event_tx
        .lock()
        .expect("event lock")
        .clone();
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

async fn event_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
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

fn part_snapshot(part_id: &str, part_type: &str, text: &str) -> ScriptStep {
    ScriptStep::Event(json!({
        "id": "evt_text", "type": "message.part.updated",
        "properties": { "part": {
            "id": part_id, "type": part_type, "text": text, "messageID": "msg_a", "sessionID": "__fill__"
        }, "time": 1786918535912_u64 }
    }))
}

pub fn tool_state(part_id: &str, call_id: &str, tool: &str, status: &str, output: Option<&str>) -> ScriptStep {
    let state = match (status, output) {
        ("completed", Some(output)) => json!({ "status": status, "input": {"path": "notes.md"}, "output": output, "time": {"start": 1786918540000_i64, "end": 1786918541000_i64 } }),
        ("error", Some(message)) => json!({ "status": status, "input": {"path": "notes.md"}, "error": message, "time": {"start": 1786918540000_i64, "end": 1786918541000_i64 } }),
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
