use circulo_core::{Message, MessagePart, MessageRole, Session};

pub fn project_picker_locked(session: Option<&Session>) -> bool {
    session.is_some_and(|session| session.first_send_at.is_some())
}

pub fn can_send(has_session: bool, draft: &str, generating: bool) -> bool {
    has_session && !draft.trim().is_empty() && !generating
}

pub fn summarize_message(message: &Message) -> String {
    let mut lines = Vec::new();
    for part in &message.parts {
        match part {
            MessagePart::Text { content } => lines.push(content.clone()),
            MessagePart::ToolCall { tool_call } => {
                lines.push(format!("{} ({:?})", tool_call.name, tool_call.status));
            }
            MessagePart::TaskList { tasks } => {
                for task in tasks {
                    lines.push(format!("• {}", task.title));
                }
            }
            MessagePart::Question { question } => lines.push(question.prompt.clone()),
        }
    }
    if lines.is_empty() {
        match message.role {
            MessageRole::User => String::new(),
            MessageRole::Assistant | MessageRole::System => String::new(),
        }
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use circulo_core::{
        AgentType, Message, MessagePart, MessageRole, MessageStatus, Session, SessionStatus,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::{can_send, project_picker_locked, summarize_message};

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
    }

    fn session(sent: bool) -> Session {
        Session {
            id: Uuid::nil(),
            project_id: None,
            title: "s".into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: now(),
            updated_at: now(),
            last_message_at: None,
            first_send_at: sent.then_some(now()),
            composer_model_id: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        }
    }

    #[test]
    fn picker_locks_after_first_send() {
        assert!(!project_picker_locked(Some(&session(false))));
        assert!(project_picker_locked(Some(&session(true))));
        assert!(!project_picker_locked(None));
    }

    #[test]
    fn send_requires_session_draft_and_idle() {
        assert!(!can_send(false, "hi", false));
        assert!(!can_send(true, "   ", false));
        assert!(!can_send(true, "hi", true));
        assert!(can_send(true, "hi", false));
    }

    #[test]
    fn summarize_joins_text() {
        let message = Message {
            id: Uuid::nil(),
            session_id: Uuid::nil(),
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                content: "Hello".into(),
            }],
            status: MessageStatus::Complete,
            created_at: now(),
            is_streaming: false,
        };
        assert_eq!(summarize_message(&message), "Hello");
    }
}
