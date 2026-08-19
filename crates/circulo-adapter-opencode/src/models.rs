use std::collections::HashSet;

use circulo_core::{model_catalog_id, ModelCatalogEntry};

/// Parse OpenCode `GET /provider` into Circulo model catalog entries.
pub fn parse_provider_catalog(body: &serde_json::Value) -> Vec<ModelCatalogEntry> {
    let connected = body
        .get("connected")
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .collect::<HashSet<&str>>()
        })
        .unwrap_or_default();
    let filter_connected = !connected.is_empty();
    let providers = body
        .get("all")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut models = Vec::new();
    for provider in providers {
        let provider_id = provider
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if provider_id.is_empty() {
            continue;
        }
        if filter_connected && !connected.contains(provider_id) {
            continue;
        }
        let provider_name = provider
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or(provider_id)
            .to_string();
        let model_map = provider
            .get("models")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();
        for (model_id, model) in model_map {
            let model_name = model
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or(&model_id)
                .to_string();
            models.push(ModelCatalogEntry {
                id: model_catalog_id(provider_id, &model_id),
                name: model_name,
                provider_id: provider_id.to_string(),
                provider_name: provider_name.clone(),
                model_id: model_id.clone(),
                context_window: format_context_window(&model),
                reasoning_variants: parse_reasoning_variants(&model),
                agent: circulo_core::AgentType::OpenCode,
            });
        }
    }
    models.sort_by(|left, right| {
        left.provider_name
            .cmp(&right.provider_name)
            .then_with(|| left.name.cmp(&right.name))
    });
    models
}

fn format_context_window(model: &serde_json::Value) -> Option<String> {
    let limit = model.get("limit").and_then(|value| value.as_u64());
    limit.map(format_token_limit)
}

fn parse_reasoning_variants(model: &serde_json::Value) -> Vec<String> {
    let variants = model.get("variants");
    if let Some(array) = variants.and_then(|value| value.as_array()) {
        let mut ids: Vec<String> = array
            .iter()
            .filter_map(|entry| {
                entry
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| entry.get("id").and_then(|id| id.as_str()).map(str::to_string))
            })
            .collect();
        ids.sort();
        ids.dedup();
        return ids;
    }
    if let Some(map) = variants.and_then(|value| value.as_object()) {
        let mut ids: Vec<String> = map.keys().cloned().collect();
        ids.sort();
        return ids;
    }
    Vec::new()
}

fn format_token_limit(tokens: u64) -> String {
    if tokens >= 1_000_000 && tokens % 1_000_000 == 0 {
        format!("{}M", tokens / 1_000_000)
    } else if tokens >= 1_000 && tokens % 1_000 == 0 {
        format!("{}K", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connected_provider_models() {
        let body = serde_json::json!({
            "connected": ["meta"],
            "all": [
                {
                    "id": "meta",
                    "name": "Meta",
                    "models": {
                        "spark": {
                            "name": "Spark",
                            "limit": 128000
                        }
                    }
                },
                {
                    "id": "other",
                    "name": "Other",
                    "models": {
                        "big": { "name": "Big" }
                    }
                }
            ]
        });
        let models = parse_provider_catalog(&body);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "meta/spark");
        assert_eq!(models[0].name, "Spark");
        assert_eq!(models[0].provider_name, "Meta");
        assert_eq!(models[0].context_window.as_deref(), Some("128K"));
    }

    #[test]
    fn parses_reasoning_variants_from_array_and_object() {
        let array_body = serde_json::json!({
            "variants": [
                { "id": "high" },
                { "id": "low" }
            ]
        });
        let object_body = serde_json::json!({
            "variants": {
                "medium": { "reasoningEffort": "medium" },
                "max": { "reasoningEffort": "max" }
            }
        });
        assert_eq!(
            parse_reasoning_variants(&array_body),
            ["high", "low"]
        );
        assert_eq!(
            parse_reasoning_variants(&object_body),
            ["max", "medium"]
        );
    }
}
