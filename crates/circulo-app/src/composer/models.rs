//! Composer model catalog (UI-facing).

use circulo_core::ModelCatalogEntry;
use circulo_i18n::Catalog;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerModel {
    pub id: String,
    pub name: String,
    /// Human-readable context window from model metadata (e.g. `128K`).
    pub context_window: Option<String>,
}

impl ComposerModel {
    pub fn context_label(&self) -> Option<&str> {
        self.context_window.as_deref()
    }
}

impl From<&ModelCatalogEntry> for ComposerModel {
    fn from(entry: &ModelCatalogEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            context_window: entry.context_window.clone(),
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
        },
        ComposerModel {
            id: "placeholder/sonnet".into(),
            name: catalog.get("composer.model.sonnet").to_string(),
            context_window: Some("200K".into()),
        },
        ComposerModel {
            id: "placeholder/gpt4o".into(),
            name: catalog.get("composer.model.gpt4o").to_string(),
            context_window: Some("128K".into()),
        },
    ]
}
