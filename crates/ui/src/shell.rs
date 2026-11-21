//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns which screen the main column shows and the live transcript
//! the daemon streams into. The sidebar still runs on sample data until
//! sessions are persisted.

use async_channel::Receiver;
use circulo_protocol::ServerMessage;
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use crate::composer::composer;
use crate::live::Transcript;
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
    transcript: Transcript,
    prompt: String,
}

impl Shell {
    /// Starts listening to the daemon. Messages wake the view only when they
    /// arrive, so an idle window costs nothing.
    pub fn new(cx: &mut Context<Self>, receiver: Receiver<ServerMessage>, prompt: String) -> Self {
        let shell = Self {
            view: View::Session,
            workspace: mock::workspace(),
            transcript: Transcript::default(),
            prompt,
        };

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let awake = this.update(cx, |shell, cx| {
                    shell.transcript.apply(message);
                    cx.notify();
                });

                if awake.is_err() {
                    break;
                }
            }
        })
        .detach();

        shell
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
            .child(transcript(
                &theme,
                view,
                &self.workspace,
                &self.transcript,
                &self.prompt,
            ))
            .when(view == View::Session, |column| {
                column.child(composer(&theme, &self.workspace, &self.transcript))
            });

        div()
            .flex()
            .size_full()
            .text_color(theme.text)
            .child(sidebar(&theme, &self.workspace, cx))
            .child(main)
    }
}
