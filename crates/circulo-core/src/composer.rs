use serde::{Deserialize, Serialize};

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
}

pub fn model_catalog_id(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

pub fn split_model_catalog_id(id: &str) -> Option<(String, String)> {
    id.split_once('/').map(|(provider, model)| (provider.to_string(), model.to_string()))
}

/// User preferences persisted locally (composer model visibility, etc.).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    #[serde(default)]
    pub enabled_model_ids: Vec<String>,
}
