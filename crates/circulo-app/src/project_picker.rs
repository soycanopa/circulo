//! Project folder picker palette (composer “Open project…”).

use circulo_core::{Project, ProjectStatus, Uuid};
use circulo_i18n::Catalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectPickerItemKind {
    BrowseFinder,
    Project(Uuid),
}

#[derive(Debug, Clone)]
pub struct ProjectPickerItem {
    pub kind: ProjectPickerItemKind,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectPickerCatalog {
    pub actions: Vec<ProjectPickerItem>,
    pub projects: Vec<ProjectPickerItem>,
}

impl ProjectPickerCatalog {
    pub fn selectable_items(&self) -> Vec<&ProjectPickerItem> {
        self.actions.iter().chain(self.projects.iter()).collect()
    }

    pub fn selectable_len(&self) -> usize {
        self.actions.len() + self.projects.len()
    }

    pub fn selectable_item(&self, index: usize) -> Option<&ProjectPickerItem> {
        if index < self.actions.len() {
            self.actions.get(index)
        } else {
            self.projects.get(index - self.actions.len())
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

pub fn project_picker_catalog(
    projects: &[Project],
    query: &str,
    catalog: &Catalog,
) -> ProjectPickerCatalog {
    let q = query.trim();
    let browse = catalog.get("composer.project_picker.browse");
    let mut actions = Vec::new();
    if query_matches(q, browse, &["browse", "finder", "folder", "open", "choose", "path"]) {
        actions.push(ProjectPickerItem {
            kind: ProjectPickerItemKind::BrowseFinder,
            label: browse.to_string(),
            detail: Some(catalog.get("composer.project_picker.browse_detail").to_string()),
        });
    }

    let mut project_items = Vec::new();
    for project in projects {
        if project.status != ProjectStatus::Active {
            continue;
        }
        if !query_matches(q, &project.name, &["project", "folder"]) {
            continue;
        }
        let detail = project
            .description
            .clone()
            .filter(|d| !d.is_empty());
        project_items.push(ProjectPickerItem {
            kind: ProjectPickerItemKind::Project(project.id),
            label: project.name.clone(),
            detail,
        });
    }

    ProjectPickerCatalog {
        actions,
        projects: project_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_i18n::Catalog;
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts")
    }

    fn project(name: &str) -> Project {
        Project {
            id: Uuid::from_u128(1),
            name: name.into(),
            description: None,
            color: None,
            folder_path: None,
            status: ProjectStatus::Active,
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn empty_query_lists_browse_and_projects() {
        let catalog = Catalog::english();
        let items = project_picker_catalog(&[project("Circulo")], "", &catalog);
        assert_eq!(items.actions.len(), 1);
        assert_eq!(items.projects.len(), 1);
        assert_eq!(items.selectable_len(), 2);
    }

    #[test]
    fn query_filters_projects() {
        let catalog = Catalog::english();
        let items = project_picker_catalog(
            &[project("Alpha"), project("Beta")],
            "alp",
            &catalog,
        );
        assert_eq!(items.projects.len(), 1);
        assert_eq!(items.projects[0].label, "Alpha");
    }
}
