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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    Unavailable { message: String },
    Failed { message: String },
}

impl AdapterError {
    pub fn message(&self) -> &str {
        match self {
            Self::Unavailable { message } | Self::Failed { message } => message,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "unavailable",
            Self::Failed { .. } => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdapterEvent {
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
    use super::*;

    #[test]
    fn error_has_kind_and_message() {
        let err = AdapterError::Failed {
            message: "The agent stopped unexpectedly.".into(),
        };
        assert_eq!(err.kind(), "failed");
        assert!(!err.message().is_empty());
    }

    #[test]
    fn unavailable_kind() {
        let err = AdapterError::Unavailable {
            message: "OpenCode is not available.".into(),
        };
        assert_eq!(err.kind(), "unavailable");
    }
}
