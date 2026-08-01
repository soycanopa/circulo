use std::path::Path;

use agent_client_protocol::AcpAgent;

use crate::cli_resolve::resolve_opencode;

pub const AGENT_ID_OPENCODE: &str = "opencode";
pub const DEFAULT_AGENT_ID: &str = AGENT_ID_OPENCODE;

pub fn normalize_agent_id(agent_id: Option<&str>) -> &'static str {
    match agent_id {
        Some(id) if id == AGENT_ID_OPENCODE => AGENT_ID_OPENCODE,
        Some(_) | None => DEFAULT_AGENT_ID,
    }
}

pub fn agent_command_label(agent_id: &str) -> &'static str {
    match agent_id {
        AGENT_ID_OPENCODE => "opencode acp",
        _ => "unknown",
    }
}

pub fn build_agent(agent_id: &str, _project_path: &Path) -> Result<AcpAgent, String> {
    match agent_id {
        AGENT_ID_OPENCODE => build_opencode_agent(),
        other => Err(format!("Agente no soportado todavía: {other}")),
    }
}

/// Spawn OpenCode exactly as documented for ACP clients (Zed / JetBrains / nvim):
/// `command: "opencode"`, `args: ["acp"]` over stdio.
///
/// Working directory is **not** set on the process — ACP requires absolute `cwd`
/// on `session/new` (see agentclientprotocol.com session-setup).
///
/// Source: https://opencode.ai/docs/acp/
pub fn build_opencode_agent() -> Result<AcpAgent, String> {
    let opencode = resolve_opencode()?;
    AcpAgent::from_args([opencode.display().to_string(), "acp".to_string()])
        .map_err(|err| format!("Failed to configure OpenCode agent: {err}"))
}
