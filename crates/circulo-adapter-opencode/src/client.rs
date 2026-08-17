//! Sync HTTP + SSE client for the OpenCode server (wire shapes pinned in
//! `tests/fixtures/EVENTS.md`).

use std::io::{BufRead, BufReader};
use std::time::Duration;

use circulo_adapter::{AdapterError, ErrorReason};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Upper bound for per-read inactivity on the SSE stream; `generate` tightens
/// it to the remaining turn deadline.
pub const MAX_STREAM_READ_TIMEOUT: Duration = Duration::from_secs(90);

pub struct OpenCodeClient {
    base: String,
    http: ureq::Agent,
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

    pub fn create_session(&self) -> Result<String, AdapterError> {
        let response = self
            .http
            .post(format!("{}/session", self.base).as_str())
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

    pub fn prompt_async(
        &self,
        session_id: &str,
        user_text: &str,
        model: Option<(String, String)>,
        agent: Option<&str>,
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
        if let Some(agent) = agent {
            body["agent"] = serde_json::Value::String(agent.to_string());
        }
        let response = self
            .http
            .post(format!("{}/session/{session_id}/prompt_async", self.base).as_str())
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
    ) -> Result<(), AdapterError> {
        let response = self
            .http
            .request("PATCH", format!("{}/session/{session_id}", self.base).as_str())
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

    /// Opens the global SSE stream. Must be called before `prompt_async` so the
    /// first turn events are not missed.
    pub fn open_event_stream(&self) -> Result<EventStream, AdapterError> {
        let response = self
            .http
            .get(format!("{}/event", self.base).as_str())
            .call()
            .map_err(map_call_error)?;
        Ok(EventStream {
            reader: BufReader::new(response.into_reader()),
        })
    }
}

/// Yields the JSON envelope (`{"id", "type", "properties"}`) of each SSE frame.
pub struct EventStream {
    reader: BufReader<Box<dyn std::io::Read + Send + Sync + 'static>>,
}

impl EventStream {
    /// Blocks until the next `data:` frame. A closed stream is an error: clean
    /// turns end at the `session.idle` event, before the server closes.
    pub fn next_event(&mut self) -> Result<serde_json::Value, AdapterError> {
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
                Ok(envelope) => return Ok(envelope),
                Err(_) => continue,
            }
        }
    }
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
