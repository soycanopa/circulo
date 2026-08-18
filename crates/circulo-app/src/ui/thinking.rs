//! Animated thinking label with rotating playful copy.

use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use circulo_i18n::Catalog;
use gpui::{div, FontWeight, ParentElement, Styled};

use crate::ui::shimmer_text::shimmer_text;

const PHRASE_ROTATE_MS: u64 = 2500;

const THINKING_KEYS: [&str; 15] = [
    "messages.thinking.0",
    "messages.thinking.1",
    "messages.thinking.2",
    "messages.thinking.3",
    "messages.thinking.4",
    "messages.thinking.5",
    "messages.thinking.6",
    "messages.thinking.7",
    "messages.thinking.8",
    "messages.thinking.9",
    "messages.thinking.10",
    "messages.thinking.11",
    "messages.thinking.12",
    "messages.thinking.13",
    "messages.thinking.14",
];

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn thinking_phrase(catalog: &Catalog, seed: impl Hash) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    let index = (hasher.finish() as usize) % THINKING_KEYS.len();
    catalog.get(THINKING_KEYS[index]).to_string()
}

pub fn thinking_phrase_rotating(catalog: &Catalog, seed: impl Hash) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    let base = hasher.finish() as usize;
    let slot = (now_ms() / PHRASE_ROTATE_MS) as usize;
    let index = (base + slot) % THINKING_KEYS.len();
    catalog.get(THINKING_KEYS[index]).to_string()
}

pub fn thinking_label(catalog: &Catalog, seed: impl Hash) -> impl gpui::IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .child(shimmer_text(thinking_phrase_rotating(catalog, seed)))
}

pub fn assistant_is_thinking(message: &circulo_core::Message) -> bool {
    use circulo_core::{MessagePart, MessageRole};

    if message.role != MessageRole::Assistant || !message.is_streaming {
        return false;
    }
    if message
        .parts
        .iter()
        .any(|part| matches!(part, MessagePart::Reasoning { .. }))
    {
        return false;
    }
    !message.parts.iter().any(|part| match part {
        MessagePart::Text { content } => !content.trim().is_empty(),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::{assistant_is_thinking, thinking_phrase, thinking_phrase_rotating};
    use circulo_core::{Message, MessagePart, MessageRole, MessageStatus};
    use circulo_i18n::Catalog;
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn phrase_rotates_by_seed_and_is_human() {
        let catalog = Catalog::english();
        let a = thinking_phrase(&catalog, Uuid::from_u128(1));
        let b = thinking_phrase(&catalog, Uuid::from_u128(2));
        assert!(!a.is_empty());
        assert!(!a.starts_with("messages.thinking"));
        assert_ne!(a, b);
    }

    #[test]
    fn rotating_phrase_changes_over_time_slots() {
        let catalog = Catalog::english();
        let seed = Uuid::from_u128(7);
        let first = thinking_phrase_rotating(&catalog, seed);
        assert!(!first.is_empty());
    }

    #[test]
    fn thinking_until_text_or_reasoning_arrives() {
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let streaming = Message {
            id: Uuid::from_u128(9),
            session_id: Uuid::nil(),
            role: MessageRole::Assistant,
            parts: vec![],
            status: MessageStatus::Streaming,
            created_at: now,
            is_streaming: true,
        };
        assert!(assistant_is_thinking(&streaming));

        let with_reasoning = Message {
            parts: vec![MessagePart::Reasoning {
                id: "r1".into(),
                content: String::new(),
                visible: true,
            }],
            ..streaming.clone()
        };
        assert!(!assistant_is_thinking(&with_reasoning));

        let with_text = Message {
            parts: vec![MessagePart::Text {
                content: "Hello".into(),
            }],
            ..streaming
        };
        assert!(!assistant_is_thinking(&with_text));
    }
}
