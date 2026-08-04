use agent_client_protocol::schema::v1::{
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions, SessionModeState, SetSessionConfigOptionRequest,
};
use agent_client_protocol::{Agent, ConnectionTo};
use serde_json::{Map, Value};

use crate::state::{ConfigOptionDto, ConfigOptionValueDto};

pub fn map_config_options(options: Option<&[SessionConfigOption]>) -> Vec<ConfigOptionDto> {
    options
        .unwrap_or_default()
        .iter()
        .map(|option| {
            let (current_value, values) = match &option.kind {
                SessionConfigKind::Select(select) => {
                    let values = match &select.options {
                        SessionConfigSelectOptions::Ungrouped(items) => items
                            .iter()
                            .map(|item| ConfigOptionValueDto {
                                value: item.value.to_string(),
                                name: item.name.clone(),
                                description: item.description.clone(),
                                group: None,
                            })
                            .collect(),
                        SessionConfigSelectOptions::Grouped(groups) => groups
                            .iter()
                            .flat_map(|group| {
                                group.options.iter().map(|item| ConfigOptionValueDto {
                                    value: item.value.to_string(),
                                    name: item.name.clone(),
                                    description: item.description.clone(),
                                    group: Some(group.name.clone()),
                                })
                            })
                            .collect(),
                        _ => Vec::new(),
                    };

                    (select.current_value.to_string(), values)
                }
                SessionConfigKind::Boolean(boolean) => {
                    (boolean.current_value.to_string(), Vec::new())
                }
                _ => (String::new(), Vec::new()),
            };

            ConfigOptionDto {
                id: option.id.to_string(),
                name: option.name.clone(),
                category: option
                    .category
                    .as_ref()
                    .map(category_label),
                current_value,
                options: values,
            }
        })
        .collect()
}

fn category_label(category: &SessionConfigOptionCategory) -> String {
    match category {
        SessionConfigOptionCategory::Mode => "mode".to_string(),
        SessionConfigOptionCategory::Model => "model".to_string(),
        SessionConfigOptionCategory::ModelConfig => "model_config".to_string(),
        SessionConfigOptionCategory::ThoughtLevel => "thought_level".to_string(),
        SessionConfigOptionCategory::Other(value) => value.clone(),
        _ => "other".to_string(),
    }
}

pub fn map_modes_to_config(modes: &SessionModeState) -> Vec<ConfigOptionDto> {
    let options = modes
        .available_modes
        .iter()
        .map(|mode| ConfigOptionValueDto {
            value: mode.id.0.to_string(),
            name: mode.name.clone(),
            description: mode.description.clone(),
            group: None,
        })
        .collect();

    vec![ConfigOptionDto {
        id: "mode".to_string(),
        name: "Mode".to_string(),
        category: Some("mode".to_string()),
        current_value: modes.current_mode_id.0.to_string(),
        options,
    }]
}

pub fn map_meta_session_config(meta: &Map<String, Value>) -> Vec<ConfigOptionDto> {
    let Some(config) = meta.get("x.ai/sessionConfig") else {
        return Vec::new();
    };
    let Some(options) = config.get("options").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut by_category: std::collections::HashMap<String, ConfigOptionDto> =
        std::collections::HashMap::new();

    for entry in options {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let category = obj
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("other")
            .to_string();
        if category != "model" && category != "mode" && category != "thought_level" {
            continue;
        }
        let id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if id.is_empty() {
            continue;
        }
        let label = obj
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let selected = obj.get("selected").and_then(|v| v.as_bool()).unwrap_or(false);

        let kind_key = match category.as_str() {
            "model" => "model",
            "thought_level" => "reasoning",
            _ => "mode",
        };

        let slot = by_category.entry(kind_key.to_string()).or_insert_with(|| {
            let (slot_id, slot_name) = match kind_key {
                "model" => ("model".to_string(), "Model".to_string()),
                "reasoning" => ("reasoning".to_string(), "Reasoning".to_string()),
                _ => ("mode".to_string(), "Mode".to_string()),
            };
            ConfigOptionDto {
                id: slot_id,
                name: slot_name,
                category: Some(kind_key.to_string()),
                current_value: String::new(),
                options: Vec::new(),
            }
        });

        slot.options.push(ConfigOptionValueDto {
            value: id.clone(),
            name: label,
            description,
            group: None,
        });
        if selected || slot.current_value.is_empty() {
            slot.current_value = id;
        }
    }

    by_category.into_values().collect()
}

fn has_config_kind(options: &[ConfigOptionDto], kind: &str) -> bool {
    options.iter().any(|option| {
        option
            .category
            .as_deref()
            .map(|c| c.eq_ignore_ascii_case(kind))
            .unwrap_or(false)
            || option.id.eq_ignore_ascii_case(kind)
    })
}

pub fn bridge_session_config_sync(
    mapped: Vec<ConfigOptionDto>,
    modes: Option<&SessionModeState>,
    meta: Option<&Map<String, Value>>,
) -> Vec<ConfigOptionDto> {
    let mut config_options = mapped;

    if !has_config_kind(&config_options, "mode") {
        if let Some(modes) = modes {
            config_options.extend(map_modes_to_config(modes));
        }
    }

    if !has_config_kind(&config_options, "model") {
        if let Some(meta) = meta {
            config_options.extend(map_meta_session_config(meta));
        }
    }

    dedupe_config_slots(config_options)
}

pub fn needs_config_refresh(
    config_options: &[ConfigOptionDto],
    modes: Option<&SessionModeState>,
    meta: Option<&Map<String, Value>>,
) -> bool {
    if has_config_kind(config_options, "model") {
        return false;
    }
    modes.is_some() || meta.and_then(grok_current_model_id).is_some()
}

/// Whether to spawn a background `set_config_option` refresh after `session/new`.
/// Cursor Agent exposes modes immediately; extra RPCs rarely return models (Zed pattern).
pub fn should_auto_refresh_config(
    agent_id: &str,
    config_options: &[ConfigOptionDto],
    modes: Option<&SessionModeState>,
    meta: Option<&Map<String, Value>>,
) -> bool {
    if !needs_config_refresh(config_options, modes, meta) {
        return false;
    }
    if agent_id == crate::agents::AGENT_ID_CURSOR && has_config_kind(config_options, "mode") {
        return false;
    }
    true
}

pub async fn refresh_session_config(
    connection: &ConnectionTo<Agent>,
    session_id: &str,
    modes: Option<&SessionModeState>,
    meta: Option<&Map<String, Value>>,
) -> Result<Vec<ConfigOptionDto>, String> {
    if let Some(modes) = modes {
        let request = SetSessionConfigOptionRequest::new(
            session_id.to_string(),
            "mode".to_string(),
            modes.current_mode_id.0.as_ref(),
        );
        if let Ok(response) = connection.send_request(request).block_task().await {
            let refreshed = map_config_options(Some(&response.config_options));
            if has_config_kind(&refreshed, "model") {
                return Ok(refreshed);
            }
        }
    }

    if let Some(model_id) = meta.and_then(grok_current_model_id) {
        let request = SetSessionConfigOptionRequest::new(
            session_id.to_string(),
            "model".to_string(),
            model_id.as_str(),
        );
        if let Ok(response) = connection.send_request(request).block_task().await {
            let refreshed = map_config_options(Some(&response.config_options));
            if !refreshed.is_empty() {
                return Ok(refreshed);
            }
        }
    }

    Err("Config refresh did not return model options".to_string())
}

pub fn merge_config_options(
    partial: Vec<ConfigOptionDto>,
    refreshed: Vec<ConfigOptionDto>,
) -> Vec<ConfigOptionDto> {
    let mut by_id: std::collections::HashMap<String, ConfigOptionDto> = partial
        .into_iter()
        .map(|option| (option.id.clone(), option))
        .collect();
    for option in refreshed {
        by_id.insert(option.id.clone(), option);
    }
    dedupe_config_slots(by_id.into_values().collect())
}

/// Blocking finalize — kept for integration tests and future sync callers.
#[allow(dead_code)]
pub async fn finalize_session_config(
    connection: &ConnectionTo<Agent>,
    session_id: &str,
    mapped: Vec<ConfigOptionDto>,
    modes: Option<&SessionModeState>,
    meta: Option<&Map<String, Value>>,
) -> Result<Vec<ConfigOptionDto>, String> {
    let partial = bridge_session_config_sync(mapped, modes, meta);
    if !should_auto_refresh_config(crate::agents::DEFAULT_AGENT_ID, &partial, modes, meta) {
        return Ok(partial);
    }
    match refresh_session_config(connection, session_id, modes, meta).await {
        Ok(refreshed) => Ok(refreshed),
        Err(_) => Ok(partial),
    }
}

fn grok_current_model_id(meta: &Map<String, Value>) -> Option<String> {
    meta.get("x.ai/sessionDetail")
        .and_then(|v| v.get("currentModelId"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            meta.get("modelState")
                .and_then(|v| v.get("currentModelId"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

fn dedupe_config_slots(options: Vec<ConfigOptionDto>) -> Vec<ConfigOptionDto> {
    let mut by_id: std::collections::HashMap<String, ConfigOptionDto> =
        std::collections::HashMap::new();
    for option in options {
        by_id.entry(option.id.clone()).or_insert(option);
    }
    let mut out: Vec<_> = by_id.into_values().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_refresh_false_without_modes_or_meta() {
        assert!(!needs_config_refresh(&[], None, None));
    }

    #[test]
    fn no_refresh_when_model_present() {
        let partial = vec![ConfigOptionDto {
            id: "model".to_string(),
            name: "Model".to_string(),
            category: Some("model".to_string()),
            current_value: "gpt".to_string(),
            options: vec![],
        }];
        assert!(!needs_config_refresh(&partial, None, None));
    }

    #[test]
    fn cursor_skips_refresh_when_mode_bridged() {
        let partial = vec![ConfigOptionDto {
            id: "mode".to_string(),
            name: "Mode".to_string(),
            category: Some("mode".to_string()),
            current_value: "agent".to_string(),
            options: vec![],
        }];
        assert!(!should_auto_refresh_config(
            crate::agents::AGENT_ID_CURSOR,
            &partial,
            None,
            None,
        ));
    }

    #[test]
    fn grok_still_refreshes_without_model() {
        let mut meta = Map::new();
        meta.insert(
            "x.ai/sessionDetail".to_string(),
            serde_json::json!({ "currentModelId": "grok-1" }),
        );
        assert!(should_auto_refresh_config(
            crate::agents::AGENT_ID_GROK,
            &[],
            None,
            Some(&meta),
        ));
    }

    #[test]
    fn merge_prefers_refreshed_values() {
        let partial = vec![ConfigOptionDto {
            id: "mode".to_string(),
            name: "Mode".to_string(),
            category: Some("mode".to_string()),
            current_value: "agent".to_string(),
            options: vec![ConfigOptionValueDto {
                value: "agent".to_string(),
                name: "Agent".to_string(),
                description: None,
                group: None,
            }],
        }];
        let refreshed = vec![ConfigOptionDto {
            id: "model".to_string(),
            name: "Model".to_string(),
            category: Some("model".to_string()),
            current_value: "gpt".to_string(),
            options: vec![ConfigOptionValueDto {
                value: "gpt".to_string(),
                name: "GPT".to_string(),
                description: None,
                group: None,
            }],
        }];
        let merged = merge_config_options(partial, refreshed);
        assert_eq!(merged.len(), 2);
        assert!(has_config_kind(&merged, "mode"));
        assert!(has_config_kind(&merged, "model"));
    }
}
