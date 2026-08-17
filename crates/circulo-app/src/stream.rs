//! Live session stream: SSE transport from the daemon plus a pure reducer
//! that folds protocol events into the transcript.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use circulo_core::{Message, MessageStatus};
use circulo_protocol::ProtocolEvent;

/// Blocking SSE reader over `GET /v1/sessions/{id}/events`.
///
/// The daemon sends `event:`/`data:` frames and a periodic keep-alive; only
/// `data:` payloads carry protocol events.
pub struct SessionEventStream {
    reader: BufReader<Box<dyn std::io::Read + Send + 'static>>,
}

impl SessionEventStream {
    pub fn new(body: Box<dyn std::io::Read + Send + 'static>) -> Self {
        Self {
            reader: BufReader::new(body),
        }
    }

    /// Blocks until the next protocol event. `Ok(None)` means the stream ended.
    /// Keep-alive comments, `event:` name lines, and unparsable payloads are
    /// skipped without failing the stream.
    pub fn next_event(&mut self) -> Result<Option<ProtocolEvent>, String> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .reader
                .read_line(&mut line)
                .map_err(|err| err.to_string())?;
            if read == 0 {
                return Ok(None);
            }
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            match parse_event(payload.trim()) {
                Ok(event) => return Ok(Some(event)),
                Err(_) => continue,
            }
        }
    }
}

pub fn parse_event(payload: &str) -> Result<ProtocolEvent, String> {
    serde_json::from_str(payload).map_err(|err| err.to_string())
}

/// Read timeout for the stream; comfortably above the daemon's 15 s keep-alive.
pub const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Reconnect attempts for a single outage before the shell surfaces an error.
pub const MAX_RESUBSCRIBE_ATTEMPTS: u32 = 3;

/// Backoff for attempt 1/2/3: 1s, 2s, 4s. `None` means the budget is spent.
pub fn resubscribe_delay(attempt: u32) -> Option<Duration> {
    if attempt == 0 || attempt > MAX_RESUBSCRIBE_ATTEMPTS {
        None
    } else {
        Some(Duration::from_secs(1u64 << (attempt - 1).min(2)))
    }
}

/// A completed SSE handshake starts a fresh outage budget.
pub fn stream_attempts_after_event(event: &ProtocolEvent, attempts: u32) -> u32 {
    if matches!(event, ProtocolEvent::ServerConnected { .. }) {
        0
    } else {
        attempts
    }
}

/// Apply a background refetch only if this session is still selected and the
/// subscription generation has not moved on (a newer stream's events win).
pub fn should_apply_refresh_transcript(
    selected_still_same: bool,
    snapshot_gen: u64,
    current_gen: u64,
) -> bool {
    selected_still_same && snapshot_gen == current_gen
}

/// The POST's `list_messages` is the consistency anchor: use it when the
/// stream never spoke, the live transcript still looks in-flight, or the
/// server has more messages than the local fold (e.g. assistant updates
/// arrived without a prior `SessionMessageCreated`).
pub fn should_apply_post_transcript(
    saw_stream_event: bool,
    local: &[Message],
    server: &[Message],
) -> bool {
    !saw_stream_event
        || local.iter().any(|message| message.is_streaming)
        || server.len() != local.len()
}

/// Folds one protocol event into the transcript. The daemon emits full message
/// snapshots, so updates replace by id. Returns whether anything changed.
pub fn apply_protocol_event(messages: &mut Vec<Message>, event: &ProtocolEvent) -> bool {
    match event {
        ProtocolEvent::SessionMessageCreated { message, .. } => {
            if messages.iter().any(|existing| existing.id == message.id) {
                false
            } else {
                messages.push(message.clone());
                true
            }
        }
        ProtocolEvent::SessionMessageUpdated { message, .. }
        | ProtocolEvent::SessionMessageCompleted { message, .. } => upsert_message(messages, message),
        ProtocolEvent::SessionMessageFailed {
            message_id, error, ..
        } => {
            let Some(existing) = messages
                .iter_mut()
                .find(|existing| existing.id == *message_id)
            else {
                return false;
            };
            existing.status = MessageStatus::Error;
            existing.is_streaming = false;
            existing.parts.push(circulo_core::MessagePart::Text {
                content: error.message.clone(),
            });
            true
        }
        // Part-level events are already folded into the message snapshots the
        // daemon emits alongside them; the handshake carries no transcript data.
        ProtocolEvent::SessionPartAppended { .. }
        | ProtocolEvent::SessionPartUpdated { .. }
        | ProtocolEvent::SessionToolCallUpdated { .. }
        | ProtocolEvent::ServerConnected { .. } => false,
    }
}

fn upsert_message(messages: &mut Vec<Message>, message: &Message) -> bool {
    match messages
        .iter_mut()
        .find(|existing| existing.id == message.id)
    {
        Some(existing) => {
            *existing = message.clone();
            true
        }
        None => {
            messages.push(message.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        apply_protocol_event, parse_event, resubscribe_delay, should_apply_post_transcript,
        should_apply_refresh_transcript, stream_attempts_after_event, SessionEventStream,
        MAX_RESUBSCRIBE_ATTEMPTS,
    };
    use circulo_core::{Message, MessagePart, MessageRole, MessageStatus, Uuid};
    use circulo_protocol::{ApiError, ProtocolEvent};
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts")
    }

    fn message(id: u128, streaming_text: &str) -> Message {
        Message {
            id: Uuid::from_u128(id),
            session_id: Uuid::nil(),
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                content: streaming_text.into(),
            }],
            status: MessageStatus::Streaming,
            created_at: now(),
            is_streaming: true,
        }
    }

    fn created(id: u128, text: &str) -> ProtocolEvent {
        ProtocolEvent::SessionMessageCreated {
            api_version: 1,
            session_id: Uuid::nil(),
            message: message(id, text),
        }
    }

    #[test]
    fn parses_frames_and_skips_noise() {
        let body = Box::new(
            ": keep-alive\n\
             event: server.connected\n\
             data: {\"type\":\"server.connected\",\"api_version\":1}\n\n\
             data: not-json-at-all\n\
             data: {\"type\":\"session.message.created\",\"api_version\":1,\"session_id\":\"00000000-0000-0000-0000-000000000000\",\"message\":{\"id\":\"00000000-0000-0000-0000-00000000000a\",\"session_id\":\"00000000-0000-0000-0000-000000000000\",\"role\":\"user\",\"parts\":[],\"status\":\"complete\",\"created_at\":\"2023-11-14T22:13:20Z\",\"is_streaming\":false}}\n\n"
                .as_bytes(),
        );
        let mut stream = SessionEventStream::new(body);
        assert!(matches!(
            stream.next_event().unwrap(),
            Some(ProtocolEvent::ServerConnected { .. })
        ));
        let second = stream.next_event().unwrap().expect("created event");
        assert!(matches!(
            second,
            ProtocolEvent::SessionMessageCreated { .. }
        ));
        assert_eq!(stream.next_event().unwrap(), None);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_event("nope").is_err());
    }

    #[test]
    fn reducer_appends_updates_and_completes() {
        let mut messages = Vec::new();
        assert!(apply_protocol_event(&mut messages, &created(1, "He")));
        let mut grown = message(1, "Hello world");
        grown.status = MessageStatus::Streaming;
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageUpdated {
                api_version: 1,
                session_id: Uuid::nil(),
                message: grown.clone(),
            }
        ));
        let mut done = grown;
        done.status = MessageStatus::Complete;
        done.is_streaming = false;
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageCompleted {
                api_version: 1,
                session_id: Uuid::nil(),
                message_id: done.id,
                message: done.clone(),
            }
        ));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].status, MessageStatus::Complete);
        assert!(!messages[0].is_streaming);
        assert_eq!(text_content(&messages[0].parts[0]), "Hello world");
    }

    #[test]
    fn reducer_marks_failures_with_error_copy() {
        let mut messages = vec![message(2, "Partial")];
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageFailed {
                api_version: 1,
                session_id: Uuid::nil(),
                message_id: Uuid::from_u128(2),
                error: ApiError::unavailable("It broke."),
            }
        ));
        assert_eq!(messages[0].status, MessageStatus::Error);
        assert!(!messages[0].is_streaming);
        let last = messages[0].parts.last().expect("error part");
        assert_eq!(text_content(last), "It broke.");
    }

    #[test]
    fn reducer_upserts_unknown_updates_and_ignores_duplicates() {
        let mut messages = Vec::new();
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageUpdated {
                api_version: 1,
                session_id: Uuid::nil(),
                message: message(7, "Ghost"),
            }
        ));
        assert!(apply_protocol_event(&mut messages, &created(3, "Hi")));
        assert!(!apply_protocol_event(&mut messages, &created(3, "Hi")));
        assert!(!apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageFailed {
                api_version: 1,
                session_id: Uuid::nil(),
                message_id: Uuid::from_u128(99),
                error: ApiError::internal(),
            }
        ));
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn live_turn_upserts_text_tools_and_tasks() {
        let mut messages = Vec::new();
        assert!(apply_protocol_event(&mut messages, &created(1, "He")));
        let mut mid = message(1, "Hello");
        mid.parts.push(MessagePart::ToolCall {
            tool_call: circulo_core::ToolCall {
                id: "t1".into(),
                name: "read_file".into(),
                status: circulo_core::ToolCallStatus::Running,
                input: serde_json::json!({"path": "a.md"}),
                output: None,
                started_at: Some(now()),
                finished_at: None,
            },
        });
        mid.parts.push(MessagePart::TaskList {
            tasks: vec![circulo_core::Task {
                id: "1".into(),
                title: "Draft".into(),
                description: None,
                status: circulo_core::TaskStatus::InProgress,
                order: 0,
            }],
        });
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageUpdated {
                api_version: 1,
                session_id: Uuid::nil(),
                message: mid.clone(),
            }
        ));
        assert_eq!(messages[0].parts.len(), 3);
        let mut done = mid;
        done.status = MessageStatus::Complete;
        done.is_streaming = false;
        assert!(apply_protocol_event(
            &mut messages,
            &ProtocolEvent::SessionMessageCompleted {
                api_version: 1,
                session_id: Uuid::nil(),
                message_id: done.id,
                message: done,
            }
        ));
        assert!(!messages[0].is_streaming);
        assert_eq!(messages[0].status, MessageStatus::Complete);
    }

    fn text_content(part: &MessagePart) -> &str {
        match part {
            MessagePart::Text { content } => content,
            _ => panic!("expected text part"),
        }
    }

    #[test]
    fn handshake_resets_reconnect_budget() {
        let connected = ProtocolEvent::ServerConnected { api_version: 1 };
        assert_eq!(stream_attempts_after_event(&connected, 3), 0);
        assert_eq!(stream_attempts_after_event(&created(1, "Hi"), 2), 2);
    }

    #[test]
    fn resubscribe_delay_is_per_outage() {
        assert_eq!(resubscribe_delay(1), Some(Duration::from_secs(1)));
        assert_eq!(resubscribe_delay(2), Some(Duration::from_secs(2)));
        assert_eq!(resubscribe_delay(3), Some(Duration::from_secs(4)));
        assert_eq!(resubscribe_delay(MAX_RESUBSCRIBE_ATTEMPTS + 1), None);
    }

    #[test]
    fn refresh_transcript_skips_when_generation_moved() {
        assert!(should_apply_refresh_transcript(true, 4, 4));
        assert!(!should_apply_refresh_transcript(true, 4, 5));
        assert!(!should_apply_refresh_transcript(false, 4, 4));
    }

    #[test]
    fn post_transcript_applies_when_stream_silent_or_still_open() {
        let user = message(1, "hi");
        let assistant = message(2, "partial");
        assert!(should_apply_post_transcript(false, &[], &[user.clone()]));
        assert!(should_apply_post_transcript(true, &[], &[user.clone(), assistant.clone()]));
        assert!(should_apply_post_transcript(true, &[assistant.clone()], &[user, assistant]));
        let mut done = message(1, "done");
        done.status = MessageStatus::Complete;
        done.is_streaming = false;
        assert!(!should_apply_post_transcript(true, &[done.clone()], &[done]));
    }
}
