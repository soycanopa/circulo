//! OpenCode `AgentAdapter`.
//!
//! Owns a local `opencode serve` process (see `server`), drives one OpenCode
//! session per Circulo session, and streams normalized turn events from the
//! server's SSE bus (see `mapping` and `tests/fixtures/EVENTS.md`).
//!
//! Must not depend on `circulo-app`.

mod client;
mod mapping;
mod models;
mod permission;
mod server;

#[cfg(feature = "test-support")]
pub mod testing;

pub use server::{ServerConfig, ServerManager, DEFAULT_OPENCODE_PORT};

use std::time::{Duration, Instant};
use std::sync::atomic::Ordering;

use circulo_adapter::{
    AdapterError, AdapterEvent, AdapterHealth, AgentAdapter, AgentSessionSettings,
    ErrorReason, GenerateRequest, ModelCatalogEntry, OpenCodeHealth,
};
use circulo_core::{split_model_catalog_id, ComposerInteractionMode};

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

    fn opencode_health(&self) -> Option<OpenCodeHealth> {
        if self.servers.ensure_running().is_err() {
            return Some(OpenCodeHealth {
                available: false,
                version: None,
            });
        }
        let client = client::OpenCodeClient::with_read_timeout(
            self.servers.config().port,
            client::REQUEST_TIMEOUT,
        );
        match client.global_health() {
            Ok(health) => Some(OpenCodeHealth {
                available: health.healthy,
                version: (!health.version.is_empty()).then_some(health.version),
            }),
            Err(_) => Some(OpenCodeHealth {
                available: false,
                version: None,
            }),
        }
    }

    fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        self.servers.ensure_running()?;
        let client = client::OpenCodeClient::with_read_timeout(
            self.servers.config().port,
            client::MAX_STREAM_READ_TIMEOUT,
        );
        let body = client.list_providers()?;
        Ok(models::parse_provider_catalog(&body))
    }

    fn sync_session_settings(
        &self,
        agent_session_id: &str,
        settings: &AgentSessionSettings,
    ) -> Result<(), AdapterError> {
        if let Some(mode) = settings.composer_permission_mode {
            self.servers.ensure_running()?;
            let client = client::OpenCodeClient::with_read_timeout(
                self.servers.config().port,
                client::REQUEST_TIMEOUT,
            );
            client.update_session_permission(
                agent_session_id,
                permission::ruleset_for(mode),
                None,
            )?;
        }
        Ok(())
    }

    fn generate(
        &self,
        request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError> {
        self.servers.ensure_running()?;
        let read_timeout = client::MAX_STREAM_READ_TIMEOUT.min(self.turn_timeout);
        let client =
            client::OpenCodeClient::with_read_timeout(self.servers.config().port, read_timeout);

        let directory = request.working_directory.as_deref();

        let agent_session_id = match request.agent_session_id {
            Some(id) => id,
            None => {
                let id = client.create_session(directory)?;
                emit(AdapterEvent::SessionBound {
                    agent_session_id: id.clone(),
                });
                id
            }
        };

        if let Some(mode) = request.composer_permission_mode {
            client.update_session_permission(
                &agent_session_id,
                permission::ruleset_for(mode),
                directory,
            )?;
        }

        let model = request
            .composer_model_id
            .as_deref()
            .and_then(split_model_catalog_id);
        let interaction = request
            .composer_interaction_mode
            .unwrap_or(ComposerInteractionMode::Build);
        let agent = interaction.agent_name();

        // Subscribe before prompting so early turn events are not missed.
        let mut stream = client.open_event_stream()?;
        client.prompt_async(
            &agent_session_id,
            &request.user_text,
            model,
            request.composer_model_variant.as_deref(),
            Some(agent),
            directory,
        )?;

        let deadline = Instant::now() + self.turn_timeout;
        let mut state = mapping::TurnState::default();
        let mut stream_reconnects = 0u8;
        loop {
            if request
                .cancel
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::SeqCst))
            {
                let _ = client.abort_session(&agent_session_id, directory);
                let error = user_cancelled();
                emit(AdapterEvent::Failed {
                    error: error.clone(),
                });
                return Err(error);
            }
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
                    if state.needs_todo_reconciliation() {
                        if let Ok(tasks) =
                            client.list_session_todos(&agent_session_id, directory)
                        {
                            if !tasks.is_empty() {
                                emit(AdapterEvent::TaskList {
                                    tasks: tasks.clone(),
                                });
                            }
                        }
                        state.mark_todo_reconciled();
                        if stream_reconnects == 0 {
                            stream_reconnects += 1;
                            match client.open_event_stream() {
                                Ok(replacement) => {
                                    stream = replacement;
                                    continue;
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    emit(AdapterEvent::Failed {
                        error: error.clone(),
                    });
                    return Err(error);
                }
            };
            match mapping::apply(&envelope, &agent_session_id, &mut state, emit) {
                mapping::TurnOutcome::Continue => {}
                mapping::TurnOutcome::PermissionRequired(permission) => {
                    let allow = request
                        .permission_responder
                        .as_ref()
                        .map(|responder| responder.respond(permission.clone()).is_allow())
                        .unwrap_or(false);
                    client.reply_permission(
                        &agent_session_id,
                        &permission.id,
                        allow,
                        directory,
                    )?;
                }
                mapping::TurnOutcome::Completed => {
                    if request
                        .cancel
                        .as_ref()
                        .is_some_and(|flag| flag.load(Ordering::SeqCst))
                    {
                        let error = user_cancelled();
                        emit(AdapterEvent::Failed {
                            error: error.clone(),
                        });
                        return Err(error);
                    }
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

    fn abort_turn(
        &self,
        agent_session_id: &str,
        working_directory: Option<&std::path::Path>,
    ) -> Result<(), AdapterError> {
        self.servers.ensure_running()?;
        let client = client::OpenCodeClient::with_read_timeout(
            self.servers.config().port,
            client::REQUEST_TIMEOUT,
        );
        client.abort_session(agent_session_id, working_directory)
    }

    fn delete_agent_session(
        &self,
        agent_session_id: &str,
        working_directory: Option<&std::path::Path>,
    ) -> Result<(), AdapterError> {
        self.servers.ensure_running()?;
        let client = client::OpenCodeClient::with_read_timeout(
            self.servers.config().port,
            client::REQUEST_TIMEOUT,
        );
        client.delete_session(agent_session_id, working_directory)
    }
}

fn user_cancelled() -> AdapterError {
    AdapterError::failed(ErrorReason::Cancelled, "The reply was stopped.")
}
