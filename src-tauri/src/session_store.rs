use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::SessionInfoDto;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectSessionStore {
    pub active_session_id: Option<String>,
    pub sessions: Vec<StoredSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub session_id: String,
    pub title: Option<String>,
    pub updated_at: Option<String>,
}

impl ProjectSessionStore {
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        let raw = fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let raw = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        fs::write(path, raw).map_err(|err| err.to_string())
    }

    pub fn ids(&self) -> HashSet<String> {
        self.sessions.iter().map(|s| s.session_id.clone()).collect()
    }

    pub fn upsert(&mut self, session: &SessionInfoDto) {
        if let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|entry| entry.session_id == session.session_id)
        {
            existing.title = session.title.clone();
            existing.updated_at = session.updated_at.clone();
            return;
        }
        self.sessions.push(StoredSession {
            session_id: session.session_id.clone(),
            title: session.title.clone(),
            updated_at: session.updated_at.clone(),
        });
    }

    pub fn remove(&mut self, session_id: &str) {
        self.sessions.retain(|entry| entry.session_id != session_id);
        if self.active_session_id.as_deref() == Some(session_id) {
            self.active_session_id = self.sessions.first().map(|entry| entry.session_id.clone());
        }
    }

    pub fn set_active(&mut self, session_id: &str) {
        self.active_session_id = Some(session_id.to_string());
    }

    pub fn filter_agent_sessions(&self, sessions: Vec<SessionInfoDto>) -> Vec<SessionInfoDto> {
        if self.sessions.is_empty() {
            return sessions;
        }
        let ids = self.ids();
        sessions
            .into_iter()
            .filter(|session| ids.contains(&session.session_id))
            .collect()
    }

    pub fn preferred_active_id(&self, available: &[SessionInfoDto]) -> Option<String> {
        if let Some(active) = self.active_session_id.clone() {
            if available.iter().any(|session| session.session_id == active) {
                return Some(active);
            }
        }
        available.first().map(|session| session.session_id.clone())
    }
}

pub fn store_path_for(app_data_dir: &Path, project_path: &Path) -> PathBuf {
    let key = project_path
        .to_string_lossy()
        .replace('/', "_")
        .replace(':', "_");
    app_data_dir.join("sessions").join(format!("{key}.json"))
}