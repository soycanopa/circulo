//! Shared shadcn-style UI primitives.

pub mod input;
pub mod menu_chip;
pub mod permission_banner;
pub mod reasoning_block;
pub mod reasoning_tag;
pub mod thinking;

pub use input::{field_label, init_text_input, TextInput, TextInputEvent};
pub use menu_chip::menu_chip;
pub use permission_banner::{permission_banner, PendingPermission};
pub use reasoning_block::reasoning_block;
pub use reasoning_tag::{reasoning_effort_colors, reasoning_effort_tag};
pub use thinking::{assistant_is_thinking, thinking_label, thinking_phrase};
