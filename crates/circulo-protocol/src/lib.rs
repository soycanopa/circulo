//! Versioned JSON contract between `circulo-app` and `circulo-daemon`.

use circulo_core::{
    ComposerInteractionMode, ComposerPermissionMode, Message, MessagePart, ToolCall,
    UserPreferences,
};
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
    #[serde(rename = "session.permission.requested")]
    SessionPermissionRequested {
        api_version: u32,
        session_id: Uuid,
        permission_id: String,
        permission: String,
        summary: String,
    },
    #[serde(rename = "session.title.updated")]
    SessionTitleUpdated {
        api_version: u32,
        session_id: Uuid,
        title: String,
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
            | Self::SessionMessageFailed { session_id, .. }
            | Self::SessionPermissionRequested { session_id, .. }
            | Self::SessionTitleUpdated { session_id, .. } => Some(*session_id),
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
    #[serde(default)]
    pub folder_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProjectRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub folder_path: Option<String>,
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
    #[serde(default)]
    pub composer_model_id: Option<String>,
    #[serde(default)]
    pub composer_model_variant: Option<String>,
    #[serde(default)]
    pub composer_permission_mode: Option<ComposerPermissionMode>,
    #[serde(default)]
    pub composer_interaction_mode: Option<ComposerInteractionMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionReplyRequest {
    pub allow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreferencesBody {
    #[serde(default)]
    pub enabled_model_ids: Vec<String>,
}

impl From<UserPreferences> for PreferencesBody {
    fn from(value: UserPreferences) -> Self {
        Self {
            enabled_model_ids: value.enabled_model_ids,
        }
    }
}

impl From<PreferencesBody> for UserPreferences {
    fn from(value: PreferencesBody) -> Self {
        Self {
            enabled_model_ids: value.enabled_model_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeHealthBody {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub api_version: u32,
    pub daemon: String,
    pub adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opencode: Option<OpenCodeHealthBody>,
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

    #[test]
    fn session_title_updated_roundtrip() {
        let session_id = Uuid::from_u128(12);
        let event = ProtocolEvent::SessionTitleUpdated {
            api_version: API_VERSION,
            session_id,
            title: "Launch checklist".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session.title.updated");
        assert_eq!(json["title"], "Launch checklist");
        let back: ProtocolEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn health_response_includes_opencode_block() {
        let health = HealthResponse {
            api_version: API_VERSION,
            daemon: "ok".into(),
            adapter: "available".into(),
            adapter_message: None,
            opencode: Some(OpenCodeHealthBody {
                available: true,
                version: Some("1.18.18".into()),
            }),
        };
        let json = serde_json::to_value(&health).unwrap();
        assert_eq!(json["opencode"]["available"], true);
        assert_eq!(json["opencode"]["version"], "1.18.18");
        let back: HealthResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back, health);
    }
}
