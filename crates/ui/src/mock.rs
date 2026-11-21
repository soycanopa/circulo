//! Stand-in labels for the parts of the window the core does not own yet.
//!
//! Sessions come from the database now; the project name, the branch and the
//! composer's pickers do not. When projects and `configOptions` land, those
//! fields move to the core too and this module goes away.

pub(crate) struct Workspace {
    pub project: &'static str,
    pub branch: &'static str,
    pub model: &'static str,
    pub reasoning: &'static str,
    pub access: &'static str,
    pub agent_mode: &'static str,
    pub worked: &'static str,
}

pub(crate) fn workspace() -> Workspace {
    Workspace {
        project: "circulo",
        branch: "main",
        model: "DeepSeek V4 Flash",
        reasoning: "High",
        access: "Full access",
        agent_mode: "Build",
        worked: "Worked",
    }
}
