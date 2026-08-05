use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::v1::AgentCapabilities;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex, Notify};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionDto {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub current_value: String,
    pub options: Vec<ConfigOptionValueDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValueDto {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilitiesDto {
    pub load_session: bool,
    pub list_sessions: bool,
    pub resume_session: bool,
    pub close_session: bool,
    /// OpenCode (and most ACP agents) accept multiple sessions active per process.
    /// We surface this so the UI can keep N chats in flight concurrently.
    pub concurrent_sessions: bool,
    /// MCP transport support advertised in `initialize`. Stdio is mandatory per
    /// ACP, so it always reflects `true`; http/sse come from `mcp_capabilities`.
    pub mcp_stdio: bool,
    pub mcp_http: bool,
    pub mcp_sse: bool,
    /// Empirically observed: the agent delegated a `terminal/*` request to the
    /// client (Grok does, OpenCode runs bash internally). Starts `false`.
    pub terminal_delegation: bool,
}

impl AgentCapabilitiesDto {
    pub fn from_capabilities(caps: &AgentCapabilities) -> Self {
        Self {
            load_session: caps.load_session,
            list_sessions: caps.session_capabilities.list.is_some(),
            resume_session: caps.session_capabilities.resume.is_some(),
            close_session: caps.session_capabilities.close.is_some(),
            // Default to true: OpenCode and most ACP agents support multiple
            // concurrent sessions per process. Users can opt-out per agent later.
            concurrent_sessions: true,
            mcp_stdio: true,
            mcp_http: caps.mcp_capabilities.http,
            mcp_sse: caps.mcp_capabilities.sse,
            terminal_delegation: false,
        }
    }

    pub fn empty() -> Self {
        Self {
            load_session: false,
            list_sessions: false,
            resume_session: false,
            close_session: false,
            concurrent_sessions: true,
            mcp_stdio: true,
            mcp_http: false,
            mcp_sse: false,
            terminal_delegation: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub connected: bool,
    pub project_path: Option<String>,
    pub agent_id: Option<String>,
    pub connection_generation: Option<u64>,
    pub session_id: Option<String>,
    pub config_options: Vec<ConfigOptionDto>,
    pub capabilities: Option<AgentCapabilitiesDto>,
    pub agent_command: String,
}

pub enum AgentCommand {
    SendPrompt {
        session_id: String,
        text: String,
        context_files: Vec<ContextFile>,
    },
    SetConfigOption {
        session_id: String,
        config_id: String,
        value: String,
    },
    CreateSession {
        done: oneshot::Sender<Result<(), String>>,
    },
    LoadSession {
        session_id: String,
        done: oneshot::Sender<Result<(), String>>,
    },
    CloseSession {
        session_id: String,
        done: oneshot::Sender<Result<(), String>>,
    },
    CancelPrompt {
        session_id: String,
    },
    SetVisibleSession {
        session_id: Option<String>,
        done: oneshot::Sender<Result<(), String>>,
    },
    Shutdown {
        ack: oneshot::Sender<()>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

pub struct ActiveAgent {
    pub generation: u64,
    pub project_path: PathBuf,
    pub agent_id: String,
    pub agent_capabilities: AgentCapabilitiesDto,
    pub cmd_tx: mpsc::Sender<AgentCommand>,
    /// Notify fired when the agent process finishes `initialize` (single per process).
    pub agent_done: Arc<Notify>,
    pub connected: bool,
    /// All ACP sessions this agent process knows about, keyed by `session_id`.
    pub sessions: HashMap<String, SessionHandle>,
    /// The session currently bound to the UI (one at a time). May be `None` while the
    /// UI is presenting history or no chat is open.
    pub visible_session_id: Option<String>,
}

/// Per-session state on the agent process. Multiple of these can run concurrently
/// once the agent advertises the `concurrent_sessions` capability.
pub struct SessionHandle {
    pub session_id: String,
    pub session_ready_for_ui: bool,
    pub prompt_in_flight: bool,
    /// True after the user sends at least one prompt in this session.
    pub user_prompt_sent: bool,
    pub config_options: Vec<ConfigOptionDto>,
}

impl ActiveAgent {
    /// Compatibility accessor: the visible session's id, or empty string.
    pub fn session_id(&self) -> &str {
        self.visible_session_id
            .as_deref()
            .unwrap_or("")
    }

    pub fn session_ready_for_ui(&self) -> bool {
        self.visible_session_id
            .as_ref()
            .and_then(|sid| self.sessions.get(sid))
            .is_some_and(|s| s.session_ready_for_ui)
    }

    pub fn config_options(&self) -> Vec<ConfigOptionDto> {
        if let Some(visible) = &self.visible_session_id {
            if let Some(session) = self.sessions.get(visible) {
                return session.config_options.clone();
            }
        }
        // Draft prewarm session (before auto-publish event reaches the UI).
        self.sessions
            .iter()
            .find(|(_, handle)| !handle.session_ready_for_ui)
            .map(|(_, handle)| handle.config_options.clone())
            .unwrap_or_default()
    }

    /// Visible session, or a hidden prewarm session for config/prompt bootstrap.
    pub fn resolve_interactive_session_id(&self) -> Option<String> {
        if let Some(visible) = &self.visible_session_id {
            if self.sessions.contains_key(visible) {
                return Some(visible.clone());
            }
        }
        self.sessions
            .iter()
            .find(|(_, handle)| !handle.session_ready_for_ui)
            .map(|(sid, _)| sid.clone())
    }

    pub fn visible_session_ready(&self) -> bool {
        self.visible_session_id
            .as_ref()
            .and_then(|sid| self.sessions.get(sid))
            .is_some_and(|s| s.session_ready_for_ui)
    }
}

pub use agent_client_protocol::schema::v1::PermissionOptionId;
pub struct PermissionWaiter {
    pub tx: oneshot::Sender<String>,
    pub allowed_option_ids: Vec<PermissionOptionId>,
    pub session_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentPoolKey {
    pub project_path: PathBuf,
    pub agent_id: String,
}

impl Hash for AgentPoolKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.project_path.to_string_lossy().hash(state);
        self.agent_id.hash(state);
    }
}

pub struct CirculoState {
    pub agent: Option<ActiveAgent>,
    /// Standby warm processes keyed by (cwd, agent_id).
    pub warm_pool: HashMap<AgentPoolKey, ActiveAgent>,
    pub next_generation: u64,
    pub permission_waiters: HashMap<String, PermissionWaiter>,
}

impl CirculoState {
    pub fn new() -> Self {
        Self {
            agent: None,
            warm_pool: HashMap::new(),
            next_generation: 0,
            permission_waiters: HashMap::new(),
        }
    }

    pub fn status(&self) -> ProjectStatus {
        match &self.agent {
            Some(agent) => ProjectStatus {
                connected: agent.connected,
                project_path: Some(agent.project_path.display().to_string()),
                agent_id: Some(agent.agent_id.clone()),
                connection_generation: Some(agent.generation),
                // Hide prewarmed sessions until published to the UI.
                session_id: agent
                    .visible_session_id
                    .clone()
                    .or_else(|| {
                        agent
                            .sessions
                            .iter()
                            .find(|(_, handle)| handle.session_ready_for_ui)
                            .map(|(sid, _)| sid.clone())
                    })
                    .and_then(|id| normalize_session_id(&id)),
                config_options: agent.config_options(),
                capabilities: Some(agent.agent_capabilities.clone()),
                agent_command: crate::agents::agent_command_label(&agent.agent_id),
            },
            None => ProjectStatus {
                connected: false,
                project_path: None,
                agent_id: None,
                connection_generation: None,
                session_id: None,
                config_options: Vec::new(),
                capabilities: None,
                agent_command: crate::agents::agent_command_label(crate::agents::DEFAULT_AGENT_ID),
            },
        }
    }

    pub fn is_known_generation(&self, generation: u64) -> bool {
        self.agent_for_generation(generation).is_some()
    }

    pub fn is_current_generation(&self, generation: u64) -> bool {
        self.agent
            .as_ref()
            .is_some_and(|agent| agent.generation == generation)
    }

    pub fn agent_for_generation(&self, generation: u64) -> Option<&ActiveAgent> {
        if let Some(agent) = &self.agent {
            if agent.generation == generation {
                return Some(agent);
            }
        }
        self.warm_pool
            .values()
            .find(|agent| agent.generation == generation)
    }

    pub fn agent_for_generation_mut(&mut self, generation: u64) -> Option<&mut ActiveAgent> {
        if let Some(agent) = &mut self.agent {
            if agent.generation == generation {
                return Some(agent);
            }
        }
        self.warm_pool
            .values_mut()
            .find(|agent| agent.generation == generation)
    }
}

fn normalize_session_id(session_id: &str) -> Option<String> {
    if session_id.is_empty() || session_id == "pending" {
        None
    } else {
        Some(session_id.to_string())
    }
}

pub type SharedState = Arc<Mutex<CirculoState>>;
