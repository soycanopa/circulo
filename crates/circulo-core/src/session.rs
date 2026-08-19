use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DomainError;
use crate::{ComposerInteractionMode, ComposerPermissionMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenCode,
    CommandCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Archived,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub agent: AgentType,
    pub status: SessionStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_message_at: Option<OffsetDateTime>,
    /// Set when the first user message is sent. After that, `project_id` is locked.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub first_send_at: Option<OffsetDateTime>,
    /// Selected model catalog id (`providerID/modelID`).
    #[serde(default)]
    pub composer_model_id: Option<String>,
    /// OpenCode model variant (reasoning effort overlay), when the model supports variants.
    #[serde(default)]
    pub composer_model_variant: Option<String>,
    #[serde(default)]
    pub composer_permission_mode: Option<ComposerPermissionMode>,
    #[serde(default)]
    pub composer_interaction_mode: Option<ComposerInteractionMode>,
}

impl Session {
    pub fn can_change_project(&self) -> bool {
        self.first_send_at.is_none()
    }

    pub fn assign_project(&mut self, project_id: Option<Uuid>) -> Result<(), DomainError> {
        if !self.can_change_project() {
            return Err(DomainError::ProjectAssignmentLocked);
        }
        self.project_id = project_id;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn mark_first_send(&mut self, at: OffsetDateTime) {
        if self.first_send_at.is_none() {
            self.first_send_at = Some(at);
        }
        self.last_message_at = Some(at);
        self.updated_at = at;
    }
}

/// Title assigned when a session is created without an explicit name.
pub const DEFAULT_SESSION_TITLE: &str = "New session";

pub fn is_default_session_title(title: &str) -> bool {
    title.trim() == DEFAULT_SESSION_TITLE
}

#[cfg(test)]
mod tests {
    use super::{is_default_session_title, DEFAULT_SESSION_TITLE};

    #[test]
    fn default_session_title_matches_new_session_only() {
        assert!(is_default_session_title(DEFAULT_SESSION_TITLE));
        assert!(is_default_session_title(" New session "));
        assert!(!is_default_session_title("Launch checklist"));
    }
}
