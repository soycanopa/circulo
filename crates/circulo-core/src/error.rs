use serde::{Deserialize, Serialize};

/// Recoverable domain failure. Callers map this to protocol or UI errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainError {
    ProjectAssignmentLocked,
}

impl DomainError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ProjectAssignmentLocked => "project_assignment_locked",
        }
    }
}
