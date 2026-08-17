//! Provider-agnostic adapter trait.
//!
//! Implementations live in sibling crates. This crate must not depend on GPUI
//! or on a specific agent CLI.

pub use circulo_core::{Task, TaskStatus, ToolCall, ToolCallStatus, ToolOutput, Uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterHealth {
    Available,
    Missing,
    Error { message: String },
}

/// Stable machine cause of an adapter failure. The daemon maps each variant to
/// locale-catalog copy; adapters never render user-facing text themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReason {
    BinaryMissing,
    StartFailed,
    PortOccupied,
    Unauthorized,
    StreamFailed,
    Timeout,
    ProviderFailed,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    kind: ErrorKind,
    reason: ErrorReason,
    message: String,
}

impl AdapterError {
    pub fn unavailable(reason: ErrorReason, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Unavailable,
            reason,
            message: message.into(),
        }
    }

    pub fn failed(reason: ErrorReason, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Failed,
            reason,
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn reason(&self) -> ErrorReason {
        self.reason
    }

    pub fn kind(&self) -> &'static str {
        match self.kind {
            ErrorKind::Unavailable => "unavailable",
            ErrorKind::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdapterEvent {
    /// Emitted once when the adapter created the provider-side session for a
    /// Circulo session. Always the first event of a turn that binds.
    SessionBound { agent_session_id: String },
    TextDelta { content: String },
    TaskList { tasks: Vec<Task> },
    ToolCallStarted { tool_call: ToolCall },
    ToolCallUpdated { tool_call: ToolCall },
    Completed,
    Failed { error: AdapterError },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateRequest {
    pub session_id: Uuid,
    pub user_text: String,
    /// Provider-side session binding persisted by the daemon, when one exists.
    pub agent_session_id: Option<String>,
}

pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    fn probe(&self) -> AdapterHealth;

    fn generate(
        &self,
        request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError>;
}

#[cfg(test)]
mod tests {
    use super::{AdapterError, AdapterEvent, ErrorReason};

    #[test]
    fn error_has_kind_reason_and_message() {
        let err = AdapterError::failed(
            ErrorReason::StreamFailed,
            "The event stream dropped mid-turn.",
        );
        assert_eq!(err.kind(), "failed");
        assert_eq!(err.reason(), ErrorReason::StreamFailed);
        assert!(!err.message().is_empty());
    }

    #[test]
    fn unavailable_kind() {
        let err = AdapterError::unavailable(ErrorReason::BinaryMissing, "No opencode binary.");
        assert_eq!(err.kind(), "unavailable");
        assert_eq!(err.reason(), ErrorReason::BinaryMissing);
    }

    #[test]
    fn session_bound_is_a_plain_event() {
        let event = AdapterEvent::SessionBound {
            agent_session_id: "ses_1".into(),
        };
        assert_eq!(
            event,
            AdapterEvent::SessionBound {
                agent_session_id: "ses_1".into()
            }
        );
    }
}
