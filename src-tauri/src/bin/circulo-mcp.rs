//! `circulo-mcp` — the MCP orchestrator sidecar.
//!
//! Injected once into every ACP `session/new` (stdio transport, mandatory for
//! all ACP agents). It reads Circulo's own registry (`~/.circulo/mcp.json`)
//! and exposes three tool families:
//!
//! - **Orchestrator**: `mcp_list`, `mcp_load(name)`, `mcp_call(name, tool, args)`
//!   — on-demand loading of the user's registered MCP servers.
//! - **Token-optimizer**: `estimate_tokens`, `compact_result`,
//!   `retrieve_original`, `summarize` — reversible output compression.
//! - **Semantic**: `find_symbol`, `get_references`, `outline` — scoped to the
//!   project root (via `CIRCULO_PROJECT_ROOT`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use circulo_lib::mcp_client::{tool_result_text, McpStdioClient, McpToolInfo};
use circulo_lib::persistence::{load_mcp_servers, ManagedMcpServer, McpServerKind};
use circulo_lib::semantic;
use circulo_lib::token_optimizer;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const SERVER_NAME: &str = "circulo-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

type LoadedClients = Arc<Mutex<HashMap<String, Arc<Mutex<McpStdioClient>>>>>;
type LoadedTools = Arc<Mutex<HashMap<String, Vec<McpToolInfo>>>>;

#[tokio::main]
async fn main() {
    let registry = load_mcp_servers().unwrap_or_default();
    let clients: LoadedClients = Arc::new(Mutex::new(HashMap::new()));
    let tools_cache: LoadedTools = Arc::new(Mutex::new(HashMap::new()));

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let request: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let id = request.get("id").cloned();
                let method = request
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or_default()
                    .to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                if method == "notifications/initialized" || method.starts_with("notifications/") {
                    continue;
                }

                let result = handle_method(
                    method.as_str(),
                    &params,
                    &registry,
                    &clients,
                    &tools_cache,
                )
                .await;

                let response = match (id, result) {
                    (Some(id), Ok(result)) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                    (Some(id), Err(err)) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32000, "message": err }
                    }),
                    (None, _) => continue,
                };
                let mut raw = serde_json::to_vec(&response).unwrap_or_default();
                raw.push(b'\n');
                let _ = stdout.write_all(&raw).await;
                let _ = stdout.flush().await;
            }
            Err(_) => break,
        }
    }
}

async fn handle_method(
    method: &str,
    params: &Value,
    registry: &[ManagedMcpServer],
    clients: &LoadedClients,
    tools_cache: &LoadedTools,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_catalog() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "tool name required".to_string())?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            call_tool(name, &args, registry, clients, tools_cache).await
        }
        other => Err(format!("Method not found: {other}")),
    }
}

fn tool_catalog() -> Vec<Value> {
    vec![
        json!({
            "name": "mcp_list",
            "description": "Lista los servidores MCP registrados en Circulo (con su estado on-demand / auto-load).",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "mcp_load",
            "description": "Carga on-demand un servidor MCP registrado: lanza el proceso, hace el handshake y devuelve sus tools. Después invoca sus tools con mcp_call.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nombre del servidor registrado (ver mcp_list)." }
                },
                "required": ["name"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "mcp_call",
            "description": "Invoca una tool de un servidor MCP ya cargado (mcp_load lo hace automáticamente si hace falta).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Servidor MCP registrado." },
                    "tool": { "type": "string", "description": "Nombre de la tool a invocar." },
                    "arguments": { "type": "object", "description": "Argumentos de la tool." }
                },
                "required": ["name", "tool"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "estimate_tokens",
            "description": "Estima los tokens de un texto (~4 caracteres/token, CJK cuenta 1).",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "compact_result",
            "description": "Comprime un resultado grande (JSON, git status/diff, tests, logs, rutas). El original se guarda en ~/.circulo/cache y es reversible con retrieve_original.",
            "inputSchema": {
                "type": "object",
                "properties": { "content": { "type": "string" } },
                "required": ["content"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "retrieve_original",
            "description": "Restaura el texto completo de un resultado comprimido con compact_result.",
            "inputSchema": {
                "type": "object",
                "properties": { "reference": { "type": "string", "description": "Ref circulo-cache://<hash> devuelta por compact_result." } },
                "required": ["reference"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "summarize",
            "description": "Resumen extractivo de un texto largo para caber en maxTokens.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "maxTokens": { "type": "number", "description": "Presupuesto de tokens (default 2000)." }
                },
                "required": ["content"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "find_symbol",
            "description": "Busca declaraciones (fn, struct, class, const…) que coincidan con un nombre en el proyecto.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "get_references",
            "description": "Lista los usos de un símbolo en el proyecto (coincidencia de palabra completa).",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "outline",
            "description": "Lista los símbolos de un archivo del proyecto (ruta relativa o absoluta dentro del root).",
            "inputSchema": {
                "type": "object",
                "properties": { "file": { "type": "string" } },
                "required": ["file"],
                "additionalProperties": false
            }
        }),
    ]
}

async fn call_tool(
    name: &str,
    args: &Value,
    registry: &[ManagedMcpServer],
    clients: &LoadedClients,
    tools_cache: &LoadedTools,
) -> Result<Value, String> {
    match name {
        "mcp_list" => {
            let servers: Vec<Value> = registry
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "name": s.name,
                        "kind": format!("{:?}", s.kind).to_lowercase(),
                        "enabled": s.enabled,
                        "autoLoad": s.auto_load,
                        "command": s.command,
                        "source": "registry",
                    })
                })
                .collect();
            Ok(json!({ "servers": servers, "count": servers.len() }))
        }
        "mcp_load" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "mcp_load: 'name' required".to_string())?;
            let (server, client) = ensure_client(name, registry, clients).await?;
            let tools = {
                let mut tools = client.lock().await.list_tools().await?;
                let mut cache = tools_cache.lock().await;
                cache.insert(server.name.clone(), tools.clone());
                for t in &mut tools {
                    t.description.truncate(200);
                }
                tools
            };
            Ok(json!({
                "name": server.name,
                "loaded": true,
                "tools": tools,
                "hint": format!("Usa mcp_call con name=\"{}\" para invocar estas tools.", server.name),
            }))
        }
        "mcp_call" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "mcp_call: 'name' required".to_string())?;
            let tool = args
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "mcp_call: 'tool' required".to_string())?;
            let tool_args = args.get("arguments").cloned().unwrap_or(json!({}));
            let (_server, client) = ensure_client(name, registry, clients).await?;
            let result = client.lock().await.call_tool(tool, tool_args).await?;
            let text = tool_result_text(&result);
            let is_error = result
                .get("isError")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(json!({
                "server": name,
                "tool": tool,
                "isError": is_error,
                "result": text,
            }))
        }
        "estimate_tokens" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            Ok(json!({
                "chars": text.chars().count(),
                "tokens": token_optimizer::estimate_tokens(text),
            }))
        }
        "compact_result" => {
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "compact_result: 'content' required".to_string())?;
            let (compacted, stats) = token_optimizer::compact_result(content)?;
            Ok(json!({
                "compacted": compacted,
                "stats": stats,
            }))
        }
        "retrieve_original" => {
            let reference = args
                .get("reference")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "retrieve_original: 'reference' required".to_string())?;
            token_optimizer::retrieve_original(reference)
                .map(|original| json!({ "reference": reference, "original": original }))
                .ok_or_else(|| format!("No cached original for {reference}"))
        }
        "summarize" => {
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "summarize: 'content' required".to_string())?;
            let max_tokens = args
                .get("maxTokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(2000) as usize;
            let (summary, original_tokens) = token_optimizer::summarize(content, max_tokens);
            Ok(json!({ "summary": summary, "originalTokens": original_tokens }))
        }
        "find_symbol" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "find_symbol: 'name' required".to_string())?;
            let root = semantic::project_root();
            let matches = semantic::find_symbol(&root, name);
            Ok(json!({ "matches": matches, "count": matches.len() }))
        }
        "get_references" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "get_references: 'name' required".to_string())?;
            let root = semantic::project_root();
            let matches = semantic::get_references(&root, name);
            Ok(json!({ "matches": matches, "count": matches.len() }))
        }
        "outline" => {
            let file = args
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "outline: 'file' required".to_string())?;
            let root = semantic::project_root();
            let path = resolve_file(&root, file)?;
            let symbols = semantic::outline(&root, &path)?;
            Ok(json!({ "symbols": symbols, "count": symbols.len() }))
        }
        other => Err(format!("Unknown tool: {other}")),
    }
}

async fn ensure_client(
    name: &str,
    registry: &[ManagedMcpServer],
    clients: &LoadedClients,
) -> Result<(ManagedMcpServer, Arc<Mutex<McpStdioClient>>), String> {
    {
        let cache = clients.lock().await;
        if let Some(client) = cache.get(name) {
            let server = registry
                .iter()
                .find(|s| s.name == name || s.id == name)
                .cloned()
                .ok_or_else(|| format!("Unknown MCP server: {name}"))?;
            return Ok((server, client.clone()));
        }
    }

    let server = registry
        .iter()
        .find(|s| s.name == name || s.id == name)
        .cloned()
        .ok_or_else(|| format!("MCP server '{name}' no está registrado en Circulo (Settings > MCP disponibles)."))?;
    if !server.enabled {
        return Err(format!("MCP server '{name}' está deshabilitado (Settings > MCP disponibles)."));
    }
    let (command, args, env) = match server.kind {
        McpServerKind::Stdio => {
            let args = server.args.clone();
            let env: Vec<(String, String)> = server
                .env
                .iter()
                .map(|e| (e.name.clone(), e.value.clone()))
                .collect();
            (server.command.clone(), args, env)
        }
        McpServerKind::Http | McpServerKind::Sse => {
            return Err(format!(
                "mcp_load solo soporta transporte stdio por ahora ({} es {:?})",
                server.name, server.kind,
            ));
        }
    };
    let client = McpStdioClient::spawn(&command, &args, &env)
        .await
        .map_err(|err| format!("{err} (server '{}')", server.name))?;
    let shared = Arc::new(Mutex::new(client));
    clients.lock().await.insert(server.name.clone(), shared.clone());
    Ok((server, shared))
}

/// Resolve `file` (relative to root or absolute under root) to an absolute path.
fn resolve_file(root: &PathBuf, file: &str) -> Result<PathBuf, String> {
    let candidate = if file.starts_with('/') {
        PathBuf::from(file)
    } else {
        root.join(file)
    };
    let canonical_root = root.canonicalize().map_err(|err| err.to_string())?;
    let canonical = candidate.canonicalize().map_err(|err| err.to_string())?;
    if !canonical.starts_with(&canonical_root) {
        return Err("File escapes project root".to_string());
    }
    Ok(canonical)
}
