//! Collapsible reasoning transcript block (Waku activity-card style).

use std::collections::HashSet;

use circulo_i18n::Catalog;
use gpui::{div, IntoElement, ParentElement, Styled, Window};

use crate::icons::path;
use crate::parts::render_text;
use crate::theme::TEXT_MUTED;
use crate::ui::disclosure::{activity_card, activity_card_body, activity_card_header};
use crate::ui::shimmer_text::shimmer_text;

pub fn reasoning_block(
    id: &str,
    content: &str,
    visible: bool,
    live: bool,
    streaming: bool,
    catalog: &Catalog,
    expanded: &HashSet<String>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let open = expanded.contains(id);
    let label = if live {
        catalog.get("messages.reasoning_thinking").to_string()
    } else {
        catalog.get("messages.reasoning").to_string()
    };
    let show_shimmer = live;
    let expandable = !content.trim().is_empty() || !visible || streaming;
    let has_body = open && ((!visible && !streaming) || !content.trim().is_empty());

    let title = if show_shimmer {
        shimmer_text(label).into_any_element()
    } else {
        div()
            .child(label)
            .into_any_element()
    };

    let header = activity_card_header(
        ("reasoning-toggle", hash_id(id)),
        path::MESSAGE_CIRCLE,
        title,
        None::<String>,
        open,
        expandable,
        None::<gpui::AnyElement>,
        on_click,
    );

    let body = has_body.then(|| {
        let inner = if !visible && !streaming {
            div()
                .text_xs()
                .text_color(TEXT_MUTED)
                .child(catalog.get("messages.reasoning_unavailable").to_string())
        } else {
            div()
                .w_full()
                .min_w_0()
                .text_color(TEXT_MUTED)
                .child(render_text(content))
        };
        activity_card_body(inner)
    });

    activity_card(("reasoning", hash_id(id)), header, body)
}

fn hash_id(value: &str) -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as usize
}
