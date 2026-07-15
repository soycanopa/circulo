use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub connected: bool,
    pub project_path: Option<String>,
    pub session_id: Option<String>,
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
}

pub struct ForgeState {
    pub project: Option<ActiveProject>,
    pub permission_waiters: HashMap<String, oneshot::Sender<String>>,
}

impl ForgeState {
    pub fn new() -> Self {
        Self {
            project: None,
            permission_waiters: HashMap::new(),
        }
    }

    pub fn status(&self) -> ProjectStatus {
        match &self.project {
            Some(project) => ProjectStatus {
                connected: true,
                project_path: Some(project.project_path.display().to_string()),
                session_id: Some(project.session_id.clone()),
                agent_command: "opencode acp".to_string(),
            },
            None => ProjectStatus {
                connected: false,
                project_path: None,
                session_id: None,
                agent_command: "opencode acp".to_string(),
            },
        }
    }
}

pub type SharedState = Arc<Mutex<ForgeState>>;