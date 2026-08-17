//! OpenCode `AgentAdapter`.
//!
//! Owns a local `opencode serve` process (see `server`), drives one OpenCode
//! session per Circulo session, and streams normalized turn events from the
//! server's SSE bus (see `mapping` and `tests/fixtures/EVENTS.md`).
//!
//! Must not depend on `circulo-app`.

mod client;
mod mapping;
mod server;

#[cfg(feature = "test-support")]
pub mod testing;

pub use server::{ServerConfig, ServerManager, DEFAULT_OPENCODE_PORT};

use std::time::{Duration, Instant};

use circulo_adapter::{
    AdapterError, AdapterEvent, AdapterHealth, AgentAdapter, ErrorReason, GenerateRequest,
};

const DEFAULT_TURN_TIMEOUT: Duration = Duration::from_secs(120);

pub struct OpenCodeAdapter {
    servers: ServerManager,
    turn_timeout: Duration,
}

impl OpenCodeAdapter {
    pub fn from_env() -> Self {
        let turn_timeout = std::env::var("CIRCULO_OPENCODE_TURN_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_TURN_TIMEOUT);
        Self::new(ServerConfig::from_env(), turn_timeout)
    }

    pub fn new(config: ServerConfig, turn_timeout: Duration) -> Self {
        Self {
            servers: ServerManager::new(config),
            turn_timeout,
        }
    }
}

impl AgentAdapter for OpenCodeAdapter {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn probe(&self) -> AdapterHealth {
        match self.servers.ensure_running() {
            Ok(()) => AdapterHealth::Available,
            Err(err) if err.reason() == ErrorReason::BinaryMissing => AdapterHealth::Missing,
            Err(err) => AdapterHealth::Error {
                message: err.message().to_owned(),
            },
        }
    }

    fn generate(
        &self,
        request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError> {
        self.servers.ensure_running()?;
        let read_timeout = client::MAX_STREAM_READ_TIMEOUT.min(self.turn_timeout);
        let client = client::OpenCodeClient::with_read_timeout(
            self.servers.config().port,
            read_timeout,
        );

        let agent_session_id = match request.agent_session_id {
            Some(id) => id,
            None => {
                let id = client.create_session()?;
                emit(AdapterEvent::SessionBound {
                    agent_session_id: id.clone(),
                });
                id
            }
        };

        // Subscribe before prompting so early turn events are not missed.
        let mut stream = client.open_event_stream()?;
        client.prompt_async(&agent_session_id, &request.user_text)?;

        let deadline = Instant::now() + self.turn_timeout;
        let mut state = mapping::TurnState::default();
        loop {
            if Instant::now() >= deadline {
                let error = AdapterError::failed(
                    ErrorReason::Timeout,
                    "OpenCode did not finish the reply in time.",
                );
                emit(AdapterEvent::Failed {
                    error: error.clone(),
                });
                return Err(error);
            }
            let envelope = match stream.next_event() {
                Ok(envelope) => envelope,
                Err(error) => {
                    emit(AdapterEvent::Failed {
                        error: error.clone(),
                    });
                    return Err(error);
                }
            };
            match mapping::apply(&envelope, &agent_session_id, &mut state, emit) {
                mapping::TurnOutcome::Continue => {}
                mapping::TurnOutcome::Completed => {
                    emit(AdapterEvent::Completed);
                    return Ok(());
                }
                mapping::TurnOutcome::Failed { message, auth } => {
                    let error = mapping::failure_to_error(message, auth);
                    emit(AdapterEvent::Failed {
                        error: error.clone(),
                    });
                    return Err(error);
                }
            }
        }
    }
}
