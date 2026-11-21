//! Stand-in content for the interface while the core is not wired.
//!
//! Everything here mirrors the shape a real snapshot will have, so the views
//! are built against data instead of hardcoded strings. When the protocol lands
//! this module goes away, and the views keep working.

pub(crate) struct Workspace {
    pub title: &'static str,
    pub project: &'static str,
    pub branch: &'static str,
    pub model: &'static str,
    pub reasoning: &'static str,
    pub access: &'static str,
    pub agent_mode: &'static str,
    pub worked: &'static str,
    pub groups: Vec<Group>,
    pub empty: Empty,
}

/// The state before a project is open.
pub(crate) struct Empty {
    pub header: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub primary: &'static str,
    pub secondary: &'static str,
}

pub(crate) struct Group {
    pub label: &'static str,
    pub tasks: Vec<Task>,
}

pub(crate) struct Task {
    pub title: &'static str,
    pub project: &'static str,
    pub when: &'static str,
    pub selected: bool,
}

impl Group {
    /// The folder affordance only shows next to the group that holds the open
    /// task, matching the reference.
    pub fn has_selection(&self) -> bool {
        self.tasks.iter().any(|task| task.selected)
    }
}

pub(crate) fn workspace() -> Workspace {
    Workspace {
        title: "Lay out the transcript renderer",
        project: "circulo",
        branch: "main",
        model: "DeepSeek V4 Flash",
        reasoning: "High",
        access: "Full access",
        agent_mode: "Build",
        worked: "Worked",
        groups: vec![
            Group {
                label: "Yesterday",
                tasks: vec![
                    Task {
                        title: "Lay out the transcript renderer",
                        project: "circulo",
                        when: "16m",
                        selected: true,
                    },
                    Task {
                        title: "Protocol parts and deltas",
                        project: "circulo",
                        when: "3h",
                        selected: false,
                    },
                    Task {
                        title: "Sidebar groups and pinning",
                        project: "No project",
                        when: "5h",
                        selected: false,
                    },
                ],
            },
            Group {
                label: "This Month",
                tasks: vec![Task {
                    title: "Checkpoint rewind plan",
                    project: "circulo",
                    when: "2d",
                    selected: false,
                }],
            },
        ],
        empty: Empty {
            header: "New task",
            title: "Open a project to begin",
            subtitle: "Circulo runs coding agents in folders you choose. Your code, sessions and \
                       history stay on this machine.",
            primary: "Open project folder…",
            secondary: "Don't work in a project",
        },
    }
}
