use std::path::{Path, PathBuf};
use std::sync::Arc;

use circulo_adapter::{
    AdapterError, AdapterEvent, AgentAdapter, ErrorReason, GenerateRequest, PermissionDecision,
    PermissionRequest, PermissionResponder,
};
use circulo_core::{
    ComposerInteractionMode, ComposerPermissionMode, Message, MessagePart, MessageRole,
    MessageStatus, OffsetDateTime, ToolCall, Uuid, is_default_session_title,
};
use circulo_i18n::Catalog;
use circulo_persist::Store;
use circulo_protocol::{ApiError, ProtocolEvent, API_VERSION};

fn default_working_directory() -> PathBuf {
    std::env::var_os("CIRCULO_OPENCODE_CWD")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn resolve_working_directory(store: &Store, session: &circulo_core::Session) -> PathBuf {
    let Some(project_id) = session.project_id else {
        return default_working_directory();
    };
    let Ok(Some(project)) = store.get_project(project_id) else {
        return default_working_directory();
    };
    let Some(folder_path) = project.folder_path else {
        return default_working_directory();
    };
    let path = Path::new(&folder_path);
    if path.is_dir() {
        path.to_path_buf()
    } else {
        eprintln!(
            "circulo-daemon: project folder missing at {folder_path}, using default cwd"
        );
        default_working_directory()
    }
}

pub fn run_turn(
    store: &Store,
    adapter: &dyn AgentAdapter,
    session_id: Uuid,
    user_text: &str,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    turn_registry: Option<(&crate::turn_registry::TurnRegistry, Uuid)>,
    permission_waiter: Option<&crate::permission_waiter::PermissionWaiter>,
    protocol_events: Option<&tokio::sync::broadcast::Sender<ProtocolEvent>>,
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

    let session = store
        .get_session(session_id)
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("Session not found."))?;

    let agent_session_id = store
        .opencode_session_id(session_id)
        .map_err(|_| ApiError::internal())?;

    let mut failed: Option<ApiError> = None;
    let working_directory = Some(resolve_working_directory(store, &session));
    let permission_responder =
        permission_responder_for(&session, session_id, permission_waiter, protocol_events);
    let gen_result = adapter.generate(
        GenerateRequest {
            session_id,
            user_text: user_text.to_owned(),
            agent_session_id,
            composer_model_id: session.composer_model_id.clone(),
            composer_model_variant: session.composer_model_variant.clone(),
            composer_permission_mode: session.composer_permission_mode,
            composer_interaction_mode: session
                .composer_interaction_mode
                .or(Some(ComposerInteractionMode::Build)),
            working_directory,
            cancel,
            permission_responder,
        },
        &mut |event| {
            if let AdapterEvent::SessionBound {
                agent_session_id: bound,
            } = &event
            {
                if let Some((registry, session_id)) = turn_registry {
                    registry.note_agent_session(session_id, bound.clone());
                }
                // Persist before any text streams so a crash mid-turn still
                // leaves the binding durable. Write-once; the same id is a
                // no-op, a different id is a real problem.
                if let Err(err) = store.bind_opencode_session(session_id, bound) {
                    eprintln!("circulo-daemon: session binding failed: {err}");
                    failed = Some(ApiError::internal());
                }
                return;
            }
            if let AdapterEvent::SessionTitleUpdated { title } = event {
                try_apply_auto_title(store, session_id, title, emit);
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
        AdapterEvent::ReasoningDelta { part_id, content } => {
            upsert_reasoning(assistant, part_id, content);
        }
        AdapterEvent::ReasoningOpaque { part_id } => {
            mark_reasoning_opaque(assistant, part_id);
        }
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
        AdapterEvent::SessionTitleUpdated { .. } => {}
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

fn upsert_reasoning(assistant: &mut Message, part_id: String, content: String) {
    if let Some(part) = assistant.parts.iter_mut().find_map(|part| match part {
        MessagePart::Reasoning { id, content: existing, .. } if *id == part_id => Some(existing),
        _ => None,
    }) {
        part.push_str(&content);
        return;
    }
    assistant.parts.push(MessagePart::Reasoning {
        id: part_id,
        content,
        visible: true,
    });
}

fn mark_reasoning_opaque(assistant: &mut Message, part_id: String) {
    if let Some(part) = assistant.parts.iter_mut().find_map(|part| match part {
        MessagePart::Reasoning { id, .. } if *id == part_id => Some(part),
        _ => None,
    }) {
        if let MessagePart::Reasoning { visible, .. } = part {
            *visible = false;
        }
        return;
    }
    assistant.parts.push(MessagePart::Reasoning {
        id: part_id,
        content: String::new(),
        visible: false,
    });
}

fn emit_for_assistant(session_id: Uuid, assistant: &Message, emit: &mut dyn FnMut(ProtocolEvent)) {
    emit(ProtocolEvent::SessionMessageUpdated {
        api_version: API_VERSION,
        session_id,
        message: assistant.clone(),
    });
}

fn try_apply_auto_title(
    store: &Store,
    session_id: Uuid,
    title: String,
    emit: &mut dyn FnMut(ProtocolEvent),
) {
    let Ok(Some(mut session)) = store.get_session(session_id) else {
        return;
    };
    if !is_default_session_title(&session.title) {
        return;
    }
    session.title = title.clone();
    session.updated_at = OffsetDateTime::now_utc();
    if store.update_session(&session).is_err() {
        return;
    }
    emit(ProtocolEvent::SessionTitleUpdated {
        api_version: API_VERSION,
        session_id,
        title,
    });
}

fn agent_error_text(error: &AdapterError) -> String {
    let catalog = Catalog::default_locale();
    let key = match error.reason() {
        ErrorReason::BinaryMissing => "opencode.error.binary_missing",
        ErrorReason::StartFailed => "opencode.error.start_failed",
        ErrorReason::PortOccupied => "opencode.error.port_occupied",
        ErrorReason::Unauthorized => "opencode.error.unauthorized",
        ErrorReason::StreamFailed => "opencode.error.stream_failed",
        ErrorReason::Timeout => "opencode.error.timeout",
        ErrorReason::Cancelled => "opencode.error.cancelled",
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

fn permission_responder_for(
    session: &circulo_core::Session,
    session_id: Uuid,
    permission_waiter: Option<&crate::permission_waiter::PermissionWaiter>,
    protocol_events: Option<&tokio::sync::broadcast::Sender<ProtocolEvent>>,
) -> Option<PermissionResponder> {
    let mode = session.composer_permission_mode?;
    if !matches!(
        mode,
        ComposerPermissionMode::Supervised | ComposerPermissionMode::AutoAcceptEdits
    ) {
        return None;
    }
    let permission_waiter = permission_waiter?.clone();
    let protocol_events = protocol_events?.clone();
    Some(PermissionResponder::new(move |request: PermissionRequest| {
        let _ = protocol_events.send(ProtocolEvent::SessionPermissionRequested {
            api_version: API_VERSION,
            session_id,
            permission_id: request.id.clone(),
            permission: request.permission.clone(),
            summary: request.summary.clone(),
        });
        if permission_waiter.wait(session_id, request.id) {
            PermissionDecision::AllowOnce
        } else {
            PermissionDecision::Deny
        }
    }))
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
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        };
        store.create_session(&session).unwrap();
        let adapter = BindingFakeAdapter {
            turns: AtomicUsize::new(0),
        };

        let mut events = Vec::new();
        let assistant = run_turn(
            &store,
            &adapter,
            session.id,
            "hi",
            None,
            None,
            None,
            None,
            &mut |event| events.push(event),
        )
        .unwrap();

        assert_eq!(assistant.status, MessageStatus::Complete);
        assert_eq!(
            store.opencode_session_id(session.id).unwrap().as_deref(),
            Some("ses_bound_1")
        );
        assert!(events
            .iter()
            .any(|event| matches!(event, ProtocolEvent::SessionMessageCompleted { .. })));

        let second = run_turn(
            &store,
            &adapter,
            session.id,
            "again",
            None,
            None,
            None,
            None,
            &mut |_| {},
        )
        .unwrap();
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
            ErrorReason::Cancelled,
            ErrorReason::ProviderFailed,
            ErrorReason::Internal,
        ] {
            let error = AdapterError::failed(reason, "raw message");
            let text = agent_error_text(&error);
            assert!(!text.is_empty(), "reason {reason:?} must resolve to copy");
            assert_ne!(text, "raw message", "text must come from the catalog");
        }
    }

    struct TitleFakeAdapter;

    impl AgentAdapter for TitleFakeAdapter {
        fn name(&self) -> &'static str {
            "title-fake"
        }

        fn probe(&self) -> AdapterHealth {
            AdapterHealth::Available
        }

        fn generate(
            &self,
            _request: GenerateRequest,
            emit: &mut dyn FnMut(AdapterEvent),
        ) -> Result<(), AdapterError> {
            emit(AdapterEvent::SessionTitleUpdated {
                title: "Launch checklist".into(),
            });
            emit(AdapterEvent::TextDelta {
                content: "Done.".into(),
            });
            emit(AdapterEvent::Completed);
            Ok(())
        }
    }

    #[test]
    fn auto_title_overwrites_only_default_titles() {
        let store = Store::open_in_memory().unwrap();
        let default_session = circulo_core::Session {
            id: Uuid::new_v4(),
            project_id: None,
            title: circulo_core::DEFAULT_SESSION_TITLE.into(),
            agent: circulo_core::AgentType::OpenCode,
            status: circulo_core::SessionStatus::Active,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            last_message_at: None,
            first_send_at: None,
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        };
        store.create_session(&default_session).unwrap();
        store
            .bind_opencode_session(default_session.id, "ses_bound_1")
            .unwrap();

        let mut events = Vec::new();
        run_turn(
            &store,
            &TitleFakeAdapter,
            default_session.id,
            "hi",
            None,
            None,
            None,
            None,
            &mut |event| events.push(event),
        )
        .unwrap();

        let updated = store.get_session(default_session.id).unwrap().unwrap();
        assert_eq!(updated.title, "Launch checklist");
        assert!(events.iter().any(|event| matches!(
            event,
            ProtocolEvent::SessionTitleUpdated {
                title,
                ..
            } if title == "Launch checklist"
        )));

        let renamed = circulo_core::Session {
            id: Uuid::new_v4(),
            project_id: None,
            title: "My custom title".into(),
            agent: circulo_core::AgentType::OpenCode,
            status: circulo_core::SessionStatus::Active,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            last_message_at: None,
            first_send_at: None,
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        };
        store.create_session(&renamed).unwrap();
        store
            .bind_opencode_session(renamed.id, "ses_bound_2")
            .unwrap();

        run_turn(
            &store,
            &TitleFakeAdapter,
            renamed.id,
            "hi",
            None,
            None,
            None,
            None,
            &mut |_| {},
        )
        .unwrap();

        let unchanged = store.get_session(renamed.id).unwrap().unwrap();
        assert_eq!(unchanged.title, "My custom title");
    }
}
