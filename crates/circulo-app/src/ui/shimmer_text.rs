//! shadcn-style text shimmer: a highlight band sweeps across the label.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{
    div, prelude::*, App, Bounds, Element, ElementId, FontWeight, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, SharedString, ShapedLine, Style, TextRun,
    Window,
};

use crate::theme::{TEXT, TEXT_MUTED};

const SHIMMER_DURATION_MS: f64 = 2000.0;
const SHIMMER_BAND: f32 = 0.20;
const SHIMMER_TRAVEL: f32 = 1.35;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn shimmer_phase() -> f32 {
    let millis = now_ms() as f64;
    ((millis % SHIMMER_DURATION_MS) / SHIMMER_DURATION_MS) as f32
}

fn mix_rgba(base: gpui::Rgba, highlight: gpui::Rgba, amount: f32) -> gpui::Rgba {
    gpui::Rgba {
        r: base.r + (highlight.r - base.r) * amount,
        g: base.g + (highlight.g - base.g) * amount,
        b: base.b + (highlight.b - base.b) * amount,
        a: base.a + (highlight.a - base.a) * amount,
    }
}

fn shimmer_mix(index: usize, len: usize, phase: f32) -> f32 {
    if len == 0 {
        return 0.0;
    }
    let pos = index as f32 / len as f32;
    let center = phase * SHIMMER_TRAVEL - SHIMMER_BAND;
    let dist = (pos - center).abs();
    if dist >= SHIMMER_BAND {
        return 0.0;
    }
    let t = 1.0 - dist / SHIMMER_BAND;
    t * t * (3.0 - 2.0 * t)
}

fn shimmer_runs(text: &str, font: gpui::Font, phase: f32) -> Vec<TextRun> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let base = gpui::Rgba {
        r: TEXT_MUTED.r,
        g: TEXT_MUTED.g,
        b: TEXT_MUTED.b,
        a: 0.55,
    };
    let highlight = TEXT;
    chars
        .into_iter()
        .enumerate()
        .map(|(index, ch)| {
            let mix = shimmer_mix(index, len, phase);
            TextRun {
                len: ch.len_utf8(),
                font: font.clone(),
                color: mix_rgba(base, highlight, mix).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }
        })
        .collect()
}

pub fn shimmer_text(text: impl Into<SharedString>) -> impl IntoElement {
    ShimmerTextElement {
        text: text.into(),
    }
}

struct ShimmerTextElement {
    text: SharedString,
}

struct ShimmerTextPrepaint {
    line: ShapedLine,
    bounds: Bounds<Pixels>,
}

impl IntoElement for ShimmerTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ShimmerTextElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<ShimmerTextPrepaint>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let runs = shimmer_runs(self.text.as_ref(), style.font(), shimmer_phase());
        let line = window
            .text_system()
            .shape_line(self.text.clone(), font_size, &runs, None);
        let width = line.x_for_index(self.text.len());
        let height = window.line_height();
        let mut layout = Style::default();
        layout.size.width = width.into();
        layout.size.height = height.into();
        (window.request_layout(layout, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let runs = shimmer_runs(self.text.as_ref(), style.font(), shimmer_phase());
        let line = window
            .text_system()
            .shape_line(self.text.clone(), font_size, &runs, None);
        Some(ShimmerTextPrepaint { line, bounds })
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(state) = prepaint.take() else {
            return;
        };
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let _ = state.line.paint(state.bounds.origin, font_size, window, cx);
    }
}

pub fn shimmer_label(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .child(shimmer_text(text))
}

#[cfg(test)]
mod tests {
    use super::{shimmer_mix, shimmer_phase};

    #[test]
    fn phase_wraps_in_unit_interval() {
        assert!(shimmer_phase() >= 0.0);
        assert!(shimmer_phase() < 1.0);
    }

    #[test]
    fn highlight_travels_across_glyphs() {
        let len = 10;
        let early: f32 = (0..len).map(|i| shimmer_mix(i, len, 0.1)).sum();
        let late: f32 = (0..len).map(|i| shimmer_mix(i, len, 0.7)).sum();
        assert!(early > 0.0);
        assert!(late > 0.0);
        assert_ne!(early, late);
    }
}
