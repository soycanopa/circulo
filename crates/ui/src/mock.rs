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
    pub user_message: &'static str,
    pub worked: &'static str,
    pub groups: Vec<Group>,
    pub blocks: Vec<Block>,
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

/// One block of assistant output. Inline `code` and **bold** are parsed by
/// `text::inline`; the real part model replaces this enum.
pub(crate) enum Block {
    Heading(&'static str),
    Paragraph(&'static str),
    Bullet(&'static str),
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
        user_message: "Lay out the transcript, the composer and the sidebar with the measured Waku tokens.",
        worked: "Worked for 12 seconds",
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
        blocks: vec![
            Block::Heading("Done. The window now matches the reference layout."),
            Block::Paragraph(
                "The shell is a **three-region frame**: a translucent sidebar of 252pt, the \
                 transcript with 34pt of padding, and a composer that floats above the bottom \
                 edge over the same canvas.",
            ),
            Block::Paragraph("What is in place:"),
            Block::Bullet(
                "`theme.rs` holds the measured graphite tokens for dark and light, one struct, no \
                 scattered hex values.",
            ),
            Block::Bullet(
                "`icons.rs` embeds Lucide SVGs and tints them with `currentColor`, so one file \
                 serves every state.",
            ),
            Block::Bullet(
                "`transcript.rs` renders user bubbles, the worked divider and assistant blocks \
                 with inline `code`.",
            ),
            Block::Heading("Next"),
            Block::Paragraph(
                "The transcript is static. Wiring it to the protocol is the first commit of Phase \
                 1, and it does not change these components: the same view renders history and \
                 live parts.",
            ),
        ],
    }
}
