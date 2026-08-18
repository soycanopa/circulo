//! Colored reasoning-effort tags for the composer model chip.

use gpui::{div, px, FontWeight, IntoElement, ParentElement, Rgba, Styled};

use crate::theme::{ACCENT, ACCENT_SURFACE, BORDER, DANGER, SUCCESS, TEXT, TEXT_MUTED};

pub fn reasoning_effort_tag(label: impl Into<String>) -> impl gpui::IntoElement {
    let label = label.into();
    let (bg, border, text) = reasoning_effort_colors(&label);
    div()
        .flex_none()
        .px(px(5.))
        .py(px(2.))
        .rounded(px(4.))
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(text)
        .child(label)
}

pub fn reasoning_effort_colors(variant: &str) -> (Rgba, Rgba, Rgba) {
    match variant.to_ascii_lowercase() {
        s if s == "none" || s == "minimal" => (
            ACCENT_SURFACE,
            BORDER,
            TEXT_MUTED,
        ),
        s if s == "low" => (
            Rgba {
                r: 0.14,
                g: 0.24,
                b: 0.18,
                a: 1.0,
            },
            Rgba {
                r: 0.22,
                g: 0.42,
                b: 0.28,
                a: 1.0,
            },
            SUCCESS,
        ),
        s if s == "medium" => (
            Rgba {
                r: 0.28,
                g: 0.22,
                b: 0.10,
                a: 1.0,
            },
            Rgba {
                r: 0.55,
                g: 0.40,
                b: 0.12,
                a: 1.0,
            },
            Rgba {
                r: 0.92,
                g: 0.72,
                b: 0.28,
                a: 1.0,
            },
        ),
        s if s == "high" => (
            Rgba {
                r: 0.32,
                g: 0.18,
                b: 0.10,
                a: 1.0,
            },
            Rgba {
                r: 0.62,
                g: 0.32,
                b: 0.14,
                a: 1.0,
            },
            Rgba {
                r: 0.98,
                g: 0.62,
                b: 0.28,
                a: 1.0,
            },
        ),
        s if s == "xhigh" || s == "max" => (
            Rgba {
                r: 0.28,
                g: 0.14,
                b: 0.22,
                a: 1.0,
            },
            Rgba {
                r: 0.45,
                g: 0.20,
                b: 0.38,
                a: 1.0,
            },
            DANGER,
        ),
        _ => (ACCENT_SURFACE, BORDER, ACCENT),
    }
}
