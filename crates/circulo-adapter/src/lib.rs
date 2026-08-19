//! Provider-agnostic adapter trait.
//!
//! Implementations live in sibling crates. This crate must not depend on GPUI
//! or on a specific agent CLI.

use std::path::PathBuf;

pub use circulo_core::{
    AgentType, ComposerInteractionMode, ComposerPermissionMode, ModelCatalogEntry, Task, TaskStatus,
    ToolCall, ToolCallStatus, ToolOutput, Uuid,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterHealth {
    Available,
    Missing,
    Error { message: String },
}

/// OpenCode-specific health from `GET /global/health`. Other adapters omit this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodeHealth {
    pub available: bool,
    pub version: Option<String>,
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
    Cancelled,
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
    SessionBound {
        agent_session_id: String,
    },
    TextDelta {
        content: String,
    },
    ReasoningDelta {
        part_id: String,
        content: String,
    },
    /// Reasoning part finished with no readable text (provider-encrypted or hidden).
    ReasoningOpaque {
        part_id: String,
    },
    TaskList {
        tasks: Vec<Task>,
    },
    ToolCallStarted {
        tool_call: ToolCall,
    },
    ToolCallUpdated {
        tool_call: ToolCall,
    },
    Completed,
    Failed {
        error: AdapterError,
    },
    SessionTitleUpdated {
        title: String,
    },
}

/// Structured option for an interactive agent question (OpenCode `question.asked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionOption {
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionAnswer {
    pub question_id: String,
    pub answers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionRequest {
    pub id: String,
    pub questions: Vec<UserQuestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestionResponse {
    pub answers: Vec<QuestionAnswer>,
}

/// Mid-turn permission prompt surfaced by the provider during supervised turns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub id: String,
    pub permission: String,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    AllowOnce,
    Deny,
}

impl PermissionDecision {
    pub fn is_allow(self) -> bool {
        matches!(self, Self::AllowOnce)
    }
}

/// Blocking callback invoked by the adapter while the turn waits for user input.
#[derive(Clone)]
pub struct PermissionResponder {
    inner: std::sync::Arc<dyn Fn(PermissionRequest) -> PermissionDecision + Send + Sync>,
}

impl PermissionResponder {
    pub fn new<F>(respond: F) -> Self
    where
        F: Fn(PermissionRequest) -> PermissionDecision + Send + Sync + 'static,
    {
        Self {
            inner: std::sync::Arc::new(respond),
        }
    }

    pub fn respond(&self, request: PermissionRequest) -> PermissionDecision {
        (self.inner)(request)
    }
}

impl std::fmt::Debug for PermissionResponder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermissionResponder").finish_non_exhaustive()
    }
}

/// Blocking callback invoked while the provider waits for structured question answers.
#[derive(Clone)]
pub struct QuestionResponder {
    inner: std::sync::Arc<dyn Fn(QuestionRequest) -> QuestionResponse + Send + Sync>,
}

impl QuestionResponder {
    pub fn new<F>(respond: F) -> Self
    where
        F: Fn(QuestionRequest) -> QuestionResponse + Send + Sync + 'static,
    {
        Self {
            inner: std::sync::Arc::new(respond),
        }
    }

    pub fn respond(&self, request: QuestionRequest) -> QuestionResponse {
        (self.inner)(request)
    }
}

impl std::fmt::Debug for QuestionResponder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuestionResponder").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub session_id: Uuid,
    pub user_text: String,
    /// Provider-side session binding persisted by the daemon, when one exists.
    pub agent_session_id: Option<String>,
    pub composer_model_id: Option<String>,
    pub composer_model_variant: Option<String>,
    pub composer_permission_mode: Option<ComposerPermissionMode>,
    pub composer_interaction_mode: Option<ComposerInteractionMode>,
    /// OpenCode `directory` query param for this turn (project folder or default cwd).
    pub working_directory: Option<PathBuf>,
    /// When set, a true value requests the turn to stop (user abort).
    pub cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// When set, mid-turn permission prompts block until the callback returns.
    pub permission_responder: Option<PermissionResponder>,
    /// When set, interactive question prompts block until the callback returns.
    pub question_responder: Option<QuestionResponder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionSettings {
    pub composer_permission_mode: Option<ComposerPermissionMode>,
}

pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    fn probe(&self) -> AdapterHealth;

    /// When implemented, surfaces OpenCode `{ healthy, version }` from
    /// `GET /global/health` after the managed server is reachable.
    fn opencode_health(&self) -> Option<OpenCodeHealth> {
        None
    }

    fn generate(
        &self,
        request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError>;

    fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        Ok(Vec::new())
    }

    fn sync_session_settings(
        &self,
        agent_session_id: &str,
        settings: &AgentSessionSettings,
    ) -> Result<(), AdapterError> {
        let _ = (agent_session_id, settings);
        Ok(())
    }

    fn abort_turn(
        &self,
        agent_session_id: &str,
        working_directory: Option<&std::path::Path>,
    ) -> Result<(), AdapterError> {
        let _ = (agent_session_id, working_directory);
        Ok(())
    }

    fn delete_agent_session(
        &self,
        agent_session_id: &str,
        working_directory: Option<&std::path::Path>,
    ) -> Result<(), AdapterError> {
        let _ = (agent_session_id, working_directory);
        Ok(())
    }
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
