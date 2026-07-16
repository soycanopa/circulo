use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::v1::{AgentCapabilities, SessionInfo};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex};

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

pub struct ActiveProject {
    pub project_path: PathBuf,
    pub session_id: String,
    pub cmd_tx: mpsc::Sender<AgentCommand>,
    pub config_options: Vec<ConfigOptionDto>,
    pub sessions: Vec<SessionInfoDto>,
    pub agent_capabilities: AgentCapabilitiesDto,
    pub list_cursor: Option<String>,
}

impl ActiveProject {
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialResponseDto {
    pub action: String,
    #[serde(default)]
    pub values: Option<HashMap<String, String>>,
}

pub struct CirculoState {
    pub project: Option<ActiveProject>,
    pub permission_waiters: HashMap<String, oneshot::Sender<String>>,
    pub credential_waiters: HashMap<String, oneshot::Sender<CredentialResponseDto>>,
}

impl CirculoState {
    pub fn new() -> Self {
        Self {
            project: None,
            permission_waiters: HashMap::new(),
            credential_waiters: HashMap::new(),
        }
    }

    pub fn status(&self) -> ProjectStatus {
        match &self.project {
            Some(project) => ProjectStatus {
                connected: true,
                project_path: Some(project.project_path.display().to_string()),
                session_id: Some(project.session_id.clone()),
                active_session_id: Some(project.session_id.clone()),
                sessions: project.sessions.clone(),
                capabilities: Some(project.agent_capabilities.clone()),
                agent_command: "opencode acp".to_string(),
            },
            None => ProjectStatus {
                connected: false,
                project_path: None,
                session_id: None,
                active_session_id: None,
                sessions: Vec::new(),
                capabilities: None,
                agent_command: "opencode acp".to_string(),
            },
        }
    }
}

pub type SharedState = Arc<Mutex<CirculoState>>;