//! Colored reasoning-effort tags for the composer model chip.

use gpui::{div, px, FontWeight, ParentElement, Rgba, Styled};

pub fn reasoning_effort_tag(label: impl Into<String>) -> impl gpui::IntoElement {
    let label = label.into();
    let (bg, border, text) = reasoning_effort_colors(&label);
    div()
        .flex_none()
        .px(px(4.))
        .py(px(1.))
        .rounded_full()
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_size(px(11.))
        .line_height(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(text)
        .child(label)
}

pub fn reasoning_effort_colors(variant: &str) -> (Rgba, Rgba, Rgba) {
    match variant.to_ascii_lowercase().as_str() {
        "none" | "minimal" => (
            rgba_hex(0x2B, 0x2B, 0x30),
            rgba_hex(0x38, 0x38, 0x3D),
            rgba_hex(0x9E, 0x9E, 0xA6),
        ),
        "low" => (
            rgba_hex(0x24, 0x3D, 0x2E),
            rgba_hex(0x38, 0x6B, 0x47),
            rgba_hex(0x73, 0xC7, 0x8C),
        ),
        "med" | "medium" => (
            rgba_hex(0x47, 0x38, 0x1A),
            rgba_hex(0x8C, 0x66, 0x1F),
            rgba_hex(0xEB, 0xB8, 0x47),
        ),
        "high" => (
            rgba_hex(0x52, 0x20, 0x14),
            rgba_hex(0x9E, 0x52, 0x14),
            rgba_hex(0xFA, 0x9E, 0x47),
        ),
        "max" | "xhigh" => (
            rgba_hex(0x47, 0x24, 0x38),
            rgba_hex(0x73, 0x33, 0x66),
            rgba_hex(0xE6, 0x6B, 0x6B),
        ),
        _ => (
            rgba_hex(0x2B, 0x2B, 0x30),
            rgba_hex(0x38, 0x38, 0x3D),
            rgba_hex(0x9E, 0x9E, 0xA6),
        ),
    }
}

fn rgba_hex(r: u8, g: u8, b: u8) -> Rgba {
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::reasoning_effort_colors;

    #[test]
    fn high_tag_matches_design_tokens() {
        let (bg, border, text) = reasoning_effort_colors("High");
        assert_eq!(channel(bg.r), 0x52);
        assert_eq!(channel(border.r), 0x9E);
        assert_eq!(channel(text.r), 0xFA);
    }

    fn channel(value: f32) -> u8 {
        (value * 255.0).round() as u8
    }
}
