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

use circulo_protocol::Part;

pub trait Driver: Send + Sync {
    /// The provider name shown in the UI, e.g. `opencode`.
    fn name(&self) -> &'static str;

    /// Runs one turn and emits its parts in order. Returning an error ends the
    /// turn as failed; parts already emitted stay emitted.
    fn start_turn(&self, prompt: &str, emit: &mut dyn FnMut(Part)) -> Result<(), DriverError>;
}

#[derive(Debug, thiserror::Error)]
#[error("the provider failed: {0}")]
pub struct DriverError(String);

impl DriverError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
