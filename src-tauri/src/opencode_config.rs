use std::collections::HashMap;
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
pub struct CommandEntryDto {
    pub name: String,
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
    pub source: String,
    pub config_path: String,
    pub read_only: bool,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn global_config_dir() -> PathBuf {
    home_dir().join(".config").join("opencode")
}

fn managed_config_dir() -> PathBuf {
    PathBuf::from("/Library/Application Support/opencode")
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

fn parse_frontmatter_description(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("description:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn list_commands_in_dir(commands_dir: &Path, scope: &str) -> Vec<CommandEntryDto> {
    if !commands_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(commands_dir) else {
        return entries;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext != "md" {
            continue;
        }
        let Some(name) = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
        else {
            continue;
        };
        let description = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| parse_frontmatter_description(&content));
        entries.push(CommandEntryDto {
            name,
            description,
            scope: scope.to_string(),
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn parse_commands_from_config(value: &Value, scope: &str) -> Vec<CommandEntryDto> {
    let Some(command) = value.get("command").and_then(Value::as_object) else {
        return Vec::new();
    };

    let mut entries: Vec<CommandEntryDto> = command
        .iter()
        .map(|(name, config)| {
            let description = config
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string);
            CommandEntryDto {
                name: name.clone(),
                description,
                scope: scope.to_string(),
            }
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn load_config_commands(base: &Path, scope: &str) -> Vec<CommandEntryDto> {
    let mut entries = Vec::new();
    for name in ["opencode.json", "opencode.jsonc"] {
        let path = base.join(name);
        if let Some(value) = load_json_config(&path) {
            entries.extend(parse_commands_from_config(&value, scope));
            break;
        }
    }
    entries
}

pub fn list_opencode_commands(project_path: Option<String>) -> Vec<CommandEntryDto> {
    let global_dir = global_config_dir();
    let mut commands = list_commands_in_dir(&global_dir.join("commands"), "global");
    commands.extend(load_config_commands(&global_dir, "global"));

    if let Some(path) = project_path {
        let project = PathBuf::from(path);
        commands.extend(list_commands_in_dir(
            &project.join(".opencode").join("commands"),
            "project",
        ));
        if let Some(config_path) = project_config_path(&project) {
            if let Some(parent) = config_path.parent() {
                commands.extend(load_config_commands(parent, "project"));
            }
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands
}

pub fn list_opencode_skills(project_path: Option<String>) -> Vec<SkillEntryDto> {
    let mut skills = list_skills_in_dir(&global_config_dir(), "global");

    if let Some(path) = project_path {
        let project = PathBuf::from(path);
        skills.extend(list_skills_in_dir(&project.join(".opencode"), "project"));
        skills.extend(list_skills_in_dir(&project.join(".agents"), "project"));
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn load_json_config(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn infer_server_type(config: &Value) -> Option<String> {
    if config.get("url").is_some() {
        return Some("remote".to_string());
    }
    if config.get("command").is_some() {
        return Some("local".to_string());
    }
    None
}

fn parse_enabled_flag(config: &Value) -> bool {
    if let Some(disabled) = config.get("disabled").and_then(Value::as_bool) {
        return !disabled;
    }
    config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_server_type(config: &Value) -> Option<String> {
    config
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| infer_server_type(config))
}

fn parse_mcp_object(
    mcp: &Map<String, Value>,
    scope: &str,
    source: &str,
    config_path: &Path,
    read_only: bool,
) -> Vec<McpServerEntryDto> {
    let path_label = config_path.display().to_string();
    let mut entries: Vec<McpServerEntryDto> = mcp
        .iter()
        .map(|(name, config)| McpServerEntryDto {
            name: name.clone(),
            enabled: parse_enabled_flag(config),
            scope: scope.to_string(),
            server_type: parse_server_type(config),
            source: source.to_string(),
            config_path: path_label.clone(),
            read_only,
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn parse_mcp_from_opencode_config(
    value: &Value,
    scope: &str,
    config_path: &Path,
) -> Vec<McpServerEntryDto> {
    let Some(mcp) = value.get("mcp").and_then(Value::as_object) else {
        return Vec::new();
    };
    parse_mcp_object(mcp, scope, "opencode", config_path, false)
}

fn parse_mcp_from_mcp_servers_config(
    value: &Value,
    scope: &str,
    source: &str,
    config_path: &Path,
) -> Vec<McpServerEntryDto> {
    let Some(mcp) = value.get("mcpServers").and_then(Value::as_object) else {
        return Vec::new();
    };
    parse_mcp_object(mcp, scope, source, config_path, true)
}

fn merge_opencode_entries(
    merged: &mut HashMap<(String, String), McpServerEntryDto>,
    entries: Vec<McpServerEntryDto>,
) {
    for entry in entries {
        let key = (entry.scope.clone(), entry.name.clone());
        merged.insert(key, entry);
    }
}

fn load_opencode_mcp_from_dir(base: &Path, scope: &str) -> Vec<McpServerEntryDto> {
    let mut entries = Vec::new();
    for name in ["opencode.json", "opencode.jsonc"] {
        let path = base.join(name);
        if let Some(value) = load_json_config(&path) {
            entries.extend(parse_mcp_from_opencode_config(&value, scope, &path));
        }
    }
    entries
}

fn git_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn project_config_dirs(project: &Path) -> Vec<PathBuf> {
    let root = git_repo_root(project).unwrap_or_else(|| project.to_path_buf());
    let Ok(relative) = project.strip_prefix(&root) else {
        return vec![project.to_path_buf()];
    };

    let mut dirs = vec![root.clone()];
    let mut accum = root;
    for component in relative.components() {
        if let std::path::Component::Normal(name) = component {
            accum = accum.join(name);
            dirs.push(accum.clone());
        }
    }
    dirs
}

fn project_config_path(project: &Path) -> Option<PathBuf> {
    for name in ["opencode.json", "opencode.jsonc"] {
        let path = project.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn primary_opencode_config_path(base: &Path) -> PathBuf {
    let jsonc = base.join("opencode.jsonc");
    if jsonc.is_file() {
        return jsonc;
    }
    let json = base.join("opencode.json");
    if json.is_file() {
        return json;
    }
    jsonc
}

fn collect_opencode_mcp(project_path: Option<&Path>) -> Vec<McpServerEntryDto> {
    let mut merged: HashMap<(String, String), McpServerEntryDto> = HashMap::new();

    let mut global_sources: Vec<(PathBuf, &str)> = Vec::new();
    global_sources.push((managed_config_dir(), "managed"));
    global_sources.push((global_config_dir(), "global"));

    if let Ok(custom) = std::env::var("OPENCODE_CONFIG") {
        let custom_path = PathBuf::from(custom);
        if custom_path.is_dir() {
            global_sources.push((custom_path, "global"));
        } else if custom_path.is_file() {
            if let Some(value) = load_json_config(&custom_path) {
                merge_opencode_entries(
                    &mut merged,
                    parse_mcp_from_opencode_config(&value, "global", &custom_path),
                );
            }
        } else if let Some(parent) = custom_path.parent() {
            global_sources.push((parent.to_path_buf(), "global"));
        }
    }

    for (dir, scope) in global_sources {
        merge_opencode_entries(&mut merged, load_opencode_mcp_from_dir(&dir, scope));
    }

    if let Some(project) = project_path {
        for dir in project_config_dirs(project) {
            merge_opencode_entries(&mut merged, load_opencode_mcp_from_dir(&dir, "project"));
        }
    }

    let mut entries: Vec<McpServerEntryDto> = merged.into_values().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn collect_external_mcp(project_path: Option<&Path>) -> Vec<McpServerEntryDto> {
    let mut entries = Vec::new();

    let cursor_global = home_dir().join(".cursor").join("mcp.json");
    if let Some(value) = load_json_config(&cursor_global) {
        entries.extend(parse_mcp_from_mcp_servers_config(
            &value,
            "global",
            "cursor",
            &cursor_global,
        ));
    }

    let claude_desktop = home_dir()
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");
    if let Some(value) = load_json_config(&claude_desktop) {
        entries.extend(parse_mcp_from_mcp_servers_config(
            &value,
            "global",
            "claude",
            &claude_desktop,
        ));
    }

    let claude_code = home_dir().join(".claude.json");
    if let Some(value) = load_json_config(&claude_code) {
        entries.extend(parse_mcp_from_mcp_servers_config(
            &value,
            "global",
            "claude",
            &claude_code,
        ));
    }

    let minimax = home_dir().join(".minimax").join("mcp").join("mcp.json");
    if let Some(value) = load_json_config(&minimax) {
        if let Some(mcp) = value.get("mcpServers").and_then(Value::as_object) {
            entries.extend(parse_mcp_object(
                mcp,
                "global",
                "minimax",
                &minimax,
                true,
            ));
        } else if let Some(mcp) = value.get("mcp").and_then(Value::as_object) {
            entries.extend(parse_mcp_object(mcp, "global", "minimax", &minimax, true));
        }
    }

    if let Some(project) = project_path {
        let cursor_project = project.join(".cursor").join("mcp.json");
        if let Some(value) = load_json_config(&cursor_project) {
            entries.extend(parse_mcp_from_mcp_servers_config(
                &value,
                "project",
                "cursor",
                &cursor_project,
            ));
        }
    }

    entries.sort_by(|a, b| {
        a.scope
            .cmp(&b.scope)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

pub fn list_opencode_mcp_servers(project_path: Option<String>) -> Vec<McpServerEntryDto> {
    let project = project_path.as_deref().map(Path::new);
    let mut servers = collect_opencode_mcp(project);
    servers.extend(collect_external_mcp(project));
    servers
}

pub fn set_opencode_mcp_enabled(
    name: String,
    scope: String,
    enabled: bool,
    project_path: Option<String>,
    config_path: Option<String>,
) -> Result<(), String> {
    let config_path = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        match scope.as_str() {
            "global" | "managed" => {
                let dir = if scope == "managed" {
                    managed_config_dir()
                } else {
                    global_config_dir()
                };
                std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
                primary_opencode_config_path(&dir)
            }
            "project" => {
                let path = project_path.ok_or_else(|| "Project path required".to_string())?;
                let project = PathBuf::from(path);
                project_config_path(&project).ok_or_else(|| {
                    "No opencode.json or opencode.jsonc found in the project root".to_string()
                })?
            }
            _ => return Err("Invalid MCP scope".to_string()),
        }
    };

    if !config_path.is_file() {
        let parent = config_path
            .parent()
            .ok_or_else(|| "Invalid config path".to_string())?;
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn merges_opencode_json_and_jsonc_in_same_dir() {
        let temp = std::env::temp_dir().join(format!(
            "circulo-mcp-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).expect("create temp dir");

        fs::write(
            temp.join("opencode.json"),
            r#"{"mcp":{"alpha":{"type":"local","enabled":true}}}"#,
        )
        .expect("write json");
        fs::write(
            temp.join("opencode.jsonc"),
            r#"{"mcp":{"beta":{"type":"remote","url":"https://example.com","enabled":false},"alpha":{"enabled":false}}}"#,
        )
        .expect("write jsonc");

        let mut merged: HashMap<(String, String), McpServerEntryDto> = HashMap::new();
        merge_opencode_entries(&mut merged, load_opencode_mcp_from_dir(&temp, "global"));

        let by_name: HashMap<String, McpServerEntryDto> = merged
            .into_values()
            .map(|entry| (entry.name.clone(), entry))
            .collect();

        assert_eq!(by_name.len(), 2);
        assert_eq!(by_name["alpha"].enabled, false);
        assert_eq!(by_name["beta"].enabled, false);
        assert_eq!(by_name["alpha"].source, "opencode");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn parses_cursor_mcp_servers_format() {
        let temp = std::env::temp_dir().join(format!(
            "circulo-cursor-mcp-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).expect("create temp dir");
        let path = temp.join("mcp.json");
        fs::write(
            &path,
            r#"{"mcpServers":{"paper":{"url":"http://127.0.0.1:29979/mcp"},"shell":{"command":"bash","args":["-lc","mcp"]}}}"#,
        )
        .expect("write cursor config");

        let value = load_json_config(&path).expect("load json");
        let entries = parse_mcp_from_mcp_servers_config(&value, "global", "cursor", &path);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| entry.read_only));
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.name == "paper")
                .and_then(|entry| entry.server_type.as_deref()),
            Some("remote")
        );

        let _ = fs::remove_dir_all(&temp);
    }
}