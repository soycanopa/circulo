//! Collapsible reasoning transcript block (Waku-style toggle).

use std::collections::HashSet;

use circulo_i18n::Catalog;
use gpui::{
    div, prelude::FluentBuilder, px, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};

use crate::icons::{icon, path};
use crate::parts::render_text;
use crate::theme::{BG_SIDEBAR, BORDER, TEXT_MUTED};

pub fn reasoning_block(
    id: &str,
    content: &str,
    visible: bool,
    streaming: bool,
    catalog: &Catalog,
    expanded: &HashSet<String>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let open = expanded.contains(id);
    let chevron = if open {
        path::CHEVRON_DOWN
    } else {
        path::CHEVRON_RIGHT
    };
    let label = if streaming && content.trim().is_empty() {
        catalog.get("messages.reasoning_thinking").to_string()
    } else {
        catalog.get("messages.reasoning").to_string()
    };

    div()
        .id(("reasoning", hash_id(id)))
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .min_w_0()
        .child(
            div()
                .id(("reasoning-toggle", hash_id(id)))
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .cursor_pointer()
                .on_click(on_click)
                .child(icon(chevron, px(14.), TEXT_MUTED))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(TEXT_MUTED)
                        .child(label),
                )
                .when(streaming, |row| {
                    row.child(
                        div()
                            .ml_1()
                            .text_xs()
                            .text_color(TEXT_MUTED)
                            .child("…"),
                    )
                }),
        )
        .when(open, |block| {
            if !visible && !streaming {
                block.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .border_1()
                        .border_color(BORDER)
                        .bg(BG_SIDEBAR)
                        .text_xs()
                        .text_color(TEXT_MUTED)
                        .child(catalog.get("messages.reasoning_unavailable").to_string()),
                )
            } else if !content.trim().is_empty() {
                block.child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .border_1()
                        .border_color(BORDER)
                        .bg(BG_SIDEBAR)
                        .text_color(TEXT_MUTED)
                        .child(render_text(content)),
                )
            } else {
                block
            }
        })
}

fn hash_id(value: &str) -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as usize
}
