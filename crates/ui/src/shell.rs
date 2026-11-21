//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns which screen the main column shows and the live transcript
//! the daemon streams into. The sidebar still runs on sample data until
//! sessions are persisted.

use std::collections::BTreeMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use circulo_protocol::{ClientRequest, QuestionAction, ServerMessage};
use gpui::{
    Context, FocusHandle, IntoElement, KeyDownEvent, ListState, Render, Window, div, prelude::*, px,
};

use crate::composer::{Field, KeyHandler, SendHandler, composer};
use crate::input::TextInput;
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
    /// The session every prompt goes to. The app opens it once at startup.
    session: String,
    input: TextInput,
    composer_focus: FocusHandle,
    focus_composer: bool,
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
        session: String,
        draft: String,
    ) -> Self {
        let shell = Self {
            view: View::Session,
            workspace: mock::workspace(),
            transcript: Transcript::default(),
            session,
            input: TextInput::with_text(draft),
            composer_focus: cx.focus_handle(),
            focus_composer: true,
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

    /// Editing keys for the composer. Enter sends, Shift+Enter breaks the line,
    /// and everything else is the buffer's own business.
    fn input_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.control && !modifiers.alt && !modifiers.platform;

        match key {
            "enter" if !modifiers.shift => {
                self.submit(cx);
                return;
            }
            "enter" => self.input.insert("\n"),
            "backspace" => self.input.backspace(),
            "delete" => self.input.delete(),
            "left" => self.input.left(),
            "right" => self.input.right(),
            "home" => self.input.home(),
            "end" => self.input.end(),
            _ if plain => match &event.keystroke.key_char {
                Some(text) => self.input.insert(text),
                None => return,
            },
            _ => return,
        }

        cx.notify();
    }

    /// Sends the composer's text as a prompt. The transcript remembers it, so
    /// the turn shows what was asked above what came back.
    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.input.is_blank() || self.transcript.status.running {
            return;
        }

        let text = self.input.take();
        self.transcript.expect_prompt(text.clone());
        let _ = self.answers.send_blocking(ClientRequest::Prompt {
            session: self.session.clone(),
            text,
        });
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

        // The composer has the keyboard unless a card is asking.
        if self.focus_composer {
            self.focus_composer = false;
            self.composer_focus.focus(window);
        }

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
                &self.list_state,
            ))
            .children(card)
            .when(view == View::Session, |column| {
                let shell = cx.entity();
                let on_key: KeyHandler = Rc::new(move |event, _window, cx| {
                    shell.update(cx, |shell, cx| shell.input_key(event, cx));
                });
                let shell = cx.entity();
                let on_send: SendHandler = Rc::new(move |_window, cx| {
                    shell.update(cx, |shell, cx| shell.submit(cx));
                });
                let focused = self.composer_focus.is_focused(window);

                column.child(composer(
                    &theme,
                    &self.workspace,
                    &self.transcript,
                    Field {
                        input: &self.input,
                        focus: &self.composer_focus,
                        focused,
                        on_key,
                        on_send,
                    },
                ))
            });

        div()
            .flex()
            .size_full()
            .text_color(theme.text)
            .child(sidebar(&theme, &self.workspace, cx))
            .child(main)
    }
}
