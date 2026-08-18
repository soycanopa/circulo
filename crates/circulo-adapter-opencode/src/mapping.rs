//! Translates pinned OpenCode event envelopes into normalized `AdapterEvent`s.
//! The wire contract is documented in `tests/fixtures/EVENTS.md`; unknown event
//! or part types are skipped without failing the turn.

use std::collections::{HashMap, HashSet};

use circulo_adapter::{
    AdapterError, AdapterEvent, ErrorReason, PermissionRequest, QuestionOption, QuestionRequest,
    Task, TaskStatus, ToolCall, ToolCallStatus, ToolOutput, UserQuestion,
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
    /// Reasoning parts stream on the same `field: "text"` channel but must
    /// not leak into the assistant reply.
    reasoning_parts: HashSet<String>,
    seen_tools: HashSet<String>,
    last_todo_signature: Option<String>,
    /// User-side message ids: their text parts must not leak into the
    /// assistant turn (both arrive as `message.part.updated`).
    user_message_ids: HashSet<String>,
    /// Permission ids already forwarded to the responder this turn.
    handled_permissions: HashSet<String>,
    handled_questions: HashSet<String>,
    /// Reasoning parts already marked opaque this turn.
    opaque_reasoning: HashSet<String>,
    /// Todo list already reconciled from `GET /session/:id/todo` this turn.
    todo_reconciled: bool,
}

#[derive(Debug)]
pub enum TurnOutcome {
    Continue,
    Completed,
    PermissionRequired(PermissionRequest),
    QuestionRequired(QuestionRequest),
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
            let message_id = properties
                .get("messageID")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !message_id.is_empty() && state.user_message_ids.contains(message_id) {
                return TurnOutcome::Continue;
            }
            if let Some(delta) = properties.get("delta").and_then(Value::as_str) {
                if let Some(part_id) = properties.get("partID").and_then(Value::as_str) {
                    if state.reasoning_parts.contains(part_id) {
                        bump_offset(&mut state.part_offsets, part_id, delta.len());
                        if !delta.is_empty() {
                            emit(AdapterEvent::ReasoningDelta {
                                part_id: part_id.to_owned(),
                                content: delta.to_owned(),
                            });
                        }
                        return TurnOutcome::Continue;
                    }
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
            if !part_id.is_empty() && part_type == "reasoning" {
                state.reasoning_parts.insert(part_id.to_owned());
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
                        state
                            .part_offsets
                            .insert(part_id.to_owned(), snapshot.len());
                    }
                    TurnOutcome::Continue
                }
                "reasoning" => {
                    let snapshot = part.get("text").and_then(Value::as_str).unwrap_or("");
                    let offset = state.part_offsets.get(part_id).copied().unwrap_or(0);
                    if snapshot.len() > offset {
                        let suffix = snapshot[offset..].to_owned();
                        bump_offset(&mut state.part_offsets, part_id, snapshot.len());
                        if !suffix.is_empty() {
                            emit(AdapterEvent::ReasoningDelta {
                                part_id: part_id.to_owned(),
                                content: suffix,
                            });
                        }
                    } else if snapshot.len() < offset {
                        state
                            .part_offsets
                            .insert(part_id.to_owned(), snapshot.len());
                    }
                    if reasoning_part_is_opaque(part)
                        && state.opaque_reasoning.insert(part_id.to_owned())
                    {
                        emit(AdapterEvent::ReasoningOpaque {
                            part_id: part_id.to_owned(),
                        });
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
            let name = error
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("UnknownError");
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
                let name = error
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("UnknownError");
                let auth = name.contains("Auth");
                let message = human_provider_error(error);
                return TurnOutcome::Failed { message, auth };
            }
            TurnOutcome::Continue
        }
        "session.idle" => TurnOutcome::Completed,
        "permission.asked" | "permission.v2.asked" => {
            parse_permission_asked(properties, state)
        }
        "permission.updated" => parse_permission_updated(properties, state),
        "question.asked" | "question.v2.asked" => parse_question_asked(properties, state),
        "question.replied" | "question.rejected" | "question.v2.replied" | "question.v2.rejected" => {
            TurnOutcome::Continue
        }
        "session.updated" => {
            let info = properties.get("info").unwrap_or(&Value::Null);
            if let Some(title) = info.get("title").and_then(Value::as_str) {
                let trimmed = title.trim();
                if !trimmed.is_empty() {
                    emit(AdapterEvent::SessionTitleUpdated {
                        title: trimmed.to_owned(),
                    });
                }
            }
            TurnOutcome::Continue
        }
        _ => TurnOutcome::Continue,
    }
}

fn parse_permission_asked(properties: &Value, state: &mut TurnState) -> TurnOutcome {
    let Some(request) = permission_request_from_asked(properties) else {
        return TurnOutcome::Continue;
    };
    if !state.handled_permissions.insert(request.id.clone()) {
        return TurnOutcome::Continue;
    }
    TurnOutcome::PermissionRequired(request)
}

fn parse_question_asked(properties: &Value, state: &mut TurnState) -> TurnOutcome {
    let Some(request_id) = properties.get("id").and_then(Value::as_str) else {
        return TurnOutcome::Continue;
    };
    if !state.handled_questions.insert(request_id.to_owned()) {
        return TurnOutcome::Continue;
    }
    let questions = properties
        .get("questions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, question)| parse_user_question(index, question))
        .collect::<Vec<_>>();
    if questions.is_empty() {
        return TurnOutcome::Continue;
    }
    TurnOutcome::QuestionRequired(QuestionRequest {
        id: request_id.to_owned(),
        questions,
    })
}

fn parse_user_question(index: usize, question: &Value) -> Option<UserQuestion> {
    let text = question.get("question").and_then(Value::as_str)?.trim();
    if text.is_empty() {
        return None;
    }
    let header = question
        .get("header")
        .and_then(Value::as_str)
        .filter(|header| !header.trim().is_empty())
        .unwrap_or("Question");
    let slug = header
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('-');
    let id = if slug.is_empty() {
        format!("question-{index}")
    } else {
        format!("question-{index}-{slug}")
    };
    let options = question
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let label = option.get("label").and_then(Value::as_str)?.trim();
            if label.is_empty() {
                return None;
            }
            Some(QuestionOption {
                label: label.to_owned(),
                description: option
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|description| !description.is_empty())
                    .map(str::to_owned),
            })
        })
        .collect();
    Some(UserQuestion {
        id,
        header: header.to_owned(),
        question: text.to_owned(),
        options,
        multi_select: question
            .get("multiple")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_permission_updated(properties: &Value, state: &mut TurnState) -> TurnOutcome {
    let Some(request) = permission_request_from_updated(properties) else {
        return TurnOutcome::Continue;
    };
    if !state.handled_permissions.insert(request.id.clone()) {
        return TurnOutcome::Continue;
    }
    TurnOutcome::PermissionRequired(request)
}

fn permission_request_from_asked(properties: &Value) -> Option<PermissionRequest> {
    let id = properties
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())?;
    let permission = properties
        .get("permission")
        .and_then(Value::as_str)
        .unwrap_or("tool")
        .to_owned();
    let summary = permission_summary(
        &permission,
        properties.get("patterns"),
        properties.get("metadata"),
    );
    Some(PermissionRequest {
        id: id.to_owned(),
        permission,
        summary,
    })
}

fn permission_request_from_updated(properties: &Value) -> Option<PermissionRequest> {
    let id = properties
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())?;
    let permission = properties
        .get("type")
        .or_else(|| properties.get("permission"))
        .and_then(Value::as_str)
        .unwrap_or("tool")
        .to_owned();
    let summary = properties
        .get("title")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            properties
                .get("pattern")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(|pattern| format!("{permission}: {pattern}"))
                .unwrap_or_else(|| permission.clone())
        });
    Some(PermissionRequest {
        id: id.to_owned(),
        permission,
        summary,
    })
}

fn reasoning_part_is_opaque(part: &Value) -> bool {
    let completed = part
        .get("time")
        .and_then(|time| time.get("end"))
        .and_then(Value::as_i64)
        .is_some();
    if !completed {
        return false;
    }
    part.get("text")
        .and_then(Value::as_str)
        .is_none_or(|text| text.trim().is_empty())
}

fn permission_summary(permission: &str, patterns: Option<&Value>, metadata: Option<&Value>) -> String {
    if let Some(text) = metadata
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    {
        return text.to_owned();
    }
    if let Some(first) = patterns
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    {
        return format!("{permission}: {first}");
    }
    permission.to_owned()
}

pub fn failure_to_error(message: String, auth: bool) -> AdapterError {
    if auth {
        AdapterError::failed(ErrorReason::Unauthorized, message)
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
    let status_name = tool_state
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("");
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

impl TurnState {
    pub fn needs_todo_reconciliation(&self) -> bool {
        (!self.seen_tools.is_empty() || self.last_todo_signature.is_some()) && !self.todo_reconciled
    }

    pub fn mark_todo_reconciled(&mut self) {
        self.todo_reconciled = true;
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

pub fn todos_from_value(todos: &Value) -> Vec<Task> {
    if todos.is_array() {
        map_todos(todos)
    } else {
        map_todos(todos.get("todos").unwrap_or(todos))
    }
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
    let name = error
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("UnknownError");
    match error.get("message").and_then(Value::as_str) {
        Some(detail) if !detail.trim().is_empty() => format!("{name}: {detail}"),
        _ => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_adapter::AdapterEvent;
    use serde_json::json;

    #[test]
    fn permission_asked_maps_to_required_outcome() {
        let envelope = json!({
            "id": "evt_perm",
            "type": "permission.asked",
            "properties": {
                "sessionID": "ses_1",
                "id": "perm_1",
                "permission": "bash",
                "patterns": ["npm test"],
                "metadata": {},
                "always": []
            }
        });
        let mut state = TurnState::default();
        let mut emitted = Vec::new();
        let outcome = apply(&envelope, "ses_1", &mut state, &mut |event| emitted.push(event));
        match outcome {
            TurnOutcome::PermissionRequired(request) => {
                assert_eq!(request.id, "perm_1");
                assert_eq!(request.permission, "bash");
                assert_eq!(request.summary, "bash: npm test");
            }
            other => panic!("expected permission required, got {other:?}"),
        }
        assert!(emitted.is_empty());
    }

    #[test]
    fn duplicate_permission_events_are_ignored() {
        let envelope = json!({
            "id": "evt_perm",
            "type": "permission.asked",
            "properties": {
                "sessionID": "ses_1",
                "id": "perm_1",
                "permission": "edit",
                "patterns": ["README.md"],
                "metadata": {},
                "always": []
            }
        });
        let mut state = TurnState::default();
        assert!(matches!(
            apply(&envelope, "ses_1", &mut state, &mut |_| {}),
            TurnOutcome::PermissionRequired(_)
        ));
        assert!(matches!(
            apply(&envelope, "ses_1", &mut state, &mut |_| {}),
            TurnOutcome::Continue
        ));
    }

    #[test]
    fn completed_empty_reasoning_emits_opaque_once() {
        let envelope = json!({
            "id": "evt_reason",
            "type": "message.part.updated",
            "properties": {
                "sessionID": "ses_1",
                "part": {
                    "id": "prt_hidden",
                    "type": "reasoning",
                    "text": "",
                    "metadata": { "anthropic": { "signature": "sig_test" } },
                    "messageID": "msg_a",
                    "time": { "start": 1, "end": 2 }
                }
            }
        });
        let mut state = TurnState::default();
        let mut emitted = Vec::new();
        let outcome = apply(&envelope, "ses_1", &mut state, &mut |event| emitted.push(event));
        assert!(matches!(outcome, TurnOutcome::Continue));
        assert_eq!(emitted.len(), 1);
        assert!(matches!(
            &emitted[0],
            AdapterEvent::ReasoningOpaque {
                part_id,
            } if part_id == "prt_hidden"
        ));
        emitted.clear();
        apply(&envelope, "ses_1", &mut state, &mut |event| emitted.push(event));
        assert!(emitted.is_empty());
    }

    #[test]
    fn session_updated_emits_title_for_bound_session() {
        let envelope = json!({
            "id": "evt_title",
            "type": "session.updated",
            "properties": {
                "sessionID": "ses_1",
                "info": { "title": "Launch checklist" }
            }
        });
        let mut state = TurnState::default();
        let mut emitted = Vec::new();
        let outcome = apply(&envelope, "ses_1", &mut state, &mut |event| emitted.push(event));
        assert!(matches!(outcome, TurnOutcome::Continue));
        assert_eq!(emitted.len(), 1);
        assert!(matches!(
            &emitted[0],
            AdapterEvent::SessionTitleUpdated { title }
                if title == "Launch checklist"
        ));
    }

    #[test]
    fn session_updated_ignores_other_sessions() {
        let envelope = json!({
            "id": "evt_title",
            "type": "session.updated",
            "properties": {
                "sessionID": "ses_other",
                "info": { "title": "Wrong session" }
            }
        });
        let mut state = TurnState::default();
        let mut emitted = Vec::new();
        apply(&envelope, "ses_1", &mut state, &mut |event| emitted.push(event));
        assert!(emitted.is_empty());
    }

    #[test]
    fn todos_from_value_accepts_array_or_wrapper() {
        let array = json!([
            { "content": "Draft", "status": "completed", "priority": "medium" }
        ]);
        let wrapped = json!({ "todos": array });
        assert_eq!(todos_from_value(&array).len(), 1);
        assert_eq!(todos_from_value(&wrapped).len(), 1);
    }

    #[test]
    fn todo_reconciliation_needed_after_tool_or_todo_activity() {
        let mut state = TurnState::default();
        assert!(!state.needs_todo_reconciliation());
        state.seen_tools.insert("call_1".into());
        assert!(state.needs_todo_reconciliation());
        state.mark_todo_reconciled();
        assert!(!state.needs_todo_reconciliation());
    }

    #[test]
    fn question_asked_maps_to_required_outcome() {
        let envelope = json!({
            "id": "evt_q",
            "type": "question.asked",
            "properties": {
                "sessionID": "ses_1",
                "id": "question-request",
                "questions": [{
                    "header": "Files",
                    "question": "Which files should change?",
                    "multiple": true,
                    "options": [{
                        "label": "Source",
                        "description": "Core app files"
                    }]
                }]
            }
        });
        let mut state = TurnState::default();
        let outcome = apply(&envelope, "ses_1", &mut state, &mut |_| {});
        match outcome {
            TurnOutcome::QuestionRequired(request) => {
                assert_eq!(request.id, "question-request");
                assert_eq!(request.questions[0].id, "question-0-files");
                assert!(request.questions[0].multi_select);
                assert_eq!(request.questions[0].options[0].label, "Source");
            }
            other => panic!("expected question required, got {other:?}"),
        }
    }
}
