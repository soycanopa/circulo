use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::circulo_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionSummary {
    pub session_id: String,
    pub title: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredToolCall {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<StoredToolCall>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTranscript {
    pub session_id: String,
    pub project_path: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages: Vec<StoredChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceIndex {
    project_path: String,
    chats: Vec<ChatSessionSummary>,
}

fn workspace_key(project_path: &Path) -> Result<String, String> {
    let canonical = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.as_os_str().hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn workspace_dir(project_path: &Path) -> Result<PathBuf, String> {
    let key = workspace_key(project_path)?;
    let dir = circulo_data_dir()?.join("workspaces").join(key);
    std::fs::create_dir_all(dir.join("sessions"))
        .map_err(|err| format!("Could not create workspace data dir: {err}"))?;
    Ok(dir)
}

fn index_path(project_path: &Path) -> Result<PathBuf, String> {
    Ok(workspace_dir(project_path)?.join("index.json"))
}

fn session_path(project_path: &Path, session_id: &str) -> Result<PathBuf, String> {
    let safe = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(workspace_dir(project_path)?
        .join("sessions")
        .join(format!("{safe}.json")))
}

fn load_index(project_path: &Path) -> Result<WorkspaceIndex, String> {
    let path = index_path(project_path)?;
    if !path.is_file() {
        return Ok(WorkspaceIndex {
            project_path: project_path.to_string_lossy().to_string(),
            chats: Vec::new(),
        });
    }
    let raw = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| format!("Invalid workspace index: {err}"))
}

fn save_index(project_path: &Path, index: &WorkspaceIndex) -> Result<(), String> {
    let path = index_path(project_path)?;
    let raw = serde_json::to_string_pretty(index).map_err(|err| err.to_string())?;
    std::fs::write(path, raw).map_err(|err| err.to_string())
}

fn derive_title(messages: &[StoredChatMessage]) -> String {
    messages
        .iter()
        .find(|m| m.role == "user" && !m.content.trim().is_empty())
        .map(|m| {
            let line = m.content.lines().next().unwrap_or(&m.content).trim();
            if line.chars().count() > 72 {
                format!("{}…", line.chars().take(72).collect::<String>())
            } else {
                line.to_string()
            }
        })
        .unwrap_or_else(|| "New chat".to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn list_chat_sessions(project_path: &str) -> Result<Vec<ChatSessionSummary>, String> {
    let path = PathBuf::from(project_path);
    let index = load_index(&path)?;
    let mut chats = index.chats;
    chats.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(chats)
}

pub fn load_chat_transcript(
    project_path: &str,
    session_id: &str,
) -> Result<StoredTranscript, String> {
    let path = PathBuf::from(project_path);
    let file = session_path(&path, session_id)?;
    if !file.is_file() {
        return Err("Chat transcript not found".to_string());
    }
    let raw = std::fs::read_to_string(&file).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| format!("Invalid transcript: {err}"))
}

pub fn save_chat_transcript(
    project_path: &str,
    session_id: &str,
    messages: Vec<StoredChatMessage>,
) -> Result<ChatSessionSummary, String> {
    if session_id.is_empty() || session_id == "pending" {
        return Err("Refusing to persist without a real ACP session id".to_string());
    }

    let path = PathBuf::from(project_path);
    let file = session_path(&path, session_id)?;
    let now = now_ms();
    let title = derive_title(&messages);

    let transcript = if file.is_file() {
        let mut existing: StoredTranscript =
            serde_json::from_str(&std::fs::read_to_string(&file).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid transcript: {e}"))?;
        existing.messages = messages;
        existing.title = title.clone();
        existing.updated_at = now;
        existing
    } else {
        StoredTranscript {
            session_id: session_id.to_string(),
            project_path: project_path.to_string(),
            title: title.clone(),
            created_at: now,
            updated_at: now,
            messages,
        }
    };

    let raw = serde_json::to_string_pretty(&transcript).map_err(|err| err.to_string())?;
    std::fs::write(&file, raw).map_err(|err| err.to_string())?;

    let summary = ChatSessionSummary {
        session_id: session_id.to_string(),
        title,
        updated_at: now,
    };

    let mut index = load_index(&path)?;
    index.project_path = project_path.to_string();
    if let Some(entry) = index
        .chats
        .iter_mut()
        .find(|c| c.session_id == session_id)
    {
        *entry = summary.clone();
    } else {
        index.chats.push(summary.clone());
    }
    save_index(&path, &index)?;

    Ok(summary)
}

pub fn delete_chat_transcript(project_path: &str, session_id: &str) -> Result<(), String> {
    if session_id.is_empty() || session_id == "pending" {
        return Err("Invalid session id".to_string());
    }

    let path = PathBuf::from(project_path);
    let file = session_path(&path, session_id)?;
    if file.is_file() {
        std::fs::remove_file(&file).map_err(|err| err.to_string())?;
    }

    let mut index = load_index(&path)?;
    index.chats.retain(|c| c.session_id != session_id);
    save_index(&path, &index)
}
