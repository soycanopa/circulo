//! The provider contract: one Driver trait, no provider knowledge.
//!
//! A provider crate (OpenCode, Claude Code, Codex, …) depends on this crate and
//! on `circulo-protocol`, and on nothing else of Circulo. It implements the
//! Driver trait and speaks only its own native protocol. The core depends on
//! this crate too, which is how it drives every agent without importing any of
//! them. Because the trait is the only thing both sides know, adding a provider
//! never changes the core or the UI.
//!
//! Turns are synchronous on purpose for now: the daemon gives each connection
//! its own thread, and a driver that streams parts through a callback is the
//! smallest honest shape. Async will come with a provider that needs it.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use circulo_protocol::{ConfigOption, Part, PermissionRequest, Question, QuestionAction, Usage};

/// The two sides of one conversation. The Circulo session is stable on the
/// wire and in storage; the provider's key for its own side (an ACP session
/// id, say) belongs to the provider alone — the core only carries it, never
/// reads it.
pub struct SessionRef<'a> {
    /// The Circulo session the turn belongs to.
    pub id: &'a str,
    /// The key the provider handed back on its last turn and the core stored,
    /// if any. A provider that remembers its sessions uses it to resume the
    /// same conversation after a restart.
    pub resume: Option<&'a str>,
    /// The folder this session's agent runs in: the project's canonical path,
    /// or `None` for a projectless session, where the provider falls back to
    /// its configured directory.
    pub cwd: Option<&'a str>,
    /// The files this turn carries, already copied into the core's blob store.
    /// A provider that cannot take files ignores them; the text still runs.
    pub attachments: &'a [Attachment],
}

impl<'a> SessionRef<'a> {
    /// A session with nothing to resume, no project folder and no files.
    pub fn new(id: &'a str) -> Self {
        Self {
            id,
            resume: None,
            cwd: None,
            attachments: &[],
        }
    }
}

/// One file the user attached, as the provider needs it: metadata plus the
/// stored blob the agent can read. The core owns the blob; a provider never
/// copies or deletes it.
pub struct Attachment {
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub path: PathBuf,
}

pub trait Driver: Send + Sync {
    /// The provider name shown in the UI, e.g. `opencode`.
    fn name(&self) -> &'static str;

    /// Runs one turn of `session` and emits its parts in order. Returning an
    /// error ends the turn as failed; parts already emitted stay emitted.
    ///
    /// `session.resume` carries the provider's own key from the previous turn,
    /// when one was stored. On success the driver returns the key for this
    /// session's next turn — `None` when the provider keeps no session of its
    /// own, or lost the one it had. The core stores it; nothing else reads it.
    ///
    /// A provider that needs the user before it acts calls `ask.permission`
    /// and blocks: the core sends the card and waits for the answer.
    fn start_turn(
        &self,
        session: SessionRef<'_>,
        prompt: &str,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<Option<String>, DriverError>;

    /// Stops the turn running for `session`, if any. Called from another
    /// thread while `start_turn` is blocked in the provider, so it must not
    /// wait for that turn to finish. A provider that can interrupt asks its
    /// agent to stop and lets `start_turn` return `DriverError::cancelled`;
    /// one that cannot says so, and the core reports the abort as failed.
    fn cancel(&self, session: &str) -> Result<(), DriverError> {
        let _ = session;
        Err(DriverError::new("this provider cannot cancel a turn"))
    }

    /// Drops any provider-side session for `session`: the next turn starts a
    /// fresh conversation. Used after a rewind, where the files and transcript
    /// came back but the provider has no rewind of its own.
    fn reset(&self, session: &str) {
        let _ = session;
    }

    /// The agent's current settings for `session`, with their current values.
    /// Starts no turn; a provider that needs an agent process starts a
    /// short-lived one. Empty when the provider has no settings.
    fn options(&self, session: &str) -> Result<Vec<ConfigOption>, DriverError>;
    /// Applies the user's choice for `session` and returns the refreshed list,
    /// as the agent reports it. The choice survives later turns: a provider
    /// whose agent forgets between processes re-applies it.
    fn set_option(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Vec<ConfigOption>, DriverError>;

    /// The latest usage the agent reported for `session`, if it reports any.
    /// The core asks after each turn and forwards it to the client; it is not
    /// stored with history. A provider without usage keeps the default.
    fn usage(&self, session: &str) -> Option<Usage> {
        let _ = session;
        None
    }

    /// The daemon is going away: release everything the provider keeps
    /// alive — its agent processes above all. Turns still in flight end on
    /// their own; no child process may outlive this call.
    fn shutdown(&self) {}
}

/// The user, as the provider sees it. The core implements this; a provider only
/// knows the question and the answer.
pub trait Asker {
    /// Blocks until the user answers or the client goes away; a closed channel
    /// reads as `Cancelled`.
    fn permission(&mut self, request: PermissionRequest) -> Decision;

    /// Blocks the same way. An accepted question carries one value per field.
    fn question(&mut self, question: Question) -> Answer;
}

/// What the user did with a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub action: QuestionAction,
    /// Keyed by field id. Empty unless the action is `Accept`.
    pub values: BTreeMap<String, String>,
}

impl Answer {
    pub fn accept(values: BTreeMap<String, String>) -> Self {
        Self {
            action: QuestionAction::Accept,
            values,
        }
    }

    pub fn decline() -> Self {
        Self {
            action: QuestionAction::Decline,
            values: BTreeMap::new(),
        }
    }

    pub fn cancel() -> Self {
        Self {
            action: QuestionAction::Cancel,
            values: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The provider's own option id, sent back verbatim.
    Selected(String),
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
#[error("the provider failed: {message}")]
pub struct DriverError {
    message: String,
    /// The provider stopped because the user asked it to. The core reports
    /// `FinishReason::Aborted` and shows no error.
    cancelled: bool,
}

impl DriverError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cancelled: false,
        }
    }

    /// The turn ended because the user cancelled it, not because it failed.
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cancelled: true,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}
