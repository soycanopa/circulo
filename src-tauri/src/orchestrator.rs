use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Notify};

use crate::state::{AgentCapabilitiesDto, AgentCommand, CirculoState, ConfigOptionDto, PooledAgent, ReserveSession, SessionInfoDto, SharedState};

pub const MAX_AGENT_POOL_SIZE: usize = 3;

pub fn pool_key(path: &Path) -> String {
    path.display().to_string()
}

pub fn touch_lru(state: &mut CirculoState, key: &str) {
    state.lru.retain(|entry| entry != &key);
    state.lru.push(key.to_string());
}

pub fn activate_path(state: &mut CirculoState, key: String) {
    touch_lru(state, &key);
    state.active_path = Some(key);
}

pub fn lru_eviction_candidate(state: &CirculoState, incoming_key: &str) -> Option<String> {
    if state.agents.len() < MAX_AGENT_POOL_SIZE {
        return None;
    }
    if state.agents.contains_key(incoming_key) {
        return None;
    }
    state
        .lru
        .iter()
        .find(|key| key.as_str() != incoming_key && key.as_str() != state.active_path.as_deref().unwrap_or(""))
        .cloned()
        .or_else(|| {
            state
                .lru
                .iter()
                .find(|key| key.as_str() != incoming_key)
                .cloned()
        })
}

pub async fn shutdown_agent_entry(state: &SharedState, key: &str) {
    let (cmd_tx, done) = {
        let mut guard = state.lock().await;
        let Some(agent) = guard.agents.remove(key) else {
            guard.lru.retain(|entry| entry != key);
            if guard.active_path.as_deref() == Some(key) {
                guard.active_path = None;
            }
            return;
        };

        guard.lru.retain(|entry| entry != key);
        if guard.active_path.as_deref() == Some(key) {
            guard.active_path = None;
        }

        if let Some(waiter) = guard.session_create_waiter.take() {
            let _ = waiter.send(Err("Agent disconnected".to_string()));
        }

        (agent.cmd_tx, agent.agent_done)
    };

    let _ = cmd_tx.send(AgentCommand::Shutdown).await;
    let _ = tokio::time::timeout(Duration::from_secs(10), done.notified()).await;
    tokio::time::sleep(Duration::from_millis(350)).await;
}

pub async fn shutdown_active_agent(state: &SharedState) {
    let key = state.lock().await.active_path.clone();
    if let Some(key) = key {
        shutdown_agent_entry(state, &key).await;
    }
}

pub async fn shutdown_all_agents(state: &SharedState) {
    let keys: Vec<String> = state.lock().await.agents.keys().cloned().collect();
    for key in keys {
        shutdown_agent_entry(state, &key).await;
    }
}

pub async fn ensure_pool_capacity(state: &SharedState, incoming_key: &str) {
    loop {
        let candidate = state.lock().await.lru_eviction_candidate(incoming_key);
        let Some(key) = candidate else {
            break;
        };
        shutdown_agent_entry(state, &key).await;
    }
}

pub fn insert_spawning_agent(
    state: &mut CirculoState,
    project_path: PathBuf,
    agent_id: String,
    cmd_tx: mpsc::Sender<AgentCommand>,
) {
    let key = pool_key(&project_path);
    state.agents.insert(
        key.clone(),
        PooledAgent {
            project_path,
            agent_id,
            session_id: "pending".to_string(),
            cmd_tx,
            config_options: Vec::new(),
            sessions: Vec::new(),
            agent_capabilities: AgentCapabilitiesDto {
                load_session: false,
                list_sessions: false,
                resume_session: false,
                close_session: false,
            },
            list_cursor: None,
            agent_done: Arc::new(Notify::new()),
            connected: false,
            reserve: None,
            reserve_in_flight: false,
        },
    );
    activate_path(state, key);
}

pub fn mark_agent_disconnected(state: &mut CirculoState, project_path: &Path) {
    let key = pool_key(project_path);
    state.agents.remove(&key);
    state.lru.retain(|entry| entry != &key);
    if state.active_path.as_deref() == Some(key.as_str()) {
        state.active_path = None;
    }
}

pub fn set_reserve(
    agent: &mut PooledAgent,
    session_id: String,
    config_options: Vec<ConfigOptionDto>,
    session_entry: SessionInfoDto,
) {
    agent.reserve = Some(ReserveSession {
        session_id,
        config_options,
        session_entry,
    });
    agent.reserve_in_flight = false;
}