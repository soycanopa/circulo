//! Shared main-column width rail (transcript + composer).

use gpui::{div, px, IntoElement, ParentElement, Styled};

use crate::theme::{COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX};

/// Centers content on the same max width as the composer card, with matching
/// horizontal gutters.
pub fn content_rail(child: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .px(px(COMPOSER_GUTTER_PX))
        .child(
            div()
                .w_full()
                .max_w(px(CONTENT_MAX_WIDTH_PX))
                .mx_auto()
                .min_w_0()
                .child(child),
        )
}
