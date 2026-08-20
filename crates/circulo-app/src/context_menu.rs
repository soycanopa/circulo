//! shadcn / Base UI–style context menu primitives (`ContextMenuContent`, `Item`, etc.).

use gpui::{
    div, px, prelude::FluentBuilder, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use circulo_core::AgentType;

use crate::icons::{icon, icon_sized, MODEL_PROVIDER_ICON_HEIGHT_PX, MODEL_PROVIDER_ICON_WIDTH_PX};
use crate::theme::{
    ACCENT, ACCENT_SURFACE, BORDER, DANGER, DANGER_SURFACE, BG_HOVER, BG_MAIN, BG_SIDEBAR,
    PROVIDER_OPENCODE_LIST_ICON, TEXT, TEXT_MUTED,
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
/// OpenCode logo in the model selector list.
const MODEL_SELECTOR_ICON_WIDTH_PX: f32 = MODEL_PROVIDER_ICON_WIDTH_PX;
const MODEL_SELECTOR_ICON_HEIGHT_PX: f32 = MODEL_PROVIDER_ICON_HEIGHT_PX;
const SEPARATOR_NEGATIVE_MX_PX: f32 = 4.0;

/// `ContextMenuContent` / `ContextMenuSubContent` surface.
pub fn menu_content() -> gpui::Div {
    menu_surface_base().border_1().border_color(BORDER)
}

/// Total width of the model picker when the provider tab column is shown.
pub const MODEL_PICKER_WITH_TABS_WIDTH_PX: f32 = 360.0;
const MODEL_PICKER_TAB_WIDTH_PX: f32 = 96.0;
const MODEL_PICKER_TAB_PY_PX: f32 = 6.0;
const MODEL_PICKER_TAB_GAP_PX: f32 = 2.0;
const MODEL_PICKER_TAB_RADIUS_PX: f32 = 6.0;

/// Vertical column of provider tabs for the model picker. One row per
/// `(agent, label, count, on_click)` tuple. The caller provides the
/// click handler per tab (so the caller's `cx.listener` can wire
/// per-agent behavior). The active tab gets the accent surface
/// background; inactive tabs gain the hover surface on hover.
pub fn model_picker_provider_tabs(
    current: AgentType,
    tabs: Vec<(
        AgentType,
        String,
        usize,
        Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static>,
    )>,
) -> impl IntoElement {
    let mut col = div()
        .w(px(MODEL_PICKER_TAB_WIDTH_PX))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(MODEL_PICKER_TAB_GAP_PX));
    for (agent, label, count, on_click) in tabs {
        let is_active = current == agent;
        let tab_id: &'static str = match agent {
            AgentType::OpenCode => "model-picker-tab-open-code",
            AgentType::CommandCode => "model-picker-tab-command-code",
        };
        col = col.child(
            div()
                .id(tab_id)
                .flex()
                .flex_col()
                .gap(px(2.))
                .px(px(8.))
                .py(px(MODEL_PICKER_TAB_PY_PX))
                .rounded(px(MODEL_PICKER_TAB_RADIUS_PX))
                .cursor_pointer()
                .text_color(if is_active { ACCENT } else { TEXT_MUTED })
                .when(is_active, |el| el.bg(ACCENT_SURFACE))
                .when(!is_active, |el| el.hover(|style| style.bg(BG_HOVER)))
                .on_click(on_click)
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(TEXT_MUTED)
                        .child(format!("{count}")),
                ),
        );
    }
    col
}

/// Model selector popover width (wider than default chip menus).
pub const MODEL_SELECTOR_MENU_WIDTH_PX: f32 = 220.0;
/// Reasoning-effort sub-popover width.
pub const MODEL_REASONING_MENU_WIDTH_PX: f32 = 176.0;

pub(crate) const MODEL_MENU_PADDING_PX: f32 = 6.0;
pub(crate) const MODEL_MENU_ROW_GAP_PX: f32 = 4.0;
const MODEL_ITEM_PX: f32 = 6.0;
const MODEL_ITEM_PY: f32 = 4.0;
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
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// Model selector section header: favorites label + settings shortcut.
pub fn menu_model_selector_header(
    label: String,
    on_edit: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .w_full()
        .px(px(MODEL_ITEM_PX))
        .py(px(MODEL_ITEM_PY))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(11.))
                .line_height(px(14.))
                .text_color(TEXT_MUTED)
                .child(label),
        )
        .child(
            div()
                .id("composer-model-settings")
                .flex_none()
                .cursor_pointer()
                .rounded(px(4.))
                .hover(|style| style.bg(BG_HOVER))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(on_edit)
                .child(icon("icons/pencil.svg", px(14.), TEXT_MUTED)),
        )
}

/// Reasoning-effort sub-popover (176px).
pub fn menu_chip_reasoning_selector_popover() -> gpui::Div {
    div()
        .w(px(MODEL_REASONING_MENU_WIDTH_PX))
        .min_w(px(MODEL_REASONING_MENU_WIDTH_PX))
        .flex()
        .flex_col()
        .gap(px(MODEL_MENU_ROW_GAP_PX))
        .p(px(MODEL_MENU_PADDING_PX))
        .rounded_lg()
        .bg(BG_MAIN)
        .text_color(TEXT)
        .shadow_lg()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// Reasoning-effort section label inside the sub-popover.
pub fn menu_model_selector_reasoning_header(label: String) -> impl IntoElement {
    div()
        .w_full()
        .pt(px(4.))
        .pb(px(6.))
        .px(px(10.))
        .text_size(px(11.))
        .line_height(px(14.))
        .text_color(TEXT_MUTED)
        .child(label)
}

/// Reasoning-effort row with optional trailing check when selected.
pub fn menu_item_reasoning_selector(
    id: impl Into<gpui::ElementId>,
    label: String,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let mut row = div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .px(px(MODEL_ITEM_PX))
        .py(px(MODEL_ITEM_PY))
        .rounded(px(ITEM_RADIUS_PX))
        .text_sm()
        .line_height(px(16.))
        .text_color(TEXT)
        .cursor_default()
        .when(selected, |el| el.bg(BG_HOVER))
        .when(!selected, |el| el.hover(|style| style.bg(BG_HOVER)))
        .on_click(on_click)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(label),
        );

    if selected {
        row = row.child(icon("icons/check.svg", px(11.), TEXT_MUTED));
    }

    row
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
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                el.bg(BG_HOVER)
            }
        })
        .when(!selected, |el| {
            el.hover(|style| {
                if destructive {
                    style.bg(DANGER_SURFACE)
                } else {
                    style.bg(BG_HOVER)
                }
            })
        })
        .on_click(on_click);

    if let Some(asset) = icon_asset {
        row = row.child(icon_sized(
            asset,
            px(MODEL_SELECTOR_ICON_WIDTH_PX),
            px(MODEL_SELECTOR_ICON_HEIGHT_PX),
            PROVIDER_OPENCODE_LIST_ICON,
        ));
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
        .when(selected || edit_open, |el| el.bg(BG_HOVER))
        .when(!selected && !edit_open, |el| el.hover(|style| style.bg(BG_HOVER)));

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
        label_hit = label_hit.child(icon_sized(
            asset,
            px(MODEL_SELECTOR_ICON_WIDTH_PX),
            px(MODEL_SELECTOR_ICON_HEIGHT_PX),
            PROVIDER_OPENCODE_LIST_ICON,
        ));
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
        .hover(|style| style.bg(BG_HOVER))
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
