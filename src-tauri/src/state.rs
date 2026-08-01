use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

    pub fn empty() -> Self {
        Self {
            load_session: false,
            list_sessions: false,
            resume_session: false,
            close_session: false,
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
    pub config_options: Vec<ConfigOptionDto>,
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
    CreateSession {
        done: oneshot::Sender<Result<(), String>>,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

pub struct ActiveAgent {
    pub project_path: PathBuf,
    pub agent_id: String,
    /// ACP session id if one exists on the agent process.
    pub session_id: String,
    /// Only true after New Chat publishes the session to the UI.
    /// Prewarm may hold a session_id while this stays false.
    pub session_ready_for_ui: bool,
    pub cmd_tx: mpsc::Sender<AgentCommand>,
    pub config_options: Vec<ConfigOptionDto>,
    pub agent_capabilities: AgentCapabilitiesDto,
    pub agent_done: Arc<Notify>,
    pub connected: bool,
}

pub struct CirculoState {
    pub agent: Option<ActiveAgent>,
    pub permission_waiters: HashMap<String, oneshot::Sender<String>>,
}

impl CirculoState {
    pub fn new() -> Self {
        Self {
            agent: None,
            permission_waiters: HashMap::new(),
        }
    }

    pub fn status(&self) -> ProjectStatus {
        match &self.agent {
            Some(agent) => ProjectStatus {
                connected: agent.connected,
                project_path: Some(agent.project_path.display().to_string()),
                agent_id: Some(agent.agent_id.clone()),
                // Hide prewarmed sessions until New Chat publishes them.
                session_id: if agent.session_ready_for_ui {
                    normalize_session_id(&agent.session_id)
                } else {
                    None
                },
                config_options: agent.config_options.clone(),
                capabilities: Some(agent.agent_capabilities.clone()),
                agent_command: crate::agents::agent_command_label(&agent.agent_id).to_string(),
            },
            None => ProjectStatus {
                connected: false,
                project_path: None,
                agent_id: None,
                session_id: None,
                config_options: Vec::new(),
                capabilities: None,
                agent_command: crate::agents::agent_command_label(crate::agents::DEFAULT_AGENT_ID)
                    .to_string(),
            },
        }
    }

    pub fn agent_for_path(&self, path: &Path) -> Option<&ActiveAgent> {
        self.agent
            .as_ref()
            .filter(|agent| agent.project_path == path)
    }

    pub fn agent_for_path_mut(&mut self, path: &Path) -> Option<&mut ActiveAgent> {
        self.agent
            .as_mut()
            .filter(|agent| agent.project_path == path)
    }

    /// MVP is single-agent: always mutate the active agent when present.
    /// Path equality can fail across canonicalize/string forms and left the UI stuck warming.
    pub fn active_agent_mut(&mut self) -> Option<&mut ActiveAgent> {
        self.agent.as_mut()
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
