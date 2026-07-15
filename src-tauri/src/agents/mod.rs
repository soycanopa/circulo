use std::path::Path;

use agent_client_protocol::AcpAgent;

pub const DEFAULT_AGENT_COMMAND: &str = "opencode acp";

/// Design-oriented MCP servers that should not be available when Forge runs OpenCode.
/// Paper/Figma/Craft are for the IDE design workflow (Grok/Cursor), not coding tasks.
const FORGE_OPENCODE_CONFIG_CONTENT: &str = r#"{"$schema":"https://opencode.ai/config.json","tools":{"paper_*":false,"figma_*":false,"Framelink_Figma_MCP_*":false,"craft-business_*":false,"craft-personal_*":false},"mcp":{"paper":{"enabled":false},"figma":{"enabled":false},"Framelink_Figma_MCP":{"enabled":false},"craft-business":{"enabled":false},"craft-personal":{"enabled":false}}}"#;

pub fn build_opencode_agent(project_path: &Path) -> Result<AcpAgent, String> {
    AcpAgent::from_args([
        format!("OPENCODE_CONFIG_CONTENT={FORGE_OPENCODE_CONFIG_CONTENT}"),
        "opencode".to_string(),
        "acp".to_string(),
        "--cwd".to_string(),
        project_path.display().to_string(),
    ])
    .map_err(|err| format!("Failed to configure OpenCode agent: {err}"))
}