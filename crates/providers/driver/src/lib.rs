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

use circulo_protocol::{Part, PermissionRequest, Question, QuestionAction};

pub trait Driver: Send + Sync {
    /// The provider name shown in the UI, e.g. `opencode`.
    fn name(&self) -> &'static str;

    /// Runs one turn and emits its parts in order. Returning an error ends the
    /// turn as failed; parts already emitted stay emitted.
    ///
    /// A provider that needs the user before it acts calls `ask.permission`
    /// and blocks: the core sends the card and waits for the answer.
    fn start_turn(
        &self,
        prompt: &str,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<(), DriverError>;
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
#[error("the provider failed: {0}")]
pub struct DriverError(String);

impl DriverError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
