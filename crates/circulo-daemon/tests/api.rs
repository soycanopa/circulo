use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use circulo_adapter_fake::FakeAdapter;
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
    let store = Store::open_in_memory().expect("memory store");
    let state = AppState::new(store, Arc::new(FakeAdapter::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("serve");
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
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
        .post(format!(
            "http://{addr}/v1/sessions/{}/messages",
            session.id
        ))
        .json(&CreateMessageRequest {
            content: "Hello".into(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let messages: Vec<Message> = client
        .get(format!(
            "http://{addr}/v1/sessions/{}/messages",
            session.id
        ))
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
        .post(format!(
            "http://{addr}/v1/sessions/{}/messages",
            session.id
        ))
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
    };
}
