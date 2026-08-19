use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::AgentType;

/// Tool permission posture for a session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComposerPermissionMode {
    FullAccess,
    Auto,
    #[default]
    Supervised,
    AutoAcceptEdits,
}

/// Composer interaction mode (maps to OpenCode agent: plan / build / ask).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComposerInteractionMode {
    Plan,
    #[default]
    Build,
    Ask,
}

impl ComposerInteractionMode {
    pub const ALL: [Self; 3] = [Self::Plan, Self::Build, Self::Ask];

    /// Next mode in toolbar cycle order: Plan → Build → Ask → Plan.
    pub fn next(self) -> Self {
        match self {
            Self::Plan => Self::Build,
            Self::Build => Self::Ask,
            Self::Ask => Self::Plan,
        }
    }

    pub fn agent_name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Build => "build",
            Self::Ask => "ask",
        }
    }
}

impl ComposerPermissionMode {
    pub const ALL: [Self; 4] = [
        Self::FullAccess,
        Self::Auto,
        Self::Supervised,
        Self::AutoAcceptEdits,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<String>,
    /// OpenCode variant ids for reasoning effort (`low`, `medium`, `high`, …).
    #[serde(default)]
    pub reasoning_variants: Vec<String>,
}

pub fn model_catalog_id(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

pub fn split_model_catalog_id(id: &str) -> Option<(String, String)> {
    id.split_once('/').map(|(provider, model)| (provider.to_string(), model.to_string()))
}

/// Short provider label for settings model tags (e.g. Zen, MiniMax, Grok).
pub fn model_provider_tag(provider_id: &str, provider_name: &str) -> String {
    let id = provider_id.to_ascii_lowercase();
    let name = provider_name.to_ascii_lowercase();
    if id.contains("zen") || name.contains("zen") {
        return "Zen".into();
    }
    if id.contains("minimax") || name.contains("minimax") {
        return "MiniMax".into();
    }
    if id.contains("grok") || id.contains("xai") || name.contains("grok") {
        return "Grok".into();
    }
    if id.contains("glm") || id.contains("zhipu") || name.contains("glm") {
        return "GLM".into();
    }
    if id.contains("anthropic") || name.contains("anthropic") || name.contains("claude") {
        return "Claude".into();
    }
    if id.contains("openai") || name.contains("openai") || name.contains("gpt") {
        return "OpenAI".into();
    }
    if id.contains("google") || name.contains("google") || name.contains("gemini") {
        return "Google".into();
    }
    if provider_name.is_empty() {
        return provider_id.to_string();
    }
    provider_name
        .trim()
        .strip_prefix("OpenCode ")
        .unwrap_or(provider_name.trim())
        .to_string()
}

/// User preferences persisted locally (composer model visibility, etc.).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    #[serde(default)]
    pub enabled_model_ids: Vec<String>,
    /// Providers the user has disabled in Settings. A disabled provider
    /// disappears from the AgentSelector and is rejected by the daemon on
    /// new session creation; existing sessions are migrated to OpenCode.
    #[serde(default)]
    pub disabled_agents: BTreeSet<AgentType>,
}

#[cfg(test)]
mod tests {
    use super::ComposerInteractionMode;

    #[test]
    fn interaction_mode_cycles_plan_build_ask() {
        assert_eq!(ComposerInteractionMode::Plan.next(), ComposerInteractionMode::Build);
        assert_eq!(ComposerInteractionMode::Build.next(), ComposerInteractionMode::Ask);
        assert_eq!(ComposerInteractionMode::Ask.next(), ComposerInteractionMode::Plan);
    }
}
