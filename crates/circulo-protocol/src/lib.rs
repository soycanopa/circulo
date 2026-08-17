//! Versioned JSON contract between `circulo-app` and `circulo-daemon`.

use circulo_core::{Message, MessagePart, ToolCall};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const API_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ProjectAssignmentLocked,
    NotFound,
    InvalidRequest,
    Unavailable,
    Internal,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectAssignmentLocked => "project_assignment_locked",
            Self::NotFound => "not_found",
            Self::InvalidRequest => "invalid_request",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub api_version: u32,
    pub code: ErrorCode,
    pub message: String,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            api_version: API_VERSION,
            code,
            message: message.into(),
        }
    }

    pub fn project_assignment_locked() -> Self {
        Self::new(
            ErrorCode::ProjectAssignmentLocked,
            "The project folder can only be chosen when the chat starts.",
        )
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unavailable, message)
    }

    pub fn internal() -> Self {
        Self::new(ErrorCode::Internal, "Something went wrong inside Circulo.")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolEvent {
    #[serde(rename = "server.connected")]
    ServerConnected { api_version: u32 },
    #[serde(rename = "session.message.created")]
    SessionMessageCreated {
        api_version: u32,
        session_id: Uuid,
        message: Message,
    },
    #[serde(rename = "session.message.updated")]
    SessionMessageUpdated {
        api_version: u32,
        session_id: Uuid,
        message: Message,
    },
    #[serde(rename = "session.part.appended")]
    SessionPartAppended {
        api_version: u32,
        session_id: Uuid,
        message_id: Uuid,
        part: MessagePart,
    },
    #[serde(rename = "session.part.updated")]
    SessionPartUpdated {
        api_version: u32,
        session_id: Uuid,
        message_id: Uuid,
        part: MessagePart,
    },
    #[serde(rename = "session.tool_call.updated")]
    SessionToolCallUpdated {
        api_version: u32,
        session_id: Uuid,
        message_id: Uuid,
        tool_call: ToolCall,
    },
    #[serde(rename = "session.message.completed")]
    SessionMessageCompleted {
        api_version: u32,
        session_id: Uuid,
        message_id: Uuid,
        message: Message,
    },
    #[serde(rename = "session.message.failed")]
    SessionMessageFailed {
        api_version: u32,
        session_id: Uuid,
        message_id: Uuid,
        error: ApiError,
    },
}

impl ProtocolEvent {
    pub fn server_connected() -> Self {
        Self::ServerConnected {
            api_version: API_VERSION,
        }
    }

    pub fn session_id(&self) -> Option<Uuid> {
        match self {
            Self::ServerConnected { .. } => None,
            Self::SessionMessageCreated { session_id, .. }
            | Self::SessionMessageUpdated { session_id, .. }
            | Self::SessionPartAppended { session_id, .. }
            | Self::SessionPartUpdated { session_id, .. }
            | Self::SessionToolCallUpdated { session_id, .. }
            | Self::SessionMessageCompleted { session_id, .. }
            | Self::SessionMessageFailed { session_id, .. } => Some(*session_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProjectRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchSessionRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub project_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub archive: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferencesBody {
    pub sidebar_view: circulo_core::SidebarView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub api_version: u32,
    pub daemon: String,
    pub adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_core::{Message, MessagePart, MessageRole, MessageStatus};
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts")
    }

    fn message() -> Message {
        Message {
            id: Uuid::from_u128(10),
            session_id: Uuid::from_u128(11),
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                content: "Done.".into(),
            }],
            status: MessageStatus::Complete,
            created_at: now(),
            is_streaming: false,
        }
    }

    #[test]
    fn connected_event_roundtrip() {
        let event = ProtocolEvent::server_connected();
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "server.connected");
        assert_eq!(json["api_version"], 1);
        let back: ProtocolEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn message_completed_roundtrip() {
        let session_id = Uuid::from_u128(11);
        let msg = message();
        let event = ProtocolEvent::SessionMessageCompleted {
            api_version: API_VERSION,
            session_id,
            message_id: msg.id,
            message: msg,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session.message.completed");
        assert_eq!(json["api_version"], 1);
        assert_eq!(json["session_id"], session_id.to_string());
        let back: ProtocolEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn assignment_locked_error_shape() {
        let err = ApiError::project_assignment_locked();
        assert_eq!(err.api_version, 1);
        assert_eq!(err.code, ErrorCode::ProjectAssignmentLocked);
        assert!(!err.message.is_empty());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "project_assignment_locked");
        let back: ApiError = serde_json::from_value(json).unwrap();
        assert_eq!(back, err);
    }
}
