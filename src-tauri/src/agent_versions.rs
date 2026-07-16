use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::cli_resolve::{display_command, resolve_grok, resolve_opencode};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentVersionInfo {
    pub id: String,
    pub label: String,
    pub command: String,
    pub installed: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

fn probe_resolved(id: &str, label: &str, program: &Path, args: &[&str]) -> AgentVersionInfo {
    let output = Command::new(program).args(args).output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
            let version = if !stdout.is_empty() {
                stdout.lines().next().unwrap_or(&stdout).to_string()
            } else if !stderr.is_empty() {
                stderr.lines().next().unwrap_or(&stderr).to_string()
            } else {
                "installed".to_string()
            };
            AgentVersionInfo {
                id: id.to_string(),
                label: label.to_string(),
                command: display_command(program, args),
                installed: true,
                version: Some(version),
                error: None,
            }
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            let message = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("exit status {}", result.status)
            };
            AgentVersionInfo {
                id: id.to_string(),
                label: label.to_string(),
                command: display_command(program, args),
                installed: false,
                version: None,
                error: Some(message),
            }
        }
        Err(err) => AgentVersionInfo {
            id: id.to_string(),
            label: label.to_string(),
            command: display_command(program, args),
            installed: false,
            version: None,
            error: Some(err.to_string()),
        },
    }
}

pub fn list_agent_provider_versions() -> Vec<AgentVersionInfo> {
    vec![
        match resolve_opencode() {
            Ok(path) => probe_resolved("opencode", "OpenCode", &path, &["--version"]),
            Err(err) => AgentVersionInfo {
                id: "opencode".to_string(),
                label: "OpenCode".to_string(),
                command: "opencode --version".to_string(),
                installed: false,
                version: None,
                error: Some(err),
            },
        },
        match resolve_grok() {
            Ok(path) => probe_resolved("grok", "Grok Build", &path, &["--version"]),
            Err(err) => AgentVersionInfo {
                id: "grok".to_string(),
                label: "Grok Build".to_string(),
                command: "grok --version".to_string(),
                installed: false,
                version: None,
                error: Some(err),
            },
        },
    ]
}