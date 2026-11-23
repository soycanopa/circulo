//! The neutral wire contract between clients and the Circulo daemon.
//!
//! Everything a client learns about an agent arrives as a message defined here.
//! The desktop, and later mobile or remote clients, speak this and nothing else:
//! they never import a provider, never see a provider's native protocol, and
//! never touch the daemon's database or worktrees. That is why a change here
//! has to update both sides in the same change.
//!
//! The contract is transport-agnostic. The transport we ship is a WebSocket:
//! `ws://` on loopback today, `wss://` for remote. One JSON message per
//! WebSocket text frame. TLS and header authentication arrive with remote; the
//! messages below do not change.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// Answer a pending permission request. `option` is the id of one of the
    /// options the daemon offered; `None` means cancelled, which is only right
    /// when the turn itself is going away.
    AnswerPermission { id: String, option: Option<String> },
    /// Answer a pending question. `answers` is keyed by field id; an accepted
    /// question has one entry per field it asked for.
    AnswerQuestion {
        id: String,
        action: QuestionAction,
        answers: BTreeMap<String, String>,
    },
    /// The session list, newest first: what a sidebar draws.
    ListSessions,
    /// Continue a session the database already holds. The daemon answers with
    /// its history.
    OpenSession { session: String },
    /// Apply one of the agent's own settings. The daemon answers with
    /// `Options` carrying the refreshed list, or `Error` when the session is
    /// unknown or the agent refuses the choice.
    SetOption {
        session: String,
        id: String,
        value: String,
    },
}

impl ClientRequest {
    pub fn encode(&self) -> Result<String, ProtocolError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Decodes one frame. An unknown request type returns `Ok(None)` so a newer
    /// client cannot crash an older daemon.
    pub fn decode(line: &str) -> Result<Option<Self>, ProtocolError> {
        decode_known(line, CLIENT_TYPES)
    }
}

/// The message types this contract defines. A type outside its list is a
/// message from a newer peer and is skipped, not an error.
const CLIENT_TYPES: &[&str] = &[
    "start_session",
    "prompt",
    "answer_permission",
    "answer_question",
    "list_sessions",
    "open_session",
    "set_option",
];
const SERVER_TYPES: &[&str] = &[
    "welcome",
    "session_started",
    "turn_started",
    "part",
    "turn_finished",
    "permission_requested",
    "question_asked",
    "sessions",
    "history",
    "options",
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
    /// The agent is waiting for the user before it runs a tool. The turn stays
    /// open until `AnswerPermission` arrives.
    PermissionRequested {
        session: String,
        turn: u64,
        /// The daemon's id for this request, not the provider's.
        id: String,
        request: PermissionRequest,
    },
    /// The agent needs an answer, not an approval. This is ACP elicitation in
    /// form mode: a message and a flat set of fields.
    QuestionAsked {
        session: String,
        turn: u64,
        id: String,
        question: Question,
    },
    /// The session list, newest first.
    Sessions {
        sessions: Vec<SessionSummary>,
    },
    /// A stored transcript, so a client can draw a session again. Its parts are
    /// history, not a live turn: nothing is streaming.
    History {
        session: String,
        turns: Vec<HistoryTurn>,
    },
    /// The agent's current settings. The daemon pushes this after a session
    /// opens and after every accepted `SetOption`, so the pickers always show
    /// what the agent reports rather than what the client remembers.
    Options {
        session: String,
        options: Vec<ConfigOption>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub project: Option<String>,
    /// Empty until the first prompt arrives.
    pub title: String,
    /// Seconds since the Unix epoch, bumped by every turn.
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryTurn {
    pub turn: u64,
    pub prompt: String,
    pub parts: Vec<Part>,
}

/// What the agent is asking, in the agent's own words, plus the options the
/// agent offers. The card never invents an option of its own: the agent decides
/// the scope of an "always" answer and remembers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub title: String,
    /// The command, path or diff in full: the decision is about what will run.
    pub detail: String,
    pub options: Vec<PermissionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOption {
    /// Provider-owned id, sent back verbatim when this option is chosen.
    pub id: String,
    /// The agent's wording, shown as-is.
    pub label: String,
    pub kind: PermissionOptionKind,
}

/// One agent-owned setting, such as the model. The client never invents
/// values: `current` and every choice id come from the agent, and the daemon
/// sends the chosen id back verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigOption {
    /// Provider-owned id, sent back verbatim in `SetOption` (`"model"`).
    pub id: String,
    /// The agent's wording, shown as-is (`"Model"`).
    pub label: String,
    /// The current value's id, e.g. `"opencode/deepseek-v4.1-flash-free"`.
    pub current: String,
    pub options: Vec<ConfigChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigChoice {
    /// Provider-owned value id, sent back verbatim.
    pub id: String,
    /// The agent's wording, shown as-is. A model that arrives as
    /// `deepseek/DeepSeek V4 Pro` arrives here already stripped to
    /// `DeepSeek V4 Pro` by the driver, which knows its own naming.
    pub label: String,
    /// Who serves this choice (`"deepseek"`), from the agent's own id.
    /// Empty when the choice has no provider of its own. The client groups
    /// by it; it never parses ids to find out.
    pub provider: String,
}

/// What the agent wants to know. `message` is the agent's own wording; `fields`
/// is one entry per property of its form schema, in the order the agent wrote
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub message: String,
    pub fields: Vec<QuestionField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionField {
    /// The schema property name; the answer is keyed by this.
    pub id: String,
    /// Human-readable label, from the schema title or derived from the id.
    pub label: String,
    pub kind: QuestionKind,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuestionKind {
    /// One of a fixed set of answers.
    Choice { options: Vec<String> },
    /// Free text. A client that cannot type has to decline the question.
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAction {
    Accept,
    Decline,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    /// A kind this client does not know. It renders as an ordinary option and
    /// is never treated as an allow.
    #[serde(other)]
    Other,
}

impl ServerMessage {
    /// Encodes the message as one line, without the trailing newline.
    pub fn encode(&self) -> Result<String, ProtocolError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Decodes one frame. Unknown message types return `Ok(None)` so a newer
    /// daemon can add messages without breaking an older client.
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
    ///
    /// `id` is the provider's own call id, and it is what lets a later update
    /// find the row its call opened. Providers that do not have one send an
    /// empty string and the transcript falls back to matching by name.
    Tool {
        #[serde(default)]
        id: String,
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
                    id: "call-1".into(),
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
            ServerMessage::PermissionRequested {
                session: "s1".into(),
                turn: 7,
                id: "p-1".into(),
                request: PermissionRequest {
                    title: "Run cargo test".into(),
                    detail: "cargo test --workspace".into(),
                    options: vec![
                        PermissionOption {
                            id: "allow-once".into(),
                            label: "Allow once".into(),
                            kind: PermissionOptionKind::AllowOnce,
                        },
                        PermissionOption {
                            id: "reject-once".into(),
                            label: "Reject".into(),
                            kind: PermissionOptionKind::RejectOnce,
                        },
                    ],
                },
            },
            ServerMessage::QuestionAsked {
                session: "s1".into(),
                turn: 7,
                id: "q-1".into(),
                question: Question {
                    message: "How should I approach this?".into(),
                    fields: vec![QuestionField {
                        id: "strategy".into(),
                        label: "Strategy".into(),
                        kind: QuestionKind::Choice {
                            options: vec!["conservative".into(), "balanced".into()],
                        },
                        required: true,
                    }],
                },
            },
            ServerMessage::Sessions {
                sessions: vec![SessionSummary {
                    id: "s1".into(),
                    project: Some("/tmp/project".into()),
                    title: "Lay out the transcript".into(),
                    updated_at: 1_763_000_000,
                }],
            },
            ServerMessage::History {
                session: "s1".into(),
                turns: vec![HistoryTurn {
                    turn: 1,
                    prompt: "hello".into(),
                    parts: vec![Part::Text { delta: "hi".into() }],
                }],
            },
            ServerMessage::Options {
                session: "s1".into(),
                options: vec![ConfigOption {
                    id: "model".into(),
                    label: "Model".into(),
                    current: "b".into(),
                    options: vec![
                        ConfigChoice {
                            id: "a".into(),
                            label: "A".into(),
                            provider: "vendor-a".into(),
                        },
                        ConfigChoice {
                            id: "b".into(),
                            label: "B".into(),
                            provider: String::new(),
                        },
                    ],
                }],
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
            ClientRequest::SetOption {
                session: "s1".into(),
                id: "model".into(),
                value: "b".into(),
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
