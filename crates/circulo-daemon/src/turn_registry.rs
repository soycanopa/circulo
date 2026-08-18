use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use circulo_adapter::AgentAdapter;
use circulo_core::Uuid;
use circulo_protocol::ApiError;

struct ActiveTurn {
    cancel: Arc<AtomicBool>,
    agent_session_id: Mutex<Option<String>>,
    working_directory: Mutex<Option<PathBuf>>,
}

pub struct TurnRegistry {
    active: Mutex<HashMap<Uuid, ActiveTurn>>,
}

impl TurnRegistry {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn begin(
        &self,
        session_id: Uuid,
        working_directory: Option<PathBuf>,
    ) -> Result<Arc<AtomicBool>, ApiError> {
        let mut active = self.active.lock().map_err(|_| ApiError::internal())?;
        if active.contains_key(&session_id) {
            return Err(ApiError::invalid_request(
                "A reply is already in progress for this session.",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        active.insert(
            session_id,
            ActiveTurn {
                cancel: Arc::clone(&cancel),
                agent_session_id: Mutex::new(None),
                working_directory: Mutex::new(working_directory),
            },
        );
        Ok(cancel)
    }

    pub fn note_agent_session(&self, session_id: Uuid, agent_session_id: String) {
        let guard = self.active.lock().expect("turn registry lock");
        if let Some(turn) = guard.get(&session_id) {
            *turn.agent_session_id.lock().expect("agent id lock") = Some(agent_session_id);
        }
    }

    pub fn finish(&self, session_id: Uuid) {
        self.active
            .lock()
            .expect("turn registry lock")
            .remove(&session_id);
    }

    pub fn abort(&self, session_id: Uuid, adapter: &dyn AgentAdapter) -> Result<(), ApiError> {
        let guard = self.active.lock().map_err(|_| ApiError::internal())?;
        let Some(turn) = guard.get(&session_id) else {
            return Ok(());
        };
        turn.cancel.store(true, Ordering::SeqCst);
        let agent_session_id = turn.agent_session_id.lock().expect("agent id lock").clone();
        let working_directory = turn
            .working_directory
            .lock()
            .expect("working directory lock")
            .clone();
        drop(guard);
        if let Some(agent_session_id) = agent_session_id {
            if let Err(err) = adapter.abort_turn(&agent_session_id, working_directory.as_deref()) {
                eprintln!("circulo-daemon: opencode abort failed: {}", err.message());
            }
        }
        Ok(())
    }
}
