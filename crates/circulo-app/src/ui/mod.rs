//! Shared shadcn-style UI primitives.

pub mod input;
pub mod menu_chip;

pub use input::{field_label, init_text_input, TextInput, TextInputEvent};
pub use menu_chip::menu_chip;
