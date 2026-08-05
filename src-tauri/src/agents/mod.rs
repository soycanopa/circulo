use std::path::Path;

use agent_client_protocol::AcpAgent;
use serde::Serialize;

use crate::cli_resolve::{
    resolve_cursor_agent, resolve_grok, resolve_npx, resolve_opencode, resolve_pi, resolve_pi_acp,
};

pub const AGENT_ID_OPENCODE: &str = "opencode";
pub const AGENT_ID_CURSOR: &str = "cursor-agent";
pub const AGENT_ID_GROK: &str = "grok";
pub const AGENT_ID_PI: &str = "pi";
pub const AGENT_ID_CUSTOM: &str = "custom";
pub const DEFAULT_AGENT_ID: &str = AGENT_ID_OPENCODE;

const CUSTOM_ACP_ENV: &str = "CIRCULO_CUSTOM_ACP";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDescriptor {
    pub id: String,
    pub label: String,
    pub command: String,
    pub available: bool,
}

pub fn list_agents() -> Vec<AgentDescriptor> {
    let mut agents = Vec::new();

    let opencode_available = resolve_opencode().is_ok();
    agents.push(AgentDescriptor {
        id: AGENT_ID_OPENCODE.to_string(),
        label: "OpenCode".to_string(),
        command: "opencode acp".to_string(),
        available: opencode_available,
    });

    let cursor_available = resolve_cursor_agent().is_ok();
    agents.push(AgentDescriptor {
        id: AGENT_ID_CURSOR.to_string(),
        label: "Cursor Agent".to_string(),
        command: "cursor-agent acp".to_string(),
        available: cursor_available,
    });

    let grok_available = resolve_grok().is_ok();
    agents.push(AgentDescriptor {
        id: AGENT_ID_GROK.to_string(),
        label: "Grok".to_string(),
        command: "grok agent stdio".to_string(),
        available: grok_available,
    });

    let pi_available = pi_agent_available();
    agents.push(AgentDescriptor {
        id: AGENT_ID_PI.to_string(),
        label: "Pi".to_string(),
        command: pi_agent_command_label(),
        available: pi_available,
    });

    if let Some((program, args)) = custom_agent_spec() {
        let command = format!("{} {}", program, args.join(" "));
        let available = command_exists(&program);
        agents.push(AgentDescriptor {
            id: AGENT_ID_CUSTOM.to_string(),
            label: "Custom ACP".to_string(),
            command,
            available,
        });
    }

    agents
}

pub fn is_known_agent_id(agent_id: &str) -> bool {
    list_agents().iter().any(|a| a.id == agent_id)
}

pub fn normalize_agent_id(agent_id: Option<&str>) -> &'static str {
    match agent_id {
        Some(id) if id == AGENT_ID_OPENCODE => AGENT_ID_OPENCODE,
        Some(id) if id == AGENT_ID_CURSOR => AGENT_ID_CURSOR,
        Some(id) if id == AGENT_ID_GROK => AGENT_ID_GROK,
        Some(id) if id == AGENT_ID_PI => AGENT_ID_PI,
        Some(id) if id == AGENT_ID_CUSTOM && custom_agent_spec().is_some() => AGENT_ID_CUSTOM,
        Some(_) | None => DEFAULT_AGENT_ID,
    }
}

/// Pick the agent to use: preferred if enabled and known, else first enabled+available.
pub fn resolve_enabled_agent_id(
    preferred: Option<&str>,
    enabled_ids: &[String],
) -> Result<String, String> {
    if enabled_ids.is_empty() {
        return Err("At least one agent must be enabled".to_string());
    }

    let agents = list_agents();
    let enabled_set: std::collections::HashSet<&str> =
        enabled_ids.iter().map(|s| s.as_str()).collect();

    if let Some(pref) = preferred {
        let normalized = normalize_agent_id(Some(pref));
        if enabled_set.contains(normalized) {
            if let Some(hit) = agents.iter().find(|a| a.id == normalized) {
                if hit.available {
                    return Ok(normalized.to_string());
                }
            }
        }
    }

    for id in enabled_ids {
        if let Some(hit) = agents.iter().find(|a| a.id == id.as_str()) {
            if hit.available {
                return Ok(hit.id.clone());
            }
        }
    }

    Err("No enabled agent is available on this system".to_string())
}

pub fn agent_command_label(agent_id: &str) -> String {
    list_agents()
        .into_iter()
        .find(|a| a.id == agent_id)
        .map(|a| a.command)
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn agent_progress_label(agent_id: &str) -> String {
    list_agents()
        .into_iter()
        .find(|a| a.id == agent_id)
        .map(|a| a.label)
        .unwrap_or_else(|| agent_id.to_string())
}

pub fn build_agent(agent_id: &str, _project_path: &Path) -> Result<AcpAgent, String> {
    match agent_id {
        AGENT_ID_OPENCODE => build_opencode_agent(),
        AGENT_ID_CURSOR => build_cursor_agent(),
        AGENT_ID_GROK => build_grok_agent(),
        AGENT_ID_PI => build_pi_agent(),
        AGENT_ID_CUSTOM => build_custom_agent(),
        other => Err(format!("Unsupported agent: {other}")),
    }
}

pub fn ensure_agent_available(agent_id: &str) -> Result<(), String> {
    let hit = list_agents()
        .into_iter()
        .find(|a| a.id == agent_id)
        .ok_or_else(|| format!("Unknown agent: {agent_id}"))?;
    if !hit.available {
        return Err(format!(
            "Agent '{}' is not available ({})",
            hit.label, hit.command
        ));
    }
    Ok(())
}

/// Spawn OpenCode exactly as documented for ACP clients (Zed / JetBrains / nvim):
/// `command: "opencode"`, `args: ["acp"]` over stdio.
pub fn build_opencode_agent() -> Result<AcpAgent, String> {
    let opencode = resolve_opencode()?;
    AcpAgent::from_args([opencode.display().to_string(), "acp".to_string()])
        .map_err(|err| format!("Failed to configure OpenCode agent: {err}"))
}

pub fn build_cursor_agent() -> Result<AcpAgent, String> {
    let cursor = resolve_cursor_agent()?;
    AcpAgent::from_args([cursor.display().to_string(), "acp".to_string()])
        .map_err(|err| format!("Failed to configure Cursor Agent: {err}"))
}

pub fn build_grok_agent() -> Result<AcpAgent, String> {
    let grok = resolve_grok()?;
    AcpAgent::from_args([
        grok.display().to_string(),
        "agent".to_string(),
        "stdio".to_string(),
    ])
    .map_err(|err| format!("Failed to configure Grok agent: {err}"))
}

/// Pi via the community ACP adapter (`pi-acp`), same as Zed's registry entry.
pub fn build_pi_agent() -> Result<AcpAgent, String> {
    resolve_pi()?;

    if let Ok(pi_acp) = resolve_pi_acp() {
        return AcpAgent::from_args([pi_acp.display().to_string()])
            .map_err(|err| format!("Failed to configure Pi agent: {err}"));
    }

    let npx = resolve_npx()?;
    AcpAgent::from_args([
        npx.display().to_string(),
        "-y".to_string(),
        "pi-acp".to_string(),
    ])
    .map_err(|err| format!("Failed to configure Pi agent (npx pi-acp): {err}"))
}

fn pi_agent_available() -> bool {
    resolve_pi().is_ok() && (resolve_pi_acp().is_ok() || resolve_npx().is_ok())
}

fn pi_agent_command_label() -> String {
    if resolve_pi_acp().is_ok() {
        "pi-acp".to_string()
    } else {
        "npx -y pi-acp".to_string()
    }
}

fn build_custom_agent() -> Result<AcpAgent, String> {
    let (program, args) =
        custom_agent_spec().ok_or_else(|| format!("Set {CUSTOM_ACP_ENV} to enable custom agent"))?;
    let mut argv = vec![program];
    argv.extend(args);
    AcpAgent::from_args(argv).map_err(|err| format!("Failed to configure custom agent: {err}"))
}

fn custom_agent_spec() -> Option<(String, Vec<String>)> {
    let raw = std::env::var(CUSTOM_ACP_ENV).ok()?;
    let mut parts = raw.split_whitespace().map(str::to_string);
    let program = parts.next()?;
    if program.is_empty() {
        return None;
    }
    Some((program, parts.collect()))
}

fn command_exists(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_builtin_agents() {
        let agents = list_agents();
        assert!(agents.iter().any(|a| a.id == AGENT_ID_OPENCODE));
        assert!(agents.iter().any(|a| a.id == AGENT_ID_CURSOR));
        assert!(agents.iter().any(|a| a.id == AGENT_ID_GROK));
        assert!(agents.iter().any(|a| a.id == AGENT_ID_PI));
    }

    #[test]
    fn normalize_cursor_agent_id() {
        assert_eq!(
            normalize_agent_id(Some(AGENT_ID_CURSOR)),
            AGENT_ID_CURSOR
        );
    }

    #[test]
    fn normalize_grok_agent_id() {
        assert_eq!(normalize_agent_id(Some(AGENT_ID_GROK)), AGENT_ID_GROK);
    }

    #[test]
    fn normalize_pi_agent_id() {
        assert_eq!(normalize_agent_id(Some(AGENT_ID_PI)), AGENT_ID_PI);
    }
}
