use circulo_core::{Project, Session, Uuid};
use circulo_i18n::Catalog;
use gpui::actions;

use crate::client::{filter_sessions, session_project_label};

actions!(command_palette, [OpenPalette]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteItemKind {
    NewSession,
    ToggleSidebar,
    Session(Uuid),
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub kind: PaletteItemKind,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PaletteCatalog {
    pub commands: Vec<PaletteItem>,
    pub sessions: Vec<PaletteItem>,
}

impl PaletteCatalog {
    pub fn selectable_items(&self) -> Vec<&PaletteItem> {
        self.commands
            .iter()
            .chain(self.sessions.iter())
            .collect()
    }

    pub fn selectable_len(&self) -> usize {
        self.commands.len() + self.sessions.len()
    }

    pub fn selectable_item(&self, index: usize) -> Option<&PaletteItem> {
        if index < self.commands.len() {
            self.commands.get(index)
        } else {
            self.sessions.get(index - self.commands.len())
        }
    }
}

fn query_matches(query: &str, label: &str, keywords: &[&str]) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_ascii_lowercase();
    if label.to_ascii_lowercase().contains(&q) {
        return true;
    }
    keywords.iter().any(|keyword| {
        let keyword = keyword.to_ascii_lowercase();
        keyword.contains(&q) || q.contains(&keyword)
    })
}

pub fn palette_catalog(
    sessions: &[Session],
    projects: &[Project],
    query: &str,
    sidebar_collapsed: bool,
    catalog: &Catalog,
) -> PaletteCatalog {
    let without_folder = catalog.get("session.without_folder");
    let q = query.trim();

    let mut commands = Vec::new();

    let new_session = catalog.get("command.new_session");
    if query_matches(q, new_session, &["new", "session", "start", "create"]) {
        commands.push(PaletteItem {
            kind: PaletteItemKind::NewSession,
            label: new_session.to_string(),
            detail: None,
        });
    }

    let toggle = catalog.get(if sidebar_collapsed {
        "command.show_sidebar"
    } else {
        "command.hide_sidebar"
    });
    if query_matches(q, toggle, &["sidebar", "hide", "show", "toggle"]) {
        commands.push(PaletteItem {
            kind: PaletteItemKind::ToggleSidebar,
            label: toggle.to_string(),
            detail: None,
        });
    }

    let mut session_items = Vec::new();
    for session in filter_sessions(sessions, query) {
        let folder = session_project_label(session.project_id, projects, without_folder);
        session_items.push(PaletteItem {
            kind: PaletteItemKind::Session(session.id),
            label: session.title.clone(),
            detail: Some(folder),
        });
    }

    PaletteCatalog {
        commands,
        sessions: session_items,
    }
}

pub fn init_command_palette(cx: &mut gpui::App) {
    use gpui::KeyBinding;

    cx.bind_keys([KeyBinding::new("cmd-k", OpenPalette, None)]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_core::{AgentType, SessionStatus};
    use time::OffsetDateTime;

    fn session(id: u128, title: &str) -> Session {
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts");
        Session {
            id: Uuid::from_u128(id),
            project_id: None,
            title: title.into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
            last_message_at: None,
            first_send_at: None,
        }
    }

    #[test]
    fn empty_query_lists_commands_and_sessions() {
        let catalog = Catalog::english();
        let sessions = vec![session(1, "Launch copy")];
        let catalog = palette_catalog(&sessions, &[], "", false, &catalog);
        assert_eq!(catalog.commands.len(), 2);
        assert_eq!(catalog.sessions.len(), 1);
        assert!(matches!(
            catalog.commands[0].kind,
            PaletteItemKind::NewSession
        ));
    }

    #[test]
    fn new_session_command_matches_start_keyword() {
        let catalog = Catalog::english();
        let catalog = palette_catalog(&[], &[], "start", false, &catalog);
        assert_eq!(catalog.commands.len(), 1);
        assert!(matches!(catalog.commands[0].kind, PaletteItemKind::NewSession));
        assert!(catalog.sessions.is_empty());
    }
}
