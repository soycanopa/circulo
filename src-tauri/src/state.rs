use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::v1::{AgentCapabilities, SessionInfo};
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
pub struct SessionInfoDto {
    pub session_id: String,
    pub cwd: String,
    pub additional_directories: Vec<String>,
    pub title: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilitiesDto {
    pub load_session: bool,
    pub list_sessions: bool,
    pub resume_session: bool,
    pub close_session: bool,
}

impl AgentCapabilitiesDto {
    pub fn from_capabilities(caps: &AgentCapabilities) -> Self {
        Self {
            load_session: caps.load_session,
            list_sessions: caps.session_capabilities.list.is_some(),
            resume_session: caps.session_capabilities.resume.is_some(),
            close_session: caps.session_capabilities.close.is_some(),
        }
    }
}

impl From<&SessionInfo> for SessionInfoDto {
    fn from(info: &SessionInfo) -> Self {
        Self {
            session_id: info.session_id.to_string(),
            cwd: info.cwd.display().to_string(),
            additional_directories: info
                .additional_directories
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            title: info.title.clone(),
            updated_at: info.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub connected: bool,
    pub project_path: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub active_session_id: Option<String>,
    pub sessions: Vec<SessionInfoDto>,
    pub capabilities: Option<AgentCapabilitiesDto>,
    pub agent_command: String,
}

pub enum AgentCommand {
    SendPrompt {
        text: String,
        context_files: Vec<ContextFile>,
    },
    SetConfigOption {
        config_id: String,
        value: String,
    },
    ListSessions,
    CreateSession,
    CreateReserveSession,
    LoadSession {
        id: String,
    },
    ResumeSession {
        id: String,
    },
    CloseSession {
        id: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ReserveSession {
    pub session_id: String,
    pub config_options: Vec<ConfigOptionDto>,
    pub session_entry: SessionInfoDto,
}

pub struct PooledAgent {
    pub project_path: PathBuf,
    pub agent_id: String,
    pub session_id: String,
    pub cmd_tx: mpsc::Sender<AgentCommand>,
    pub config_options: Vec<ConfigOptionDto>,
    pub sessions: Vec<SessionInfoDto>,
    pub agent_capabilities: AgentCapabilitiesDto,
    pub list_cursor: Option<String>,
    pub agent_done: Arc<Notify>,
    pub connected: bool,
    pub reserve: Option<ReserveSession>,
    pub reserve_in_flight: bool,
}

impl PooledAgent {
    pub fn supports_list_sessions(&self) -> bool {
        self.agent_capabilities.list_sessions
    }

    pub fn supports_load_session(&self) -> bool {
        self.agent_capabilities.load_session
    }

    pub fn supports_resume_session(&self) -> bool {
        self.agent_capabilities.resume_session
    }

    pub fn supports_close_session(&self) -> bool {
        self.agent_capabilities.close_session
    }
}

pub struct CirculoState {
    pub agents: HashMap<String, PooledAgent>,
    pub active_path: Option<String>,
    pub lru: Vec<String>,
    pub permission_waiters: HashMap<String, oneshot::Sender<String>>,
    pub credential_waiters: HashMap<String, oneshot::Sender<CredentialResponseDto>>,
    pub session_create_waiter: Option<oneshot::Sender<Result<(), String>>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialResponseDto {
    pub action: String,
    #[serde(default)]
    pub values: Option<HashMap<String, String>>,
}

impl CirculoState {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            active_path: None,
            lru: Vec::new(),
            permission_waiters: HashMap::new(),
            credential_waiters: HashMap::new(),
            session_create_waiter: None,
        }
    }

    pub fn pool_key(path: &Path) -> String {
        path.display().to_string()
    }

    pub fn active_agent(&self) -> Option<&PooledAgent> {
        let key = self.active_path.as_ref()?;
        self.agents.get(key)
    }

    pub fn active_agent_mut(&mut self) -> Option<&mut PooledAgent> {
        let key = self.active_path.clone()?;
        self.agents.get_mut(&key)
    }

    pub fn agent_for_path(&self, path: &Path) -> Option<&PooledAgent> {
        self.agents.get(&Self::pool_key(path))
    }

    pub fn agent_for_path_mut(&mut self, path: &Path) -> Option<&mut PooledAgent> {
        let key = Self::pool_key(path);
        self.agents.get_mut(&key)
    }

    pub fn lru_eviction_candidate(&self, incoming_key: &str) -> Option<String> {
        crate::orchestrator::lru_eviction_candidate(self, incoming_key)
    }

    fn normalize_session_id(session_id: &str) -> Option<String> {
        if session_id == "pending" {
            None
        } else {
            Some(session_id.to_string())
        }
    }

    pub fn status(&self) -> ProjectStatus {
        match self.active_agent() {
            Some(agent) => ProjectStatus {
                connected: agent.connected,
                project_path: Some(agent.project_path.display().to_string()),
                agent_id: Some(agent.agent_id.clone()),
                session_id: Self::normalize_session_id(&agent.session_id),
                active_session_id: Self::normalize_session_id(&agent.session_id),
                sessions: agent.sessions.clone(),
                capabilities: Some(agent.agent_capabilities.clone()),
                agent_command: crate::agents::agent_command_label(&agent.agent_id).to_string(),
            },
            None => ProjectStatus {
                connected: false,
                project_path: None,
                agent_id: None,
                session_id: None,
                active_session_id: None,
                sessions: Vec::new(),
                capabilities: None,
                agent_command: crate::agents::agent_command_label(crate::agents::DEFAULT_AGENT_ID)
                    .to_string(),
            },
        }
    }
}

pub type SharedState = Arc<Mutex<CirculoState>>;

// Backwards compatibility alias used across the codebase during migration.
pub type ActiveProject = PooledAgent;