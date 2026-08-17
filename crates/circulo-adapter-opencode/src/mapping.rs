//! Translates pinned OpenCode event envelopes into normalized `AdapterEvent`s.
//! The wire contract is documented in `tests/fixtures/EVENTS.md`; unknown event
//! or part types are skipped without failing the turn.

use std::collections::{HashMap, HashSet};

use circulo_adapter::{
    AdapterError, AdapterEvent, ErrorReason, Task, TaskStatus, ToolCall, ToolCallStatus,
    ToolOutput,
};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Default)]
pub struct TurnState {
    /// Emitted characters per text part id (deltas and snapshots overlap).
    part_offsets: HashMap<String, usize>,
    /// Part classification from the announcing snapshot: true = text part.
    /// Deltas carry no type, so only parts announced as text stream.
    text_parts: HashSet<String>,
    seen_tools: HashSet<String>,
    last_todo_signature: Option<String>,
    /// User-side message ids: their text parts must not leak into the
    /// assistant turn (both arrive as `message.part.updated`).
    user_message_ids: HashSet<String>,
}

pub enum TurnOutcome {
    Continue,
    Completed,
    Failed { message: String, auth: bool },
}

pub fn apply(
    envelope: &Value,
    agent_session_id: &str,
    state: &mut TurnState,
    emit: &mut dyn FnMut(AdapterEvent),
) -> TurnOutcome {
    let event_type = envelope.get("type").and_then(Value::as_str).unwrap_or("");
    let properties = envelope.get("properties").unwrap_or(&Value::Null);
    if let Some(session) = properties.get("sessionID").and_then(Value::as_str) {
        if session != agent_session_id {
            return TurnOutcome::Continue;
        }
    }

    match event_type {
        "message.part.delta" => {
            if properties.get("field").and_then(Value::as_str) != Some("text") {
                return TurnOutcome::Continue;
            }
            let message_id = properties.get("messageID").and_then(Value::as_str).unwrap_or("");
            if !message_id.is_empty() && state.user_message_ids.contains(message_id) {
                return TurnOutcome::Continue;
            }
            if let Some(delta) = properties.get("delta").and_then(Value::as_str) {
                if let Some(part_id) = properties.get("partID").and_then(Value::as_str) {
                    if !state.text_parts.contains(part_id) {
                        return TurnOutcome::Continue;
                    }
                    bump_offset(&mut state.part_offsets, part_id, delta.len());
                }
                if !delta.is_empty() {
                    emit(AdapterEvent::TextDelta {
                        content: delta.to_owned(),
                    });
                }
            }
            TurnOutcome::Continue
        }
        "message.part.updated" => {
            let part = properties.get("part").unwrap_or(&Value::Null);
            let message_id = part.get("messageID").and_then(Value::as_str).unwrap_or("");
            if !message_id.is_empty() && state.user_message_ids.contains(message_id) {
                return TurnOutcome::Continue;
            }
            let part_id = part.get("id").and_then(Value::as_str).unwrap_or("");
            let part_type = part.get("type").and_then(Value::as_str).unwrap_or("");
            // The first snapshot (possibly empty) announces the part and its
            // type; deltas that follow are only trustworthy for text parts.
            if !part_id.is_empty() && part_type == "text" {
                state.text_parts.insert(part_id.to_owned());
            }
            match part_type {
                "text" => {
                    let snapshot = part.get("text").and_then(Value::as_str).unwrap_or("");
                    let part_id = part.get("id").and_then(Value::as_str).unwrap_or("");
                    let offset = state.part_offsets.get(part_id).copied().unwrap_or(0);
                    if snapshot.len() > offset {
                        let suffix = snapshot[offset..].to_owned();
                        bump_offset(&mut state.part_offsets, part_id, snapshot.len());
                        emit(AdapterEvent::TextDelta { content: suffix });
                    } else if snapshot.len() < offset {
                        // Shrunken snapshot means our offsets are stale; resync.
                        state.part_offsets.insert(part_id.to_owned(), snapshot.len());
                    }
                    TurnOutcome::Continue
                }
                "tool" => {
                    let tool_call = map_tool_part(part);
                    let is_new = state.seen_tools.insert(tool_call.id.clone());
                    if is_new {
                        emit(AdapterEvent::ToolCallStarted {
                            tool_call: tool_call.clone(),
                        });
                    }
                    emit(AdapterEvent::ToolCallUpdated { tool_call });
                    TurnOutcome::Continue
                }
                _ => TurnOutcome::Continue,
            }
        }
        "todo.updated" => {
            let todos = properties.get("todos").unwrap_or(&Value::Null);
            let signature = todos.to_string();
            if state.last_todo_signature.as_deref() != Some(signature.as_str()) {
                state.last_todo_signature = Some(signature);
                let tasks = map_todos(todos);
                if !tasks.is_empty() {
                    emit(AdapterEvent::TaskList { tasks });
                }
            }
            TurnOutcome::Continue
        }
        "session.error" => {
            let error = properties.get("error").cloned().unwrap_or(Value::Null);
            let name = error.get("name").and_then(Value::as_str).unwrap_or("UnknownError");
            let auth = name.contains("Auth");
            let message = human_provider_error(&error);
            TurnOutcome::Failed { message, auth }
        }
        "message.updated" => {
            let info = properties.get("info").unwrap_or(&Value::Null);
            if info.get("role").and_then(Value::as_str) == Some("user") {
                if let Some(id) = info.get("id").and_then(Value::as_str) {
                    state.user_message_ids.insert(id.to_owned());
                }
            }
            if let Some(error) = info.get("error") {
                let name = error.get("name").and_then(Value::as_str).unwrap_or("UnknownError");
                let auth = name.contains("Auth");
                let message = human_provider_error(error);
                return TurnOutcome::Failed { message, auth };
            }
            TurnOutcome::Continue
        }
        "session.idle" => TurnOutcome::Completed,
        _ => TurnOutcome::Continue,
    }
}

pub fn failure_to_error(message: String, auth: bool) -> AdapterError {
    if auth {
        AdapterError::failed(
            ErrorReason::Unauthorized,
            message,
        )
    } else {
        AdapterError::failed(ErrorReason::ProviderFailed, message)
    }
}

fn bump_offset(offsets: &mut HashMap<String, usize>, part_id: &str, added: usize) {
    let entry = offsets.entry(part_id.to_owned()).or_insert(0);
    *entry += added;
}

fn map_tool_part(part: &Value) -> ToolCall {
    let id = part
        .get("callID")
        .and_then(Value::as_str)
        .or_else(|| part.get("id").and_then(Value::as_str))
        .unwrap_or("call_unknown")
        .to_owned();
    let tool_state = part.get("state").cloned().unwrap_or(Value::Null);
    let status_name = tool_state.get("status").and_then(Value::as_str).unwrap_or("");
    let (status, output) = match status_name {
        "pending" => (ToolCallStatus::Pending, None),
        "running" => (ToolCallStatus::Running, None),
        "completed" => (
            ToolCallStatus::Success,
            tool_state
                .get("output")
                .and_then(Value::as_str)
                .map(|text| ToolOutput::Text {
                    content: text.to_owned(),
                }),
        ),
        "error" => (
            ToolCallStatus::Error,
            tool_state
                .get("error")
                .and_then(Value::as_str)
                .map(|text| ToolOutput::Error {
                    message: text.to_owned(),
                }),
        ),
        _ => (ToolCallStatus::Pending, None),
    };
    ToolCall {
        id,
        name: part
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or("unknown_tool")
            .to_owned(),
        status,
        input: tool_state.get("input").cloned().unwrap_or(Value::Null),
        output,
        started_at: timestamp_ms(&tool_state, "time", "start"),
        finished_at: timestamp_ms(&tool_state, "time", "end"),
    }
}

fn map_todos(todos: &Value) -> Vec<Task> {
    todos
        .as_array()
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, todo)| {
                    let title = todo.get("content").and_then(Value::as_str)?;
                    let status = match todo.get("status").and_then(Value::as_str)? {
                        "pending" => TaskStatus::Pending,
                        "in_progress" => TaskStatus::InProgress,
                        "completed" => TaskStatus::Completed,
                        "cancelled" => TaskStatus::Cancelled,
                        _ => return None,
                    };
                    Some(Task {
                        id: format!("todo_{index}"),
                        title: title.to_owned(),
                        description: None,
                        status,
                        order: index as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn timestamp_ms(parent: &Value, time_field: &str, bound: &str) -> Option<OffsetDateTime> {
    let millis = parent
        .get(time_field)
        .and_then(|time| time.get(bound))
        .and_then(Value::as_i64)?;
    let base = OffsetDateTime::from_unix_timestamp(millis.div_euclid(1000)).ok()?;
    Some(base + time::Duration::milliseconds(millis.rem_euclid(1000)))
}

fn human_provider_error(error: &Value) -> String {
    let name = error.get("name").and_then(Value::as_str).unwrap_or("UnknownError");
    match error.get("message").and_then(Value::as_str) {
        Some(detail) if !detail.trim().is_empty() => format!("{name}: {detail}"),
        _ => name.to_owned(),
    }
}
