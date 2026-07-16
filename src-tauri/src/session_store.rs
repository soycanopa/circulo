use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::SessionInfoDto;

const CIRCULO_SESSION_SOURCE: &str = "circulo";
const LEGACY_POLLUTION_THRESHOLD: usize = 30;

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
    #[serde(default)]
    pub source: Option<String>,
}

impl ProjectSessionStore {
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        let raw = fs::read_to_string(path).unwrap_or_default();
        let mut store: Self = serde_json::from_str(&raw).unwrap_or_default();
        store.migrate_polluted_legacy();
        store
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let raw = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        fs::write(path, raw).map_err(|err| err.to_string())
    }

    pub fn ids(&self) -> HashSet<String> {
        self.tracked_sessions()
            .into_iter()
            .map(|entry| entry.session_id.clone())
            .collect()
    }

    /// Registers a session Circulo created or adopted explicitly.
    pub fn register(&mut self, session: &SessionInfoDto) {
        if let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|entry| entry.session_id == session.session_id)
        {
            existing.title = session.title.clone();
            existing.updated_at = session.updated_at.clone();
            if existing.source.is_none() {
                existing.source = Some(CIRCULO_SESSION_SOURCE.to_string());
            }
            return;
        }
        self.sessions.push(StoredSession {
            session_id: session.session_id.clone(),
            title: session.title.clone(),
            updated_at: session.updated_at.clone(),
            source: Some(CIRCULO_SESSION_SOURCE.to_string()),
        });
    }

    fn migrate_polluted_legacy(&mut self) {
        if self.sessions.len() < LEGACY_POLLUTION_THRESHOLD {
            return;
        }

        let has_circulo_source = self
            .sessions
            .iter()
            .any(|entry| entry.source.as_deref() == Some(CIRCULO_SESSION_SOURCE));

        if has_circulo_source {
            self.sessions.retain(|entry| {
                entry.source.as_deref() == Some(CIRCULO_SESSION_SOURCE)
            });
        } else if let Some(active) = self.active_session_id.clone() {
            self.sessions.retain(|entry| entry.session_id == active);
        } else {
            self.sessions.clear();
        }

        if self.active_session_id.is_none() {
            self.active_session_id = self
                .sessions
                .first()
                .map(|entry| entry.session_id.clone());
        }
    }

    /// Updates metadata for a session already tracked by Circulo.
    pub fn update_metadata(&mut self, session: &SessionInfoDto) -> bool {
        let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|entry| entry.session_id == session.session_id)
        else {
            return false;
        };
        existing.title = session.title.clone();
        existing.updated_at = session.updated_at.clone();
        if existing.source.is_none() {
            existing.source = Some(CIRCULO_SESSION_SOURCE.to_string());
        }
        true
    }

    pub fn merge_agent_metadata(&mut self, agent_sessions: &[SessionInfoDto]) {
        for session in agent_sessions {
            self.update_metadata(session);
        }
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

    pub fn filter_agent_sessions(&self, sessions: &[SessionInfoDto]) -> Vec<SessionInfoDto> {
        let tracked = self.tracked_sessions();
        if tracked.is_empty() {
            return Vec::new();
        }
        let ids: HashSet<String> = tracked
            .iter()
            .map(|entry| entry.session_id.clone())
            .collect();
        sessions
            .iter()
            .filter(|session| ids.contains(&session.session_id))
            .cloned()
            .collect()
    }

    fn tracked_sessions(&self) -> Vec<&StoredSession> {
        let circulo: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| entry.source.as_deref() == Some(CIRCULO_SESSION_SOURCE))
            .collect();
        if !circulo.is_empty() {
            return circulo;
        }
        self.sessions.iter().collect()
    }

    pub fn preferred_active_id(&self, available: &[SessionInfoDto]) -> Option<String> {
        if let Some(active) = self.active_session_id.clone() {
            if available.iter().any(|session| session.session_id == active) {
                return Some(active);
            }
        }
        available.first().map(|session| session.session_id.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.tracked_sessions().is_empty()
    }

    pub fn as_session_dtos(&self, project_path: &Path) -> Vec<SessionInfoDto> {
        let cwd = project_path.display().to_string();
        self.tracked_sessions()
            .into_iter()
            .map(|stored| SessionInfoDto {
                session_id: stored.session_id.clone(),
                cwd: cwd.clone(),
                additional_directories: Vec::new(),
                title: stored.title.clone(),
                updated_at: stored.updated_at.clone(),
            })
            .collect()
    }
}

pub fn store_path_for(app_data_dir: &Path, project_path: &Path) -> PathBuf {
    let key = project_path
        .to_string_lossy()
        .replace('/', "_")
        .replace(':', "_");
    app_data_dir.join("sessions").join(format!("{key}.json"))
}