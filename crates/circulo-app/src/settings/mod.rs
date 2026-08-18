//! Settings UI fragments (no `AppShell` dependency).

mod general;
mod models;
mod projects;

pub use general::general_settings_panel;
pub use models::models_settings_panel;
pub use projects::{archived_projects_panel, active_projects_panel};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Projects,
    Archived,
    Models,
}

impl SettingsSection {
    pub const ALL: [Self; 4] = [
        Self::General,
        Self::Projects,
        Self::Archived,
        Self::Models,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::General => "settings.section.general",
            Self::Projects => "settings.section.projects",
            Self::Archived => "settings.section.archived",
            Self::Models => "settings.section.models",
        }
    }

    pub fn nav_id(self) -> &'static str {
        match self {
            Self::General => "settings-nav-general",
            Self::Projects => "settings-nav-projects",
            Self::Archived => "settings-nav-archived",
            Self::Models => "settings-nav-models",
        }
    }
}
