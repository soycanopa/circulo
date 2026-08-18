//! shadcn / Base UI–style context menu primitives (`ContextMenuContent`, `Item`, etc.).

use gpui::{
    div, px, prelude::FluentBuilder, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::icons::icon;
use crate::theme::{
    ACCENT_SURFACE, BORDER, DANGER, DANGER_SURFACE, BG_MAIN, BG_SIDEBAR, PROVIDER_OPENCODE_LIST_ICON,
    TEXT, TEXT_MUTED,
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
/// OpenCode logo in the model selector list (smaller than default menu icons).
const MODEL_SELECTOR_ICON_SIZE_PX: f32 = 10.5;
const SEPARATOR_NEGATIVE_MX_PX: f32 = 4.0;

/// `ContextMenuContent` / `ContextMenuSubContent` surface.
pub fn menu_content() -> gpui::Div {
    menu_surface_base().border_1().border_color(BORDER)
}

/// Model selector popover width (wider than default chip menus).
pub const MODEL_SELECTOR_MENU_WIDTH_PX: f32 = 220.0;

const MODEL_MENU_PADDING_PX: f32 = 8.0;
const MODEL_MENU_ROW_GAP_PX: f32 = 2.0;
const MODEL_ITEM_PX: f32 = 10.0;
const MODEL_ITEM_PY: f32 = 7.0;
const MODEL_ITEM_GAP_PX: f32 = 8.0;

/// Chip dropdown popover: elevated opaque surface, no border.
pub fn menu_chip_popover() -> gpui::Div {
    menu_chip_popover_base().overflow_hidden()
}

/// Popover surface that allows nested sub-menus to extend outside the panel.
pub fn menu_chip_popover_layer() -> gpui::Div {
    menu_chip_popover_base()
}

/// Model selector popover with comfortable padding and row spacing.
pub fn menu_chip_model_selector_popover() -> gpui::Div {
    div()
        .w(px(MODEL_SELECTOR_MENU_WIDTH_PX))
        .min_w(px(MODEL_SELECTOR_MENU_WIDTH_PX))
        .flex()
        .flex_col()
        .gap(px(MODEL_MENU_ROW_GAP_PX))
        .p(px(MODEL_MENU_PADDING_PX))
        .rounded_lg()
        .bg(BG_MAIN)
        .text_color(TEXT)
        .shadow_lg()
        .occlude()
}

fn menu_chip_popover_base() -> gpui::Div {
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

/// Model selector row (`menu_item` with looser padding).
pub fn menu_item_model_selector(
    id: impl Into<gpui::ElementId>,
    label: String,
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
        .gap(px(MODEL_ITEM_GAP_PX))
        .px(px(MODEL_ITEM_PX))
        .py(px(MODEL_ITEM_PY))
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
        row = row.child(icon(asset, px(MODEL_SELECTOR_ICON_SIZE_PX), PROVIDER_OPENCODE_LIST_ICON));
    }

    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .child(label),
    );

    row
}

/// Model row with trailing Edit action (reasoning-effort configuration).
pub fn menu_item_with_edit(
    row_id: impl Into<gpui::ElementId>,
    select_id: impl Into<gpui::ElementId>,
    edit_id: impl Into<gpui::ElementId>,
    label: String,
    selected: bool,
    icon_asset: Option<&'static str>,
    edit_label: String,
    on_select: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let mut row = div()
        .id(row_id)
        .flex()
        .items_center()
        .gap(px(ITEM_GAP_PX))
        .px(px(ITEM_PX))
        .py(px(ITEM_PY))
        .rounded(px(ITEM_RADIUS_PX))
        .text_sm()
        .text_color(TEXT)
        .cursor_default()
        .when(selected, |el| el.bg(ACCENT_SURFACE));

    let mut label_hit = div()
        .id(select_id)
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(ITEM_GAP_PX))
        .cursor_pointer()
        .when(!selected, |el| el.hover(|style| style.bg(ACCENT_SURFACE)))
        .on_click(on_select);

    if let Some(asset) = icon_asset {
        label_hit = label_hit.child(icon(asset, px(ICON_SIZE_PX), TEXT));
    }

    label_hit = label_hit.child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .child(label),
    );

    row = row.child(label_hit);

    row = row.child(
        div()
            .id(edit_id)
            .flex_none()
            .px(px(4.))
            .py(px(2.))
            .rounded(px(4.))
            .text_xs()
            .text_color(TEXT_MUTED)
            .cursor_pointer()
            .hover(|style| style.bg(ACCENT_SURFACE))
            .on_click(on_edit)
            .child(edit_label),
    );

    row
}

/// Model selector row with Edit action (`menu_item_with_edit` with looser padding).
pub fn menu_item_with_edit_model_selector(
    row_id: impl Into<gpui::ElementId>,
    select_id: impl Into<gpui::ElementId>,
    edit_id: impl Into<gpui::ElementId>,
    hover_group: SharedString,
    label: String,
    selected: bool,
    edit_open: bool,
    icon_asset: Option<&'static str>,
    edit_label: String,
    on_select: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let mut row = div()
        .id(row_id)
        .group(hover_group.clone())
        .flex()
        .items_center()
        .gap(px(MODEL_ITEM_GAP_PX))
        .px(px(MODEL_ITEM_PX))
        .py(px(MODEL_ITEM_PY))
        .rounded(px(ITEM_RADIUS_PX))
        .text_sm()
        .text_color(TEXT)
        .cursor_default()
        .when(selected, |el| el.bg(ACCENT_SURFACE))
        .when(!selected, |el| el.hover(|style| style.bg(ACCENT_SURFACE)));

    let mut label_hit = div()
        .id(select_id)
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(MODEL_ITEM_GAP_PX))
        .cursor_pointer()
        .on_click(on_select);

    if let Some(asset) = icon_asset {
        label_hit = label_hit.child(icon(asset, px(MODEL_SELECTOR_ICON_SIZE_PX), PROVIDER_OPENCODE_LIST_ICON));
    }

    label_hit = label_hit.child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .child(label),
    );

    row = row.child(label_hit);

    let mut edit_btn = div()
        .id(edit_id)
        .flex_none()
        .ml(px(4.))
        .px(px(6.))
        .py(px(3.))
        .rounded(px(4.))
        .text_xs()
        .text_color(TEXT_MUTED)
        .cursor_pointer()
        .hover(|style| style.bg(ACCENT_SURFACE))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_edit)
        .child(edit_label);

    if edit_open {
        edit_btn = edit_btn.opacity(1.);
    } else {
        edit_btn = edit_btn
            .opacity(0.)
            .group_hover(hover_group, |style| style.opacity(1.));
    }

    row = row.child(edit_btn);

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
