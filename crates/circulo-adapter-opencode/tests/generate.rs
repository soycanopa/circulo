//! `OpenCodeAdapter::generate` against the scripted fake server.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use circulo_adapter::{
    AdapterEvent, AdapterHealth, AgentAdapter, ErrorReason, GenerateRequest, OpenCodeHealth,
    PermissionDecision, PermissionRequest, PermissionResponder, TaskStatus, ToolCallStatus,
    ToolOutput, Uuid,
};
use circulo_adapter_opencode::testing::{
    drop_stream, idle, message_with_error, permission_asked, reasoning_opaque_snapshot,
    reasoning_snapshot, session_error, session_title_updated, sleep_ms, sleep_with_heartbeats,
    text_delta, text_snapshot, todo_list, tool_state, unknown_event, FakeOpenCodeServer,
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

fn request_with_responder(
    agent_session_id: Option<String>,
    working_directory: Option<PathBuf>,
    cancel: Option<Arc<AtomicBool>>,
    permission_responder: Option<PermissionResponder>,
) -> GenerateRequest {
    GenerateRequest {
        session_id: Uuid::nil(),
        user_text: "List the files".into(),
        agent_session_id,
        composer_model_id: None,
        composer_model_variant: None,
        composer_permission_mode: None,
        composer_interaction_mode: None,
        working_directory,
        cancel,
        permission_responder,
    }
}

fn request(
    agent_session_id: Option<String>,
    working_directory: Option<PathBuf>,
    cancel: Option<Arc<AtomicBool>>,
) -> GenerateRequest {
    request_with_responder(agent_session_id, working_directory, cancel, None)
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

fn reasoning_of(events: &[AdapterEvent]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::ReasoningDelta { content, .. } => Some(content.as_str()),
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

    let (result, events) = collect(&adapter, request(None, None, None));

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
    let (prompted_session, prompted_text, prompted_directory) =
        server.last_prompt().expect("prompt recorded");
    assert!(prompted_session.starts_with("ses_fake_"));
    assert_eq!(prompted_text, "List the files");
    assert!(prompted_directory.is_none());
}

#[test]
fn project_working_directory_is_sent_on_prompt() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Hi"), idle()]);
    let adapter = adapter_for(&server, Duration::from_secs(5));
    let project_dir = std::env::temp_dir().join("circulo-cwd-test");
    let _ = std::fs::create_dir_all(&project_dir);

    let (result, _) = collect(
        &adapter,
        request(None, Some(project_dir.clone()), None),
    );

    assert!(result.is_ok());
    let (_, _, directory) = server.last_prompt().expect("prompt recorded");
    assert_eq!(directory.as_deref(), Some(project_dir.to_string_lossy().as_ref()));
    let _ = std::fs::remove_dir_all(project_dir);
}

#[test]
fn abort_mid_turn_ends_with_cancelled() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        text_delta("prt_1", "Partial"),
        sleep_ms(5000),
        idle(),
    ]);
    let port = server.port;
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_flag = Arc::clone(&cancel);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        cancel_flag.store(true, Ordering::SeqCst);
        let url = format!("http://127.0.0.1:{port}/session/ses_stop/abort");
        let _ = ureq::post(&url).call();
    });
    let adapter = adapter_for(&server, Duration::from_secs(10));

    let (result, events) = collect(
        &adapter,
        request(Some("ses_stop".into()), None, Some(cancel)),
    );

    let err = result.expect_err("aborted turn must fail");
    assert_eq!(err.reason(), ErrorReason::Cancelled);
    assert!(events
        .iter()
        .any(|event| matches!(event, AdapterEvent::Failed { .. })));
}

#[test]
fn supervised_permission_reply_continues_turn() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        permission_asked("perm_1", "bash", "npm test"),
        text_snapshot("prt_1", "Done."),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));
    let responder = PermissionResponder::new(|request: PermissionRequest| {
        assert_eq!(request.id, "perm_1");
        assert_eq!(request.permission, "bash");
        PermissionDecision::AllowOnce
    });

    let (result, events) = collect(
        &adapter,
        request_with_responder(Some("ses_perm".into()), None, None, Some(responder)),
    );

    assert!(result.is_ok());
    assert_eq!(text_of(&events), "Done.");
    let reply = server
        .last_permission_reply()
        .expect("permission reply recorded");
    assert_eq!(reply.0, "ses_perm");
    assert_eq!(reply.1, "perm_1");
    assert!(reply.2);
}

#[test]
fn bound_session_is_reused_without_new_binding() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Hi"), idle()]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(Some("ses_fake_existing".into()), None, None));

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

    let (result, events) = collect(&adapter, request(None, None, None));

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

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_ok());
    assert_eq!(text_of(&events), "Still fine");
    assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
}

#[test]
fn todo_refetch_reconciles_after_stream_drop() {
    let server = FakeOpenCodeServer::spawn();
    server.set_todo_snapshot(&[
        ("Sort notes", "completed"),
        ("Archive duplicates", "in_progress"),
    ]);
    server.set_script(vec![
        tool_state(
            "prt_t1",
            "call_1",
            "read",
            "completed",
            Some("<entries>notes.md</entries>"),
        ),
        drop_stream(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(2));

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_err(), "turn should still fail without idle after reconnect");
    let task_lists: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AdapterEvent::TaskList { tasks } => Some(tasks.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(task_lists.len(), 1, "expected refetched todo list: {events:?}");
    assert_eq!(task_lists[0].len(), 2);
    assert_eq!(task_lists[0][0].title, "Sort notes");
    assert_eq!(task_lists[0][0].status, TaskStatus::Completed);
    assert_eq!(task_lists[0][1].status, TaskStatus::InProgress);
}

#[test]
fn dropped_stream_fails_the_turn() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Partial"), drop_stream()]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None, None, None));

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

    let (result, _) = collect(&adapter, request(None, None, None));

    let err = result.expect_err("auth error must fail");
    assert_eq!(err.reason(), ErrorReason::Unauthorized);
}

#[test]
fn message_error_maps_to_provider_failure() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![message_with_error("UnknownError", "boom")]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, _) = collect(&adapter, request(None, None, None));

    let err = result.expect_err("provider error must fail");
    assert_eq!(err.reason(), ErrorReason::ProviderFailed);
}

#[test]
fn heartbeats_prevent_read_timeout_during_quiet_work() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        text_snapshot("prt_1", ""),
        text_delta("prt_1", "Hi"),
        sleep_with_heartbeats(2500),
        text_snapshot("prt_1", "Hi"),
        text_delta("prt_1", " there"),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(3));

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_ok(), "heartbeats should keep the stream alive: {result:?}");
    assert_eq!(text_of(&events), "Hi there");
}

#[test]
fn quiet_gap_without_heartbeats_hits_read_timeout() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![text_delta("prt_1", "Hi"), sleep_ms(1200), idle()]);
    let adapter = adapter_for(&server, Duration::from_millis(800));

    let (result, _) = collect(&adapter, request(None, None, None));

    assert_eq!(
        result.expect_err("quiet gap should time out").reason(),
        ErrorReason::Timeout
    );
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

    let (result, _) = collect(&adapter, request(None, None, None));

    let err = result.expect_err("stalled turn must time out");
    assert_eq!(err.reason(), ErrorReason::Timeout);
}

#[test]
fn unauthorized_server_fails_session_creation() {
    let server = FakeOpenCodeServer::spawn();
    server.require_auth(true);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, _) = collect(&adapter, request(None, None, None));

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

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_ok());
    let text = text_of(&events);
    assert_eq!(text, "A cortado is espresso cut with steamed milk.");
    assert!(reasoning_of(&events).contains("question"));
    assert!(!text.contains("question"));
}

#[test]
fn encrypted_reasoning_marks_opaque_without_leaking() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        reasoning_opaque_snapshot("prt_hidden"),
        text_snapshot("prt_answer", "Visible reply."),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_ok());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AdapterEvent::ReasoningOpaque {
                part_id,
            } if part_id == "prt_hidden"
        )
    }));
    assert_eq!(text_of(&events), "Visible reply.");
    assert!(reasoning_of(&events).is_empty());
}

#[test]
fn delete_bound_session_calls_opencode() {
    let server = FakeOpenCodeServer::spawn();
    let adapter = adapter_for(&server, Duration::from_secs(5));

    adapter
        .delete_agent_session("ses_fake_1", None)
        .expect("delete should succeed");

    assert_eq!(server.deleted_sessions(), vec!["ses_fake_1"]);
}

#[test]
fn session_title_update_emits_adapter_event() {
    let server = FakeOpenCodeServer::spawn();
    server.set_script(vec![
        session_title_updated("Launch checklist"),
        text_snapshot("prt_1", "Done."),
        idle(),
    ]);
    let adapter = adapter_for(&server, Duration::from_secs(5));

    let (result, events) = collect(&adapter, request(None, None, None));

    assert!(result.is_ok());
    assert!(events.iter().any(|event| matches!(
        event,
        AdapterEvent::SessionTitleUpdated { title }
            if title == "Launch checklist"
    )));
}

#[test]
fn probe_reports_available_when_server_healthy() {
    let server = FakeOpenCodeServer::spawn();
    let adapter = adapter_for(&server, Duration::from_secs(5));
    assert_eq!(adapter.probe(), AdapterHealth::Available);
}

#[test]
fn opencode_health_surfaces_version_on_adapter() {
    let server = FakeOpenCodeServer::spawn();
    let adapter = adapter_for(&server, Duration::from_secs(5));
    let health = adapter.opencode_health().expect("opencode health");
    assert_eq!(
        health,
        OpenCodeHealth {
            available: true,
            version: Some("0.0.0-test".into()),
        }
    );
}
