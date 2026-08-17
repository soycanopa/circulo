//! Mapping regression against the real wire captures in `tests/fixtures/`.
//!
//! Each capture is replayed through `generate` with the fixture's own session
//! id as the binding; the adapter must complete and produce the turn the real
//! server produced, without leaking user-message text into the reply.

use std::path::PathBuf;
use std::time::Duration;

use circulo_adapter::{
    AdapterEvent, AgentAdapter, GenerateRequest, ToolCallStatus, ToolOutput, Uuid,
};
use circulo_adapter_opencode::testing::{FakeOpenCodeServer, ScriptStep};
use circulo_adapter_opencode::{OpenCodeAdapter, ServerConfig};
use serde_json::Value;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn load_script(name: &str) -> (Vec<ScriptStep>, String) {
    let raw = std::fs::read_to_string(fixture_path(name)).expect("fixture readable");
    let mut steps = Vec::new();
    let mut session_id = String::new();
    for line in raw.lines() {
        let Some(payload) = line.strip_prefix("data: ") else {
            continue;
        };
        let envelope: Value = serde_json::from_str(payload).expect("fixture JSON valid");
        if session_id.is_empty() {
            if let Some(id) = envelope
                .get("properties")
                .and_then(|props| props.get("sessionID"))
                .and_then(Value::as_str)
            {
                session_id = id.to_owned();
            }
        }
        steps.push(ScriptStep::Event(envelope));
    }
    (steps, session_id)
}

fn adapter_for(server: &FakeOpenCodeServer) -> OpenCodeAdapter {
    OpenCodeAdapter::new(
        ServerConfig {
            port: server.port,
            command: None,
            cwd: PathBuf::from("."),
            startup_timeout: Duration::from_secs(1),
        },
        Duration::from_secs(10),
    )
}

fn replay(name: &str) -> Vec<AdapterEvent> {
    let (steps, session_id) = load_script(name);
    let server = FakeOpenCodeServer::spawn();
    server.set_script(steps);
    let adapter = adapter_for(&server);
    let mut events = Vec::new();
    adapter
        .generate(
            GenerateRequest {
                session_id: Uuid::nil(),
                user_text: "replayed from fixture".into(),
                agent_session_id: Some(session_id),
            },
            &mut |event| events.push(event),
        )
        .expect("captured turn replays successfully");
    events
}

fn text_of(events: &[AdapterEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::TextDelta { content } => Some(content.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn captured_text_tool_turn_maps_end_to_end() {
    let events = replay("turn-text-tool.sse");

    assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
    let text = text_of(&events);
    assert!(
        text.contains("I see 11 entries in /private/tmp, mostly logs, build folders, and session files."),
        "assistant text must match the capture, got: {text:?}"
    );
    assert!(
        !text.contains("Use your list tool"),
        "user prompt text must not leak into the assistant turn"
    );
    let final_tool = events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::ToolCallUpdated { tool_call } => Some(tool_call),
            _ => None,
        })
        .last()
        .expect("captured tool call");
    assert_eq!(final_tool.name, "read");
    assert_eq!(final_tool.status, ToolCallStatus::Success);
    assert!(matches!(final_tool.output, Some(ToolOutput::Text { .. })));
    assert!(final_tool.started_at.is_some());
    assert!(final_tool.finished_at.is_some());
}

#[test]
fn captured_todo_turn_maps_task_statuses() {
    let events = replay("turn-todo.sse");

    assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
    let last_tasks = events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::TaskList { tasks } => Some(tasks.clone()),
            _ => None,
        })
        .last()
        .expect("captured todo updates");
    assert!(last_tasks.len() >= 3);
    assert!(last_tasks.iter().all(|task| !task.title.is_empty()));
    let statuses: Vec<_> = last_tasks.iter().map(|task| task.status).collect();
    assert!(
        statuses
            .iter()
            .any(|status| matches!(status, circulo_adapter::TaskStatus::Completed)),
        "final todo snapshot should contain completed items: {statuses:?}"
    );
}
