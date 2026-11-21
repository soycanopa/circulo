//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns which screen the main column shows and the live transcript
//! the daemon streams into. The sidebar still runs on sample data until
//! sessions are persisted.

use std::collections::BTreeMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use circulo_protocol::{ClientRequest, QuestionAction, ServerMessage};
use gpui::{Context, FocusHandle, IntoElement, ListState, Render, Window, div, prelude::*, px};

use crate::composer::composer;
use crate::live::{Card, Transcript};
use crate::mock::{self, Workspace};
use crate::permission::{Answer, permission_card};
use crate::question::{QuestionEvent, QuestionHandler, question_card};
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
    /// The card above the composer owns the keyboard while it is open.
    card_focus: FocusHandle,
    focus_card: bool,
    /// Choices made on the question card that is showing. Cleared when a new
    /// question takes the slot.
    question_answers: BTreeMap<String, String>,
    question_shown: Option<String>,
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
            card_focus: cx.focus_handle(),
            focus_card: false,
            question_answers: BTreeMap::new(),
            question_shown: None,
        };

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let asked = matches!(
                    message,
                    ServerMessage::PermissionRequested { .. } | ServerMessage::QuestionAsked { .. }
                );
                let awake = this.update(cx, |shell, cx| {
                    shell.transcript.apply(message);
                    if asked {
                        // The card is the only thing to do next; put the
                        // keyboard on it so Enter and Escape work at once.
                        shell.focus_card = true;
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
        self.pop_card(&id);
        let _ = self
            .answers
            .send_blocking(ClientRequest::AnswerPermission { id, option });
        cx.notify();
    }

    fn on_question(&mut self, event: QuestionEvent, cx: &mut Context<Self>) {
        let Some(Card::Question(request)) = self.transcript.cards.front().cloned() else {
            return;
        };

        match event {
            QuestionEvent::Select { field, value } => {
                self.question_answers.insert(field, value);
            }
            QuestionEvent::Submit => {
                let answers = std::mem::take(&mut self.question_answers);
                self.pop_card(&request.id);
                let _ = self.answers.send_blocking(ClientRequest::AnswerQuestion {
                    id: request.id,
                    action: QuestionAction::Accept,
                    answers,
                });
            }
            QuestionEvent::Decline | QuestionEvent::Cancel => {
                let action = match event {
                    QuestionEvent::Decline => QuestionAction::Decline,
                    _ => QuestionAction::Cancel,
                };
                self.pop_card(&request.id);
                let _ = self.answers.send_blocking(ClientRequest::AnswerQuestion {
                    id: request.id,
                    action,
                    answers: BTreeMap::new(),
                });
            }
        }

        cx.notify();
    }

    fn pop_card(&mut self, id: &str) {
        if let Some(index) = self
            .transcript
            .cards
            .iter()
            .position(|card| card.id() == id)
        {
            self.transcript.cards.remove(index);
        }
        self.question_answers.clear();
        self.question_shown = None;
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let view = self.view;

        // A new card takes the keyboard once, so Enter and Escape act on it.
        if self.focus_card && !self.transcript.cards.is_empty() {
            self.focus_card = false;
            self.card_focus.focus(window);
        }

        let front = self.transcript.cards.front().cloned();

        // A question starts with no choices; answers belong to the card showing.
        if let Some(Card::Question(request)) = &front
            && self.question_shown.as_deref() != Some(request.id.as_str())
        {
            self.question_shown = Some(request.id.clone());
            self.question_answers.clear();
        }

        let focus = self.card_focus.clone();
        let card = front.map(|card| {
            let shell = cx.entity();
            match card {
                Card::Permission(request) => {
                    let id = request.id.clone();
                    let answer: Answer = Rc::new(move |option, _window, cx| {
                        let id = id.clone();
                        shell.update(cx, |shell, cx| shell.answer_permission(id, option, cx));
                    });
                    permission_card(&theme, &request, &focus, answer)
                }
                Card::Question(request) => {
                    let on: QuestionHandler = Rc::new(move |event, _window, cx| {
                        shell.update(cx, |shell, cx| shell.on_question(event, cx));
                    });
                    question_card(&theme, &request, &self.question_answers, &focus, on)
                }
            }
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
