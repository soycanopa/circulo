//! Icons, embedded at compile time.
//!
//! They come from [Iconoir](https://iconoir.com) (MIT), the regular line set:
//! a 24 grid with a 1.5 stroke, drawn with `currentColor`, so the theme tints
//! every state. `opencode.svg` is the provider's own mark, kept to identify it
//! in the picker.

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
    "attachment.svg",
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
pub(crate) fn icon(path: &'static str, size: f32, color: Rgba) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .child(svg().path(path).size(px(size)).text_color(color))
}
