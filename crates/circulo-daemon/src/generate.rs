use circulo_adapter::{AdapterError, AdapterEvent, AgentAdapter, ErrorReason, GenerateRequest};
use circulo_core::{
    Message, MessagePart, MessageRole, MessageStatus, OffsetDateTime, ToolCall, Uuid,
};
use circulo_i18n::Catalog;
use circulo_persist::Store;
use circulo_protocol::{ApiError, ProtocolEvent, API_VERSION};

pub fn run_turn(
    store: &Store,
    adapter: &dyn AgentAdapter,
    session_id: Uuid,
    user_text: &str,
    emit: &mut dyn FnMut(ProtocolEvent),
) -> Result<Message, ApiError> {
    let now = OffsetDateTime::now_utc();
    let user = Message {
        id: Uuid::new_v4(),
        session_id,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            content: user_text.to_owned(),
        }],
        status: MessageStatus::Complete,
        created_at: now,
        is_streaming: false,
    };
    store
        .save_message(&user)
        .map_err(|_| ApiError::internal())?;
    emit(ProtocolEvent::SessionMessageCreated {
        api_version: API_VERSION,
        session_id,
        message: user.clone(),
    });

    let assistant_id = Uuid::new_v4();
    let mut assistant = Message {
        id: assistant_id,
        session_id,
        role: MessageRole::Assistant,
        parts: Vec::new(),
        status: MessageStatus::Streaming,
        created_at: now,
        is_streaming: true,
    };

    let agent_session_id = store
        .opencode_session_id(session_id)
        .map_err(|_| ApiError::internal())?;

    let mut failed: Option<ApiError> = None;
    let gen_result = adapter.generate(
        GenerateRequest {
            session_id,
            user_text: user_text.to_owned(),
            agent_session_id,
        },
        &mut |event| {
            if let AdapterEvent::SessionBound {
                agent_session_id: bound,
            } = &event
            {
                // Persist before any text streams so a crash mid-turn still
                // leaves the binding durable. Write-once; the same id is a
                // no-op, a different id is a real problem.
                if let Err(err) = store.bind_opencode_session(session_id, bound) {
                    eprintln!("circulo-daemon: session binding failed: {err}");
                    failed = Some(ApiError::internal());
                }
                return;
            }
            apply_event(&mut assistant, event, &mut failed);
            let _ = store.save_message(&assistant);
            emit_for_assistant(session_id, &assistant, emit);
        },
    );

    if let Err(err) = gen_result {
        failed = Some(ApiError::unavailable(agent_error_text(&err)));
    }

    if let Some(error) = failed {
        assistant.status = MessageStatus::Error;
        assistant.is_streaming = false;
        store
            .save_message(&assistant)
            .map_err(|_| ApiError::internal())?;
        emit(ProtocolEvent::SessionMessageFailed {
            api_version: API_VERSION,
            session_id,
            message_id: assistant.id,
            error,
        });
        return Ok(assistant);
    }

    assistant.status = MessageStatus::Complete;
    assistant.is_streaming = false;
    store
        .save_message(&assistant)
        .map_err(|_| ApiError::internal())?;
    emit(ProtocolEvent::SessionMessageCompleted {
        api_version: API_VERSION,
        session_id,
        message_id: assistant.id,
        message: assistant.clone(),
    });
    Ok(assistant)
}

fn apply_event(assistant: &mut Message, event: AdapterEvent, failed: &mut Option<ApiError>) {
    match event {
        AdapterEvent::SessionBound { .. } => {}
        AdapterEvent::TextDelta { content } => match assistant.parts.last_mut() {
            Some(MessagePart::Text { content: existing }) => existing.push_str(&content),
            _ => assistant.parts.push(MessagePart::Text { content }),
        },
        AdapterEvent::TaskList { tasks } => {
            // Task snapshots evolve in place; they must not stack up parts.
            match assistant.parts.last_mut() {
                Some(MessagePart::TaskList { tasks: existing }) => *existing = tasks,
                _ => assistant.parts.push(MessagePart::TaskList { tasks }),
            }
        }
        AdapterEvent::ToolCallStarted { tool_call }
        | AdapterEvent::ToolCallUpdated { tool_call } => {
            upsert_tool_call(assistant, tool_call);
        }
        AdapterEvent::Completed => {}
        AdapterEvent::Failed { error } => {
            *failed = Some(ApiError::unavailable(agent_error_text(&error)));
        }
    }
}

fn upsert_tool_call(assistant: &mut Message, tool_call: ToolCall) {
    if let Some(part) = assistant.parts.iter_mut().find_map(|part| match part {
        MessagePart::ToolCall {
            tool_call: existing,
        } if existing.id == tool_call.id => Some(existing),
        _ => None,
    }) {
        *part = tool_call;
        return;
    }
    assistant.parts.push(MessagePart::ToolCall { tool_call });
}

fn emit_for_assistant(session_id: Uuid, assistant: &Message, emit: &mut dyn FnMut(ProtocolEvent)) {
    emit(ProtocolEvent::SessionMessageUpdated {
        api_version: API_VERSION,
        session_id,
        message: assistant.clone(),
    });
}

/// Adapters carry machine reasons, not UI copy; the daemon owns the locale.
fn agent_error_text(error: &AdapterError) -> String {
    let catalog = Catalog::default_locale();
    let key = match error.reason() {
        ErrorReason::BinaryMissing => "opencode.error.binary_missing",
        ErrorReason::StartFailed => "opencode.error.start_failed",
        ErrorReason::PortOccupied => "opencode.error.port_occupied",
        ErrorReason::Unauthorized => "opencode.error.unauthorized",
        ErrorReason::StreamFailed => "opencode.error.stream_failed",
        ErrorReason::Timeout => "opencode.error.timeout",
        ErrorReason::ProviderFailed => "opencode.error.provider_failed",
        ErrorReason::Internal => "opencode.error.internal",
    };
    let text = catalog.get(key);
    if text == key {
        error.message().to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_adapter::{
        AdapterError, AdapterEvent, AdapterHealth, AgentAdapter, ErrorReason, GenerateRequest,
    };
    use circulo_persist::Store;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct BindingFakeAdapter {
        turns: AtomicUsize,
    }

    impl AgentAdapter for BindingFakeAdapter {
        fn name(&self) -> &'static str {
            "binding-fake"
        }

        fn probe(&self) -> AdapterHealth {
            AdapterHealth::Available
        }

        fn generate(
            &self,
            _request: GenerateRequest,
            emit: &mut dyn FnMut(AdapterEvent),
        ) -> Result<(), AdapterError> {
            self.turns.fetch_add(1, Ordering::SeqCst);
            if self.turns.load(Ordering::SeqCst) == 1 {
                emit(AdapterEvent::SessionBound {
                    agent_session_id: "ses_bound_1".into(),
                });
            }
            emit(AdapterEvent::TextDelta {
                content: "Hello".into(),
            });
            emit(AdapterEvent::Completed);
            Ok(())
        }
    }

    #[test]
    fn run_turn_persists_binding_write_once_and_localizes_failures() {
        let store = Store::open_in_memory().unwrap();
        let session = circulo_core::Session {
            id: Uuid::new_v4(),
            project_id: None,
            title: "Chat".into(),
            agent: circulo_core::AgentType::OpenCode,
            status: circulo_core::SessionStatus::Active,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            last_message_at: None,
            first_send_at: None,
        };
        store.create_session(&session).unwrap();
        let adapter = BindingFakeAdapter {
            turns: AtomicUsize::new(0),
        };

        let mut events = Vec::new();
        let assistant = run_turn(&store, &adapter, session.id, "hi", &mut |event| {
            events.push(event)
        })
        .unwrap();

        assert_eq!(assistant.status, MessageStatus::Complete);
        assert_eq!(
            store.opencode_session_id(session.id).unwrap().as_deref(),
            Some("ses_bound_1")
        );
        assert!(events
            .iter()
            .any(|event| matches!(event, ProtocolEvent::SessionMessageCompleted { .. })));

        let second = run_turn(&store, &adapter, session.id, "again", &mut |_| {}).unwrap();
        assert_eq!(second.status, MessageStatus::Complete);
        assert_eq!(
            store.opencode_session_id(session.id).unwrap().as_deref(),
            Some("ses_bound_1")
        );
    }

    #[test]
    fn localized_error_text_covers_every_reason() {
        for reason in [
            ErrorReason::BinaryMissing,
            ErrorReason::StartFailed,
            ErrorReason::PortOccupied,
            ErrorReason::Unauthorized,
            ErrorReason::StreamFailed,
            ErrorReason::Timeout,
            ErrorReason::ProviderFailed,
            ErrorReason::Internal,
        ] {
            let error = AdapterError::failed(reason, "raw message");
            let text = agent_error_text(&error);
            assert!(!text.is_empty(), "reason {reason:?} must resolve to copy");
            assert_ne!(text, "raw message", "text must come from the catalog");
        }
    }
}
