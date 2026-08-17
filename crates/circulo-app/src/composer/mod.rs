//! Composer: dedicated input entity and toolbar container.

mod events;
mod helpers;
mod input;
pub mod view;

pub use events::{ComposerEvent, ComposerInputEvent};
pub use helpers::{can_send, project_picker_locked, summarize_message};
pub use input::{init as init_composer_input, ComposerInput};
pub use view::Composer;
