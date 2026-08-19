//! Settings UI fragments (no `AppShell` dependency).

mod general;
mod models;
mod projects;
mod providers;

pub use general::general_settings_panel;
pub use models::models_settings_panel;
pub use projects::{active_projects_panel, archived_projects_panel};
pub use providers::providers_panel;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Projects,
    Archived,
    Providers,
    Models,
}

impl SettingsSection {
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Projects,
        Self::Archived,
        Self::Providers,
        Self::Models,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::General => "settings.section.general",
            Self::Projects => "settings.section.projects",
            Self::Archived => "settings.section.archived",
            Self::Providers => "settings.section.providers",
            Self::Models => "settings.section.models",
        }
    }

    pub fn nav_id(self) -> &'static str {
        match self {
            Self::General => "settings-nav-general",
            Self::Projects => "settings-nav-projects",
            Self::Archived => "settings-nav-archived",
            Self::Providers => "settings-nav-providers",
            Self::Models => "settings-nav-models",
        }
    }
}
