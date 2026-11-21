//! Icons, embedded at compile time.
//!
//! The SVGs come from [Lucide](https://lucide.dev) (ISC license), the same
//! visual family the reference uses. They are tinted with the theme through
//! `currentColor`, so one file serves every state.

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
    "arrow-left.svg",
    "arrow-right.svg",
    "arrow-up.svg",
    "asterisk.svg",
    "brain.svg",
    "chevron-down.svg",
    "chevron-right.svg",
    "file-text.svg",
    "folder-plus.svg",
    "folder.svg",
    "git-branch.svg",
    "hammer.svg",
    "loader-circle.svg",
    "lock.svg",
    "monitor.svg",
    "panel-left.svg",
    "panel-right.svg",
    "search.svg",
    "settings.svg",
    "square-pen.svg",
    "terminal.svg",
);

/// A fixed-size icon that never shrinks in a row and takes the given tint.
pub(crate) fn icon(path: &'static str, size: f32, color: Rgba) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .child(svg().path(path).size(px(size)).text_color(color))
}
