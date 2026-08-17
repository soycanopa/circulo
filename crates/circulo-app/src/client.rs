use std::path::PathBuf;
use std::time::Duration;

use circulo_core::{Message, Project, Session, Uuid};
use circulo_protocol::{
    CreateMessageRequest, CreateProjectRequest, CreateSessionRequest, HealthResponse,
    PatchSessionRequest,
};
use time::{OffsetDateTime, UtcOffset};

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

    pub fn list_messages(&self, session_id: Uuid) -> Result<Vec<Message>, String> {
        self.get(&format!("/v1/sessions/{session_id}/messages"))
    }

    pub fn post_message(&self, session_id: Uuid, content: &str) -> Result<Message, String> {
        self.post_timed(
            &format!("/v1/sessions/{session_id}/messages"),
            &CreateMessageRequest {
                content: content.into(),
            },
            Duration::from_secs(30),
        )
    }

    /// Opens the session's SSE stream. Blocking reads on the returned iterator
    /// wait up to `STREAM_READ_TIMEOUT` between frames; there is no overall
    /// timeout because the stream lives as long as the subscription.
    pub fn session_events(
        &self,
        session_id: Uuid,
    ) -> Result<crate::stream::SessionEventStream, String> {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(crate::stream::STREAM_READ_TIMEOUT)
            .build();
        let response = agent
            .get(format!("{}/v1/sessions/{session_id}/events", self.base).as_str())
            .call()
            .map_err(|err| err.to_string())?;
        Ok(crate::stream::SessionEventStream::new(
            response.into_reader(),
        ))
    }

    pub fn set_session_project(
        &self,
        session_id: Uuid,
        project_id: Option<Uuid>,
    ) -> Result<Session, String> {
        self.patch(
            &format!("/v1/sessions/{session_id}"),
            &PatchSessionRequest {
                title: None,
                project_id: Some(project_id),
                archive: None,
            },
        )
    }

    pub fn rename_session(&self, session_id: Uuid, title: String) -> Result<Session, String> {
        self.patch(
            &format!("/v1/sessions/{session_id}"),
            &PatchSessionRequest {
                title: Some(title),
                project_id: None,
                archive: None,
            },
        )
    }

    pub fn delete_session(&self, session_id: Uuid) -> Result<(), String> {
        self.delete_once(&format!("/v1/sessions/{session_id}"))
    }

    fn delete_once(&self, path: &str) -> Result<(), String> {
        let response = ureq::request("DELETE", &format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .call()
            .map_err(|err| format_http_delete_error(err))?;
        let status = response.status();
        if status == 204 || status == 200 {
            return Ok(());
        }
        if status == 404 {
            return Err("Session not found.".into());
        }
        if status == 405 {
            return Err(
                "Session delete is not supported by the running daemon. Rebuild and restart circulo-daemon."
                    .into(),
            );
        }
        Err(format!("DELETE failed with status {status}."))
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
        self.post_timed(path, body, Duration::from_secs(2))
    }

    fn post_timed<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        timeout: Duration,
    ) -> Result<T, String> {
        ureq::post(&format!("{}{path}", self.base))
            .timeout(timeout)
            .send_json(body)
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    fn patch<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        ureq::request("PATCH", &format!("{}{path}", self.base))
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
        build_sibling_daemon()?;
        let _ = std::process::Command::new(path).spawn();
        std::thread::sleep(Duration::from_millis(400));
    }
    client.health().map(|_| ())
}

fn format_http_delete_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, _) => {
            if code == 405 {
                "Session delete is not supported by the running daemon. Rebuild and restart circulo-daemon."
                    .into()
            } else {
                format!("HTTP {code}")
            }
        }
        ureq::Error::Transport(transport) => transport.to_string(),
    }
}

/// Dev builds ship `circulo-app` and `circulo-daemon` as sibling binaries; rebuild
/// the daemon when spawning so new HTTP routes (e.g. session delete) are available.
fn build_sibling_daemon() -> Result<(), String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir
        .parent()
        .and_then(|crates| crates.parent())
        .filter(|root| root.join("Cargo.toml").exists());
    if let Some(workspace) = workspace {
        let target_dir = workspace.join("target");
        let status = std::process::Command::new("cargo")
            .args(["build", "-q", "-p", "circulo-daemon"])
            .current_dir(workspace)
            .env("CARGO_TARGET_DIR", target_dir)
            .status()
            .map_err(|err| err.to_string())?;
        if !status.success() {
            return Err("Failed to build circulo-daemon.".into());
        }
    }
    Ok(())
}

fn workspace_debug_dir() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .map(|root| root.join("target/debug"))
}

fn sibling_daemon() -> Option<PathBuf> {
    if let Some(dir) = workspace_debug_dir() {
        let path = dir.join("circulo-daemon");
        if path.exists() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join("circulo-daemon");
    path.exists().then_some(path)
}

pub fn session_activity_at(session: &Session) -> OffsetDateTime {
    session.last_message_at.unwrap_or(session.created_at)
}

pub fn session_project_label(
    project_id: Option<Uuid>,
    projects: &[Project],
    without_folder: &str,
) -> String {
    match project_id {
        None => without_folder.to_string(),
        Some(id) => projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| without_folder.to_string()),
    }
}

pub fn filter_sessions<'a>(sessions: &'a [Session], query: &str) -> Vec<&'a Session> {
    let q = query.trim().to_ascii_lowercase();
    sessions
        .iter()
        .filter(|session| q.is_empty() || session.title.to_ascii_lowercase().contains(&q))
        .collect()
}

pub fn partition_sessions_by_day<'a>(
    sessions: &[&'a Session],
    now: OffsetDateTime,
    offset: UtcOffset,
) -> (Vec<&'a Session>, Vec<&'a Session>) {
    let mut today = Vec::new();
    let mut earlier = Vec::new();
    for session in sessions {
        let activity = session_activity_at(*session);
        if crate::timefmt::is_same_local_day(now, activity, offset) {
            today.push(*session);
        } else {
            earlier.push(*session);
        }
    }
    (today, earlier)
}

#[cfg(test)]
mod tests {
    use circulo_core::{AgentType, Project, ProjectStatus, Session, SessionStatus};
    use time::{OffsetDateTime, UtcOffset};
    use uuid::Uuid;

    use super::{
        filter_sessions, partition_sessions_by_day, session_activity_at, session_project_label,
    };

    fn ts(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(secs).unwrap()
    }

    fn session_with_activity(activity: OffsetDateTime) -> Session {
        Session {
            id: Uuid::from_u128(2),
            project_id: None,
            title: "Landing copy".into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: activity,
            updated_at: activity,
            last_message_at: Some(activity),
            first_send_at: None,
        }
    }

    #[test]
    fn unassigned_uses_without_folder_label() {
        assert_eq!(
            session_project_label(None, &[], "Without Folder"),
            "Without Folder"
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
            created_at: ts(1_700_000_000),
            updated_at: ts(1_700_000_000),
        };
        assert_eq!(
            session_project_label(Some(project.id), &[project], "Without Folder"),
            "Launch"
        );
    }

    #[test]
    fn search_filters_title() {
        let session = session_with_activity(ts(1_700_000_000));
        assert_eq!(filter_sessions(&[session.clone()], "land").len(), 1);
        assert!(filter_sessions(&[session], "budget").is_empty());
    }

    #[test]
    fn partition_splits_by_local_day() {
        let offset = UtcOffset::from_hms(-5, 0, 0).unwrap();
        let now = ts(1_700_086_400);
        let today_session = session_with_activity(ts(1_700_080_000));
        let earlier_session = session_with_activity(ts(1_700_000_000));
        let (today, earlier) = partition_sessions_by_day(
            &[&today_session, &earlier_session],
            now,
            offset,
        );
        assert_eq!(today.len(), 1);
        assert_eq!(earlier.len(), 1);
    }

    #[test]
    fn activity_at_falls_back_to_created() {
        let created = ts(1_700_000_000);
        let session = Session {
            id: Uuid::from_u128(3),
            project_id: None,
            title: "New".into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: created,
            updated_at: created,
            last_message_at: None,
            first_send_at: None,
        };
        assert_eq!(session_activity_at(&session), created);
    }
}
