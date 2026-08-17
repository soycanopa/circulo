use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use circulo_adapter::AgentAdapter;
use circulo_adapter_fake::FakeAdapter;
use circulo_adapter_opencode::testing::{
    idle, text_delta, text_snapshot, todo_list, tool_state, FakeOpenCodeServer,
};
use circulo_adapter_opencode::{OpenCodeAdapter, ServerConfig};
use circulo_core::{Message, MessagePart, MessageStatus, Project, Session, ToolCallStatus};
use circulo_daemon::{listen_addr, router, AppState};
use circulo_persist::Store;
use circulo_protocol::{
    ApiError, CreateMessageRequest, CreateProjectRequest, CreateSessionRequest, ErrorCode,
    HealthResponse, PatchSessionRequest,
};
use futures_util::StreamExt;
use serde_json::json;
use tokio::net::TcpListener;

async fn spawn_server() -> (SocketAddr, reqwest::Client) {
    spawn_server_with(Arc::new(FakeAdapter::new())).await
}

async fn spawn_server_with(adapter: Arc<dyn AgentAdapter>) -> (SocketAddr, reqwest::Client) {
    let store = Store::open_in_memory().expect("memory store");
    let state = AppState::new(store, adapter);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("serve");
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");
    (addr, client)
}

#[test]
fn rejects_non_loopback_listen_addr() {
    assert!(listen_addr(Some("0.0.0.0:9")).is_err());
}

#[tokio::test]
async fn health_on_localhost() {
    let (addr, client) = spawn_server().await;
    let health: HealthResponse = client
        .get(format!("http://{addr}/v1/health"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health.api_version, 1);
    assert_eq!(health.daemon, "ok");
    assert_eq!(health.adapter, "available");
}

#[tokio::test]
async fn create_unassigned_session() {
    let (addr, client) = spawn_server().await;
    let session: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: None,
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(session.project_id.is_none());
    let fetched: Session = client
        .get(format!("http://{addr}/v1/sessions/{}", session.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched.id, session.id);
}

#[tokio::test]
async fn delete_single_session() {
    let (addr, client) = spawn_server().await;
    let keep: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: Some("Keep".into()),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let gone: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: Some("Gone".into()),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    client
        .delete(format!("http://{addr}/v1/sessions/{}", gone.id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let sessions: Vec<Session> = client
        .get(format!("http://{addr}/v1/sessions"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, keep.id);
    let missing = client
        .get(format!("http://{addr}/v1/sessions/{}", gone.id))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
}

#[tokio::test]
async fn post_message_runs_fake_turn() {
    let (addr, client) = spawn_server().await;
    let session: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: Some("Chat".into()),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    client
        .post(format!("http://{addr}/v1/sessions/{}/messages", session.id))
        .json(&CreateMessageRequest {
            content: "Hello".into(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let messages: Vec<Message> = client
        .get(format!("http://{addr}/v1/sessions/{}/messages", session.id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages
        .iter()
        .any(|m| m.role == circulo_core::MessageRole::User));
    let assistant = messages
        .iter()
        .find(|m| m.role == circulo_core::MessageRole::Assistant)
        .unwrap();
    assert_eq!(assistant.status, MessageStatus::Complete);
    assert!(assistant
        .parts
        .iter()
        .any(|p| matches!(p, MessagePart::Text { .. })));
    assert!(assistant.parts.iter().any(|p| matches!(
        p,
        MessagePart::ToolCall { tool_call }
            if tool_call.status == ToolCallStatus::Success
    )));
}

#[tokio::test]
async fn sse_starts_with_connected() {
    let (addr, client) = spawn_server().await;
    let session: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let response = client
        .get(format!("http://{addr}/v1/sessions/{}/events", session.id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let mut stream = response.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        buf.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
        if buf.contains("server.connected") {
            break;
        }
        if buf.len() > 4096 {
            panic!("no connected event in {buf}");
        }
    }
    assert!(buf.contains("\"api_version\":1"));
}

#[tokio::test]
async fn project_patch_after_first_send_is_locked() {
    let (addr, client) = spawn_server().await;
    let project: Project = client
        .post(format!("http://{addr}/v1/projects"))
        .json(&CreateProjectRequest {
            name: "Launch".into(),
            description: None,
            color: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let session: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    client
        .post(format!("http://{addr}/v1/sessions/{}/messages", session.id))
        .json(&CreateMessageRequest {
            content: "Hello".into(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let response = client
        .patch(format!("http://{addr}/v1/sessions/{}", session.id))
        .json(&json!({ "project_id": project.id }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 409);
    let err: ApiError = response.json().await.unwrap();
    assert_eq!(err.code, ErrorCode::ProjectAssignmentLocked);
    let _ = PatchSessionRequest {
        title: None,
        project_id: Some(Some(project.id)),
        archive: None,
        composer_model_id: None,
        composer_permission_mode: None,
        composer_interaction_mode: None,
    };
}

#[tokio::test]
async fn opencode_adapter_turn_binds_and_reuses_across_requests() {
    let opencode = FakeOpenCodeServer::spawn();
    opencode.set_script(vec![
        tool_state("prt_t1", "call_1", "read", "pending", None),
        tool_state("prt_t1", "call_1", "read", "completed", Some("notes found")),
        todo_list(&[("Draft reply", "completed")]),
        text_snapshot("prt_1", ""),
        text_delta("prt_1", "Here is your answer."),
        idle(),
    ]);
    let adapter = OpenCodeAdapter::new(
        ServerConfig {
            port: opencode.port,
            // Hermetic: if the fake server were ever misprobed, spawning must
            // fail instead of launching the machine's real OpenCode.
            command: Some(PathBuf::from("/nonexistent/opencode-for-tests")),
            cwd: PathBuf::from("."),
            startup_timeout: Duration::from_secs(1),
        },
        Duration::from_secs(10),
    );
    let (addr, client) = spawn_server_with(Arc::new(adapter)).await;

    let session: Session = client
        .post(format!("http://{addr}/v1/sessions"))
        .json(&CreateSessionRequest {
            project_id: None,
            title: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    for expected_text in ["Here is your answer.", "Here is your answer."] {
        client
            .post(format!("http://{addr}/v1/sessions/{}/messages", session.id))
            .json(&CreateMessageRequest {
                content: "What is in the notes?".into(),
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let messages: Vec<Message> = client
            .get(format!("http://{addr}/v1/sessions/{}/messages", session.id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let assistant = messages
            .iter()
            .find(|m| m.role == circulo_core::MessageRole::Assistant)
            .expect("assistant message");
        assert_eq!(assistant.status, MessageStatus::Complete);
        assert!(assistant.parts.iter().any(|p| matches!(
            p,
            MessagePart::Text { content } if content == expected_text
        )));
        assert!(assistant.parts.iter().any(|p| matches!(
            p,
            MessagePart::ToolCall { tool_call }
                if tool_call.status == ToolCallStatus::Success
        )));
    }

    // One OpenCode session created on the first send and reused afterwards:
    // the daemon is stateless between requests, so reuse proves the persisted
    // binding round-tripped through SQLite.
    assert_eq!(opencode.sessions_created(), 1);
    let (prompted_session, prompted_text) = opencode.last_prompt().expect("prompt recorded");
    assert!(prompted_session.starts_with("ses_fake_"));
    assert_eq!(prompted_text, "What is in the notes?");
}
