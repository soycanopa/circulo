//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The window itself is transparent to the desktop (blurred background), the
//! sidebar paints a translucent material over it, and the main column paints
//! the opaque canvas. That is how the reference gets a vibrant sidebar and a
//! flat transcript from one window.

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use crate::composer::composer;
use crate::mock::{self, Workspace};
use crate::sidebar::sidebar;
use crate::theme::Theme;
use crate::transcript::transcript;

pub struct Shell {
    workspace: Workspace,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            workspace: mock::workspace(),
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());

        div()
            .flex()
            .size_full()
            .text_color(theme.text)
            .child(sidebar(&theme, &self.workspace))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    .bg(theme.canvas)
                    .child(transcript(&theme, &self.workspace))
                    .child(composer(&theme, &self.workspace)),
            )
    }
}
