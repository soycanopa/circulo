//! The window frame: the sidebar and the main column, in the reference layout.
//!
//! The shell owns the session list, the open session and the live transcript
//! the daemon streams into. Sessions come from the database through the daemon;
//! the labels the core does not own yet still come from `mock`.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use circulo_protocol::{
    ClientRequest, ConfigOption, HistoryTurn, QuestionAction, QueuedPrompt, ServerMessage,
    SessionSummary,
};
use gpui::{
    Context, FocusHandle, IntoElement, KeyDownEvent, ListState, Pixels, Point, Render, Window, div,
    prelude::*, px,
};
use serde_json::Value;

/// The settings key the model picker's favourites live under. The value is a
/// list of choice ids, in the order they were starred.
const FAVORITES_KEY: &str = "favorites.model";

use crate::composer::{
    ChooseHandler, EditHandler, EffortHandler, Field, KeyHandler, Pickers, RemoveHandler,
    SendHandler, StarHandler, StopHandler, TabHandler, ToggleHandler, composer, queued,
};
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

/// What the app hands the window once the daemon is connected and a session
/// is open. Grouped so the constructor names one thing, not eight.
pub struct Startup {
    pub receiver: Receiver<ServerMessage>,
    pub requests: Sender<ClientRequest>,
    pub session: String,
    pub draft: String,
    pub history: Vec<HistoryTurn>,
    pub sessions: Vec<SessionSummary>,
    pub daemon: String,
}

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
    /// The daemon's settings for the open session: what the pickers draw.
    options: Vec<ConfigOption>,
    /// The option id whose menu is open above the composer, if any.
    open_picker: Option<String>,
    /// The model menu's tab while the menu is open; `None` lets the menu pick.
    picker_tab: Option<String>,
    /// The model row whose effort editor is open beside the model menu.
    open_editor: Option<String>,
    /// The provider behind the daemon ("opencode"), from the handshake: the
    /// models tab carries its name.
    daemon: String,
    /// The model choices this client starred, in the order they were starred.
    /// Stored on the daemon side, so they survive a restart.
    favorites: Vec<String>,
    /// Prompts waiting for the running turn, oldest first, as the daemon
    /// reports them. Presentation only: the daemon owns the real queue.
    queued: Vec<QueuedPrompt>,
    /// Where each picker chip sits, measured by its canvas every frame and
    /// read by the open menu the next one. Lives on the shell so the value
    /// survives the frame that wrote it — a fresh cell per render would only
    /// ever hold zero.
    picker_anchors: Rc<Cell<BTreeMap<String, Point<Pixels>>>>,
}

impl Shell {
    /// Starts listening to the daemon. Messages wake the view only when they
    /// arrive, so an idle window costs nothing.
    pub fn new(cx: &mut Context<Self>, startup: Startup) -> Self {
        let Startup {
            receiver,
            requests,
            session,
            draft,
            history,
            sessions,
            daemon,
        } = startup;

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
            options: Vec::new(),
            open_picker: None,
            picker_tab: None,
            open_editor: None,
            daemon,
            favorites: Vec::new(),
            queued: Vec::new(),
            picker_anchors: Rc::new(Cell::new(BTreeMap::new())),
        };

        // The client's own settings: the pickers want their favourites before
        // the first model menu opens.
        shell.request(ClientRequest::GetSettings);

        // A development hook, like CIRCULO_PROMPT: open one picker at startup
        // so a test can see a menu without a click. It opens on a timer, not
        // on the first frame — the chip's canvas has to have measured once,
        // or the menu would anchor to zero and pin itself to the corner.
        if let Ok(id) = std::env::var("CIRCULO_OPEN_PICKER") {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(400))
                    .await;
                let _ = this.update(cx, |shell, cx| {
                    if shell.open_picker.is_none() {
                        shell.open_picker = Some(id);
                    }
                    cx.notify();
                });
            })
            .detach();
        };

        // The same idea for the effort editor: once the menu is up, open the
        // editor on its first model so the whole path can be exercised
        // without a click.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(900))
                .await;
            let _ = this.update(cx, |shell, cx| {
                let model = std::env::var("CIRCULO_OPEN_EDIT").ok();
                let model = match model {
                    Some(substring) => shell
                        .options
                        .iter()
                        .find(|option| option.id == "model")
                        .and_then(|option| {
                            option
                                .options
                                .iter()
                                .find(|choice| choice.id.contains(&substring))
                                .map(|choice| choice.id.clone())
                        }),
                    None => None,
                };
                if let Some(model) = model {
                    shell.open_picker = Some("model".to_owned());
                    shell.open_editor = Some(model);
                    cx.notify();
                }
            });
        })
        .detach();

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
        // A turn keeps running in the daemon after the window switches
        // sessions. Its parts belong to the session that started them, not to
        // whatever row happens to be open now.
        if let Some(session) = live_session(&message)
            && session != self.session
        {
            // A background turn still changes the sidebar's title and order,
            // even though its transcript content belongs to another session.
            if matches!(message, ServerMessage::TurnFinished { .. }) {
                self.request(ClientRequest::ListSessions);
            }
            return;
        }

        match message {
            ServerMessage::Sessions { sessions } => {
                self.sessions = sessions;
            }
            ServerMessage::SessionStarted { session } => {
                self.session = session;
                self.transcript = Transcript::default();
                self.queued.clear();
                self.clear_options();
                self.request(ClientRequest::ListSessions);
            }
            ServerMessage::Queue { session, items } => {
                // The daemon owns the queue; a late list for a session the
                // window left behind is not drawn.
                if session == self.session {
                    self.queued = items;
                }
            }
            ServerMessage::Options { session, options } => {
                // A late list for a session already left behind is not drawn.
                if session != self.session {
                    return;
                }
                self.options = options;
                // The open menu may name an option the new list does not have.
                if let Some(open) = &self.open_picker
                    && !self.options.iter().any(|option| &option.id == open)
                {
                    self.open_picker = None;
                }
            }
            ServerMessage::History { session, turns } => {
                self.session = session;
                self.transcript = Transcript::default();
                self.transcript.hydrate(turns);
                self.queued.clear();
                self.clear_options();
            }
            ServerMessage::Settings { settings } => {
                // The client's own state, the whole map. Only the favourites
                // have a reader today; the rest is kept for the ones to come.
                self.favorites = settings
                    .get(FAVORITES_KEY)
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
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

    /// The options belong to the open session; a new one starts with none
    /// until the daemon reports them.
    fn clear_options(&mut self) {
        self.options.clear();
        self.close_picker();
    }

    fn close_picker(&mut self) {
        self.open_picker = None;
        self.picker_tab = None;
        self.open_editor = None;
    }

    /// Opens the option's menu, or closes it when it is already open.
    fn toggle_picker(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        match id {
            Some(id) => {
                self.open_picker = Some(id);
                self.picker_tab = None;
            }
            None => self.close_picker(),
        }
        cx.notify();
    }

    /// Sends the choice to the daemon and closes the menu. The refreshed list
    /// comes back as `Options`; until then the picker keeps its old value.
    fn choose_option(&mut self, id: String, value: String, cx: &mut Context<Self>) {
        self.close_picker();
        self.request(ClientRequest::SetOption {
            session: self.session.clone(),
            id,
            value,
        });
        cx.notify();
    }

    /// Switches the open model menu's tab. Only meaningful while it is open.
    fn pick_tab(&mut self, tab: String, cx: &mut Context<Self>) {
        self.picker_tab = Some(tab);
        cx.notify();
    }

    /// Stars or unstars a model and stores the list. The local list updates at
    /// once; the daemon's answer confirms the same map back.
    fn toggle_favorite(&mut self, value: String, cx: &mut Context<Self>) {
        if let Some(position) = self.favorites.iter().position(|id| id == &value) {
            self.favorites.remove(position);
        } else {
            self.favorites.push(value.clone());
        }
        self.request(ClientRequest::SetSetting {
            key: FAVORITES_KEY.to_owned(),
            value: Value::from(self.favorites.clone()),
        });
        cx.notify();
    }

    /// Opens or closes the effort editor beside a model row. Opening one row's
    /// editor closes another's — it is one editor, moved.
    fn toggle_editor(&mut self, model: Option<String>, cx: &mut Context<Self>) {
        self.open_editor = model;
        cx.notify();
    }

    /// Applies a model and its effort in one decision: the model switches
    /// only when it changes, and the effort rides after it. Both travel as
    /// plain agent settings; a refusal comes back as the daemon's error.
    fn choose_model_effort(
        &mut self,
        model: String,
        effort: String,
        value: String,
        cx: &mut Context<Self>,
    ) {
        self.close_picker();

        let current = self
            .options
            .iter()
            .find(|option| option.id == "model")
            .map(|option| option.current.as_str());
        if current != Some(model.as_str()) {
            self.request(ClientRequest::SetOption {
                session: self.session.clone(),
                id: "model".to_owned(),
                value: model,
            });
        }
        self.request(ClientRequest::SetOption {
            session: self.session.clone(),
            id: effort,
            value,
        });
        cx.notify();
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

    /// Editing keys for the composer. Escape closes an open option menu,
    /// Enter sends, Shift+Enter breaks the line, and everything else is the
    /// buffer's own business.
    fn input_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.control && !modifiers.alt && !modifiers.platform;

        if key == "escape" && self.open_editor.is_some() {
            self.open_editor = None;
            cx.notify();
            return;
        }

        if key == "escape" && self.open_picker.is_some() {
            self.close_picker();
            cx.notify();
            return;
        }

        // Escape stops the running turn, the way the reference does. The
        // menus above got the key first; a card owns the keyboard itself.
        if key == "escape" && self.transcript.status.running {
            self.stop_turn(cx);
            return;
        }

        match key {
            "enter" if !modifiers.shift => {
                // A running turn takes Enter as "queue" and Cmd/Ctrl+Enter as
                // "steer"; an idle session just sends.
                if self.transcript.status.running {
                    if modifiers.platform || modifiers.control {
                        self.steer(cx);
                    } else {
                        self.queue_prompt(cx);
                    }
                } else {
                    self.submit(cx);
                }
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
        self.request(ClientRequest::Prompt {
            session: self.session.clone(),
            text,
        });
        cx.notify();
    }

    /// Sends the composer's text while a turn runs. The daemon queues it and
    /// starts it when the current turn ends; the window draws the queue from
    /// the daemon's answer, not from a local hope.
    fn queue_prompt(&mut self, cx: &mut Context<Self>) {
        if self.input.is_blank() {
            return;
        }
        let text = self.input.take();
        self.request(ClientRequest::Prompt {
            session: self.session.clone(),
            text,
        });
        cx.notify();
    }

    /// Redirects the running turn: the daemon puts this text at the front of
    /// the queue and aborts the current turn, so it starts next with the
    /// agent's context intact.
    fn steer(&mut self, cx: &mut Context<Self>) {
        if self.input.is_blank() {
            return;
        }
        let text = self.input.take();
        self.request(ClientRequest::Steer {
            session: self.session.clone(),
            text,
        });
        cx.notify();
    }

    /// Drops one prompt that has not started. The daemon answers with the
    /// refreshed queue.
    fn remove_queued(&mut self, id: String, cx: &mut Context<Self>) {
        self.request(ClientRequest::RemoveQueued {
            session: self.session.clone(),
            id,
        });
        cx.notify();
    }

    /// Stops the turn running in the open session. The daemon cancels its
    /// cards and asks the agent to abort; the transcript closes it as aborted,
    /// not as an error.
    fn stop_turn(&mut self, cx: &mut Context<Self>) {
        if !self.transcript.status.running {
            return;
        }
        self.request(ClientRequest::CancelTurn {
            session: self.session.clone(),
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

/// The session a live message belongs to. The session list, settings, errors
/// and the handshake are not tied to one, and must pass through.
fn live_session(message: &ServerMessage) -> Option<&str> {
    match message {
        ServerMessage::TurnStarted { session, .. }
        | ServerMessage::Part { session, .. }
        | ServerMessage::TurnFinished { session, .. }
        | ServerMessage::PermissionRequested { session, .. }
        | ServerMessage::QuestionAsked { session, .. }
        | ServerMessage::Queue { session, .. } => Some(session),
        _ => None,
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

        let shell = cx.entity();
        let on_remove: RemoveHandler = Rc::new(move |id, _window, cx| {
            shell.update(cx, |shell, cx| shell.remove_queued(id, cx));
        });
        let queue = queued(&theme, &self.queued, on_remove);

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
            .children(queue)
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
                let shell = cx.entity();
                let on_stop: StopHandler = Rc::new(move |_window, cx| {
                    shell.update(cx, |shell, cx| shell.stop_turn(cx));
                });
                let shell = cx.entity();
                let on_toggle: ToggleHandler = Rc::new(move |id, _window, cx| {
                    shell.update(cx, |shell, cx| shell.toggle_picker(id, cx));
                });
                let shell = cx.entity();
                let on_choose: ChooseHandler = Rc::new(move |id, value, _window, cx| {
                    shell.update(cx, |shell, cx| shell.choose_option(id, value, cx));
                });
                let shell = cx.entity();
                let on_star: StarHandler = Rc::new(move |value, _window, cx| {
                    shell.update(cx, |shell, cx| shell.toggle_favorite(value, cx));
                });
                let shell = cx.entity();
                let on_tab: TabHandler = Rc::new(move |tab, _window, cx| {
                    shell.update(cx, |shell, cx| shell.pick_tab(tab, cx));
                });
                let shell = cx.entity();
                let on_edit: EditHandler = Rc::new(move |model, _window, cx| {
                    shell.update(cx, |shell, cx| shell.toggle_editor(model, cx));
                });
                let shell = cx.entity();
                let on_effort: EffortHandler = Rc::new(move |model, effort, value, _window, cx| {
                    shell.update(cx, |shell, cx| {
                        shell.choose_model_effort(model, effort, value, cx)
                    });
                });
                let focused = self.composer_focus.is_focused(window);

                column.child(composer(
                    &theme,
                    &self.workspace,
                    &self.transcript,
                    Pickers {
                        options: &self.options,
                        open: self.open_picker.as_deref(),
                        tab: self.picker_tab.as_deref(),
                        editor: self.open_editor.as_deref(),
                        favorites: &self.favorites,
                        agent_label: &self.daemon,
                        anchors: Rc::clone(&self.picker_anchors),
                        on_toggle,
                        on_choose,
                        on_star,
                        on_tab,
                        on_edit,
                        on_effort,
                    },
                    Field {
                        input: &self.input,
                        focus: &self.composer_focus,
                        focused,
                        on_key,
                        on_send,
                        on_stop,
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
