//! `OpenCodeAdapter::generate` against the scripted fake server.

use std::path::PathBuf;
use std::time::Duration;

use circulo_adapter::{
    AdapterEvent, AdapterHealth, AgentAdapter, ErrorReason, GenerateRequest, TaskStatus,
    ToolCallStatus, ToolOutput, Uuid,
};
use circulo_adapter_opencode::testing::{
    drop_stream, idle, message_with_error, reasoning_snapshot, session_error, sleep_ms, text_delta,
    text_snapshot, todo_list, tool_state, unknown_event, FakeOpenCodeServer,
};
use circulo_adapter_opencode::{OpenCodeAdapter, ServerConfig};

fn adapter_for(server: &FakeOpenCodeServer, turn_timeout: Duration) -> OpenCodeAdapter {
    OpenCodeAdapter::new(
        ServerConfig {
            port: server.port,
            command: None,
            cwd: PathBuf::from("."),
            startup_timeout: Duration::from_secs(1),
        },
        turn_timeout,
    )
}

fn request(agent_session_id: Option<String>) -> GenerateRequest {
    GenerateRequest {
        session_id: Uuid::nil(),
        user_text: "List the files".into(),
        agent_session_id,
    }
}

fn collect(
    adapter: &OpenCodeAdapter,
    request: GenerateRequest,
) -> (Result<(), circulo_adapter::AdapterError>, Vec<AdapterEvent>) {
    let mut events = Vec::new();
    let result = adapter.generate(request, &mut |event| events.push(event));
    (result, events)
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
fn text_turn_binds_streams_and_completes() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        text_delta("prt_1", "Hello"),
        text_snapshot("prt_1", "Hello world"),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None));

    assert!(result.is_ok());
    assert_eq!(server.sessions_created(), 1);
    match events.first() {
        Some(AdapterEvent::SessionBound { agent_session_id }) => {
            assert!(agent_session_id.starts_with("ses_fake_"));
        }
        other => panic!("expected session bound first, got {other:?}"),
    }
    assert_eq!(text_of(&events), "Hello world");
    assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
    let (prompted_session, prompted_text) = server.last_prompt().expect("prompt recorded");
    assert!(prompted_session.starts_with("ses_fake_"));
    assert_eq!(prompted_text, "List the files");
}

#[test]
fn bound_session_is_reused_without_new_binding() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Hi"), idle()]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(Some("ses_fake_existing".into())));

    assert!(result.is_ok());
    assert_eq!(server.sessions_created(), 0);
    assert!(!events
        .iter()
        .any(|e| matches!(e, AdapterEvent::SessionBound { .. })));
    assert_eq!(server.last_prompt().unwrap().0, "ses_fake_existing");
}

#[test]
fn tool_and_task_turn_maps_statuses() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        tool_state("prt_t1", "call_1", "read", "pending", None),
        tool_state("prt_t1", "call_1", "read", "running", None),
        tool_state(
            "prt_t1",
            "call_1",
            "read",
            "completed",
            Some("<entries>notes.md</entries>"),
        ),
        todo_list(&[
            ("Sort notes", "pending"),
            ("Archive duplicates", "in_progress"),
        ]),
        text_delta("prt_1", "Done."),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None));

    assert!(result.is_ok());
    assert!(events
        .iter()
        .any(|e| matches!(e, AdapterEvent::ToolCallStarted { .. })));
    let final_tool = events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::ToolCallUpdated { tool_call } => Some(tool_call),
            _ => None,
        })
        .last()
        .expect("tool updated");
    assert_eq!(final_tool.name, "read");
    assert_eq!(final_tool.status, ToolCallStatus::Success);
    assert!(matches!(
        &final_tool.output,
        Some(ToolOutput::Text { content }) if content.contains("notes.md")
    ));
    let task_lists: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::TaskList { tasks } => Some(tasks.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(task_lists.len(), 1);
    assert_eq!(task_lists[0].len(), 2);
    assert_eq!(task_lists[0][0].status, TaskStatus::Pending);
    assert_eq!(task_lists[0][1].status, TaskStatus::InProgress);
    assert!(final_tool.finished_at.is_some());
}

#[test]
fn unknown_event_is_skipped_without_failing() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        unknown_event(),
        text_snapshot("prt_1", ""),
        text_delta("prt_1", "Still fine"),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None));

    assert!(result.is_ok());
    assert_eq!(text_of(&events), "Still fine");
    assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
}

#[test]
fn dropped_stream_fails_the_turn() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Partial"), drop_stream()]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None));

    let err = result.expect_err("stream drop must fail");
    assert_eq!(err.reason(), ErrorReason::StreamFailed);
    assert!(events
        .iter()
        .any(|e| matches!(e, AdapterEvent::Failed { .. })));
}

#[test]
fn session_error_maps_to_unauthorized_for_auth_failures() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        session_error("ProviderAuthError", "invalid api key"),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, _) = collect(&adapter, request(None));

    let err = result.expect_err("auth error must fail");
    assert_eq!(err.reason(), ErrorReason::Unauthorized);
}

#[test]
fn message_error_maps_to_provider_failure() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![message_with_error("UnknownError", "boom")]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, _) = collect(&adapter, request(None));

    let err = result.expect_err("provider error must fail");
    assert_eq!(err.reason(), ErrorReason::ProviderFailed);
}

#[test]
fn stalled_turn_times_out() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        text_delta("prt_1", "Partial"),
        sleep_ms(3000),
        text_delta("prt_1", " too late"),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_millis(400));

    let (result, _) = collect(&adapter, request(None));

    let err = result.expect_err("stalled turn must time out");
    assert_eq!(err.reason(), ErrorReason::Timeout);
}

#[test]
fn unauthorized_server_fails_session_creation() {
    let server = FakeOpenCodeServer::spawn();
    server.require_auth(true);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, _) = collect(&adapter, request(None));

    let err = result.expect_err("401 must fail");
    assert_eq!(err.reason(), ErrorReason::Unauthorized);
}

#[test]
fn reasoning_deltas_do_not_leak_into_the_reply() {
    // Live capture (minimax model): a reasoning part announces itself with an
    // empty snapshot, streams `field: "text"` deltas, and only then a real
    // text part starts. Only announced text parts may stream.
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        reasoning_snapshot("prt_reason", ""),
        text_delta("prt_reason", "The user asks a question. "),
        text_delta("prt_reason", "I should answer concisely."),
        reasoning_snapshot(
            "prt_reason",
            "The user asks a question. I should answer concisely.",
        ),
        text_snapshot("prt_answer", ""),
        text_delta("prt_answer", "A cortado is espresso cut"),
        text_delta("prt_answer", " with steamed milk."),
        text_snapshot("prt_answer", "A cortado is espresso cut with steamed milk."),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None));

    assert!(result.is_ok());
    let text = text_of(&events);
    assert_eq!(text, "A cortado is espresso cut with steamed milk.");
}

#[test]
fn probe_reports_available_when_server_healthy() {
    let server = FakeOpenCodeServer::spawn();
    let adapter = adapter_for(&server, Duration::from_secs(5));
    assert_eq!(adapter.probe(), AdapterHealth::Available);
}
