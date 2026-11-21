//! The core: sessions and turn orchestration.
//!
//! The core owns all state and speaks the protocol to whoever is connected.
//! Providers are reached only through the Driver trait, so the core never
//! learns a provider's native protocol and providers never learn about the
//! core. Persistence and Git join this crate as the same kind of owned piece;
//! no other crate touches them directly.
//!
//! The registry is in memory for now. Sessions become rows in `circulo-git` and
//! SQLite when that piece lands, without changing this surface.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use circulo_protocol::{FinishReason, PermissionRequest, Question, QuestionAction, ServerMessage};
use circulo_provider::{Answer, Asker, Decision, Driver};
use uuid::Uuid;

pub struct Session {
    pub id: String,
    pub project: Option<String>,
}

pub struct Core {
    driver: Arc<dyn Driver>,
    sessions: Mutex<Vec<Session>>,
    turns: AtomicU64,
    permissions: Broker<Decision>,
    questions: Broker<Answer>,
}

impl Core {
    pub fn new(driver: Arc<dyn Driver>) -> Self {
        Self {
            driver,
            sessions: Mutex::new(Vec::new()),
            turns: AtomicU64::new(0),
            permissions: Broker::new("p"),
            questions: Broker::new("q"),
        }
    }

    /// Answers a pending permission request. `None` cancels it, which is only
    /// right when the turn itself is going away. Returns false for an id that
    /// is not open, so a late answer changes nothing.
    pub fn answer_permission(&self, id: &str, option: Option<String>) -> bool {
        let decision = match option {
            Some(option) => Decision::Selected(option),
            None => Decision::Cancelled,
        };
        self.permissions.answer(id, decision)
    }

    /// Answers a pending question. Returns false for an id that is not open.
    pub fn answer_question(
        &self,
        id: &str,
        action: QuestionAction,
        answers: std::collections::BTreeMap<String, String>,
    ) -> bool {
        let answer = match action {
            QuestionAction::Accept => Answer::accept(answers),
            QuestionAction::Decline => Answer::decline(),
            QuestionAction::Cancel => Answer::cancel(),
        };
        self.questions.answer(id, answer)
    }

    /// Drops every open request: the client is gone, so nothing can answer.
    /// The waiting drivers read that as cancelled and end their turns.
    pub fn cancel_permissions(&self) {
        self.permissions.cancel_all();
        self.questions.cancel_all();
    }

    pub fn driver_name(&self) -> &'static str {
        self.driver.name()
    }

    /// Opens a session and returns its id. The project is recorded but nothing
    /// is started yet: no process, no worktree.
    pub fn start_session(&self, project: Option<String>) -> String {
        let id = Uuid::new_v4().to_string();
        self.sessions().push(Session {
            id: id.clone(),
            project,
        });
        id
    }

    pub fn has_session(&self, id: &str) -> bool {
        self.sessions().iter().any(|session| session.id == id)
    }

    /// Runs one turn and emits its messages in order. The turn id is unique per
    /// core and stable for the duration of the turn, so a client can attach
    /// parts to the turn that produced them.
    pub fn run_turn(&self, session: &str, prompt: &str, mut emit: impl FnMut(ServerMessage)) {
        if !self.has_session(session) {
            emit(ServerMessage::Error {
                message: format!("unknown session {session}"),
            });
            return;
        }

        let turn = self.turns.fetch_add(1, Ordering::Relaxed) + 1;
        emit(ServerMessage::TurnStarted {
            session: session.to_owned(),
            turn,
        });

        // The driver gets two callbacks into the same emit: parts and
        // questions. `RefCell` keeps that honest without cloning the wire.
        let emit = RefCell::new(emit);
        let part_session = session.to_owned();
        let mut parts = |part| {
            (emit.borrow_mut())(ServerMessage::Part {
                session: part_session.clone(),
                turn,
                part,
            });
        };
        let mut asker = TurnAsker {
            permissions: &self.permissions,
            questions: &self.questions,
            session,
            turn,
            emit: &emit,
        };

        let result = self.driver.start_turn(prompt, &mut asker, &mut parts);

        let send = |message| (emit.borrow_mut())(message);

        let reason = match result {
            Ok(()) => FinishReason::Completed,
            Err(error) => {
                send(ServerMessage::Error {
                    message: error.to_string(),
                });
                FinishReason::Failed
            }
        };

        send(ServerMessage::TurnFinished {
            session: session.to_owned(),
            turn,
            reason,
        });
    }

    fn sessions(&self) -> std::sync::MutexGuard<'_, Vec<Session>> {
        // A panicking thread must not take the daemon down with it.
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The driver's view of the user: sends a card and waits for the answer.
struct TurnAsker<'a, F: FnMut(ServerMessage)> {
    permissions: &'a Broker<Decision>,
    questions: &'a Broker<Answer>,
    session: &'a str,
    turn: u64,
    emit: &'a RefCell<F>,
}

impl<F: FnMut(ServerMessage)> Asker for TurnAsker<'_, F> {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        let (id, answer) = self.permissions.open();

        (self.emit.borrow_mut())(ServerMessage::PermissionRequested {
            session: self.session.to_owned(),
            turn: self.turn,
            id: id.clone(),
            request,
        });

        // A closed channel means the request was dropped: the client left, or
        // the daemon cancelled it. Either way the tool must not run.
        let decision = answer.recv().unwrap_or(Decision::Cancelled);
        self.permissions.close(&id);
        decision
    }

    fn question(&mut self, question: Question) -> Answer {
        let (id, answer) = self.questions.open();

        (self.emit.borrow_mut())(ServerMessage::QuestionAsked {
            session: self.session.to_owned(),
            turn: self.turn,
            id: id.clone(),
            question,
        });

        let answer = answer.recv().unwrap_or(Answer::cancel());
        self.questions.close(&id);
        answer
    }
}

/// Open requests of one kind, keyed by the id the client saw. The prefix keeps
/// permissions and questions apart on the wire ("p-3", "q-4").
struct Broker<T> {
    prefix: &'static str,
    next: AtomicU64,
    pending: Mutex<HashMap<String, Sender<T>>>,
}

impl<T> Broker<T> {
    fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
        }
    }

    fn open(&self) -> (String, Receiver<T>) {
        let id = format!(
            "{}-{}",
            self.prefix,
            self.next.fetch_add(1, Ordering::Relaxed) + 1
        );
        let (send, receive) = channel();
        self.pending().insert(id.clone(), send);
        (id, receive)
    }

    fn answer(&self, id: &str, answer: T) -> bool {
        let sender = self.pending().remove(id);
        sender.is_some_and(|sender| sender.send(answer).is_ok())
    }

    fn close(&self, id: &str) {
        self.pending().remove(id);
    }

    fn cancel_all(&self) {
        // Dropping the senders is the cancellation: every waiting asker wakes
        // with a closed channel and answers the agent with cancelled.
        self.pending().clear();
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, HashMap<String, Sender<T>>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_protocol::Part;
    use circulo_provider::DriverError;

    struct Echo;

    impl Driver for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn start_turn(
            &self,
            prompt: &str,
            _: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<(), DriverError> {
            emit(Part::Text {
                delta: prompt.to_owned(),
            });
            Ok(())
        }
    }

    /// A driver that asks a question and reports what it was told.
    struct AsksQuestion;

    impl Driver for AsksQuestion {
        fn name(&self) -> &'static str {
            "asks-question"
        }

        fn start_turn(
            &self,
            _: &str,
            ask: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<(), DriverError> {
            let answer = ask.question(Question {
                message: "How should I approach this?".into(),
                fields: vec![circulo_protocol::QuestionField {
                    id: "strategy".into(),
                    label: "Strategy".into(),
                    kind: circulo_protocol::QuestionKind::Choice {
                        options: vec!["conservative".into(), "balanced".into()],
                    },
                    required: true,
                }],
            });

            let delta = match answer.action {
                QuestionAction::Accept => format!(
                    "accepted {}",
                    answer.values.get("strategy").cloned().unwrap_or_default()
                ),
                other => format!("{other:?}").to_lowercase(),
            };
            emit(Part::Text { delta });
            Ok(())
        }
    }

    struct Broken;

    impl Driver for Broken {
        fn name(&self) -> &'static str {
            "broken"
        }

        fn start_turn(
            &self,
            _: &str,
            _: &mut dyn Asker,
            _: &mut dyn FnMut(Part),
        ) -> Result<(), DriverError> {
            Err(DriverError::new("no"))
        }
    }

    fn core_with(driver: Arc<dyn Driver>) -> Core {
        Core::new(driver)
    }

    #[test]
    fn a_turn_is_bracketed_and_streams_its_parts() {
        let core = core_with(Arc::new(Echo));
        let session = core.start_session(Some("/tmp/project".into()));

        let mut messages = Vec::new();
        core.run_turn(&session, "hello", |message| messages.push(message));

        assert!(matches!(
            messages.first(),
            Some(ServerMessage::TurnStarted { turn: 1, .. })
        ));
        assert!(matches!(
            messages.get(1),
            Some(ServerMessage::Part { part: Part::Text { delta }, .. }) if delta == "hello"
        ));
        assert!(matches!(
            messages.last(),
            Some(ServerMessage::TurnFinished {
                turn: 1,
                reason: FinishReason::Completed,
                ..
            })
        ));
    }

    /// A driver that stops for the user before it answers.
    struct AsksOnce;

    impl Driver for AsksOnce {
        fn name(&self) -> &'static str {
            "asks"
        }

        fn start_turn(
            &self,
            _: &str,
            ask: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<(), DriverError> {
            let answer = ask.permission(PermissionRequest {
                title: "Run cargo test".into(),
                detail: "cargo test --workspace".into(),
                options: vec![
                    circulo_protocol::PermissionOption {
                        id: "allow-once".into(),
                        label: "Allow once".into(),
                        kind: circulo_protocol::PermissionOptionKind::AllowOnce,
                    },
                    circulo_protocol::PermissionOption {
                        id: "reject-once".into(),
                        label: "Reject".into(),
                        kind: circulo_protocol::PermissionOptionKind::RejectOnce,
                    },
                ],
            });

            let delta = match answer {
                Decision::Selected(option) => format!("chose {option}"),
                Decision::Cancelled => "cancelled".into(),
            };
            emit(Part::Text { delta });
            Ok(())
        }
    }

    /// Runs a turn on its own thread and hands back its messages as they come,
    /// so the test can answer while the turn is blocked.
    fn run_in_background(
        core: Arc<Core>,
        session: String,
    ) -> std::sync::mpsc::Receiver<ServerMessage> {
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            core.run_turn(&session, "go", move |message| {
                let _ = send.send(message);
            })
        });
        receive
    }

    #[test]
    fn a_permission_request_blocks_the_turn_until_it_is_answered() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let session = core.start_session(None);
        let messages = run_in_background(Arc::clone(&core), session);

        let mut answered = false;
        let mut text = None;
        let mut finished = false;

        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::PermissionRequested { id, .. } => {
                    assert!(core.answer_permission(&id, Some("allow-once".into())));
                    answered = true;
                }
                ServerMessage::Part {
                    part: Part::Text { delta },
                    ..
                } => text = Some(delta),
                ServerMessage::TurnFinished { .. } => {
                    finished = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(answered, "the card reached the client");
        assert!(finished, "the turn ended after the answer");
        assert_eq!(text.as_deref(), Some("chose allow-once"));
    }

    #[test]
    fn a_client_that_goes_away_cancels_the_request() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let session = core.start_session(None);
        let messages = run_in_background(Arc::clone(&core), session);

        let mut text = None;
        while let Ok(message) = messages.recv() {
            match message {
                // The client is gone the moment the card would have appeared.
                ServerMessage::PermissionRequested { .. } => core.cancel_permissions(),
                ServerMessage::Part {
                    part: Part::Text { delta },
                    ..
                } => text = Some(delta),
                ServerMessage::TurnFinished { .. } => break,
                _ => {}
            }
        }

        assert_eq!(text.as_deref(), Some("cancelled"), "nothing ran");
    }

    #[test]
    fn a_question_blocks_the_turn_until_it_is_answered() {
        let core = Arc::new(core_with(Arc::new(AsksQuestion)));
        let session = core.start_session(None);
        let messages = run_in_background(Arc::clone(&core), session);

        let mut answered = false;
        let mut text = None;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::QuestionAsked { id, question, .. } => {
                    assert_eq!(question.message, "How should I approach this?");
                    assert_eq!(question.fields[0].id, "strategy");
                    let answers = std::collections::BTreeMap::from([(
                        "strategy".to_string(),
                        "balanced".to_string(),
                    )]);
                    assert!(core.answer_question(&id, QuestionAction::Accept, answers));
                    answered = true;
                }
                ServerMessage::Part {
                    part: Part::Text { delta },
                    ..
                } => text = Some(delta),
                ServerMessage::TurnFinished { .. } => break,
                _ => {}
            }
        }

        assert!(answered);
        assert_eq!(text.as_deref(), Some("accepted balanced"));
    }

    #[test]
    fn a_declined_question_tells_the_agent_so() {
        let core = Arc::new(core_with(Arc::new(AsksQuestion)));
        let session = core.start_session(None);
        let messages = run_in_background(Arc::clone(&core), session);

        let mut text = None;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::QuestionAsked { id, .. } => {
                    assert!(core.answer_question(&id, QuestionAction::Decline, Default::default()));
                }
                ServerMessage::Part {
                    part: Part::Text { delta },
                    ..
                } => text = Some(delta),
                ServerMessage::TurnFinished { .. } => break,
                _ => {}
            }
        }

        assert_eq!(text.as_deref(), Some("decline"));
    }

    #[test]
    fn turns_are_numbered_in_order() {
        let core = core_with(Arc::new(Echo));
        let session = core.start_session(None);

        let mut turns = Vec::new();
        for _ in 0..2 {
            core.run_turn(&session, "hi", |message| {
                if let ServerMessage::TurnStarted { turn, .. } = message {
                    turns.push(turn);
                }
            });
        }

        assert_eq!(turns, vec![1, 2]);
    }

    #[test]
    fn an_unknown_session_is_an_error() {
        let core = core_with(Arc::new(Echo));

        let mut messages = Vec::new();
        core.run_turn("nope", "hello", |message| messages.push(message));

        assert!(matches!(messages.as_slice(), [ServerMessage::Error { .. }]));
    }

    #[test]
    fn a_failing_driver_finishes_the_turn_as_failed() {
        let core = core_with(Arc::new(Broken));
        let session = core.start_session(None);

        let mut messages = Vec::new();
        core.run_turn(&session, "hello", |message| messages.push(message));

        assert!(
            messages
                .iter()
                .any(|message| matches!(message, ServerMessage::Error { .. }))
        );
        assert!(matches!(
            messages.last(),
            Some(ServerMessage::TurnFinished {
                reason: FinishReason::Failed,
                ..
            })
        ));
    }
}
