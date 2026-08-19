//! Waku-inspired compact dropdown triggers (see egoist/waku `MenuChip`).

use gpui::{
    canvas, deferred, div, point, prelude::FluentBuilder, px, ElementId, InteractiveElement,
    IntoElement, MouseButton, ParentElement, PathBuilder, Rgba, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::icons::{icon, icon_sized, MODEL_PROVIDER_ICON_HEIGHT_PX, MODEL_PROVIDER_ICON_WIDTH_PX};
use crate::context_menu::MENU_WIDTH_PX;
use crate::theme::{ACCENT, ACCENT_SURFACE, PROVIDER_OPENCODE_CHIP_ICON, TEXT_MUTED};
use crate::ui::reasoning_effort_tag;

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

/// Model selector chip: name + optional colored reasoning-effort tag.
pub fn model_menu_chip(
    id: impl Into<ElementId>,
    icon_path: &'static str,
    label: impl Into<SharedString>,
    reasoning_tag: Option<String>,
    open: bool,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let tone = TEXT_MUTED;
    let mut chip = div()
        .id(id)
        .h(px(CHIP_HEIGHT_PX))
        .px(px(CHIP_PADDING_X_PX))
        .max_w(px(220.))
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
        .child(icon_sized(
            icon_path,
            px(MODEL_PROVIDER_ICON_WIDTH_PX),
            px(MODEL_PROVIDER_ICON_HEIGHT_PX),
            PROVIDER_OPENCODE_CHIP_ICON,
        ))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(tone)
                .child(label),
        );
    if let Some(tag) = reasoning_tag {
        chip = chip.child(reasoning_effort_tag(tag));
    }
    chip
}

/// Positions a dropdown menu above a chip trigger (Waku `MenuAlign::AboveLeft`).
pub fn menu_chip_dropdown_above(menu: impl IntoElement) -> impl IntoElement {
    div()
        .absolute()
        .left(px(0.))
        .bottom(px(MENU_CHIP_HEIGHT_PX + MENU_CHIP_ABOVE_GAP_PX))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(deferred(menu).with_priority(10))
}

/// Nested panel to the right of a menu row (reasoning-effort picker).
pub fn menu_chip_subpopover_right(menu: impl IntoElement) -> impl IntoElement {
    menu_chip_subpopover_right_offset(menu, MENU_WIDTH_PX + 4.)
}

/// Nested panel offset from the left edge of the parent row container.
///
/// Must not wrap the child in `deferred`: this panel lives inside the chip dropdown,
/// which is already deferred — nested `defer_draw` panics GPUI.
pub fn menu_chip_subpopover_right_offset(
    menu: impl IntoElement,
    left_px: f32,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(left_px))
        .top(px(0.))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(menu)
}

const CONTEXT_RING_SIZE_PX: f32 = 12.0;
const CONTEXT_RING_STROKE_PX: f32 = 1.5;
const CONTEXT_RING_TRACK: Rgba = Rgba {
    r: TEXT_MUTED.r,
    g: TEXT_MUTED.g,
    b: TEXT_MUTED.b,
    a: 0.35,
};

/// Waku-style context usage ring (track + fill arc, no label).
pub fn model_context_indicator(usage_fraction: f32) -> impl IntoElement {
    let fraction = usage_fraction.clamp(0.0, 1.0);
    div()
        .id("composer-model-context")
        .flex_none()
        .w(px(CONTEXT_RING_SIZE_PX))
        .h(px(CONTEXT_RING_SIZE_PX))
        .child(context_usage_ring(fraction))
}

fn context_usage_ring(fraction: f32) -> impl IntoElement {
    canvas(
        move |_, _, _| fraction,
        |bounds, fraction, window, _| {
            let center = bounds.center();
            let radius = px((CONTEXT_RING_SIZE_PX - CONTEXT_RING_STROKE_PX) / 2.0);
            let track = circle_stroke_path(center, radius, None);
            window.paint_path(track, CONTEXT_RING_TRACK);

            if fraction > 0.001 {
                let radius_f = f32::from(radius);
                let circumference = 2.0 * std::f32::consts::PI * radius_f;
                let filled = circumference * fraction;
                let progress = circle_stroke_path(
                    center,
                    radius,
                    Some(&[px(filled), px(circumference)]),
                );
                window.paint_path(progress, TEXT_MUTED);
            }
        },
    )
    .w(px(CONTEXT_RING_SIZE_PX))
    .h(px(CONTEXT_RING_SIZE_PX))
}

fn circle_stroke_path(
    center: gpui::Point<gpui::Pixels>,
    radius: gpui::Pixels,
    dash_array: Option<&[gpui::Pixels]>,
) -> gpui::Path<gpui::Pixels> {
    let c = center;
    let r = radius;
    let mut builder = PathBuilder::stroke(px(CONTEXT_RING_STROKE_PX));
    builder.move_to(point(c.x, c.y - r));
    builder.arc_to(point(r, r), px(0.), false, true, point(c.x + r, c.y));
    builder.arc_to(point(r, r), px(0.), false, true, point(c.x, c.y + r));
    builder.arc_to(point(r, r), px(0.), false, true, point(c.x - r, c.y));
    builder.arc_to(point(r, r), px(0.), false, true, point(c.x, c.y - r));
    if let Some(dash) = dash_array {
        builder = builder.dash_array(dash);
    }
    builder.build().expect("context ring path")
}
