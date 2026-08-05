//! Circulo's own MCP registry.
//!
//! - Durable store: `~/.circulo/mcp.json` (managed by Circulo itself).
//! - Read-only imports from the project's Claude Code `.mcp.json` and
//!   OpenCode `opencode.json` — Circulo never writes into agent configs.
//!
//! The `circulo-mcp` sidecar reads this registry (via `CIRCULO_PROJECT_ROOT` /
//! `~/.circulo`) to know which servers it can load on demand.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MCP_REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum McpServerKind {
    Stdio,
    Http,
    Sse,
}

impl Default for McpServerKind {
    fn default() -> Self {
        Self::Stdio
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpEnvVar {
    pub name: String,
    pub value: String,
}

/// A server entry in Circulo's registry. `built_in` servers (e.g. the
/// orchestrator) are always injected and cannot be deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedMcpServer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: McpServerKind,
    /// Stdio: executable path. Http/Sse: server URL.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<McpEnvVar>,
    /// Eligible for injection (on-demand loaders need this to resolve).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Auto-load servers are injected natively in `session/new` with their full
    /// tool catalogue; the rest stay on-demand via `/mcp name` → `mcp_load`.
    #[serde(default)]
    pub auto_load: bool,
    #[serde(default)]
    pub built_in: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpRegistry {
    version: u32,
    #[serde(default)]
    servers: Vec<ManagedMcpServer>,
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self {
            version: MCP_REGISTRY_VERSION,
            servers: Vec::new(),
        }
    }
}

pub fn mcp_registry_path() -> Result<PathBuf, String> {
    Ok(crate::persistence::circulo_data_dir()?.join("mcp.json"))
}

pub fn load_mcp_servers() -> Result<Vec<ManagedMcpServer>, String> {
    let path = mcp_registry_path()?;
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let registry: McpRegistry = serde_json::from_str(&raw).unwrap_or_default();
    Ok(registry.servers)
}

pub fn save_mcp_servers(servers: &[ManagedMcpServer]) -> Result<Vec<ManagedMcpServer>, String> {
    let registry = McpRegistry {
        version: MCP_REGISTRY_VERSION,
        servers: servers.to_vec(),
    };
    let raw = serde_json::to_string_pretty(&registry).map_err(|err| err.to_string())?;
    std::fs::write(mcp_registry_path()?, raw).map_err(|err| err.to_string())?;
    Ok(servers.to_vec())
}

/// Insert or replace a server by id.
pub fn upsert_mcp_server(server: ManagedMcpServer) -> Result<Vec<ManagedMcpServer>, String> {
    let mut servers = load_mcp_servers()?;
    if let Some(existing) = servers.iter_mut().find(|s| s.id == server.id) {
        *existing = server;
    } else {
        servers.push(server);
    }
    save_mcp_servers(&servers)
}

pub fn delete_mcp_server(id: &str) -> Result<Vec<ManagedMcpServer>, String> {
    let mut servers = load_mcp_servers()?;
    if servers.iter().any(|s| s.id == id && s.built_in) {
        return Err("Built-in servers cannot be deleted".to_string());
    }
    servers.retain(|s| s.id != id);
    save_mcp_servers(&servers)
}

/// Servers marked for native injection into `session/new` (auto-load).
pub fn auto_load_servers() -> Vec<ManagedMcpServer> {
    load_mcp_servers()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.enabled && s.auto_load)
        .collect()
}

// ---------------------------------------------------------------------------
// Imports (read-only) — Claude Code `.mcp.json` and OpenCode `opencode.json`
// ---------------------------------------------------------------------------

/// A server found in the project's own config files, offered for import.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportCandidate {
    pub id: String,
    pub name: String,
    pub kind: McpServerKind,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<McpEnvVar>,
    /// Which config file it came from (`.mcp.json` / `opencode.json`).
    pub source: String,
}

pub fn list_mcp_imports(project_path: &str) -> Result<Vec<McpImportCandidate>, String> {
    let project = PathBuf::from(project_path);
    let mut candidates: Vec<McpImportCandidate> = Vec::new();

    // Claude Code: `.mcp.json` — `{ "mcpServers": { name: { command, args, env, type, url } } }`
    if let Some(servers) = read_object(&project.join(".mcp.json"), &["mcpServers"]) {
        for (name, cfg) in servers {
            if let Some(mut c) = parse_server_cfg(&name, &cfg) {
                c.source = ".mcp.json".to_string();
                candidates.push(c);
            }
        }
    }

    // OpenCode: `opencode.json` / `opencode.jsonc` — `{ "mcp": { name: { type, command, args, env, url } } }`
    for config_name in ["opencode.json", "opencode.jsonc"] {
        if let Some(servers) = read_object(&project.join(config_name), &["mcp"]) {
            for (name, cfg) in servers {
                if let Some(mut c) = parse_server_cfg(&name, &cfg) {
                    c.source = config_name.to_string();
                    candidates.push(c);
                }
            }
        }
    }

    Ok(candidates)
}

/// Reads a named object field from a JSON (or loose JSONC) file.
fn read_object(
    path: &Path,
    keys: &[&str],
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let raw = std::fs::read_to_string(path).ok()?;
    let stripped = strip_jsonc_comments(&raw);
    let value: serde_json::Value = serde_json::from_str(&stripped).ok()?;
    let mut current = value;
    for key in keys {
        current = current.get(*key)?.clone();
    }
    current.as_object().cloned()
}

/// Best-effort JSONC stripping: remove `//` line comments and `/* */` blocks
/// (used by `opencode.jsonc`). Operates outside string literals.
fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn parse_server_cfg(
    name: &str,
    cfg: &serde_json::Value,
) -> Option<McpImportCandidate> {
    let obj = cfg.as_object()?;
    let type_str = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("stdio")
        .to_lowercase();
    let kind = match type_str.as_str() {
        "http" => McpServerKind::Http,
        "sse" => McpServerKind::Sse,
        _ => McpServerKind::Stdio,
    };

    let command = if kind == McpServerKind::Stdio {
        obj.get("command").and_then(|v| v.as_str())?
    } else {
        obj.get("url").and_then(|v| v.as_str())?
    };

    let args = obj
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let env = obj
        .get("env")
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| {
                    v.as_str().map(|value| McpEnvVar {
                        name: k.clone(),
                        value: value.to_string(),
                    })
                })
                .collect::<Vec<McpEnvVar>>()
        })
        .unwrap_or_default();

    Some(McpImportCandidate {
        id: slugify_id(name),
        name: name.to_string(),
        kind,
        command: command.to_string(),
        args,
        env,
        source: String::new(),
    })
}

/// Import a candidate into Circulo's registry (enabled, on-demand).
pub fn import_mcp(candidate: &McpImportCandidate) -> Result<Vec<ManagedMcpServer>, String> {
    let server = ManagedMcpServer {
        id: candidate.id.clone(),
        name: candidate.name.clone(),
        kind: candidate.kind.clone(),
        command: candidate.command.clone(),
        args: candidate.args.clone(),
        env: candidate.env.clone(),
        enabled: true,
        auto_load: false,
        built_in: false,
    };
    upsert_mcp_server(server)
}

/// Normalize a server name into a stable registry id (`[a-z0-9-]`).
pub fn slugify_id(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if c == '_' || c == '-' || c == '.' || c == '/' || c.is_whitespace() {
            out.push('-');
        }
    }
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_dash = false;
    for c in out.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push('-');
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }
    let collapsed = collapsed.trim_matches('-').to_string();
    if collapsed.is_empty() {
        "server".to_string()
    } else {
        collapsed
    }
}

/// Basic validation for a user-entered server (used by the guided form).
pub fn validate_mcp_server(server: &ManagedMcpServer) -> Result<(), String> {
    let name = server.name.trim();
    if name.is_empty() {
        return Err("Name must not be empty".to_string());
    }
    if name.len() > 64 {
        return Err("Name must be at most 64 characters".to_string());
    }
    match server.kind {
        McpServerKind::Stdio => {
            if server.command.trim().is_empty() {
                return Err("Command must not be empty".to_string());
            }
        }
        McpServerKind::Http | McpServerKind::Sse => {
            let url = server.command.trim();
            if url.is_empty() {
                return Err("URL must not be empty".to_string());
            }
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
        }
    }
    for env in &server.env {
        if env.name.trim().is_empty() {
            return Err("Environment variable names must not be empty".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_names() {
        assert_eq!(slugify_id("Paper Server"), "paper-server");
        assert_eq!(slugify_id("  npx__Server/1  "), "npx-server-1");
        assert_eq!(slugify_id("---"), "server");
        assert_eq!(slugify_id("Brave-Search"), "brave-search");
    }

    #[test]
    fn strips_jsonc_comments() {
        let input = r#"{
  // line comment
  "mcp": { "a": 1 } /* block */, "b": "http://x/y#z" // trailing
}"#;
        let out = strip_jsonc_comments(input);
        let value: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(value["mcp"]["a"], 1);
        assert_eq!(value["b"], "http://x/y#z");
    }

    #[test]
    fn parses_claude_mcp_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        std::fs::write(
            &path,
            r#"{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "abc" }
    }
  }
}"#,
        )
        .unwrap();
        let candidates = list_mcp_imports(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        assert_eq!(c.name, "github");
        assert_eq!(c.kind, McpServerKind::Stdio);
        assert_eq!(c.command, "npx");
        assert_eq!(c.args, vec!["-y", "@modelcontextprotocol/server-github"]);
        assert_eq!(c.env[0].name, "GITHUB_TOKEN");
        assert_eq!(c.source, ".mcp.json");
    }

    #[test]
    fn parses_opencode_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(
            &path,
            r#"{
  "mcp": {
    "docs": { "type": "http", "url": "https://example.com/mcp" }
  }
}"#,
        )
        .unwrap();
        let candidates = list_mcp_imports(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].kind, McpServerKind::Http);
        assert_eq!(candidates[0].command, "https://example.com/mcp");
    }

    #[test]
    fn rejects_invalid_servers() {
        let mut server = ManagedMcpServer {
            id: "x".into(),
            name: "".into(),
            kind: McpServerKind::Stdio,
            command: "".into(),
            args: vec![],
            env: vec![],
            enabled: true,
            auto_load: false,
            built_in: false,
        };
        assert!(validate_mcp_server(&server).is_err());
        server.name = "ok".into();
        assert!(validate_mcp_server(&server).is_err());
        server.command = "/bin/sh".into();
        assert!(validate_mcp_server(&server).is_ok());
        server.kind = McpServerKind::Http;
        server.command = "example.com".into();
        assert!(validate_mcp_server(&server).is_err());
        server.command = "https://example.com/mcp".into();
        assert!(validate_mcp_server(&server).is_ok());
    }
}
