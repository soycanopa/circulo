//! Waku-inspired compact dropdown triggers (see egoist/waku `MenuChip`).

use gpui::{
    deferred, div, prelude::FluentBuilder, px, ElementId, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::icons::{icon, path as icon_path};
use crate::theme::{ACCENT, ACCENT_SURFACE, TEXT_MUTED};

pub const MENU_CHIP_HEIGHT_PX: f32 = 24.0;
const MENU_CHIP_ABOVE_GAP_PX: f32 = 4.0;
const CHIP_HEIGHT_PX: f32 = MENU_CHIP_HEIGHT_PX;
const CHIP_PADDING_X_PX: f32 = 7.0;
const FOOTER_CHIP_PADDING_X_PX: f32 = 5.0;
const CHIP_GAP_PX: f32 = 6.0;
const CHIP_ICON_PX: f32 = 10.5;
const CHIP_LABEL_PX: f32 = 11.5;

/// Ghost chip: icon + truncated label, hover/selected overlay — no border.
pub fn menu_chip(
    id: impl Into<ElementId>,
    icon_path: &'static str,
    label: impl Into<SharedString>,
    open: bool,
    disabled: bool,
    compact: bool,
    accent: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let tone = if accent { ACCENT } else { TEXT_MUTED };
    let pad_x = if compact {
        FOOTER_CHIP_PADDING_X_PX
    } else {
        CHIP_PADDING_X_PX
    };
    div()
        .id(id)
        .h(px(CHIP_HEIGHT_PX))
        .px(px(pad_x))
        .max_w(px(190.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .gap(px(CHIP_GAP_PX))
        .text_size(px(CHIP_LABEL_PX))
        .line_height(px(14.))
        .cursor_default()
        .when(open, |el| el.bg(ACCENT_SURFACE))
        .when(!disabled, |el| {
            el.hover(|style| style.bg(ACCENT_SURFACE)).on_click(on_click)
        })
        .when(disabled, |el| el.opacity(0.7))
        .child(icon(icon_path, px(CHIP_ICON_PX), tone))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(tone)
                .child(label),
        )
}

/// Positions a dropdown menu above a chip trigger (Waku `MenuAlign::AboveLeft`).
pub fn menu_chip_dropdown_above(menu: impl IntoElement) -> impl IntoElement {
    div()
        .absolute()
        .left(px(0.))
        .bottom(px(MENU_CHIP_HEIGHT_PX + MENU_CHIP_ABOVE_GAP_PX))
        .occlude()
        .child(deferred(menu).with_priority(10))
}

/// Read-only context window label for the selected model (Waku model-traits chip).
pub fn model_context_indicator(label: impl Into<SharedString>) -> impl IntoElement {
    let label = label.into();
    div()
        .id("composer-model-context")
        .h(px(CHIP_HEIGHT_PX))
        .pr(px(4.))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(5.))
        .text_size(px(CHIP_LABEL_PX))
        .line_height(px(14.))
        .text_color(TEXT_MUTED)
        .child(icon(icon_path::LAYERS, px(CHIP_ICON_PX), TEXT_MUTED))
        .child(
            div()
                .truncate()
                .child(label),
        )
}
