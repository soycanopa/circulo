//! Minimal MCP client over stdio (newline-delimited JSON-RPC).
//!
//! Used in two places:
//! - The Settings "MCP disponibles" section validates a server by actually
//!   launching it and listing its tools (`validate_mcp_server_cmd`).
//! - The `circulo-mcp` orchestrator proxies loaded servers (`mcp_load` /
//!   `mcp_call`).
//!
//! One in-flight request per connection at a time (MCP servers are typically
//! single-threaded over stdio); callers hold the `Mutex`.

use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use tokio::time::timeout;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub struct McpStdioClient {
    _child: Child,
    stdin: Option<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    next_id: u64,
}

impl McpStdioClient {
    /// Spawn `command` and complete the MCP handshake (`initialize` +
    /// `notifications/initialized`). Returns an error with a readable message
    /// when the binary cannot start or the handshake fails.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<Self, String> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|err| format!("Could not launch '{command}': {err}"))?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "MCP server produced no stdout".to_string())?;

        let mut client = Self {
            _child: child,
            stdin,
            stdout: Mutex::new(BufReader::new(stdout)),
            next_id: 1,
        };

        client
            .request(
                "initialize",
                json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "circulo", "version": "0.1.0" },
                }),
            )
            .await
            .map_err(|err| format!("MCP initialize failed: {err}"))?;
        client
            .notify("notifications/initialized", json!({}))
            .await?;
        Ok(client)
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpToolInfo>, String> {
        let result = self
            .request("tools/list", json!({}))
            .await
            .map_err(|err| format!("tools/list failed: {err}"))?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "tools/list returned no tools array".to_string())?;
        Ok(tools
            .iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_string();
                let description = t
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let input_schema = t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                Some(McpToolInfo {
                    name,
                    description,
                    input_schema,
                })
            })
            .collect())
    }

    /// Call a tool and return the full MCP result object
    /// (`{ content: [...], isError?: bool }`).
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        self.request(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments,
            }),
        )
        .await
        .map_err(|err| format!("tools/call({name}) failed: {err}"))
    }

    /// Send a request and wait for the matching response by `id`.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write(&message).await?;
        self.read_response(id).await
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write(&message).await
    }

    async fn write(&mut self, message: &Value) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "MCP stdin is closed".to_string())?;
        let mut raw = serde_json::to_vec(message).map_err(|err| err.to_string())?;
        raw.push(b'\n');
        stdin
            .write_all(&raw)
            .await
            .map_err(|err| format!("MCP write failed: {err}"))
    }

    async fn read_response(&self, id: u64) -> Result<Value, String> {
        let mut reader = self.stdout.lock().await;
        let mut line = String::new();
        loop {
            line.clear();
            let read = timeout(REQUEST_TIMEOUT, reader.read_line(&mut line))
                .await
                .map_err(|_| "MCP request timed out".to_string())?
                .map_err(|err| format!("MCP read failed: {err}"))?;
            if read == 0 {
                return Err("MCP server closed the connection".to_string());
            }
            let value: Value = serde_json::from_str(&line)
                .map_err(|err| format!("Invalid MCP response: {err}"))?;
            if value.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(error) = value.get("error") {
                    let message = error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown MCP error");
                    return Err(message.to_string());
                }
                return Ok(value.get("result").cloned().unwrap_or(json!({})));
            }
        }
    }
}

/// Extract human-readable text from an MCP `tools/call` result.
pub fn tool_result_text(result: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(content) = result.get("content").and_then(|v| v.as_array()) {
        for item in content {
            let record = match item.as_object() {
                Some(r) => r,
                None => {
                    if let Some(text) = item.as_str() {
                        parts.push(text.to_string());
                    }
                    continue;
                }
            };
            if let Some(text) = record.get("text").and_then(|v| v.as_str()) {
                parts.push(text.to_string());
            }
        }
    }
    if parts.is_empty() {
        result.to_string()
    } else {
        parts.join("\n")
    }
}
