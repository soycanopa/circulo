//! shadcn / Base UI–style context menu primitives (`ContextMenuContent`, `Item`, etc.).

use gpui::{
    div, px, prelude::FluentBuilder, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};

use crate::icons::icon;
use crate::theme::{
    ACCENT_SURFACE, BORDER, DANGER, DANGER_SURFACE, BG_MAIN, BG_SIDEBAR, TEXT, TEXT_MUTED,
};

/// `ContextMenuContent` width from the shadcn demo (`w-48`).
pub const MENU_WIDTH_PX: f32 = 192.0;
const CONTENT_MIN_WIDTH_PX: f32 = 144.0;
const CONTENT_PADDING_PX: f32 = 4.0;
const ITEM_GAP_PX: f32 = 6.0;
const ITEM_PX: f32 = 6.0;
const ITEM_PY: f32 = 4.0;
const ITEM_RADIUS_PX: f32 = 6.0;
const ICON_SIZE_PX: f32 = 16.0;
const SEPARATOR_NEGATIVE_MX_PX: f32 = 4.0;

/// `ContextMenuContent` / `ContextMenuSubContent` surface.
pub fn menu_content() -> gpui::Div {
    menu_surface_base().border_1().border_color(BORDER)
}

/// Chip dropdown popover: elevated opaque surface, no border.
pub fn menu_chip_popover() -> gpui::Div {
    div()
        .w(px(MENU_WIDTH_PX))
        .min_w(px(CONTENT_MIN_WIDTH_PX))
        .flex()
        .flex_col()
        .p(px(CONTENT_PADDING_PX))
        .rounded_lg()
        .bg(BG_MAIN)
        .text_color(TEXT)
        .shadow_lg()
        .occlude()
        .overflow_hidden()
}

fn menu_surface_base() -> gpui::Div {
    div()
        .w(px(MENU_WIDTH_PX))
        .min_w(px(CONTENT_MIN_WIDTH_PX))
        .flex()
        .flex_col()
        .p(px(CONTENT_PADDING_PX))
        .rounded_lg()
        .bg(BG_SIDEBAR)
        .text_color(TEXT)
        .shadow_lg()
        .occlude()
        .overflow_hidden()
}

/// Alias kept for older call sites.
pub fn menu_surface() -> gpui::Div {
    menu_content()
}

/// `ContextMenuGroup`.
pub fn menu_group() -> gpui::Div {
    div().flex().flex_col()
}

/// `ContextMenuSeparator` (`-mx-1 my-1 h-px bg-border`).
pub fn menu_separator() -> impl IntoElement {
    div()
        .mx(px(-SEPARATOR_NEGATIVE_MX_PX))
        .my(px(4.))
        .h(px(1.))
        .bg(BORDER)
}

/// `ContextMenuShortcut` (`ml-auto text-xs tracking-widest text-muted-foreground`).
pub fn menu_shortcut(shortcut: impl Into<String>) -> impl IntoElement {
    div()
        .ml_auto()
        .pl_2()
        .text_xs()
        .text_color(TEXT_MUTED)
        .child(shortcut.into())
}

/// `ContextMenuItem` (`variant` = default | destructive).
pub fn menu_item(
    id: impl Into<gpui::ElementId>,
    label: String,
    selected: bool,
    destructive: bool,
    icon_asset: Option<&'static str>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    menu_item_with_shortcut(id, label, None, selected, destructive, icon_asset, on_click)
}

pub fn menu_item_with_shortcut(
    id: impl Into<gpui::ElementId>,
    label: String,
    shortcut: Option<String>,
    selected: bool,
    destructive: bool,
    icon_asset: Option<&'static str>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let text_color = if destructive { DANGER } else { TEXT };
    let mut row = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(ITEM_GAP_PX))
        .px(px(ITEM_PX))
        .py(px(ITEM_PY))
        .rounded(px(ITEM_RADIUS_PX))
        .text_sm()
        .text_color(text_color)
        .cursor_default()
        .when(selected, |el| {
            if destructive {
                el.bg(DANGER_SURFACE)
            } else {
                el.bg(ACCENT_SURFACE)
            }
        })
        .when(!selected, |el| {
            el.hover(|style| {
                if destructive {
                    style.bg(DANGER_SURFACE)
                } else {
                    style.bg(ACCENT_SURFACE)
                }
            })
        })
        .on_click(on_click);

    if let Some(asset) = icon_asset {
        row = row.child(icon(asset, px(ICON_SIZE_PX), text_color));
    }

    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .child(label),
    );

    if let Some(shortcut) = shortcut {
        row = row.child(menu_shortcut(shortcut));
    }

    row
}

/// Menu row with a secondary description line (Waku access-control style).
pub fn menu_item_with_description(
    id: impl Into<gpui::ElementId>,
    label: String,
    description: String,
    selected: bool,
    icon_asset: Option<&'static str>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let mut row = div()
        .id(id)
        .flex()
        .items_start()
        .gap(px(ITEM_GAP_PX))
        .px(px(ITEM_PX))
        .py(px(ITEM_PY))
        .rounded(px(ITEM_RADIUS_PX))
        .text_sm()
        .text_color(TEXT)
        .cursor_default()
        .when(selected, |el| el.bg(ACCENT_SURFACE))
        .when(!selected, |el| el.hover(|style| style.bg(ACCENT_SURFACE)))
        .on_click(on_click);

    if let Some(asset) = icon_asset {
        row = row.child(icon(asset, px(ICON_SIZE_PX), TEXT));
    }

    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .w_full()
                    .truncate()
                    .child(label),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(2.))
                    .text_size(px(10.5))
                    .line_height(px(14.))
                    .whitespace_normal()
                    .text_color(TEXT_MUTED)
                    .child(description),
            ),
    );

    if selected {
        row = row.child(icon("icons/check.svg", px(11.), TEXT_MUTED));
    }

    row
}
