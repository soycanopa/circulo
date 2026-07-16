use std::path::Path;
use std::process::Stdio;

use serde::Deserialize;
use tokio::process::Command;

use crate::state::SessionInfoDto;

#[derive(Debug, Deserialize)]
struct CliSessionRow {
    id: Option<String>,
    #[serde(rename = "sessionID")]
    session_id: Option<String>,
    title: Option<String>,
    updated: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
}

/// Fast session metadata via `opencode session list --format json`.
/// Complements ACP when the agent is still spawning or for sidebar prefetch.
pub async fn list_sessions_cli(project_path: &Path, max_count: usize) -> Result<Vec<SessionInfoDto>, String> {
    let output = Command::new("opencode")
        .arg("session")
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("-n")
        .arg(max_count.to_string())
        .current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|err| format!("Failed to run opencode session list: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("opencode session list failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<CliSessionRow> = serde_json::from_str(&stdout)
        .or_else(|_| {
            let wrapped: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|err| format!("Invalid session list JSON: {err}"))?;
            match wrapped {
                serde_json::Value::Array(items) => items
                    .into_iter()
                    .map(serde_json::from_value)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| format!("Invalid session list rows: {err}")),
                other => Err(format!("Unexpected session list JSON shape: {other}")),
            }
        })?;

    let cwd = project_path.display().to_string();
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let session_id = row.session_id.or(row.id)?;
            Some(SessionInfoDto {
                session_id,
                cwd: cwd.clone(),
                additional_directories: Vec::new(),
                title: row.title,
                updated_at: row.updated_at.or(row.updated),
            })
        })
        .collect())
}