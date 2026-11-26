//! Icons, embedded at compile time.
//!
//! They come from [Kubai Icons](https://icons.kubai.design) (MIT): the
//! two-tone style for icons with body, and the same author's line style for
//! the simple navigation glyphs the two-tone set does not draw (arrows,
//! chevrons, the reasoning mark, the panel toggles). Every file uses
//! `currentColor`, and the second tone is the same colour at reduced opacity,
//! so the theme tints the whole icon. `opencode.svg` is the provider's own
//! mark, kept to identify it in the picker.

use gpui::{Rgba, div, prelude::*, px, svg};

macro_rules! icon_assets {
    ($($file:literal),* $(,)?) => {
        pub(crate) fn bytes(path: &str) -> Option<&'static [u8]> {
            match path {
                $(concat!("icons/", $file) => Some(include_bytes!(concat!("../assets/icons/", $file))),)*
                _ => None,
            }
        }
    };
}

icon_assets!(
    "alert.svg",
    "arrow-left.svg",
    "arrow-right.svg",
    "arrow-up.svg",
    "brain.svg",
    "chevron-down.svg",
    "chevron-right.svg",
    "file-text.svg",
    "folder-plus.svg",
    "folder.svg",
    "git-branch.svg",
    "hammer.svg",
    "layers.svg",
    "loader-circle.svg",
    "lock.svg",
    "monitor.svg",
    "opencode.svg",
    "panel-left.svg",
    "panel-right.svg",
    "search.svg",
    "settings.svg",
    "square-pen.svg",
    "star.svg",
    "terminal.svg",
);

/// A fixed-size icon that never shrinks in a row and takes the given tint.
///
/// The two-tone files draw on a 16-unit grid with a margin, and a filled shape
/// reads smaller than an outline at the same box. They are rendered 18% larger
/// and 15% lighter than the requested size/tint so they match the line glyphs
/// beside them; the line files keep the exact size and colour.
pub(crate) fn icon(path: &'static str, size: f32, color: Rgba) -> impl IntoElement {
    let (size, color) = if is_duotone(path) {
        (size * 1.18, lift(color))
    } else {
        (size, color)
    };

    div()
        .flex_shrink_0()
        .child(svg().path(path).size(px(size)).text_color(color))
}

fn is_duotone(path: &str) -> bool {
    matches!(
        path,
        "icons/alert.svg"
            | "icons/file-text.svg"
            | "icons/folder-plus.svg"
            | "icons/folder.svg"
            | "icons/git-branch.svg"
            | "icons/hammer.svg"
            | "icons/layers.svg"
            | "icons/loader-circle.svg"
            | "icons/lock.svg"
            | "icons/monitor.svg"
            | "icons/search.svg"
            | "icons/settings.svg"
            | "icons/square-pen.svg"
            | "icons/star.svg"
            | "icons/terminal.svg"
    )
}

/// Blend 15% of white into the tint. Filled shapes carry more ink than a 2px
/// outline, so the same token would read heavier; this keeps the hierarchy
/// while making the two tones legible on the graphite canvas.
fn lift(color: Rgba) -> Rgba {
    Rgba {
        r: color.r + (1.0 - color.r) * 0.15,
        g: color.g + (1.0 - color.g) * 0.15,
        b: color.b + (1.0 - color.b) * 0.15,
        a: color.a,
    }
}
