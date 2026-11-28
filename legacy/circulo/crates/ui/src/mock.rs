//! Stand-in labels for the parts of the window the core does not own yet.
//!
//! Sessions and projects come from the database now; the model, access mode,
//! agent mode and the "Worked" divider do not. When the control surface and
//! the turn footer land, those fields move to the core too and this module
//! goes away.

pub(crate) struct Workspace {
    pub model: &'static str,
    pub access: &'static str,
    pub agent_mode: &'static str,
    pub worked: &'static str,
}

pub(crate) fn workspace() -> Workspace {
    Workspace {
        model: "DeepSeek V4 Flash",
        access: "Full access",
        agent_mode: "Build",
        worked: "Worked",
    }
}
