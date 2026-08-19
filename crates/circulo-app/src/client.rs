use std::path::PathBuf;
use std::time::Duration;

use circulo_core::{
    ComposerInteractionMode, ComposerPermissionMode, Message, ModelCatalogEntry, Project,
    Session, Uuid,
};
use circulo_core::AgentType;
use circulo_protocol::{
    AgentDescriptor, CreateMessageRequest, CreateProjectRequest, CreateSessionRequest,
    HealthResponse, PatchProjectRequest, PatchSessionRequest, PreferencesBody,
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

    pub fn list_archived_projects(&self) -> Result<Vec<Project>, String> {
        self.get("/v1/projects?status=archived")
    }

    pub fn archive_project(&self, project_id: Uuid) -> Result<(), String> {
        self.post_no_content(&format!("/v1/projects/{project_id}/archive"))
    }

    pub fn restore_project(&self, project_id: Uuid) -> Result<(), String> {
        self.post_no_content(&format!("/v1/projects/{project_id}/restore"))
    }

    pub fn rename_project(
        &self,
        project_id: Uuid,
        name: String,
    ) -> Result<Project, String> {
        self.patch(
            &format!("/v1/projects/{project_id}"),
            &PatchProjectRequest {
                name: Some(name),
                description: None,
                color: None,
                folder_path: None,
            },
        )
    }

    pub fn delete_project(&self, project_id: Uuid) -> Result<(), String> {
        self.delete_once(&format!("/v1/projects/{project_id}"))
    }

    pub fn create_session(&self) -> Result<Session, String> {
        self.create_session_with_project(None)
    }

    pub fn create_session_with_project(
        &self,
        project_id: Option<Uuid>,
    ) -> Result<Session, String> {
        self.create_session_with_agent(project_id, None)
    }

    pub fn create_session_with_agent(
        &self,
        project_id: Option<Uuid>,
        agent: Option<circulo_core::AgentType>,
    ) -> Result<Session, String> {
        self.post(
            "/v1/sessions",
            &CreateSessionRequest {
                project_id,
                title: None,
                agent,
            },
        )
    }

    pub fn list_agents(&self) -> Result<Vec<AgentDescriptor>, String> {
        self.get("/v1/agents")
    }

    pub fn set_provider_enabled(
        &self,
        agent: AgentType,
        enabled: bool,
    ) -> Result<PreferencesBody, String> {
        let path = if enabled {
            format!("/v1/agents/{agent}/enable")
        } else {
            format!("/v1/agents/{agent}/disable")
        };
        ureq::post(&format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .call()
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    pub fn patch_session_agent(
        &self,
        session_id: Uuid,
        agent: circulo_core::AgentType,
    ) -> Result<Session, String> {
        self.patch(
            &format!("/v1/sessions/{session_id}"),
            &PatchSessionRequest {
                title: None,
                project_id: None,
                archive: None,
                agent: Some(agent),
                composer_model_id: None,
                composer_model_variant: None,
                composer_permission_mode: None,
                composer_interaction_mode: None,
            },
        )
    }

    pub fn create_project(&self, name: &str, folder_path: Option<String>) -> Result<Project, String> {
        self.post(
            "/v1/projects",
            &CreateProjectRequest {
                name: name.into(),
                description: None,
                color: None,
                folder_path,
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
            Duration::from_secs(10),
        )
    }

    pub fn abort_session(&self, session_id: Uuid) -> Result<(), String> {
        ureq::post(&format!("{}/v1/sessions/{session_id}/abort", self.base))
            .timeout(Duration::from_secs(5))
            .call()
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    pub fn reply_permission(
        &self,
        session_id: Uuid,
        permission_id: &str,
        allow: bool,
    ) -> Result<(), String> {
        ureq::post(&format!(
            "{}/v1/sessions/{session_id}/permissions/{permission_id}/reply",
            self.base
        ))
        .timeout(Duration::from_secs(5))
        .send_json(&circulo_protocol::PermissionReplyRequest { allow })
        .map(|_| ())
        .map_err(|err| err.to_string())
    }

    pub fn reply_question(
        &self,
        session_id: Uuid,
        request_id: &str,
        answers: Vec<circulo_protocol::QuestionAnswerBody>,
    ) -> Result<(), String> {
        ureq::post(&format!(
            "{}/v1/sessions/{session_id}/questions/{request_id}/reply",
            self.base
        ))
        .timeout(Duration::from_secs(5))
        .send_json(&circulo_protocol::QuestionReplyRequest { answers })
        .map(|_| ())
        .map_err(|err| err.to_string())
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

    pub fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, String> {
        self.get("/v1/models")
    }

    pub fn get_preferences(&self) -> Result<PreferencesBody, String> {
        self.get("/v1/preferences")
    }

    pub fn put_preferences(&self, body: &PreferencesBody) -> Result<PreferencesBody, String> {
        ureq::put(&format!("{}/v1/preferences", self.base))
            .timeout(Duration::from_secs(2))
            .send_json(body)
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    pub fn patch_session_composer(
        &self,
        session_id: Uuid,
        composer_model_id: String,
        composer_model_variant: Option<String>,
        permission_mode: ComposerPermissionMode,
        interaction_mode: ComposerInteractionMode,
    ) -> Result<Session, String> {
        self.patch(
            &format!("/v1/sessions/{session_id}"),
            &PatchSessionRequest {
                title: None,
                project_id: None,
                archive: None,
                agent: None,
                composer_model_id: Some(composer_model_id),
                composer_model_variant: composer_model_variant,
                composer_permission_mode: Some(permission_mode),
                composer_interaction_mode: Some(interaction_mode),
            },
        )
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
                agent: None,
                composer_model_id: None,
                composer_model_variant: None,
                composer_permission_mode: None,
                composer_interaction_mode: None,
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
                agent: None,
                composer_model_id: None,
                composer_model_variant: None,
                composer_permission_mode: None,
                composer_interaction_mode: None,
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
            return Err("Resource not found.".into());
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
        self.post_timed(path, body, Duration::from_secs(10))
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

    fn post_no_content(&self, path: &str) -> Result<(), String> {
        let response = ureq::post(&format!("{}{path}", self.base))
            .timeout(Duration::from_secs(2))
            .call()
            .map_err(|err| err.to_string())?;
        let status = response.status();
        if status == 204 || status == 200 {
            Ok(())
        } else {
            Err(format!("POST failed with status {status}."))
        }
    }
}

pub fn ensure_daemon(client: &DaemonClient) -> Result<(), String> {
    if client.health().is_ok() {
        return Ok(());
    }
    spawn_sibling_daemon();
    if wait_for_health(client, 6, Duration::from_millis(250)).is_ok() {
        return Ok(());
    }
    kill_daemon_on_port(7432);
    spawn_sibling_daemon();
    wait_for_health(client, 8, Duration::from_millis(250))
}

fn spawn_sibling_daemon() {
    if let Some(path) = sibling_daemon() {
        let _ = build_sibling_daemon().and_then(|()| {
            std::process::Command::new(path)
                .spawn()
                .map(|_| ())
                .map_err(|err| err.to_string())
        });
    }
}

fn wait_for_health(
    client: &DaemonClient,
    attempts: u32,
    delay: Duration,
) -> Result<(), String> {
    for attempt in 0..attempts {
        if client.health().is_ok() {
            return Ok(());
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }
    client.health().map(|_| ())
}

fn kill_daemon_on_port(port: u16) {
    #[cfg(unix)]
    {
        let Ok(output) = std::process::Command::new("lsof")
            .args(["-ti", &format!("tcp:{port}")])
            .output()
        else {
            return;
        };
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid in pids.lines().map(str::trim).filter(|pid| !pid.is_empty()) {
            let _ = std::process::Command::new("kill").arg(pid).status();
        }
    }
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
    if !cfg!(debug_assertions) {
        return Ok(());
    }
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
    let exe = std::env::current_exe().ok()?;
    let sibling = exe.parent()?.join("circulo-daemon");
    if sibling.exists() {
        return Some(sibling);
    }
    if let Some(dir) = workspace_debug_dir() {
        let path = dir.join("circulo-daemon");
        if path.exists() {
            return Some(path);
        }
    }
    None
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
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
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
            folder_path: None,
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
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        };
        assert_eq!(session_activity_at(&session), created);
    }
}
