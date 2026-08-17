//! Settings UI fragments (no `AppShell` dependency).

mod models;

pub use models::models_settings_panel;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum SettingsSection {
    #[default]
    Models,
}

impl SettingsSection {
    pub const ALL: [Self; 1] = [Self::Models];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Models => "settings.section.models",
        }
    }
}
