use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::circulo_data_dir;

const SETTINGS_VERSION: u32 = 1;
const MAX_RECENT_PROJECTS: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub last_opened_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    pub recent_projects: Vec<RecentProject>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            recent_projects: Vec::new(),
        }
    }
}

fn settings_path() -> Result<std::path::PathBuf, String> {
    Ok(circulo_data_dir()?.join("config.json"))
}

pub fn load_settings() -> Result<AppSettings, String> {
    let path = settings_path()?;
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| format!("Invalid config.json: {err}"))
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    let mut next = settings.clone();
    next.version = SETTINGS_VERSION;
    let raw = serde_json::to_string_pretty(&next).map_err(|err| err.to_string())?;
    std::fs::write(path, raw).map_err(|err| err.to_string())
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
            path: path_str,
            last_opened_at: now,
        },
    );
    settings.recent_projects.truncate(MAX_RECENT_PROJECTS);
    save_settings(&settings)?;
    Ok(settings)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
