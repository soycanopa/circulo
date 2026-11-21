//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns which screen the main column shows and the live transcript
//! the daemon streams into. The sidebar still runs on sample data until
//! sessions are persisted.

use std::rc::Rc;

use async_channel::{Receiver, Sender};
use circulo_protocol::{ClientRequest, ServerMessage};
use gpui::{Context, FocusHandle, IntoElement, ListState, Render, Window, div, prelude::*, px};

use crate::composer::composer;
use crate::live::Transcript;
use crate::mock::{self, Workspace};
use crate::permission::{Answer, permission_card};
use crate::sidebar::sidebar;
use crate::theme::Theme;
use crate::transcript::{new_list_state, transcript};

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
    list_state: ListState,
    /// Requests the UI sends to the daemon, answered through the client's
    /// writer half. A permission answer travels this way.
    answers: Sender<ClientRequest>,
    permission_focus: FocusHandle,
    focus_permission: bool,
}

impl Shell {
    /// Starts listening to the daemon. Messages wake the view only when they
    /// arrive, so an idle window costs nothing.
    pub fn new(
        cx: &mut Context<Self>,
        receiver: Receiver<ServerMessage>,
        answers: Sender<ClientRequest>,
        prompt: String,
    ) -> Self {
        let shell = Self {
            view: View::Session,
            workspace: mock::workspace(),
            transcript: Transcript::default(),
            prompt,
            list_state: new_list_state(),
            answers,
            permission_focus: cx.focus_handle(),
            focus_permission: false,
        };

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let asked = matches!(message, ServerMessage::PermissionRequested { .. });
                let awake = this.update(cx, |shell, cx| {
                    shell.transcript.apply(message);
                    if asked {
                        // The card is the only thing to do next; put the
                        // keyboard on it so Enter and Escape work at once.
                        shell.focus_permission = true;
                    }
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

    /// Drops the card and sends the answer to the daemon. The daemon validates
    /// the id; a stale one comes back as an error message.
    fn answer_permission(&mut self, id: String, option: Option<String>, cx: &mut Context<Self>) {
        if let Some(index) = self
            .transcript
            .permissions
            .iter()
            .position(|request| request.id == id)
        {
            self.transcript.permissions.remove(index);
        }

        let _ = self
            .answers
            .send_blocking(ClientRequest::AnswerPermission { id, option });
        cx.notify();
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let view = self.view;

        // A new card takes the keyboard once, so Enter and Escape act on it.
        if self.focus_permission && !self.transcript.permissions.is_empty() {
            self.focus_permission = false;
            self.permission_focus.focus(window);
        }

        let card = self.transcript.permissions.front().cloned().map(|request| {
            let shell = cx.entity();
            let id = request.id.clone();
            let answer: Answer = Rc::new(move |option, _window, cx| {
                let id = id.clone();
                shell.update(cx, |shell, cx| shell.answer_permission(id, option, cx));
            });
            permission_card(&theme, &request, &self.permission_focus, answer)
        });

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
                &self.list_state,
            ))
            .children(card)
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
