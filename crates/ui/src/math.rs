//! Display math: TeX is laid out by RaTeX and rasterized once, then cached.
//!
//! GPUI paints a paragraph as one `StyledText`, and a text run cannot contain
//! an image, so `$…$` stays a mono chip with its TeX source. `$$…$$` is a block
//! of its own and becomes an image, centered in the transcript.
//!
//! RaTeX is a Rust port of KaTeX (MIT). The PNG is rendered at 2× with
//! transparent pixels and the element is sized in logical points, so it stays
//! sharp on a retina display. A complex equation must never be rendered twice:
//! the cache is keyed by the TeX source, and a source longer than
//! `MAX_SOURCE` is refused rather than handed to the parser.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{RenderImage, px};
use image::Frame;
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parse;
use ratex_render::{RenderOptions, render_to_png};
use ratex_types::color::Color;
use smallvec::SmallVec;

/// Display math reads a little larger than body text, but not like a heading.
const FONT_SIZE: f32 = 18.;
/// Breathing room so a radical or an accent is not clipped by the canvas.
const PADDING: f32 = 4.;
/// Render at 2× and size the element in logical points: sharp on retina.
const DEVICE_PIXEL_RATIO: f32 = 2.;
/// An agent that streams a wall of TeX should not stall the UI thread.
const MAX_SOURCE: usize = 4096;
/// A long session can show many equations; keep the cache bounded. Clearing
/// re-renders one on demand, which is cheaper than growing forever.
const MAX_CACHE: usize = 256;

/// One rendered equation and the size it should occupy in the transcript.
pub(crate) struct Rendered {
    pub image: Arc<RenderImage>,
    pub width: f32,
    pub height: f32,
}

/// The equation for `source`, or `None` when it is too long or invalid.
pub(crate) fn rendered(source: &str) -> Option<Arc<Rendered>> {
    if source.len() > MAX_SOURCE {
        return None;
    }

    let cache = cache();
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(hit) = cache.get(source) {
        return hit.clone();
    }

    let rendered = render(source).map(Arc::new);
    if cache.len() >= MAX_CACHE {
        cache.clear();
    }
    cache.insert(source.to_owned(), rendered.clone());
    rendered
}

fn cache() -> &'static Mutex<HashMap<String, Option<Arc<Rendered>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Rendered>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn render(source: &str) -> Option<Rendered> {
    let ast = parse(source).ok()?;
    let laid = layout(&ast, &LayoutOptions::default());
    let list = to_display_list(&laid);

    let png = render_to_png(
        &list,
        &RenderOptions {
            font_size: FONT_SIZE,
            padding: PADDING,
            background_color: Color::new(0., 0., 0., 0.),
            device_pixel_ratio: DEVICE_PIXEL_RATIO,
            ..RenderOptions::default()
        },
    )
    .ok()?;

    let rgba = image::load_from_memory(&png).ok()?.into_rgba8();
    let mut data = rgba;
    // GPUI's renderer expects BGRA, exactly like its own image loader.
    for pixel in data.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    let image = RenderImage::new(SmallVec::from_elem(Frame::new(data), 1));
    Some(Rendered {
        image: Arc::new(image),
        width: list.width as f32 * FONT_SIZE + 2. * PADDING,
        height: (list.height + list.depth) as f32 * FONT_SIZE + 2. * PADDING,
    })
}

/// The display-math block: the equation centered, or its source in a chip
/// when the renderer refuses it.
pub(crate) fn view(source: &str, theme: &crate::theme::Theme) -> gpui::AnyElement {
    use gpui::{ImageSource, IntoElement, div, img, prelude::*};

    if let Some(math) = rendered(source) {
        return div()
            .w_full()
            .min_w(px(0.))
            .flex()
            .justify_center()
            .py(px(6.))
            .child(
                img(ImageSource::Render(math.image.clone()))
                    .w(px(math.width))
                    .h(px(math.height)),
            )
            .into_any_element();
    }

    div()
        .w_full()
        .min_w(px(0.))
        .rounded(px(8.))
        .bg(theme.chip)
        .px(px(12.))
        .py(px(9.))
        .text_size(px(12.))
        .line_height(px(18.))
        .font_family("Menlo")
        .text_color(theme.text_muted)
        .child(source.to_owned())
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_equation_renders_and_is_cached() {
        let first = rendered(r"x^2 + \frac{1}{2}").expect("rendered");
        let again = rendered(r"x^2 + \frac{1}{2}").expect("cached");

        assert!(first.width > 0. && first.height > 0.);
        assert!(
            Arc::ptr_eq(&first, &again),
            "the second call is a cache hit"
        );
    }

    #[test]
    fn invalid_tex_falls_back_instead_of_panicking() {
        assert!(rendered(r"\frac{1}{").is_none());
        assert!(rendered(&"x".repeat(MAX_SOURCE + 1)).is_none());
    }
}
