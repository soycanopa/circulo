//! Composer: dedicated input entity and toolbar container.

mod events;
mod helpers;
mod input;
mod labels;
mod models;
mod text_layout;
pub mod view;

pub use events::{
    ComposerEvent, ComposerInputEvent, InteractionMode, PermissionMode, WorkMode,
};
pub use labels::{
    interaction_accent, interaction_icon, interaction_label_key, permission_description_key,
    permission_icon, permission_label_key, reasoning_display_label,
};
pub use models::{ComposerModel, placeholder_models};
pub use helpers::{
    can_send, context_usage_fraction, project_picker_locked, summarize_message,
};
pub use input::{init as init_composer_input, ComposerInput};
pub use view::Composer;
