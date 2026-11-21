//! The neutral wire contract between clients and the Circulo daemon.
//!
//! Everything a client learns about an agent arrives as a message defined here.
//! The desktop, and later mobile or remote clients, speak this and nothing else:
//! they never import a provider, never see a provider's native protocol, and
//! never touch the daemon's database or worktrees. That is why a change here
//! has to update both sides in the same change.
//!
//! The contract is transport-agnostic. [`frame`] implements the one transport
//! we ship today — loopback TCP, one JSON message per line — and remote will add
//! WebSocket + TLS behind the same types.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

mod frame;

pub use frame::{read_line, write_message};

/// Bumped whenever a message changes shape. Clients send it in the handshake
/// and the daemon refuses a mismatch instead of guessing.
pub const PROTOCOL_VERSION: u32 = 1;

/// The first line a client sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    /// Printed by the daemon at startup. Loopback-only today; a real credential
    /// for remote later.
    pub token: String,
}

impl Hello {
    pub fn encode(&self) -> Result<String, ProtocolError> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn decode(line: &str) -> Result<Self, ProtocolError> {
        Ok(serde_json::from_str(line)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRequest {
    /// Open a session, optionally attached to a project folder.
    StartSession { project: Option<String> },
    /// Send a prompt and stream the turn back.
    Prompt { session: String, text: String },
}

impl ClientRequest {
    /// Decodes one wire line. An unknown request type returns `Ok(None)` so a
    /// newer client cannot crash an older daemon.
    pub fn decode(line: &str) -> Result<Option<Self>, ProtocolError> {
        decode_known(line, CLIENT_TYPES)
    }
}

/// The message types this contract defines. A type outside its list is a
/// message from a newer peer and is skipped, not an error.
const CLIENT_TYPES: &[&str] = &["start_session", "prompt"];
const SERVER_TYPES: &[&str] = &[
    "welcome",
    "session_started",
    "turn_started",
    "part",
    "turn_finished",
    "error",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol: u32,
        /// The driver the daemon will use for the next turns.
        daemon: String,
    },
    SessionStarted {
        session: String,
    },
    TurnStarted {
        session: String,
        turn: u64,
    },
    Part {
        session: String,
        turn: u64,
        part: Part,
    },
    TurnFinished {
        session: String,
        turn: u64,
        reason: FinishReason,
    },
    Error {
        message: String,
    },
}

impl ServerMessage {
    /// Encodes the message as one line, without the trailing newline.
    pub fn encode(&self) -> Result<String, ProtocolError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Decodes one wire line. Unknown message types return `Ok(None)` so a
    /// newer daemon can add messages without breaking an older client.
    pub fn decode(line: &str) -> Result<Option<Self>, ProtocolError> {
        decode_known(line, SERVER_TYPES)
    }
}

/// One piece of a turn, in the order the driver produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Part {
    /// Streamed answer text. Deltas arrive in order and concatenate.
    Text { delta: String },
    /// Streamed reasoning, collapsed in the transcript by default.
    Reasoning { delta: String },
    /// A tool call and its state. Output arrives with the tool renderer.
    Tool {
        name: String,
        detail: String,
        state: ToolState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolState {
    Running,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Completed,
    Failed,
    Aborted,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("malformed message: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("message without a type")]
    MissingType,
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
}

/// Both enums are internally tagged, so the `type` field can be inspected
/// before committing to a shape. That is what makes unknown messages skip
/// cleanly instead of failing the whole stream.
fn decode_known<T: for<'de> Deserialize<'de>>(
    line: &str,
    known: &[&str],
) -> Result<Option<T>, ProtocolError> {
    let value: serde_json::Value = serde_json::from_str(line)?;
    let kind = value
        .get("type")
        .and_then(|kind| kind.as_str())
        .ok_or(ProtocolError::MissingType)?;

    if known.contains(&kind) {
        Ok(Some(serde_json::from_value(value)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_server_message_round_trips() {
        let messages = [
            ServerMessage::Welcome {
                protocol: PROTOCOL_VERSION,
                daemon: "demo".into(),
            },
            ServerMessage::SessionStarted {
                session: "s1".into(),
            },
            ServerMessage::TurnStarted {
                session: "s1".into(),
                turn: 7,
            },
            ServerMessage::Part {
                session: "s1".into(),
                turn: 7,
                part: Part::Text {
                    delta: "hello".into(),
                },
            },
            ServerMessage::Part {
                session: "s1".into(),
                turn: 7,
                part: Part::Tool {
                    name: "Read".into(),
                    detail: "crates/core/src/lib.rs".into(),
                    state: ToolState::Done,
                },
            },
            ServerMessage::TurnFinished {
                session: "s1".into(),
                turn: 7,
                reason: FinishReason::Completed,
            },
            ServerMessage::Error {
                message: "nope".into(),
            },
        ];

        for message in messages {
            let line = message.encode().expect("encode");
            assert!(!line.contains('\n'), "a message must stay on one line");
            let decoded = ServerMessage::decode(&line).expect("decode");
            assert_eq!(decoded, Some(message));
        }
    }

    #[test]
    fn every_request_round_trips() {
        let requests = [
            ClientRequest::StartSession {
                project: Some("/tmp/project".into()),
            },
            ClientRequest::Prompt {
                session: "s1".into(),
                text: "ship it".into(),
            },
        ];

        for request in requests {
            let line = serde_json::to_string(&request).expect("encode");
            assert_eq!(ClientRequest::decode(&line).expect("decode"), Some(request));
        }
    }

    #[test]
    fn unknown_types_are_skipped() {
        assert_eq!(
            ServerMessage::decode(r#"{"type":"future_thing","payload":1}"#).expect("decode"),
            None
        );
        assert_eq!(
            ClientRequest::decode(r#"{"type":"future_request"}"#).expect("decode"),
            None
        );
    }

    #[test]
    fn malformed_messages_are_errors() {
        assert!(ServerMessage::decode("{not json").is_err());
        assert!(ServerMessage::decode(r#"{"type":"part"}"#).is_err());
        assert!(matches!(
            ServerMessage::decode(r#"{"hello":"world"}"#),
            Err(ProtocolError::MissingType)
        ));
    }
}
