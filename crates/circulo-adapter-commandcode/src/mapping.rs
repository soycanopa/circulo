//! NDJSON frames from `cmd -p --output-format json` → `AdapterEvent`s.
//!
//! Two frame shapes:
//! - `{"type": "event", "event": {"type": "...", ...}}` — per-tool, per-chunk
//! - `{"type": "result", "subtype": "...", ...}` — the single final line
//!
//! Unknown event types are treated as `Ignored` (forward-compatible).

use circulo_adapter::{AdapterError, AdapterEvent, ErrorReason};
use circulo_core::{Task, TaskStatus, ToolCall, ToolCallStatus, ToolOutput};
use serde_json::Value;

#[derive(Debug)]
pub enum MappingOutcome {
    Emitted(AdapterEvent),
    Ignored,
    Failed(AdapterError),
}

pub fn map_ndjson_line(line: &str) -> MappingOutcome {
    let value: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return MappingOutcome::Ignored,
    };
    let frame_type = value.get("type").and_then(Value::as_str).unwrap_or("");
    match frame_type {
        "event" => map_event_frame(value.get("event").unwrap_or(&Value::Null)),
        "result" => map_result_frame(&value),
        _ => MappingOutcome::Ignored,
    }
}

fn map_event_frame(event: &Value) -> MappingOutcome {
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
    match event_type {
        "tool_running" | "tool_started" => MappingOutcome::Emitted(
            AdapterEvent::ToolCallStarted {
                tool_call: build_tool_call(event, ToolCallStatus::Running, None),
            },
        ),
        "tool_complete" | "tool_completed" | "tool_finished" => {
            let output = event
                .get("output")
                .or_else(|| event.get("result"))
                .cloned()
                .unwrap_or(Value::Null);
            MappingOutcome::Emitted(AdapterEvent::ToolCallUpdated {
                tool_call: build_tool_call(
                    event,
                    ToolCallStatus::Success,
                    Some(tool_output_from_value(output)),
                ),
            })
        }
        "tool_error" => MappingOutcome::Emitted(AdapterEvent::ToolCallUpdated {
            tool_call: build_tool_call(
                event,
                ToolCallStatus::Error,
                Some(ToolOutput::Error {
                    message: string_field(event, &["error", "message"])
                        .unwrap_or_else(|| "Tool error".to_string()),
                }),
            ),
        }),
        "text" | "text_delta" | "content" => {
            let content = string_field(event, &["content", "text", "delta"])
                .unwrap_or_default();
            if content.is_empty() {
                MappingOutcome::Ignored
            } else {
                MappingOutcome::Emitted(AdapterEvent::TextDelta { content })
            }
        }
        "session_title" | "title_updated" => {
            let title = string_field(event, &["title", "name"]).unwrap_or_default();
            if title.is_empty() {
                MappingOutcome::Ignored
            } else {
                MappingOutcome::Emitted(AdapterEvent::SessionTitleUpdated { title })
            }
        }
        "todo_updated" | "task_list_updated" => {
            let tasks = parse_tasks(event.get("tasks"));
            MappingOutcome::Emitted(AdapterEvent::TaskList { tasks })
        }
        _ => MappingOutcome::Ignored,
    }
}

fn map_result_frame(value: &Value) -> MappingOutcome {
    let subtype = value
        .get("subtype")
        .and_then(Value::as_str)
        .unwrap_or("");
    let session_id = value
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let final_text = value
        .get("finalText")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match subtype {
        "success" => {
            if let Some(sid) = session_id.as_deref() {
                return MappingOutcome::Emitted(AdapterEvent::SessionBound {
                    agent_session_id: sid.to_string(),
                });
            }
            if !final_text.is_empty() {
                return MappingOutcome::Emitted(AdapterEvent::TextDelta {
                    content: final_text,
                });
            }
            MappingOutcome::Emitted(AdapterEvent::Completed)
        }
        "error" => {
            let message = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Command Code reported an error.")
                .to_string();
            MappingOutcome::Failed(AdapterError::failed(
                ErrorReason::ProviderFailed,
                message,
            ))
        }
        "max_turns" => MappingOutcome::Failed(AdapterError::failed(
            ErrorReason::Cancelled,
            "Command Code hit the max turns limit.",
        )),
        _ => MappingOutcome::Ignored,
    }
}

fn build_tool_call(
    event: &Value,
    status: ToolCallStatus,
    output: Option<ToolOutput>,
) -> ToolCall {
    let id = string_field(event, &["toolCallId", "id", "tool_call_id"])
        .unwrap_or_else(|| "tool".to_string());
    let name = string_field(event, &["toolName", "name", "tool_name"])
        .unwrap_or_else(|| "tool".to_string());
    let input = event
        .get("input")
        .or_else(|| event.get("args"))
        .or_else(|| event.get("description"))
        .cloned()
        .unwrap_or(Value::Null);
    ToolCall {
        id,
        name,
        status,
        input,
        output,
        started_at: None,
        finished_at: None,
    }
}

fn tool_output_from_value(value: Value) -> ToolOutput {
    if value.is_string() {
        ToolOutput::Text {
            content: value.as_str().unwrap_or_default().to_string(),
        }
    } else if let Some(content) = value.get("content").and_then(Value::as_str) {
        ToolOutput::Text {
            content: content.to_string(),
        }
    } else if let Some(message) = value.get("error").and_then(Value::as_str) {
        ToolOutput::Error {
            message: message.to_string(),
        }
    } else {
        ToolOutput::Json { data: value }
    }
}

fn parse_tasks(value: Option<&Value>) -> Vec<Task> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            let id = string_field(item, &["id"]).unwrap_or_else(|| format!("task_{idx}"));
            let title = string_field(item, &["title", "name", "content"])
                .unwrap_or_else(|| "Task".to_string());
            let status = match string_field(item, &["status"]).as_deref() {
                Some("pending") => TaskStatus::Pending,
                Some("in_progress") | Some("in-progress") | Some("running") => {
                    TaskStatus::InProgress
                }
                Some("completed") | Some("done") | Some("success") => TaskStatus::Completed,
                Some("cancelled") | Some("canceled") => TaskStatus::Cancelled,
                _ => TaskStatus::Pending,
            };
            let order = item.get("order").and_then(Value::as_u64).unwrap_or(idx as u64) as u32;
            Some(Task {
                id,
                title,
                description: string_field(item, &["description"]),
                status,
                order,
            })
        })
        .collect()
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    for name in names {
        if let Some(s) = value.get(*name).and_then(Value::as_str) {
            return Some(s.to_string());
        }
    }
    None
}

pub fn map_exit_code(code: i32, stderr: &str) -> AdapterError {
    let message = if stderr.trim().is_empty() {
        format!("`cmd` exited with code {code}.")
    } else {
        format!("`cmd` exited with code {code}: {}", stderr.trim())
    };
    match code {
        3 => AdapterError::unavailable(
            ErrorReason::Unauthorized,
            "Sign in required. Run `cmd login` in your terminal.",
        ),
        5 => AdapterError::failed(ErrorReason::Timeout, message),
        6 | 7 => AdapterError::failed(ErrorReason::StreamFailed, message),
        8 => AdapterError::failed(
            ErrorReason::Cancelled,
            "`cmd` hit the max turns limit.",
        ),
        9 => AdapterError::failed(ErrorReason::ProviderFailed, message),
        10 => AdapterError::unavailable(
            ErrorReason::Unauthorized,
            "Insufficient Command Code credits.",
        ),
        130 => AdapterError::failed(ErrorReason::Cancelled, "Command interrupted."),
        _ => AdapterError::failed(ErrorReason::Internal, message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_frame_type_is_ignored() {
        let outcome = map_ndjson_line(r#"{"type":"something_else","foo":1}"#);
        assert!(matches!(outcome, MappingOutcome::Ignored));
    }

    #[test]
    fn malformed_json_is_ignored() {
        let outcome = map_ndjson_line("not json at all");
        assert!(matches!(outcome, MappingOutcome::Ignored));
    }

    #[test]
    fn tool_running_emits_started() {
        let outcome = map_ndjson_line(
            r#"{"type":"event","event":{"type":"tool_running","toolCallId":"t1","toolName":"read_file"}}"#,
        );
        match outcome {
            MappingOutcome::Emitted(AdapterEvent::ToolCallStarted { tool_call }) => {
                assert_eq!(tool_call.id, "t1");
                assert_eq!(tool_call.name, "read_file");
                assert_eq!(tool_call.status, ToolCallStatus::Running);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn tool_complete_emits_updated() {
        let outcome = map_ndjson_line(
            r#"{"type":"event","event":{"type":"tool_complete","toolCallId":"t1","toolName":"read_file","output":{"content":"hi"}}}"#,
        );
        match outcome {
            MappingOutcome::Emitted(AdapterEvent::ToolCallUpdated { tool_call }) => {
                assert_eq!(tool_call.status, ToolCallStatus::Success);
                assert!(matches!(tool_call.output, Some(ToolOutput::Text { .. })));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn text_event_emits_delta() {
        let outcome = map_ndjson_line(
            r#"{"type":"event","event":{"type":"text","content":"hello"}}"#,
        );
        match outcome {
            MappingOutcome::Emitted(AdapterEvent::TextDelta { content }) => {
                assert_eq!(content, "hello");
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn success_result_emits_session_bound() {
        let outcome = map_ndjson_line(
            r#"{"type":"result","subtype":"success","sessionId":"ses_xyz","finalText":"done"}"#,
        );
        match outcome {
            MappingOutcome::Emitted(AdapterEvent::SessionBound { agent_session_id }) => {
                assert_eq!(agent_session_id, "ses_xyz");
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn error_result_returns_failed() {
        let outcome = map_ndjson_line(
            r#"{"type":"result","subtype":"error","error":"boom"}"#,
        );
        assert!(matches!(outcome, MappingOutcome::Failed(_)));
    }

    #[test]
    fn max_turns_returns_failed() {
        let outcome = map_ndjson_line(r#"{"type":"result","subtype":"max_turns"}"#);
        assert!(matches!(outcome, MappingOutcome::Failed(_)));
    }

    #[test]
    fn exit_code_3_is_unauthorized() {
        let err = map_exit_code(3, "");
        assert_eq!(err.reason(), ErrorReason::Unauthorized);
    }

    #[test]
    fn exit_code_8_is_cancelled() {
        let err = map_exit_code(8, "");
        assert_eq!(err.reason(), ErrorReason::Cancelled);
    }

    #[test]
    fn exit_code_unknown_is_internal() {
        let err = map_exit_code(42, "");
        assert_eq!(err.reason(), ErrorReason::Internal);
    }
}
