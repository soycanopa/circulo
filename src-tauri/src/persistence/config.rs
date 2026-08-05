use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::circulo_data_dir;

const SETTINGS_VERSION: u32 = 6;
const MAX_RECENT_PROJECTS: usize = 24;
const MAX_WORKSPACES: usize = 12;
pub const MAX_CUSTOM_SLASH_COMMANDS: usize = 32;
const DEFAULT_WORKSPACE_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub last_opened_at: u64,
}

/// A user-defined slash command; its label is sent verbatim as a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomSlashCommand {
    pub command: String,
    pub label: String,
    pub description: String,
}

/// One isolated space: own general chats folder + own project list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub id: String,
    /// Absolute paths of projects belonging only to this space (not general chats).
    #[serde(default)]
    pub project_paths: Vec<String>,
    /// Last active cwd in this space (project or general chats).
    #[serde(default)]
    pub last_path: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    /// Global open-project history (palette); not the sidebar Projects tree.
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceEntry>,
    #[serde(default)]
    pub active_workspace_id: Option<String>,
    #[serde(default)]
    pub preferred_agent_id: Option<String>,
    /// Agent ids shown in the composer selector (subset of discovered agents).
    #[serde(default)]
    pub enabled_agent_ids: Vec<String>,
    /// ACP model ids marked as favorites in the composer (e.g. opencode/gpt-5.5).
    #[serde(default)]
    pub favorite_model_ids: Vec<String>,
    /// Recently selected ACP model ids (most recent first, capped at 5).
    #[serde(default)]
    pub recent_model_ids: Vec<String>,
    /// When true, Circulo auto-approves tool permissions (allow-always when offered).
    #[serde(default)]
    pub auto_approve_enabled: bool,
    /// Tool patterns the user chose to always allow (remembered across sessions).
    #[serde(default)]
    pub allowed_tool_patterns: Vec<String>,
    /// User-defined slash commands shown in the composer menu.
    #[serde(default)]
    pub custom_slash_commands: Vec<CustomSlashCommand>,
    /// Optional Vercel OIDC token for the authenticated skills.sh `/api/v1`
    /// endpoints (search + skill detail). Empty when the public fallback is used.
    #[serde(default)]
    pub vercel_oidc_token: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut settings = Self {
            version: SETTINGS_VERSION,
            recent_projects: Vec::new(),
            workspaces: Vec::new(),
            active_workspace_id: None,
            preferred_agent_id: None,
            enabled_agent_ids: default_enabled_agent_ids(),
            favorite_model_ids: Vec::new(),
            recent_model_ids: Vec::new(),
            auto_approve_enabled: false,
            allowed_tool_patterns: Vec::new(),
            custom_slash_commands: Vec::new(),
            vercel_oidc_token: None,
        };
        let _ = ensure_workspaces(&mut settings);
        settings
    }
}

fn settings_path() -> Result<PathBuf, String> {
    Ok(circulo_data_dir()?.join("config.json"))
}

pub fn load_settings() -> Result<AppSettings, String> {
    let path = settings_path()?;
    let mut settings = if !path.is_file() {
        AppSettings {
            version: SETTINGS_VERSION,
            recent_projects: Vec::new(),
            workspaces: Vec::new(),
            active_workspace_id: None,
            preferred_agent_id: None,
            enabled_agent_ids: Vec::new(),
            favorite_model_ids: Vec::new(),
            recent_model_ids: Vec::new(),
            auto_approve_enabled: false,
            allowed_tool_patterns: Vec::new(),
            custom_slash_commands: Vec::new(),
            vercel_oidc_token: None,
        }
    } else {
        let raw = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
        serde_json::from_str(&raw).map_err(|err| format!("Invalid config.json: {err}"))?
    };
    ensure_workspaces(&mut settings)?;
    ensure_enabled_agents(&mut settings);
    // Persist migration (v1 → v2 workspaces) once.
    if settings.version < SETTINGS_VERSION || path_needs_rewrite(&settings) {
        save_settings(&settings)?;
    }
    Ok(settings)
}

fn path_needs_rewrite(settings: &AppSettings) -> bool {
    settings.workspaces.is_empty()
        || settings.active_workspace_id.is_none()
        || settings.enabled_agent_ids.is_empty()
}

fn default_enabled_agent_ids() -> Vec<String> {
    vec![
        crate::agents::AGENT_ID_OPENCODE.to_string(),
        crate::agents::AGENT_ID_CURSOR.to_string(),
    ]
}

fn ensure_enabled_agents(settings: &mut AppSettings) {
    if settings.enabled_agent_ids.is_empty() {
        settings.enabled_agent_ids = default_enabled_agent_ids();
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    let mut next = settings.clone();
    next.version = SETTINGS_VERSION;
    ensure_workspaces(&mut next)?;
    ensure_enabled_agents(&mut next);
    let raw = serde_json::to_string_pretty(&next).map_err(|err| err.to_string())?;
    std::fs::write(path, raw).map_err(|err| err.to_string())
}

/// Ensure at least one workspace exists; migrate flat recents into default space once.
pub fn ensure_workspaces(settings: &mut AppSettings) -> Result<(), String> {
    if settings.workspaces.is_empty() {
        let mut project_paths: Vec<String> = settings
            .recent_projects
            .iter()
            .filter(|p| !is_general_chats_path(&p.path))
            .map(|p| p.path.clone())
            .collect();
        project_paths.truncate(MAX_RECENT_PROJECTS);

        settings.workspaces.push(WorkspaceEntry {
            id: DEFAULT_WORKSPACE_ID.to_string(),
            project_paths,
            last_path: None,
            created_at: now_ms(),
        });
        settings.active_workspace_id = Some(DEFAULT_WORKSPACE_ID.to_string());
    }

    if settings
        .active_workspace_id
        .as_ref()
        .map(|id| settings.workspaces.iter().any(|w| &w.id == id))
        != Some(true)
    {
        settings.active_workspace_id = settings.workspaces.first().map(|w| w.id.clone());
    }

    // Ensure general chats dirs exist for every space.
    for ws in &settings.workspaces {
        let _ = workspace_chats_dir(&ws.id)?;
    }
    Ok(())
}

fn is_general_chats_path(path: &str) -> bool {
    path.contains("/.circulo/chats")
        || path.contains("/.circulo/spaces/") && path.ends_with("/chats")
}

/// Reject workspace ids that escape `~/.circulo/spaces/{id}`. Workspace ids are
/// generated internally (`ws_<timestamp>`) but config files can be hand-edited,
/// so we never trust them blindly when constructing filesystem paths.
pub fn is_safe_workspace_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    if id == DEFAULT_WORKSPACE_ID {
        return true;
    }
    let mut chars = id.chars();
    let prefix = chars.by_ref().take(3).collect::<String>();
    if prefix != "ws_" {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// General chats folder for a workspace.
/// Default space keeps legacy `~/.circulo/chats`; others use `~/.circulo/spaces/{id}/chats`.
pub fn workspace_chats_dir(workspace_id: &str) -> Result<PathBuf, String> {
    if !is_safe_workspace_id(workspace_id) {
        return Err(format!("Invalid workspace id: {workspace_id}"));
    }
    let path = if workspace_id == DEFAULT_WORKSPACE_ID {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        PathBuf::from(home).join(".circulo").join("chats")
    } else {
        circulo_data_dir()?
            .join("spaces")
            .join(workspace_id)
            .join("chats")
    };
    std::fs::create_dir_all(&path)
        .map_err(|err| format!("Could not create workspace chats dir: {err}"))?;
    Ok(path)
}

pub fn active_workspace_chats_path() -> Result<String, String> {
    let settings = load_settings()?;
    let id = settings
        .active_workspace_id
        .as_deref()
        .unwrap_or(DEFAULT_WORKSPACE_ID);
    Ok(workspace_chats_dir(id)?.display().to_string())
}

pub fn touch_recent_project(project_path: &Path) -> Result<AppSettings, String> {
    let path_str = project_path.to_string_lossy().to_string();
    let now = now_ms();
    let mut settings = load_settings()?;

    settings
        .recent_projects
        .retain(|entry| entry.path != path_str);

    settings.recent_projects.insert(
        0,
        RecentProject {
            path: path_str.clone(),
            last_opened_at: now,
        },
    );
    settings.recent_projects.truncate(MAX_RECENT_PROJECTS);

    // Attach real projects to the active workspace (never general chats folder).
    if !is_general_chats_path(&path_str) {
        if let Some(id) = settings.active_workspace_id.clone() {
            if let Some(ws) = settings.workspaces.iter_mut().find(|w| w.id == id) {
                ws.project_paths.retain(|p| p != &path_str);
                ws.project_paths.insert(0, path_str.clone());
                ws.project_paths.truncate(MAX_RECENT_PROJECTS);
                ws.last_path = Some(path_str.clone());
            }
        }
    } else if let Some(id) = settings.active_workspace_id.clone() {
        if let Some(ws) = settings.workspaces.iter_mut().find(|w| w.id == id) {
            ws.last_path = Some(path_str);
        }
    }

    save_settings(&settings)?;
    Ok(settings)
}

/// Create a new empty workspace and make it active.
pub fn create_workspace() -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    if settings.workspaces.len() >= MAX_WORKSPACES {
        return Err(format!("Maximum of {MAX_WORKSPACES} workspaces"));
    }

    let id = format!("ws_{}", now_ms());
    let _ = workspace_chats_dir(&id)?;
    settings.workspaces.push(WorkspaceEntry {
        id: id.clone(),
        project_paths: Vec::new(),
        last_path: None,
        created_at: now_ms(),
    });
    settings.active_workspace_id = Some(id);
    save_settings(&settings)?;
    Ok(settings)
}

/// Switch active workspace (does not spawn agent — UI opens last/general path).
pub fn set_active_workspace(workspace_id: String) -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    if !settings.workspaces.iter().any(|w| w.id == workspace_id) {
        return Err(format!("Unknown workspace: {workspace_id}"));
    }
    settings.active_workspace_id = Some(workspace_id);
    save_settings(&settings)?;
    Ok(settings)
}

/// Remove a workspace. Cannot delete the last remaining space.
/// Does not delete on-disk chat transcripts (safe); only the space membership.
pub fn delete_workspace(workspace_id: String) -> Result<AppSettings, String> {
    let mut settings = load_settings()?;
    if settings.workspaces.len() <= 1 {
        return Err("Cannot delete the last workspace".to_string());
    }
    if !settings.workspaces.iter().any(|w| w.id == workspace_id) {
        return Err(format!("Unknown workspace: {workspace_id}"));
    }

    settings.workspaces.retain(|w| w.id != workspace_id);

    if settings.active_workspace_id.as_deref() == Some(workspace_id.as_str()) {
        settings.active_workspace_id = settings.workspaces.first().map(|w| w.id.clone());
    }

    save_settings(&settings)?;
    Ok(settings)
}

/// Remove a project from the active workspace list (does not delete transcripts).
pub fn remove_project_from_workspace(project_path: &str) -> Result<AppSettings, String> {
    let path_str = project_path.trim();
    if path_str.is_empty() {
        return Err("Project path must not be empty".to_string());
    }
    if is_general_chats_path(path_str) {
        return Err("Cannot remove the general chats folder".to_string());
    }

    let mut settings = load_settings()?;
    let workspace_id = settings
        .active_workspace_id
        .clone()
        .ok_or_else(|| "No active workspace".to_string())?;
    let ws = settings
        .workspaces
        .iter_mut()
        .find(|w| w.id == workspace_id)
        .ok_or_else(|| format!("Unknown workspace: {workspace_id}"))?;

    let before = ws.project_paths.len();
    ws.project_paths.retain(|p| p != path_str);
    if ws.project_paths.len() == before {
        return Err("Project is not in this workspace".to_string());
    }

    if ws.last_path.as_deref() == Some(path_str) {
        ws.last_path = ws
            .project_paths
            .first()
            .cloned()
            .or_else(|| workspace_chats_dir(&workspace_id).ok().map(|p| p.display().to_string()));
    }

    save_settings(&settings)?;
    Ok(settings)
}

/// Preferred path to open when entering a workspace.
pub fn workspace_entry_path(settings: &AppSettings, workspace_id: &str) -> Result<String, String> {
    let ws = settings
        .workspaces
        .iter()
        .find(|w| w.id == workspace_id)
        .ok_or_else(|| format!("Unknown workspace: {workspace_id}"))?;

    if let Some(last) = &ws.last_path {
        if Path::new(last).exists() || is_general_chats_path(last) {
            return Ok(last.clone());
        }
    }
    if let Some(first) = ws.project_paths.first() {
        return Ok(first.clone());
    }
    Ok(workspace_chats_dir(workspace_id)?.display().to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::is_safe_workspace_id;

    #[test]
    fn default_workspace_id_is_safe() {
        assert!(is_safe_workspace_id("default"));
    }

    #[test]
    fn generated_workspace_ids_are_safe() {
        assert!(is_safe_workspace_id("ws_1700000000000"));
        assert!(is_safe_workspace_id("ws_team-alpha"));
    }

    #[test]
    fn rejects_traversal_and_invalid_ids() {
        assert!(!is_safe_workspace_id(""));
        assert!(!is_safe_workspace_id("ws_.."));
        assert!(!is_safe_workspace_id("ws_../etc"));
        assert!(!is_safe_workspace_id("ws_a/b"));
        assert!(!is_safe_workspace_id("ws_a b"));
        assert!(!is_safe_workspace_id("../default"));
        // Generic strings without the ws_ prefix are also rejected.
        assert!(!is_safe_workspace_id("not_default"));
        assert!(!is_safe_workspace_id(&"x".repeat(65)));
    }
}
