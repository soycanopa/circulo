//! Shared shadcn-style UI primitives.

pub mod activity_cluster;
pub mod disclosure;
pub mod input;
pub mod layout;
pub mod menu_chip;
pub mod permission_banner;
pub mod question_card;
pub mod reasoning_block;
pub mod reasoning_tag;
pub mod shimmer_text;
pub mod thinking;

pub use input::{field_label, init_text_input, TextInput, TextInputEvent};
pub use activity_cluster::{activity_cluster, message_segments, MessageSegment};
pub use layout::content_rail;
pub use menu_chip::menu_chip;
pub use permission_banner::{permission_banner, PendingPermission};
pub use question_card::{question_card, PendingQuestion};
pub use reasoning_block::reasoning_block;
pub use reasoning_tag::{reasoning_effort_colors, reasoning_effort_tag};
pub use shimmer_text::{shimmer_label, shimmer_text};
pub use thinking::{assistant_is_thinking, thinking_label, thinking_phrase};
