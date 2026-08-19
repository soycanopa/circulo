//! Circulo adapter for Command Code headless mode.
//!
//! Command Code (`npm i -g command-code`) ships a `cmd` binary. Headless
//! mode runs a single query and emits the result as NDJSON on stdout.
//! The adapter spawns one subprocess per turn, parses each line as JSON,
//! and maps the frames to `AdapterEvent`s. Exit codes map to
//! `AdapterError`s; auth (3) becomes `Unavailable(Unauthorized)`.

mod discovery;
mod mapping;
mod subprocess;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use circulo_adapter::{
    AdapterError, AdapterEvent, AdapterHealth, AgentAdapter, AgentSessionSettings, ErrorReason,
    GenerateRequest, ModelCatalogEntry, OpenCodeHealth,
};

pub use discovery::discover_commandcode_binary;

pub struct CommandCodeAdapter {
    binary: PathBuf,
    /// Single live child. The daemon serializes turns via `TurnRegistry`,
    /// so we never have two concurrent children per adapter in practice.
    /// If we did, a new turn would replace the slot and the previous one
    /// would be leaked; this is acceptable for the MVP.
    current_child: Mutex<Option<Arc<Mutex<subprocess::ChildHandle>>>>,
}

impl CommandCodeAdapter {
    pub fn from_env() -> Option<Self> {
        discover_commandcode_binary().map(|binary| Self::new(binary))
    }

    pub fn with_binary(binary: PathBuf) -> Self {
        Self::new(binary)
    }

    fn new(binary: PathBuf) -> Self {
        Self {
            binary,
            current_child: Mutex::new(None),
        }
    }
}

impl AgentAdapter for CommandCodeAdapter {
    fn name(&self) -> &'static str {
        "commandcode"
    }

    fn probe(&self) -> AdapterHealth {
        match subprocess::probe(&self.binary) {
            subprocess::ProbeOutcome::Available => AdapterHealth::Available,
            subprocess::ProbeOutcome::AuthRequired => AdapterHealth::Error {
                message: "Sign in required. Run `cmd login` in your terminal.".into(),
            },
            subprocess::ProbeOutcome::Missing => AdapterHealth::Error {
                message: "Command Code binary not found.".into(),
            },
            subprocess::ProbeOutcome::Other(message) => AdapterHealth::Error { message },
        }
    }

    fn opencode_health(&self) -> Option<OpenCodeHealth> {
        None
    }

    fn generate(
        &self,
        request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError> {
        let mut started = subprocess::StartedTurn::start(self.binary.clone(), &request)?;
        let handle = started.handle.take();
        if let Some(handle) = handle.as_ref() {
            *self
                .current_child
                .lock()
                .map_err(|_| AdapterError::failed(ErrorReason::Internal, "child slot poisoned"))? =
                Some(Arc::clone(handle));
        }
        let result = started.drive(emit);
        *self
            .current_child
            .lock()
            .map_err(|_| AdapterError::failed(ErrorReason::Internal, "child slot poisoned"))? = None;
        result
    }

    fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        Ok(Vec::new())
    }

    fn sync_session_settings(
        &self,
        _agent_session_id: &str,
        _settings: &AgentSessionSettings,
    ) -> Result<(), AdapterError> {
        Ok(())
    }

    fn abort_turn(
        &self,
        _agent_session_id: &str,
        _working_directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let arc = self
            .current_child
            .lock()
            .map_err(|_| AdapterError::failed(ErrorReason::Internal, "child slot poisoned"))?
            .take();
        if let Some(arc) = arc {
            if let Ok(mut g) = arc.lock() {
                g.kill_wait();
            }
        }
        Ok(())
    }

    fn delete_agent_session(
        &self,
        _agent_session_id: &str,
        _working_directory: Option<&Path>,
    ) -> Result<(), AdapterError> {
        // Command Code sessions are persisted to disk by directory; there
        // is no remote handle to delete. Treating as a no-op keeps the
        // session-cleanup path symmetric across providers.
        Ok(())
    }
}

#[allow(dead_code)]
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
