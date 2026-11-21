//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns which screen the main column shows. With no project open it
//! is the empty state; with a task open it is the conversation and its
//! composer. Both are still sample data.

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use crate::composer::composer;
use crate::mock::{self, Workspace};
use crate::sidebar::sidebar;
use crate::theme::Theme;
use crate::transcript::transcript;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    /// No project: the onboarding screen, no composer.
    Empty,
    /// A task is open: transcript plus composer.
    Session,
}

pub struct Shell {
    view: View,
    workspace: Workspace,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            view: View::Session,
            workspace: mock::workspace(),
        }
    }

    pub(crate) fn show_empty(&mut self, cx: &mut Context<Self>) {
        self.view = View::Empty;
        cx.notify();
    }

    pub(crate) fn open_task(&mut self, cx: &mut Context<Self>) {
        self.view = View::Session;
        cx.notify();
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let view = self.view;

        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .bg(theme.canvas)
            .child(transcript(&theme, view, &self.workspace))
            .when(view == View::Session, |column| {
                column.child(composer(&theme, &self.workspace))
            });

        div()
            .flex()
            .size_full()
            .text_color(theme.text)
            .child(sidebar(&theme, &self.workspace, cx))
            .child(main)
    }
}
