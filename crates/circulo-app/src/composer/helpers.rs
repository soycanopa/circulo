use circulo_core::{Message, MessagePart, MessageRole, Session};

pub fn project_picker_locked(session: Option<&Session>) -> bool {
    session.is_some_and(|session| session.first_send_at.is_some())
}

pub fn can_send(has_session: bool, draft: &str, generating: bool) -> bool {
    has_session && !draft.trim().is_empty() && !generating
}

/// Parse human context labels (`128K`, `200K`, `1M`) into token counts.
pub fn parse_token_limit_label(label: &str) -> Option<u64> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(num) = trimmed
        .strip_suffix('K')
        .or_else(|| trimmed.strip_suffix('k'))
    {
        num.trim().parse::<u64>().ok().map(|n| n * 1_000)
    } else if let Some(num) = trimmed
        .strip_suffix('M')
        .or_else(|| trimmed.strip_suffix('m'))
    {
        num.trim().parse::<u64>().ok().map(|n| n * 1_000_000)
    } else {
        trimmed.parse().ok()
    }
}

fn estimate_context_chars(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(|message| {
            message
                .parts
                .iter()
                .map(|part| match part {
                    MessagePart::Text { content } => content.len(),
                    MessagePart::ToolCall { tool_call } => {
                        tool_call.name.len() + tool_call.input.to_string().len()
                    }
                    MessagePart::TaskList { tasks } => {
                        tasks.iter().map(|task| task.title.len()).sum()
                    }
                    MessagePart::Question { question } => question.prompt.len(),
                })
                .sum::<usize>()
        })
        .sum()
}

/// Rough context usage for the ring (chars ÷ 4 vs model limit). Updates as messages grow.
pub fn context_usage_fraction(messages: &[Message], context_window: Option<&str>) -> f32 {
    let limit = context_window
        .and_then(parse_token_limit_label)
        .filter(|limit| *limit > 0);
    match limit {
        Some(limit) => {
            let chars = estimate_context_chars(messages);
            let estimated_tokens = chars as u64 / 4;
            (estimated_tokens as f32 / limit as f32).clamp(0.0, 1.0)
        }
        None => 0.0,
    }
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

    use super::{
        can_send, context_usage_fraction, parse_token_limit_label, project_picker_locked,
        summarize_message,
    };

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
            composer_model_variant: None,
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
    fn parses_token_limit_labels() {
        assert_eq!(parse_token_limit_label("128K"), Some(128_000));
        assert_eq!(parse_token_limit_label("1M"), Some(1_000_000));
        assert_eq!(parse_token_limit_label(""), None);
    }

    #[test]
    fn context_fraction_from_messages() {
        let message = Message {
            id: Uuid::nil(),
            session_id: Uuid::nil(),
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                content: "x".repeat(4000),
            }],
            status: MessageStatus::Complete,
            created_at: now(),
            is_streaming: false,
        };
        let fraction = context_usage_fraction(&[message], Some("128K"));
        assert!((fraction - 0.0078125).abs() < 0.0001);
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
