//! Composer model catalog (UI-facing).

use circulo_core::{AgentType, ModelCatalogEntry};
use circulo_i18n::Catalog;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerModel {
    pub id: String,
    pub name: String,
    /// Human-readable context window from model metadata (e.g. `128K`).
    pub context_window: Option<String>,
    /// OpenCode variant ids (`low`, `medium`, `high`, …).
    pub reasoning_variants: Vec<String>,
    /// Which Circulo provider serves this model. The composer
    /// popover uses it to filter by tab.
    pub agent: AgentType,
}

impl ComposerModel {
    pub fn context_label(&self) -> Option<&str> {
        self.context_window.as_deref()
    }

    pub fn supports_reasoning(&self) -> bool {
        !self.reasoning_variants.is_empty()
    }

    pub fn resolve_variant(&self, selected: Option<&str>) -> Option<String> {
        if self.reasoning_variants.is_empty() {
            return None;
        }
        if let Some(value) = selected {
            if self.reasoning_variants.iter().any(|variant| variant == value) {
                return Some(value.to_string());
            }
        }
        self.reasoning_variants.first().cloned()
    }
}

impl From<&ModelCatalogEntry> for ComposerModel {
    fn from(entry: &ModelCatalogEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            context_window: entry.context_window.clone(),
            reasoning_variants: entry.reasoning_variants.clone(),
            agent: entry.agent,
        }
    }
}

/// Fallback when the daemon cannot reach OpenCode yet.
pub fn placeholder_models(catalog: &Catalog) -> Vec<ComposerModel> {
    vec![
        ComposerModel {
            id: "placeholder/default".into(),
            name: catalog.get("composer.model.default").to_string(),
            context_window: Some("128K".into()),
            reasoning_variants: vec!["low".into(), "medium".into(), "high".into()],
            agent: AgentType::OpenCode,
        },
        ComposerModel {
            id: "placeholder/sonnet".into(),
            name: catalog.get("composer.model.sonnet").to_string(),
            context_window: Some("200K".into()),
            reasoning_variants: vec![
                "low".into(),
                "medium".into(),
                "high".into(),
                "max".into(),
            ],
            agent: AgentType::CommandCode,
        },
        ComposerModel {
            id: "placeholder/gpt4o".into(),
            name: catalog.get("composer.model.gpt4o").to_string(),
            context_window: Some("128K".into()),
            reasoning_variants: vec!["low".into(), "medium".into(), "high".into()],
            agent: AgentType::OpenCode,
        },
    ]
}
