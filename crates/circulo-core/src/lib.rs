//! Domain types shared by Circulo crates.

mod composer;
mod error;
mod message;
mod project;
mod session;

pub use composer::{
    ComposerInteractionMode, ComposerPermissionMode, ModelCatalogEntry, UserPreferences,
    model_catalog_id, model_provider_tag, split_model_catalog_id,
};
pub use error::DomainError;
pub use message::{
    Message, MessagePart, MessageRole, MessageStatus, Question, QuestionStatus, QuestionType, Task,
    TaskStatus, ToolCall, ToolCallStatus, ToolOutput,
};
pub use project::{Project, ProjectStatus};
pub use session::{AgentType, Session, SessionStatus};

pub use time::OffsetDateTime;
pub use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts")
    }

    fn session() -> Session {
        Session {
            id: Uuid::nil(),
            project_id: None,
            title: "New session".into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: now(),
            updated_at: now(),
            last_message_at: None,
            first_send_at: None,
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        }
    }

    #[test]
    fn new_session_serializes_null_project_id() {
        let json = serde_json::to_value(&session()).unwrap();
        assert_eq!(json["project_id"], serde_json::Value::Null);
        let back: Session = serde_json::from_value(json).unwrap();
        assert!(back.project_id.is_none());
    }

    #[test]
    fn assign_project_before_first_send() {
        let mut s = session();
        let project = Uuid::from_u128(1);
        s.assign_project(Some(project)).unwrap();
        assert_eq!(s.project_id, Some(project));
    }

    #[test]
    fn assign_project_after_first_send_is_rejected() {
        let mut s = session();
        s.mark_first_send(now());
        let err = s.assign_project(Some(Uuid::from_u128(1))).unwrap_err();
        assert_eq!(err, DomainError::ProjectAssignmentLocked);
        assert_eq!(err.code(), "project_assignment_locked");
        assert!(s.project_id.is_none());
    }

    #[test]
    fn archived_project_roundtrips() {
        let project = Project {
            id: Uuid::from_u128(2),
            name: "Launch".into(),
            description: None,
            color: None,
            status: ProjectStatus::Archived,
            created_at: now(),
            updated_at: now(),
        };
        let json = serde_json::to_value(&project).unwrap();
        assert_eq!(json["status"], "archived");
        assert_eq!(json["name"], "Launch");
        let back: Project = serde_json::from_value(json).unwrap();
        assert_eq!(back, project);
    }

    #[test]
    fn mixed_parts_roundtrip() {
        let message = Message {
            id: Uuid::from_u128(3),
            session_id: Uuid::from_u128(4),
            role: MessageRole::Assistant,
            status: MessageStatus::Complete,
            is_streaming: false,
            created_at: now(),
            parts: vec![
                MessagePart::Text {
                    content: "Here are the landing copy improvements:".into(),
                },
                MessagePart::TaskList {
                    tasks: vec![Task {
                        id: "task_1".into(),
                        title: "Rewrite the hero headline".into(),
                        description: None,
                        status: TaskStatus::Completed,
                        order: 0,
                    }],
                },
                MessagePart::ToolCall {
                    tool_call: ToolCall {
                        id: "tc_001".into(),
                        name: "edit_file".into(),
                        status: ToolCallStatus::Success,
                        input: json!({"path": "landing.md"}),
                        output: Some(ToolOutput::Diff {
                            file_path: "landing.md".into(),
                            old_content: None,
                            new_content: "new".into(),
                            diff: Some("--- a\n+++ b".into()),
                        }),
                        started_at: None,
                        finished_at: None,
                    },
                },
            ],
        };
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["parts"][0]["type"], "text");
        assert_eq!(json["parts"][1]["type"], "task_list");
        assert_eq!(json["parts"][2]["type"], "tool_call");
        let back: Message = serde_json::from_value(json).unwrap();
        assert_eq!(back, message);
    }

    #[test]
    fn tool_call_statuses_roundtrip() {
        for status in [
            ToolCallStatus::Pending,
            ToolCallStatus::Running,
            ToolCallStatus::Success,
            ToolCallStatus::Error,
        ] {
            let json = serde_json::to_value(status).unwrap();
            let back: ToolCallStatus = serde_json::from_value(json).unwrap();
            assert_eq!(back, status);
        }
    }
}
