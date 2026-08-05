use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::circulo_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Automation {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub created_at: u64,
    pub updated_at: u64,
}

fn automations_path() -> Result<PathBuf, String> {
    Ok(circulo_data_dir()?.join("automations.json"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn list_automations() -> Result<Vec<Automation>, String> {
    let path = automations_path()?;
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| format!("Invalid automations.json: {err}"))
}

fn write_automations(automations: &[Automation]) -> Result<(), String> {
    let path = automations_path()?;
    let raw =
        serde_json::to_string_pretty(automations).map_err(|err| err.to_string())?;
    std::fs::write(path, raw).map_err(|err| err.to_string())
}

pub fn save_automation(title: String, prompt: String) -> Result<Automation, String> {
    let title = title.trim();
    let prompt = prompt.trim();
    if title.is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if prompt.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    let mut automations = list_automations()?;
    let now = now_ms();
    let entry = Automation {
        id: format!("auto_{}", Uuid::new_v4()),
        title: title.to_string(),
        prompt: prompt.to_string(),
        created_at: now,
        updated_at: now,
    };
    automations.push(entry.clone());
    write_automations(&automations)?;
    Ok(entry)
}

pub fn delete_automation(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("Automation id required".to_string());
    }
    let mut automations = list_automations()?;
    let before = automations.len();
    automations.retain(|item| item.id != id);
    if automations.len() == before {
        return Err("Automation not found".to_string());
    }
    write_automations(&automations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", temp.path());
        f();
    }

    #[test]
    #[serial]
    fn saves_lists_and_deletes_automation() {
        with_temp_home(|| {
            let saved = save_automation("Daily standup".into(), "Summarize git status".into())
                .unwrap();
            let list = list_automations().unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].id, saved.id);
            delete_automation(&saved.id).unwrap();
            assert!(list_automations().unwrap().is_empty());
        });
    }
}
