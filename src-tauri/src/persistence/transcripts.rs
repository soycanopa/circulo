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

/// Persist a placeholder transcript with a provisional title so the sidebar can
/// render the chat immediately after `session_ready`, before the first chunk
/// arrives. Subsequent `save_chat_transcript` calls overwrite the title.
pub fn seed_chat_transcript(
    project_path: &str,
    session_id: &str,
    title: &str,
) -> Result<ChatSessionSummary, String> {
    if session_id.is_empty() || session_id == "pending" {
        return Err("Refusing to seed without a real ACP session id".to_string());
    }

    let path = PathBuf::from(project_path);
    let file = session_path(&path, session_id)?;
    let now = now_ms();
    let trimmed = title.trim();
    let resolved_title = if trimmed.is_empty() { "New chat" } else { trimmed };

    if file.is_file() {
        let mut existing: StoredTranscript = serde_json::from_str(
            &std::fs::read_to_string(&file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Invalid transcript: {e}"))?;
        existing.title = resolved_title.to_string();
        existing.updated_at = now;
        let raw =
            serde_json::to_string_pretty(&existing).map_err(|err| err.to_string())?;
        std::fs::write(&file, raw).map_err(|err| err.to_string())?;
        let summary = ChatSessionSummary {
            session_id: session_id.to_string(),
            title: resolved_title.to_string(),
            updated_at: now,
        };
        upsert_summary(&path, project_path, &summary)?;
        return Ok(summary);
    }

    let transcript = StoredTranscript {
        session_id: session_id.to_string(),
        project_path: project_path.to_string(),
        title: resolved_title.to_string(),
        created_at: now,
        updated_at: now,
        messages: Vec::new(),
    };
    let raw = serde_json::to_string_pretty(&transcript).map_err(|err| err.to_string())?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(&file, raw).map_err(|err| err.to_string())?;
    let summary = ChatSessionSummary {
        session_id: session_id.to_string(),
        title: resolved_title.to_string(),
        updated_at: now,
    };
    upsert_summary(&path, project_path, &summary)?;
    Ok(summary)
}

fn upsert_summary(
    path: &PathBuf,
    project_path: &str,
    summary: &ChatSessionSummary,
) -> Result<(), String> {
    let mut index = load_index(path)?;
    index.project_path = project_path.to_string();
    if let Some(entry) = index
        .chats
        .iter_mut()
        .find(|c| c.session_id == summary.session_id)
    {
        *entry = summary.clone();
    } else {
        index.chats.push(summary.clone());
    }
    save_index(path, &index)
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

pub fn rename_chat_transcript(
    project_path: &str,
    session_id: &str,
    title: &str,
) -> Result<ChatSessionSummary, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if session_id.is_empty() || session_id == "pending" {
        return Err("Invalid session id".to_string());
    }

    let path = PathBuf::from(project_path);
    let file = session_path(&path, session_id)?;
    if !file.is_file() {
        return Err("Chat transcript not found".to_string());
    }

    let mut transcript: StoredTranscript =
        serde_json::from_str(&std::fs::read_to_string(&file).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Invalid transcript: {e}"))?;
    let now = now_ms();
    transcript.title = title.to_string();
    transcript.updated_at = now;

    let raw = serde_json::to_string_pretty(&transcript).map_err(|err| err.to_string())?;
    std::fs::write(&file, raw).map_err(|err| err.to_string())?;

    let summary = ChatSessionSummary {
        session_id: session_id.to_string(),
        title: title.to_string(),
        updated_at: now,
    };

    let mut index = load_index(&path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::tempdir;

    struct HomeGuard(Option<std::ffi::OsString>);

    impl HomeGuard {
        fn set(path: &Path) -> Self {
            let previous = std::env::var_os("HOME");
            std::env::set_var("HOME", path);
            Self(previous)
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(previous) => std::env::set_var("HOME", previous),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    fn message(role: &str, content: &str) -> StoredChatMessage {
        StoredChatMessage {
            id: "message-1".to_string(),
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: Vec::new(),
            timestamp: 0,
        }
    }

    #[test]
    fn derives_title_from_first_non_empty_user_message() {
        let messages = vec![
            message("assistant", "Ignored"),
            message("user", "  First line\nSecond line  "),
            message("user", "Later"),
        ];

        assert_eq!(derive_title(&messages), "First line");
    }

    #[test]
    fn truncates_long_titles_at_seventy_two_characters() {
        let content = "a".repeat(73);
        let title = derive_title(&[message("user", &content)]);

        assert_eq!(title, format!("{}…", "a".repeat(72)));
    }

    #[test]
    fn returns_default_title_without_user_content() {
        let messages = vec![message("assistant", "Answer"), message("user", "   ")];

        assert_eq!(derive_title(&messages), "New chat");
    }

    #[test]
    fn refuses_placeholder_session_ids_before_writing() {
        let messages = vec![message("user", "Hello")];

        assert_eq!(
            save_chat_transcript("/unused", "", messages.clone()).unwrap_err(),
            "Refusing to persist without a real ACP session id"
        );
        assert_eq!(
            save_chat_transcript("/unused", "pending", messages).unwrap_err(),
            "Refusing to persist without a real ACP session id"
        );
    }

    #[test]
    #[serial]
    fn saves_loads_renames_and_deletes_transcript() {
        let home = tempdir().unwrap();
        let project = tempdir().unwrap();
        let _home = HomeGuard::set(home.path());
        let project_path = project.path().to_string_lossy().to_string();
        let messages = vec![message("user", "Build the feature")];

        let saved = save_chat_transcript(&project_path, "session/one", messages).unwrap();
        assert_eq!(saved.title, "Build the feature");

        let loaded = load_chat_transcript(&project_path, "session/one").unwrap();
        assert_eq!(loaded.session_id, "session/one");
        assert_eq!(loaded.messages.len(), 1);

        let renamed = rename_chat_transcript(&project_path, "session/one", "New title").unwrap();
        assert_eq!(renamed.title, "New title");
        assert_eq!(list_chat_sessions(&project_path).unwrap()[0].title, "New title");

        delete_chat_transcript(&project_path, "session/one").unwrap();
        assert!(list_chat_sessions(&project_path).unwrap().is_empty());
        assert_eq!(
            load_chat_transcript(&project_path, "session/one").unwrap_err(),
            "Chat transcript not found"
        );
    }

    #[test]
    #[serial]
    fn stores_sanitized_session_filename() {
        let home = tempdir().unwrap();
        let project = tempdir().unwrap();
        let _home = HomeGuard::set(home.path());

        let file = session_path(project.path(), "session/one:two").unwrap();

        assert_eq!(file.file_name().unwrap(), "session_one_two.json");
    }
}
