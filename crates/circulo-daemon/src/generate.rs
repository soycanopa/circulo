use circulo_adapter::{AdapterEvent, AgentAdapter, GenerateRequest};
use circulo_core::{
    Message, MessagePart, MessageRole, MessageStatus, OffsetDateTime, ToolCall, Uuid,
};
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

    let mut failed: Option<ApiError> = None;
    let gen_result = adapter.generate(
        GenerateRequest {
            session_id,
            user_text: user_text.to_owned(),
        },
        &mut |event| {
            apply_event(&mut assistant, event, &mut failed);
            let _ = store.save_message(&assistant);
            emit_for_assistant(session_id, &assistant, emit);
        },
    );

    if let Err(err) = gen_result {
        failed = Some(ApiError::unavailable(err.message()));
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
        AdapterEvent::TextDelta { content } => match assistant.parts.last_mut() {
            Some(MessagePart::Text { content: existing }) => existing.push_str(&content),
            _ => assistant.parts.push(MessagePart::Text { content }),
        },
        AdapterEvent::TaskList { tasks } => {
            assistant.parts.push(MessagePart::TaskList { tasks });
        }
        AdapterEvent::ToolCallStarted { tool_call } | AdapterEvent::ToolCallUpdated { tool_call } => {
            upsert_tool_call(assistant, tool_call);
        }
        AdapterEvent::Completed => {}
        AdapterEvent::Failed { error } => {
            *failed = Some(ApiError::unavailable(error.message()));
        }
    }
}

fn upsert_tool_call(assistant: &mut Message, tool_call: ToolCall) {
    if let Some(part) = assistant.parts.iter_mut().find_map(|part| match part {
        MessagePart::ToolCall { tool_call: existing } if existing.id == tool_call.id => {
            Some(existing)
        }
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
