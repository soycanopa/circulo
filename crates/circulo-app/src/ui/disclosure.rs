//! Waku-style transcript disclosures: flat toggles and activity cards.

use gpui::{
    div, prelude::*, px, AnyElement, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Window,
};

use crate::icons::{icon, path};
use crate::theme::{
    ACTIVITY_HOVER, ACTIVITY_SURFACE, BORDER, BORDER_STRONG, TEXT, TEXT_MUTED, TEXT_TERTIARY,
};

pub const ACTIVITY_HEADER_H_PX: f32 = 28.0;
pub const ACTIVITY_RADIUS_PX: f32 = 9.0;
pub const DISCLOSURE_CHEVRON_PX: f32 = 10.0;

pub fn chevron_icon(open: bool) -> &'static str {
    if open {
        path::CHEVRON_DOWN
    } else {
        path::CHEVRON_RIGHT
    }
}

/// Flat disclosure row (Waku activity section header): label + trailing chevron.
pub fn disclosure_header(
    id: impl Into<ElementId>,
    label: impl IntoElement,
    open: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .min_w_0()
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.5))
        .line_height(px(16.))
        .cursor_pointer()
        .hover(|style| style.text_color(TEXT))
        .on_click(on_click)
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_MUTED)
                .child(label),
        )
        .child(div().flex_1())
        .child(icon(
            chevron_icon(open),
            px(DISCLOSURE_CHEVRON_PX),
            TEXT_TERTIARY,
        ))
}

/// Nested list rail (Waku expanded activity stack).
pub fn disclosure_rail(content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .min_w_0()
        .pl(px(18.))
        .pb(px(2.))
        .border_l_1()
        .border_color(BORDER)
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(content)
}

/// Activity card shell (Waku tool / reasoning row).
pub fn activity_card(
    id: impl Into<ElementId>,
    header: impl IntoElement,
    body: Option<impl IntoElement>,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .min_w_0()
        .rounded(px(ACTIVITY_RADIUS_PX))
        .border_1()
        .border_color(BORDER_STRONG)
        .bg(ACTIVITY_SURFACE)
        .flex()
        .flex_col()
        .child(header)
        .when_some(body, |card, body| {
            card.child(
                div()
                    .w_full()
                    .min_w_0()
                    .border_t_1()
                    .border_color(BORDER_STRONG)
                    .child(body),
            )
        })
}

/// Clickable activity header row with icon, title, optional preview, trailing slot.
pub fn activity_card_header(
    id: impl Into<ElementId>,
    icon_path: &'static str,
    title: impl IntoElement,
    preview: Option<String>,
    open: bool,
    expandable: bool,
    trailing: Option<AnyElement>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let has_preview = preview.as_ref().is_some_and(|text| !text.is_empty());
    div()
        .id(id)
        .w_full()
        .min_w_0()
        .h(px(ACTIVITY_HEADER_H_PX))
        .px(px(8.))
        .overflow_hidden()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_size(px(12.))
        .line_height(px(16.))
        .when(expandable, |row| {
            row.cursor_pointer()
                .hover(|style| style.bg(ACTIVITY_HOVER))
                .active(|style| style.bg(ACTIVITY_HOVER))
                .on_click(on_click)
        })
        .child(icon(icon_path, px(12.), TEXT_TERTIARY))
        .child({
            let mut middle = div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .flex()
                .items_center()
                .gap(px(4.));
            middle = middle.child(
                div()
                    .min_w_0()
                    .when(has_preview, |el| el.flex_none())
                    .when(!has_preview, |el| el.flex_1())
                    .truncate()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(TEXT_MUTED)
                    .child(title),
            );
            if has_preview {
                middle = middle.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(TEXT_MUTED)
                        .child(preview.unwrap_or_default()),
                );
            }
            middle
        })
        .when_some(trailing, |row, trailing| {
            row.child(div().flex_none().child(trailing))
        })
        .when(expandable, |row| {
            row.child(
                div()
                    .flex_none()
                    .child(icon(
                        chevron_icon(open),
                        px(DISCLOSURE_CHEVRON_PX),
                        TEXT_TERTIARY,
                    )),
            )
        })
}

pub fn activity_card_body(content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .min_w_0()
        .px(px(12.))
        .py(px(8.))
        .child(
            div()
                .id("activity-card-body-scroll")
                .w_full()
                .min_w_0()
                .max_h(px(400.))
                .overflow_x_scroll()
                .overflow_y_scroll()
                .child(content),
        )
}

pub fn activity_detail_section(
    label: impl Into<String>,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_TERTIARY)
                .child(label.into()),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .child(content),
        )
}
