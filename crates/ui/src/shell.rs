//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns the session list, the open session and the live transcript
//! the daemon streams into. Sessions come from the database through the daemon;
//! the labels the core does not own yet still come from `mock`.

use std::collections::BTreeMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use circulo_protocol::{ClientRequest, HistoryTurn, QuestionAction, ServerMessage, SessionSummary};
use gpui::{
    Context, FocusHandle, IntoElement, KeyDownEvent, ListState, Render, Window, div, prelude::*, px,
};

use crate::composer::{Field, KeyHandler, SendHandler, composer};
use crate::input::TextInput;
use crate::live::{Card, Transcript};
use crate::mock::{self, Workspace};
use crate::permission::{Answer, permission_card};
use crate::question::{
    Draft, Handlers, QuestionEvent, QuestionHandler, TextKeyHandler, question_card,
};
use crate::sidebar::sidebar;
use crate::theme::Theme;
use crate::transcript::{new_list_state, transcript};

pub struct Shell {
    workspace: Workspace,
    transcript: Transcript,
    /// The session every prompt goes to. The app opens it once at startup.
    session: String,
    input: TextInput,
    composer_focus: FocusHandle,
    focus_composer: bool,
    list_state: ListState,
    /// Requests the UI sends to the daemon through the client's writer half:
    /// prompts, card answers, session opens.
    requests: Sender<ClientRequest>,
    /// Newest first, as the daemon returns it.
    sessions: Vec<SessionSummary>,
    /// The card above the composer owns the keyboard while it is open.
    card_focus: FocusHandle,
    focus_card: bool,
    /// Choices made on the question card that is showing. Cleared when a new
    /// question takes the slot.
    question_answers: BTreeMap<String, String>,
    /// One buffer per free-text field of that card.
    question_inputs: BTreeMap<String, TextInput>,
    /// One focus handle per free-text field, so a click lands in the right one.
    question_focuses: BTreeMap<String, FocusHandle>,
    /// The field to put the keyboard on when a card appears.
    focus_text: Option<String>,
    question_shown: Option<String>,
}

impl Shell {
    /// Starts listening to the daemon. Messages wake the view only when they
    /// arrive, so an idle window costs nothing.
    pub fn new(
        cx: &mut Context<Self>,
        receiver: Receiver<ServerMessage>,
        requests: Sender<ClientRequest>,
        session: String,
        draft: String,
        history: Vec<HistoryTurn>,
        sessions: Vec<SessionSummary>,
    ) -> Self {
        let mut transcript = Transcript::default();
        transcript.hydrate(history);

        let shell = Self {
            workspace: mock::workspace(),
            transcript,
            session,
            input: TextInput::with_text(draft),
            composer_focus: cx.focus_handle(),
            focus_composer: true,
            list_state: new_list_state(),
            requests,
            sessions,
            card_focus: cx.focus_handle(),
            focus_card: false,
            question_answers: BTreeMap::new(),
            question_inputs: BTreeMap::new(),
            question_focuses: BTreeMap::new(),
            focus_text: None,
            question_shown: None,
        };

        cx.spawn(async move |this, cx| {
            while let Ok(message) = receiver.recv().await {
                let asked = matches!(
                    message,
                    ServerMessage::PermissionRequested { .. } | ServerMessage::QuestionAsked { .. }
                );
                let awake = this.update(cx, |shell, cx| {
                    shell.receive(message);
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

    /// Everything the daemon sends ends up here. Session bookkeeping stays in
    /// the shell; the rest is transcript content.
    fn receive(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Sessions { sessions } => {
                self.sessions = sessions;
            }
            ServerMessage::SessionStarted { session } => {
                self.session = session;
                self.transcript = Transcript::default();
                self.request(ClientRequest::ListSessions);
            }
            ServerMessage::History { session, turns } => {
                self.session = session;
                self.transcript = Transcript::default();
                self.transcript.hydrate(turns);
            }
            ServerMessage::TurnFinished { .. } => {
                self.transcript.apply(message);
                // The turn changed the title and the order of the list.
                self.request(ClientRequest::ListSessions);
            }
            other => self.transcript.apply(other),
        }
    }

    /// Starts a session. The daemon answers with `SessionStarted`.
    pub(crate) fn new_session(&mut self, cx: &mut Context<Self>) {
        self.request(ClientRequest::StartSession { project: None });
        cx.notify();
    }

    /// Opens a stored session. The daemon answers with its history.
    pub(crate) fn open_session(&mut self, id: String, cx: &mut Context<Self>) {
        if id == self.session {
            return;
        }
        self.request(ClientRequest::OpenSession { session: id });
        cx.notify();
    }

    fn request(&self, request: ClientRequest) {
        let _ = self.requests.send_blocking(request);
    }

    /// Drops the card and sends the answer to the daemon. The daemon validates
    /// the id; a stale one comes back as an error message.
    fn answer_permission(&mut self, id: String, option: Option<String>, cx: &mut Context<Self>) {
        self.pop_card(&id);
        self.request(ClientRequest::AnswerPermission { id, option });
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
                // Choices and typed answers travel in one map: the agent's
                // schema decides how each field is read.
                let mut answers = std::mem::take(&mut self.question_answers);
                for (field, input) in std::mem::take(&mut self.question_inputs) {
                    if !input.text().is_empty() {
                        answers.insert(field, input.text().to_owned());
                    }
                }
                self.pop_card(&request.id);
                self.request(ClientRequest::AnswerQuestion {
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
                self.request(ClientRequest::AnswerQuestion {
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
        self.request(ClientRequest::Prompt {
            session: self.session.clone(),
            text,
        });
        cx.notify();
    }

    /// Editing keys for a free-text answer. Enter submits the card, the way it
    /// sends the composer.
    fn question_text_key(&mut self, field: &str, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.control && !modifiers.alt && !modifiers.platform;

        match key {
            "enter" => {
                self.on_question(QuestionEvent::Submit, cx);
                return;
            }
            "escape" => {
                self.on_question(QuestionEvent::Cancel, cx);
                return;
            }
            _ => {}
        }

        let input = self.question_inputs.entry(field.to_owned()).or_default();
        match key {
            "backspace" => input.backspace(),
            "delete" => input.delete(),
            "left" => input.left(),
            "right" => input.right(),
            "home" => input.home(),
            "end" => input.end(),
            _ if plain => match &event.keystroke.key_char {
                Some(text) => input.insert(text),
                None => return,
            },
            _ => return,
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
        self.question_inputs.clear();
        self.question_focuses.clear();
        self.focus_text = None;
        self.question_shown = None;
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());

        // The header names the open session; a session with no prompt yet has
        // no title.
        let title = self
            .sessions
            .iter()
            .find(|session| session.id == self.session)
            .map(|session| session.title.clone())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| "New session".to_owned());

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

        // A question starts empty; answers belong to the card showing. Its text
        // fields get a focus handle each, and the first one takes the keyboard.
        if let Some(Card::Question(request)) = &front
            && self.question_shown.as_deref() != Some(request.id.as_str())
        {
            self.question_shown = Some(request.id.clone());
            self.question_answers.clear();
            self.question_inputs.clear();
            self.question_focuses.clear();

            for field in &request.fields {
                if matches!(field.kind, circulo_protocol::QuestionKind::Text) {
                    self.question_focuses
                        .insert(field.id.clone(), cx.focus_handle());
                    self.question_inputs
                        .insert(field.id.clone(), TextInput::default());
                }
            }
            self.focus_text = self.question_focuses.keys().next().cloned();
        }

        if let Some(field) = self.focus_text.take()
            && let Some(handle) = self.question_focuses.get(&field)
        {
            handle.focus(window);
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
                    let shell = cx.entity();
                    let text_key: TextKeyHandler = Rc::new(move |field, event, _window, cx| {
                        let field = field.to_owned();
                        shell.update(cx, |shell, cx| shell.question_text_key(&field, event, cx));
                    });
                    let handlers = Handlers {
                        event: on,
                        text_key,
                    };
                    let draft = Draft {
                        choices: &self.question_answers,
                        texts: &self.question_inputs,
                        focuses: &self.question_focuses,
                    };
                    question_card(&theme, &request, &draft, &focus, window, &handlers)
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
                &self.workspace,
                &self.transcript,
                &self.list_state,
                &title,
            ))
            .children(card)
            .when(true, |column| {
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
            .child(sidebar(&theme, &self.sessions, &self.session, cx))
            .child(main)
    }
}
