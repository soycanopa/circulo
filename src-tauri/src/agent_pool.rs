//! Background warm pool: keep enabled ACP agents initialized for the same cwd
//! so switching agents reuses a hot process instead of cold-starting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tracing::info;

use crate::acp::start_agent_connection;
use crate::state::{
    ActiveAgent, AgentCapabilitiesDto, AgentPoolKey, CirculoState, SharedState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSlot {
    /// Bound to the UI (`CirculoState::agent`).
    Active,
    /// Standby warm process (`CirculoState::warm_pool`).
    Pool,
}

pub fn pool_key(project_path: &Path, agent_id: &str) -> AgentPoolKey {
    AgentPoolKey {
        project_path: project_path.to_path_buf(),
        agent_id: agent_id.to_string(),
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Emit UI events for an agent that was promoted from the warm pool.
pub fn emit_agent_activation(app: &AppHandle, agent: &ActiveAgent) {
    let _ = app.emit(
        "agent:ready",
        serde_json::json!({
            "projectPath": agent.project_path.display().to_string(),
            "capabilities": agent.agent_capabilities,
            "connectionGeneration": agent.generation,
        }),
    );
    if let Some(session_id) = agent.resolve_interactive_session_id() {
        let config_options = agent.config_options();
        let _ = app.emit(
            "acp:config_options",
            serde_json::json!({
                "configOptions": config_options,
                "sessionId": session_id,
                "connectionGeneration": agent.generation,
            }),
        );
    }
}

/// Queue initialize + session prewarm for `agent_id` at `project_path`.
pub async fn spawn_agent(
    app: &AppHandle,
    state: &SharedState,
    project_path: PathBuf,
    agent_id: String,
    slot: AgentSlot,
) -> Result<u64, String> {
    let (cmd_tx, cmd_rx_external) = mpsc::channel(32);
    let (loop_tx, cmd_rx) = mpsc::channel(32);
    let agent_done = Arc::new(tokio::sync::Notify::new());

    info!(
        path = %project_path.display(),
        agent_id = %agent_id,
        slot = ?slot,
        "Spawning ACP agent process"
    );

    let generation = {
        let mut guard = state.lock().await;
        guard.next_generation = guard.next_generation.saturating_add(1);
        let generation = guard.next_generation;
        let agent = ActiveAgent {
            generation,
            project_path: project_path.clone(),
            agent_id: agent_id.clone(),
            agent_capabilities: AgentCapabilitiesDto::empty(),
            cmd_tx: cmd_tx.clone(),
            agent_done: agent_done.clone(),
            connected: false,
            sessions: HashMap::new(),
            visible_session_id: None,
        };
        match slot {
            AgentSlot::Active => guard.agent = Some(agent),
            AgentSlot::Pool => {
                let key = pool_key(&project_path, &agent_id);
                guard.warm_pool.insert(key, agent);
            }
        }
        generation
    };

    tauri::async_runtime::spawn(async move {
        let mut rx = cmd_rx_external;
        while let Some(cmd) = rx.recv().await {
            if loop_tx.send(cmd).await.is_err() {
                break;
            }
        }
    });

    let shared: SharedState = Arc::clone(state);
    let app_clone = app.clone();
    let path_clone = project_path.clone();

    tauri::async_runtime::spawn(async move {
        let result = start_agent_connection(
            app_clone.clone(),
            shared.clone(),
            path_clone,
            generation,
            cmd_rx,
        )
        .await;

        if let Err(err) = result {
            let guard = shared.lock().await;
            let is_current = guard.is_current_generation(generation);
            let still_known = guard.is_known_generation(generation);
            drop(guard);
            if !still_known {
                return;
            }
            if is_current {
                let _ = app_clone.emit(
                    "acp:error",
                    serde_json::json!({
                        "message": err,
                        "connectionGeneration": generation,
                    }),
                );
            }
            let mut guard = shared.lock().await;
            if let Some(agent) = guard.agent_for_generation_mut(generation) {
                agent.connected = false;
                agent.agent_done.notify_waiters();
            }
            drop(guard);
            if is_current {
                let _ = app_clone.emit(
                    "agent:disconnected",
                    serde_json::json!({ "connectionGeneration": generation }),
                );
            }
        }
    });

    Ok(generation)
}

/// Park the active agent into the warm pool (same cwd, different agent switch).
pub fn park_active_agent(guard: &mut CirculoState) -> Option<AgentPoolKey> {
    let current = guard.agent.take()?;
    let key = pool_key(&current.project_path, &current.agent_id);
    guard.warm_pool.insert(key.clone(), current);
    Some(key)
}

/// Promote a pooled agent to active. Returns `true` when the agent was already connected.
pub fn promote_from_pool(guard: &mut CirculoState, key: &AgentPoolKey) -> Option<bool> {
    let pooled = guard.warm_pool.remove(key)?;
    let connected = pooled.connected;
    guard.agent = Some(pooled);
    Some(connected)
}

pub fn find_pooled_agent<'a>(
    guard: &'a CirculoState,
    project_path: &Path,
    agent_id: &str,
) -> Option<&'a ActiveAgent> {
    let key = pool_key(project_path, agent_id);
    guard.warm_pool.get(&key)
}

pub fn is_active_match(
    guard: &CirculoState,
    project_path: &Path,
    agent_id: &str,
) -> bool {
    guard
        .agent
        .as_ref()
        .is_some_and(|a| paths_equal(&a.project_path, project_path) && a.agent_id == agent_id)
}

pub async fn warm_agent_in_pool(
    app: &AppHandle,
    state: &SharedState,
    project_path: PathBuf,
    agent_id: String,
) -> Result<(), String> {
    crate::agents::ensure_agent_available(&agent_id)?;

    {
        let guard = state.lock().await;
        if is_active_match(&guard, &project_path, &agent_id) {
            return Ok(());
        }
        let key = pool_key(&project_path, &agent_id);
        if guard.warm_pool.contains_key(&key) {
            return Ok(());
        }
    }

    spawn_agent(app, state, project_path, agent_id, AgentSlot::Pool).await?;
    Ok(())
}

pub async fn shutdown_pooled_agent(
    app: &AppHandle,
    state: &SharedState,
    key: &AgentPoolKey,
) {
    use crate::state::AgentCommand;
    use std::time::Duration;

    const SHUTDOWN_ACK_TIMEOUT: Duration = Duration::from_secs(3);

    let Some((cmd_tx, generation)) = ({
        let mut guard = state.lock().await;
        guard
            .warm_pool
            .remove(key)
            .map(|a| (a.cmd_tx, a.generation))
    }) else {
        return;
    };

    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();
    if cmd_tx
        .send(AgentCommand::Shutdown { ack: ack_tx })
        .await
        .is_ok()
    {
        let _ = tokio::time::timeout(SHUTDOWN_ACK_TIMEOUT, ack_rx).await;
    }
    let _ = app.emit(
        "agent:disconnected",
        serde_json::json!({ "connectionGeneration": generation }),
    );
}

pub async fn shutdown_all_pooled(app: &AppHandle, state: &SharedState) {
    let keys: Vec<AgentPoolKey> = {
        let guard = state.lock().await;
        guard.warm_pool.keys().cloned().collect()
    };
    for key in keys {
        shutdown_pooled_agent(app, state, &key).await;
    }
}

pub async fn shutdown_pool_for_path(app: &AppHandle, state: &SharedState, path: &Path) {
    let keys: Vec<AgentPoolKey> = {
        let guard = state.lock().await;
        guard
            .warm_pool
            .keys()
            .filter(|k| paths_equal(&k.project_path, path))
            .cloned()
            .collect()
    };
    for key in keys {
        shutdown_pooled_agent(app, state, &key).await;
    }
}
