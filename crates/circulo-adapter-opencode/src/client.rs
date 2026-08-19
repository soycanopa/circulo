//! Sync HTTP + SSE client for the OpenCode server (wire shapes pinned in
//! `tests/fixtures/EVENTS.md`).

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

use circulo_adapter::{AdapterError, ErrorReason, Task};

use crate::mapping;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Default read timeout for non-turn HTTP calls that may block briefly.
pub const MAX_STREAM_READ_TIMEOUT: Duration = Duration::from_secs(90);

pub struct OpenCodeClient {
    base: String,
    http: ureq::Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalHealth {
    pub healthy: bool,
    pub version: String,
}

impl OpenCodeClient {
    pub fn with_read_timeout(port: u16, read_timeout: Duration) -> Self {
        let http = ureq::AgentBuilder::new()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(read_timeout)
            .build();
        Self {
            base: format!("http://127.0.0.1:{port}"),
            http,
        }
    }

    pub fn create_session(&self, directory: Option<&Path>) -> Result<String, AdapterError> {
        let url = session_url(&self.base, "/session", directory);
        let response = self
            .http
            .post(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .send_json(serde_json::json!({}))
            .map_err(map_call_error)?;
        let body: serde_json::Value = response.into_json().map_err(|err| {
            AdapterError::failed(
                ErrorReason::StreamFailed,
                format!("Unexpected response from OpenCode: {err}"),
            )
        })?;
        body.get("id")
            .and_then(|id| id.as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                AdapterError::failed(
                    ErrorReason::StreamFailed,
                    "OpenCode did not return a session id.",
                )
            })
    }

    pub fn global_health(&self) -> Result<GlobalHealth, AdapterError> {
        let response = self
            .http
            .get(format!("{}/global/health", self.base).as_str())
            .timeout(REQUEST_TIMEOUT)
            .call()
            .map_err(map_call_error)?;
        let body: serde_json::Value = response.into_json().map_err(|err| {
            AdapterError::failed(
                ErrorReason::StreamFailed,
                format!("Unexpected health response from OpenCode: {err}"),
            )
        })?;
        parse_global_health(&body)
    }

    pub fn list_session_todos(
        &self,
        session_id: &str,
        directory: Option<&Path>,
    ) -> Result<Vec<Task>, AdapterError> {
        let url = session_url(
            &self.base,
            &format!("/session/{session_id}/todo"),
            directory,
        );
        let response = self
            .http
            .get(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .call()
            .map_err(map_call_error)?;
        let body: serde_json::Value = response.into_json().map_err(|err| {
            AdapterError::failed(
                ErrorReason::StreamFailed,
                format!("Unexpected todo response from OpenCode: {err}"),
            )
        })?;
        Ok(mapping::todos_from_value(&body))
    }

    pub fn prompt_async(
        &self,
        session_id: &str,
        user_text: &str,
        model: Option<(String, String)>,
        variant: Option<&str>,
        agent: Option<&str>,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let mut body = serde_json::json!({
            "parts": [{ "type": "text", "text": user_text }],
        });
        if let Some((provider_id, model_id)) = model {
            body["model"] = serde_json::json!({
                "providerID": provider_id,
                "modelID": model_id,
            });
        }
        if let Some(variant) = variant {
            body["variant"] = serde_json::Value::String(variant.to_string());
        }
        if let Some(agent) = agent {
            body["agent"] = serde_json::Value::String(agent.to_string());
        }
        let url = session_url(
            &self.base,
            &format!("/session/{session_id}/prompt_async"),
            directory,
        );
        let response = self
            .http
            .post(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .send_json(body)
            .map_err(map_call_error)?;
        if response.status() == 204 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected the prompt with status {}.",
                    response.status()
                ),
            ))
        }
    }

    pub fn abort_session(
        &self,
        session_id: &str,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let url = session_url(
            &self.base,
            &format!("/session/{session_id}/abort"),
            directory,
        );
        let response = self
            .http
            .post(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .call()
            .map_err(map_call_error)?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected abort with status {}.",
                    response.status()
                ),
            ))
        }
    }

    pub fn delete_session(
        &self,
        session_id: &str,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let url = session_url(&self.base, &format!("/session/{session_id}"), directory);
        let response = self
            .http
            .request("DELETE", url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .call()
            .map_err(map_call_error)?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected session delete with status {}.",
                    response.status()
                ),
            ))
        }
    }

    pub fn list_providers(&self) -> Result<serde_json::Value, AdapterError> {
        let response = self
            .http
            .get(format!("{}/provider", self.base).as_str())
            .timeout(REQUEST_TIMEOUT)
            .call()
            .map_err(map_call_error)?;
        response.into_json().map_err(|err| {
            AdapterError::failed(
                ErrorReason::StreamFailed,
                format!("Unexpected provider list from OpenCode: {err}"),
            )
        })
    }

    pub fn update_session_permission(
        &self,
        session_id: &str,
        permission: Vec<serde_json::Value>,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let url = session_url(
            &self.base,
            &format!("/session/{session_id}"),
            directory,
        );
        let response = self
            .http
            .request("PATCH", url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .send_json(serde_json::json!({ "permission": permission }))
            .map_err(map_call_error)?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected the session update with status {}.",
                    response.status()
                ),
            ))
        }
    }

    pub fn reply_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        allow: bool,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let url = session_url(
            &self.base,
            &format!("/session/{session_id}/permissions/{permission_id}"),
            directory,
        );
        let response = self
            .http
            .post(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .send_json(serde_json::json!({
                "response": if allow { "once" } else { "reject" },
            }))
            .map_err(map_call_error)?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected the permission reply with status {}.",
                    response.status()
                ),
            ))
        }
    }

    pub fn reply_question(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
        directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let url = session_url(
            &self.base,
            &format!("/question/{request_id}/reply"),
            directory,
        );
        let response = self
            .http
            .post(url.as_str())
            .timeout(REQUEST_TIMEOUT)
            .send_json(serde_json::json!({ "answers": answers }))
            .map_err(map_call_error)?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(AdapterError::failed(
                ErrorReason::StreamFailed,
                format!(
                    "OpenCode rejected the question reply with status {}.",
                    response.status()
                ),
            ))
        }
    }

    /// Opens the SSE stream for turn events. Must be called before `prompt_async`
    /// so early frames are not missed. Project-scoped sessions require the same
    /// `directory` query param as prompts (OpenCode 1.18.18+).
    pub fn open_event_stream(&self, directory: Option<&Path>) -> Result<EventStream, AdapterError> {
        let url = session_url(&self.base, "/event", directory);
        let response = self
            .http
            .get(url.as_str())
            .call()
            .map_err(map_call_error)?;
        Ok(EventStream {
            reader: BufReader::new(response.into_reader()),
        })
    }
}

fn session_url(base: &str, path: &str, directory: Option<&Path>) -> String {
    let mut url = format!("{base}{path}");
    if let Some(directory) = directory {
        url.push('?');
        url.push_str("directory=");
        append_query_encoded(&mut url, &directory.to_string_lossy());
    }
    url
}

fn append_query_encoded(out: &mut String, value: &str) {
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(byte >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(byte & 0x0f) as usize]));
            }
        }
    }
}

/// Yields the JSON envelope (`{"id", "type", "properties"}`) of each SSE frame.
pub struct EventStream {
    reader: BufReader<Box<dyn std::io::Read + Send + Sync + 'static>>,
}

impl EventStream {
    /// Blocks until the next substantive `data:` frame. Liveness frames
    /// (`server.connected`, `server.heartbeat`) are consumed without being
    /// returned; `on_activity` runs for every frame so callers can extend an
    /// inactivity deadline while OpenCode is still alive.
    pub fn next_event_with_activity(
        &mut self,
        mut on_activity: impl FnMut(),
    ) -> Result<serde_json::Value, AdapterError> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self.reader.read_line(&mut line).map_err(map_stream_error)?;
            if read == 0 {
                return Err(AdapterError::failed(
                    ErrorReason::StreamFailed,
                    "The OpenCode event stream ended before the reply finished.",
                ));
            }
            let payload = match line.strip_prefix("data:") {
                Some(payload) => payload.trim(),
                None => continue,
            };
            match serde_json::from_str(payload) {
                Ok(envelope) if is_liveness_event(&envelope) => {
                    on_activity();
                    continue;
                }
                Ok(envelope) => {
                    on_activity();
                    return Ok(envelope);
                }
                Err(_) => continue,
            }
        }
    }
}

fn is_liveness_event(envelope: &serde_json::Value) -> bool {
    matches!(
        envelope.get("type").and_then(serde_json::Value::as_str),
        Some("server.connected" | "server.heartbeat")
    )
}

fn map_call_error(err: ureq::Error) -> AdapterError {
    match &err {
        ureq::Error::Status(401, _) => AdapterError::failed(
            ErrorReason::Unauthorized,
            "This OpenCode server requires credentials Circulo does not have.",
        ),
        ureq::Error::Status(status, _) => AdapterError::failed(
            ErrorReason::StreamFailed,
            format!("OpenCode answered with status {status}."),
        ),
        ureq::Error::Transport(transport) => {
            if matches!(transport.kind(), ureq::ErrorKind::Io)
                && transport
                    .message()
                    .map(|msg| msg.contains("timed out"))
                    .unwrap_or(false)
            {
                AdapterError::failed(ErrorReason::Timeout, "OpenCode was too slow to answer.")
            } else {
                AdapterError::unavailable(
                    ErrorReason::StreamFailed,
                    "Could not talk to the OpenCode server.",
                )
            }
        }
    }
}

fn map_stream_error(err: std::io::Error) -> AdapterError {
    match err.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => AdapterError::failed(
            ErrorReason::Timeout,
            "OpenCode stopped streaming before the reply finished.",
        ),
        _ => AdapterError::failed(
            ErrorReason::StreamFailed,
            "The OpenCode event stream failed mid-reply.",
        ),
    }
}

fn parse_global_health(body: &serde_json::Value) -> Result<GlobalHealth, AdapterError> {
    let healthy = body
        .get("healthy")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let version = body
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Ok(GlobalHealth { healthy, version })
}

#[cfg(test)]
mod tests {
    use super::{is_liveness_event, parse_global_health, GlobalHealth};
    use serde_json::json;

    #[test]
    fn parse_global_health_reads_healthy_and_version() {
        let health = parse_global_health(&json!({
            "healthy": true,
            "version": "1.18.18"
        }))
        .unwrap();
        assert_eq!(
            health,
            GlobalHealth {
                healthy: true,
                version: "1.18.18".into(),
            }
        );
    }

    #[test]
    fn parse_global_health_defaults_missing_fields() {
        let health = parse_global_health(&json!({})).unwrap();
        assert!(!health.healthy);
        assert!(health.version.is_empty());
    }

    #[test]
    fn liveness_events_are_recognized() {
        assert!(is_liveness_event(&json!({
            "type": "server.heartbeat",
            "properties": {}
        })));
        assert!(is_liveness_event(&json!({
            "type": "server.connected",
            "properties": {}
        })));
        assert!(!is_liveness_event(&json!({
            "type": "session.idle",
            "properties": {}
        })));
    }
}
