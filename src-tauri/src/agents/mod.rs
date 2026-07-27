use std::path::Path;

use agent_client_protocol::AcpAgent;

use crate::cli_resolve::resolve_opencode;

pub const AGENT_ID_OPENCODE: &str = "opencode";
pub const DEFAULT_AGENT_ID: &str = AGENT_ID_OPENCODE;

/// Disable design-oriented MCP servers when Circulo drives OpenCode as a coding agent.
const OPENCODE_CONFIG_CONTENT: &str = r#"{"$schema":"https://opencode.ai/config.json","tools":{"paper_*":false,"figma_*":false,"Framelink_Figma_MCP_*":false,"craft-business_*":false,"craft-personal_*":false},"mcp":{"paper":{"enabled":false},"figma":{"enabled":false},"Framelink_Figma_MCP":{"enabled":false},"craft-business":{"enabled":false},"craft-personal":{"enabled":false}}}"#;

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

pub fn build_agent(agent_id: &str, project_path: &Path) -> Result<AcpAgent, String> {
    match agent_id {
        AGENT_ID_OPENCODE => build_opencode_agent(project_path),
        other => Err(format!("Agente no soportado todavía: {other}")),
    }
}

pub fn build_opencode_agent(project_path: &Path) -> Result<AcpAgent, String> {
    let opencode = resolve_opencode()?;
    AcpAgent::from_args([
        format!("OPENCODE_CONFIG_CONTENT={OPENCODE_CONFIG_CONTENT}"),
        opencode.display().to_string(),
        "acp".to_string(),
        "--cwd".to_string(),
        project_path.display().to_string(),
    ])
    .map_err(|err| format!("Failed to configure OpenCode agent: {err}"))
}
