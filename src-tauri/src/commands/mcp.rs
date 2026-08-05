//! Tauri commands for Circulo's MCP registry and the `circulo-mcp` sidecar.

use std::path::{Path, PathBuf};

use agent_client_protocol::schema::v1::{
    EnvVariable, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::mcp_client::McpStdioClient;
use crate::persistence::{
    delete_mcp_server, import_mcp, list_mcp_imports, load_mcp_servers, save_mcp_servers,
    upsert_mcp_server, validate_mcp_server, ManagedMcpServer, McpImportCandidate, McpServerKind,
};

#[tauri::command]
pub fn get_mcp_servers_cmd() -> Result<Vec<ManagedMcpServer>, String> {
    load_mcp_servers()
}

/// Add or update a managed server (guided form path — validated before saving).
#[tauri::command]
pub fn upsert_mcp_server_cmd(
    server: ManagedMcpServer,
) -> Result<Vec<ManagedMcpServer>, String> {
    validate_mcp_server(&server)?;
    upsert_mcp_server(server)
}

#[tauri::command]
pub fn delete_mcp_server_cmd(id: String) -> Result<Vec<ManagedMcpServer>, String> {
    delete_mcp_server(&id)
}

/// Toggle a server between on-demand (`enabled`) and native auto-load.
#[tauri::command]
pub fn set_mcp_server_state_cmd(
    id: String,
    enabled: bool,
    auto_load: bool,
) -> Result<Vec<ManagedMcpServer>, String> {
    let mut servers = load_mcp_servers()?;
    let server = servers
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("Unknown MCP server: {id}"))?;
    if server.built_in {
        return Err("Built-in servers cannot be disabled".to_string());
    }
    server.enabled = enabled;
    server.auto_load = enabled && auto_load;
    save_mcp_servers(&servers)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpValidationResult {
    pub ok: bool,
    pub error: Option<String>,
    pub tools: Vec<String>,
    pub tool_count: usize,
}

/// Actually launch the candidate server and list its tools — the guided form's
/// "test before saving" step. Runs in a blocking task to keep the UI free.
#[tauri::command]
pub async fn validate_mcp_server_cmd(
    server: ManagedMcpServer,
) -> Result<McpValidationResult, String> {
    validate_mcp_server(&server)?;

    let (command, args, env) = match server.kind {
        McpServerKind::Http | McpServerKind::Sse => {
            return Ok(McpValidationResult {
                ok: true,
                error: None,
                tools: vec!["http".to_string()],
                tool_count: 1,
            });
        }
        McpServerKind::Stdio => {
            let args = server.args.clone();
            let env: Vec<(String, String)> = server
                .env
                .iter()
                .map(|e| (e.name.clone(), e.value.clone()))
                .collect();
            (server.command.clone(), args, env)
        }
    };

    tauri::async_runtime::spawn_blocking(move || {
        let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
        runtime.block_on(async {
            match McpStdioClient::spawn(&command, &args, &env).await {
                Ok(mut client) => match client.list_tools().await {
                    Ok(tools) => Ok(McpValidationResult {
                        ok: true,
                        error: None,
                        tools: tools.iter().map(|t| t.name.clone()).collect(),
                        tool_count: tools.len(),
                    }),
                    Err(err) => Ok(McpValidationResult {
                        ok: false,
                        error: Some(err),
                        tools: Vec::new(),
                        tool_count: 0,
                    }),
                },
                Err(err) => Ok(McpValidationResult {
                    ok: false,
                    error: Some(err),
                    tools: Vec::new(),
                    tool_count: 0,
                }),
            }
        })
    })
    .await
    .map_err(|err| format!("Validation task failed: {err}"))?
}

#[tauri::command]
pub fn list_mcp_imports_cmd(
    project_path: String,
) -> Result<Vec<McpImportCandidate>, String> {
    list_mcp_imports(&project_path)
}

/// Import a candidate server from the project's `.mcp.json` / `opencode.json`.
#[tauri::command]
pub fn import_mcp_cmd(
    project_path: String,
    server_id: String,
) -> Result<Vec<ManagedMcpServer>, String> {
    let candidates = list_mcp_imports(&project_path)?;
    let candidate = candidates
        .iter()
        .find(|c| c.id == server_id)
        .ok_or_else(|| format!("No importable server with id '{server_id}'"))?;
    import_mcp(candidate)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CirculoMcpStatus {
    pub available: bool,
    pub path: Option<String>,
    pub registry_path: String,
}

/// Whether the orchestrator sidecar binary is resolvable on this machine.
#[tauri::command]
pub fn get_circulo_mcp_status_cmd(
    app: AppHandle,
) -> Result<CirculoMcpStatus, String> {
    let path = resolve_circulo_mcp_binary(&app);
    Ok(CirculoMcpStatus {
        available: path.is_some(),
        path: path.map(|p| p.display().to_string()),
        registry_path: crate::persistence::mcp_registry_path()?
            .display()
            .to_string(),
    })
}

/// The sidecar binary must be a real compiled file — the build script may
/// create an empty placeholder to satisfy `cargo check`/`cargo test`, which
/// must never be injected.
fn is_real_binary(path: &Path) -> bool {
    path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

/// Resolve the `circulo-mcp` sidecar binary.
///
/// Priority:
/// 1. A `circulo-mcp` executable next to the running Circulo binary
///    (`target/debug/circulo-mcp` in dev, `Contents/MacOS/circulo-mcp` bundled).
/// 2. The Tauri resource directory (`BaseDirectory::Resource`).
/// 3. A real sidecar in `src-tauri/binaries/` (dev fallback).
pub fn resolve_circulo_mcp_binary(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("circulo-mcp");
            if is_real_binary(&candidate) {
                return Some(candidate);
            }
        }
    }
    if let Ok(resource) = app.path().resolve(
        "circulo-mcp",
        tauri::path::BaseDirectory::Resource,
    ) {
        if is_real_binary(&resource) {
            return Some(resource);
        }
    }
    // Dev fallback: any real sidecar under `src-tauri/binaries/`.
    let binaries_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
    if let Ok(entries) = std::fs::read_dir(&binaries_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("circulo-mcp-") && is_real_binary(&entry.path()) {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Servers injected into every `session/new`:
/// 1. `circulo-mcp` (orchestrator) — always, when the sidecar binary exists.
/// 2. Auto-load servers with their full tool catalogue (native injection).
pub fn build_injected_mcp_servers(app: &AppHandle, project_path: &Path) -> Vec<McpServer> {
    let mut servers = Vec::new();
    if let Some(sidecar) = resolve_circulo_mcp_binary(app) {
        servers.push(McpServer::Stdio(
            McpServerStdio::new("circulo-mcp", sidecar).env(vec![EnvVariable::new(
                "CIRCULO_PROJECT_ROOT",
                project_path.display().to_string(),
            )]),
        ));
    } else {
        tracing::warn!("circulo-mcp sidecar binary not found — MCP orchestration disabled");
    }
    for managed in crate::persistence::auto_load_servers() {
        match managed.kind {
            McpServerKind::Stdio => servers.push(McpServer::Stdio(
                McpServerStdio::new(managed.name.clone(), managed.command.clone())
                    .args(managed.args.clone())
                    .env(
                        managed
                            .env
                            .iter()
                            .map(|e| EnvVariable::new(e.name.clone(), e.value.clone()))
                            .collect(),
                    ),
            )),
            McpServerKind::Http => servers.push(McpServer::Http(McpServerHttp::new(
                managed.name.clone(),
                managed.command.clone(),
            ))),
            McpServerKind::Sse => servers.push(McpServer::Sse(McpServerSse::new(
                managed.name.clone(),
                managed.command.clone(),
            ))),
        }
    }
    servers
}
