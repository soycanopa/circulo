use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntryDto {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub scope: String,
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpServerEntryDto {
    pub name: String,
    pub enabled: bool,
    pub scope: String,
    pub server_type: Option<String>,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn global_config_dir() -> PathBuf {
    home_dir().join(".config").join("opencode")
}

fn read_skill_description(skill_dir: &Path) -> Option<String> {
    let skill_md = skill_dir.join("SKILL.md");
    let content = std::fs::read_to_string(skill_md).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let title = trimmed.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
        if !trimmed.is_empty() {
            return Some(trimmed.chars().take(140).collect());
        }
    }
    None
}

fn list_skills_in_dir(base: &Path, scope: &str) -> Vec<SkillEntryDto> {
    let skills_dir = base.join("skills");
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(&skills_dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(SkillEntryDto {
            name: name.clone(),
            description: read_skill_description(&path),
            path: path.display().to_string(),
            scope: scope.to_string(),
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

pub fn list_opencode_skills(project_path: Option<String>) -> Vec<SkillEntryDto> {
    let mut skills = list_skills_in_dir(&global_config_dir(), "global");

    if let Some(path) = project_path {
        let project = PathBuf::from(path);
        skills.extend(list_skills_in_dir(&project.join(".opencode"), "project"));
    }

    skills
}

fn load_json_config(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn parse_mcp_from_config(value: &Value, scope: &str) -> Vec<McpServerEntryDto> {
    let Some(mcp) = value.get("mcp").and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut entries: Vec<McpServerEntryDto> = mcp
        .iter()
        .map(|(name, config)| {
            let enabled = config
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let server_type = config
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string);
            McpServerEntryDto {
                name: name.clone(),
                enabled,
                scope: scope.to_string(),
                server_type,
            }
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn project_config_path(project: &Path) -> Option<PathBuf> {
    let json = project.join("opencode.json");
    if json.is_file() {
        return Some(json);
    }
    let jsonc = project.join("opencode.jsonc");
    if jsonc.is_file() {
        return Some(jsonc);
    }
    None
}

pub fn list_opencode_mcp_servers(project_path: Option<String>) -> Vec<McpServerEntryDto> {
    let mut servers = Vec::new();
    let global_dir = global_config_dir();

    for name in ["opencode.json", "opencode.jsonc"] {
        let path = global_dir.join(name);
        if let Some(value) = load_json_config(&path) {
            servers.extend(parse_mcp_from_config(&value, "global"));
            break;
        }
    }

    if let Some(path) = project_path {
        let project = PathBuf::from(path);
        if let Some(config_path) = project_config_path(&project) {
            if let Some(value) = load_json_config(&config_path) {
                servers.extend(parse_mcp_from_config(&value, "project"));
            }
        }
    }

    servers
}

pub fn set_opencode_mcp_enabled(
    name: String,
    scope: String,
    enabled: bool,
    project_path: Option<String>,
) -> Result<(), String> {
    let config_path = match scope.as_str() {
        "global" => {
            let dir = global_config_dir();
            std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
            let json = dir.join("opencode.json");
            if json.is_file() {
                json
            } else {
                dir.join("opencode.jsonc")
            }
        }
        "project" => {
            let path = project_path.ok_or_else(|| "Project path required".to_string())?;
            let project = PathBuf::from(path);
            project_config_path(&project).ok_or_else(|| {
                "No opencode.json or opencode.jsonc found in the project root".to_string()
            })?
        }
        _ => return Err("Invalid MCP scope".to_string()),
    };

    let mut value = if config_path.is_file() {
        let content = std::fs::read_to_string(&config_path).map_err(|err| err.to_string())?;
        serde_json::from_str(&content).map_err(|err| err.to_string())?
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {}
        })
    };

    let root = value
        .as_object_mut()
        .ok_or_else(|| "Invalid config root".to_string())?;
    let mcp = root
        .entry("mcp")
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp_obj = mcp
        .as_object_mut()
        .ok_or_else(|| "Invalid mcp object".to_string())?;
    let entry = mcp_obj
        .entry(name)
        .or_insert_with(|| serde_json::json!({ "type": "local" }));
    let entry_obj = entry
        .as_object_mut()
        .ok_or_else(|| "Invalid MCP entry".to_string())?;
    entry_obj.insert("enabled".to_string(), Value::Bool(enabled));

    let content = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    std::fs::write(&config_path, content).map_err(|err| err.to_string())?;
    Ok(())
}