use std::path::PathBuf;
use std::time::Duration;

use circulo_core::{Project, Session, SidebarView, Uuid};
use circulo_protocol::{
    CreateProjectRequest, CreateSessionRequest, HealthResponse, PreferencesBody,
};

const DEFAULT_BASE: &str = "http://127.0.0.1:7432";

#[derive(Debug, Clone)]
pub struct DaemonClient {
    base: String,
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE.into(),
        }
    }
}

impl DaemonClient {
    pub fn health(&self) -> Result<HealthResponse, String> {
        self.get("/v1/health")
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, String> {
        self.get("/v1/sessions")
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        self.get("/v1/projects")
    }

    pub fn create_session(&self) -> Result<Session, String> {
        self.post(
            "/v1/sessions",
            &CreateSessionRequest {
                project_id: None,
                title: None,
            },
        )
    }

    pub fn create_project(&self, name: &str) -> Result<Project, String> {
        self.post(
            "/v1/projects",
            &CreateProjectRequest {
                name: name.into(),
                description: None,
                color: None,
            },
        )
    }

    pub fn preferences(&self) -> Result<PreferencesBody, String> {
        self.get("/v1/preferences")
    }

    pub fn set_view(&self, view: SidebarView) -> Result<PreferencesBody, String> {
        self.put("/v1/preferences", &PreferencesBody { sidebar_view: view })
    }

    fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        ureq::get(&format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .call()
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        ureq::post(&format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .send_json(body)
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    fn put<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        ureq::put(&format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .send_json(body)
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }
}

pub fn ensure_daemon(client: &DaemonClient) -> Result<(), String> {
    if client.health().is_ok() {
        return Ok(());
    }
    if let Some(path) = sibling_daemon() {
        let _ = std::process::Command::new(path).spawn();
        std::thread::sleep(Duration::from_millis(200));
    }
    client.health().map(|_| ())
}

fn sibling_daemon() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join("circulo-daemon");
    path.exists().then_some(path)
}

pub fn resolve_view(stored: Option<SidebarView>) -> SidebarView {
    stored.unwrap_or(SidebarView::Sessions)
}

pub fn session_project_label(
    project_id: Option<Uuid>,
    projects: &[Project],
    no_project: &str,
) -> String {
    match project_id {
        None => no_project.to_string(),
        Some(id) => projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| no_project.to_string()),
    }
}

pub fn groups_need_new_project(projects: &[Project]) -> bool {
    projects.is_empty()
}

pub fn filter_sessions<'a>(sessions: &'a [Session], query: &str) -> Vec<&'a Session> {
    let q = query.trim().to_ascii_lowercase();
    sessions
        .iter()
        .filter(|session| {
            q.is_empty() || session.title.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use circulo_core::{AgentType, Project, ProjectStatus, Session, SessionStatus, SidebarView};
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::{filter_sessions, groups_need_new_project, resolve_view, session_project_label};

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
    }

    #[test]
    fn view_falls_back_to_sessions() {
        assert_eq!(resolve_view(None), SidebarView::Sessions);
        assert_eq!(resolve_view(Some(SidebarView::Groups)), SidebarView::Groups);
    }

    #[test]
    fn unassigned_uses_no_project_label() {
        assert_eq!(
            session_project_label(None, &[], "No project"),
            "No project"
        );
    }

    #[test]
    fn assigned_uses_project_name() {
        let project = Project {
            id: Uuid::from_u128(1),
            name: "Launch".into(),
            description: None,
            color: None,
            status: ProjectStatus::Active,
            created_at: now(),
            updated_at: now(),
        };
        assert_eq!(
            session_project_label(Some(project.id), &[project], "No project"),
            "Launch"
        );
    }

    #[test]
    fn empty_groups_needs_cta() {
        assert!(groups_need_new_project(&[]));
    }

    #[test]
    fn search_filters_title() {
        let session = Session {
            id: Uuid::from_u128(2),
            project_id: None,
            title: "Landing copy".into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: now(),
            updated_at: now(),
            last_message_at: None,
            first_send_at: None,
        };
        assert_eq!(filter_sessions(&[session.clone()], "land").len(), 1);
        assert!(filter_sessions(&[session], "budget").is_empty());
    }
}
