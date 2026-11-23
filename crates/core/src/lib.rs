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
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use circulo_persistence::{Store, StoreError};
use circulo_protocol::{
    ConfigOption, FinishReason, HistoryTurn, PermissionRequest, Question, QuestionAction,
    ServerMessage, SessionSummary,
};
use circulo_provider::{Answer, Asker, Decision, Driver, SessionRef};
use serde_json::Value;
use uuid::Uuid;

pub struct Session {
    pub id: String,
    pub project: Option<String>,
}

pub struct Core {
    driver: Arc<dyn Driver>,
    sessions: Mutex<Vec<Session>>,
    /// The database. `None` for a memory-only core, which is what a test wants.
    store: Option<Arc<Store>>,
    turns: AtomicU64,
    permissions: Broker<Decision>,
    questions: Broker<Answer>,
}

impl Core {
    pub fn new(driver: Arc<dyn Driver>) -> Self {
        Self::with_store(driver, None)
    }

    /// A core that also writes sessions, turns and parts to the database.
    pub fn with_store(driver: Arc<dyn Driver>, store: Option<Arc<Store>>) -> Self {
        Self {
            driver,
            sessions: Mutex::new(Vec::new()),
            store,
            turns: AtomicU64::new(0),
            permissions: Broker::new("p"),
            questions: Broker::new("q"),
        }
    }

    /// Sessions from the database, newest first. A memory-only core has none:
    /// its sessions live and die with the process.
    pub fn sessions(&self) -> Result<Vec<SessionSummary>, StoreError> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };

        // The core is the one place that knows both shapes, so it translates
        // here instead of leaking a storage type onto the wire.
        Ok(store
            .sessions()?
            .into_iter()
            .map(|session| SessionSummary {
                id: session.id,
                project: session.project,
                title: session.title,
                updated_at: session.updated_at,
            })
            .collect())
    }

    /// Opens a session that already exists in the database so a client can
    /// continue it. Returns false when there is no such session.
    pub fn reopen_session(&self, id: &str) -> Result<bool, StoreError> {
        let Some(store) = &self.store else {
            return Ok(false);
        };
        if !store.has_session(id)? {
            return Ok(false);
        }

        let mut open = self.open_sessions();
        if !open.iter().any(|session| session.id == id) {
            open.push(Session {
                id: id.to_owned(),
                project: None,
            });
        }
        Ok(true)
    }

    /// The stored transcript of a session, ready for the UI to draw again.
    pub fn transcript(&self, id: &str) -> Result<Vec<HistoryTurn>, StoreError> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };

        Ok(store
            .transcript(id)?
            .into_iter()
            .map(|turn| HistoryTurn {
                turn: turn.turn,
                prompt: turn.prompt,
                parts: turn.parts,
            })
            .collect())
    }

    /// The client's own settings — interface state the core only stores.
    /// A memory-only core has none.
    pub fn settings(&self) -> Result<BTreeMap<String, Value>, StoreError> {
        match &self.store {
            Some(store) => store.settings(),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Records one client setting and returns the whole map, the way the
    /// wire answers. A write failure reads as an error message, not a lost
    /// turn: settings are never urgent.
    pub fn set_setting(&self, key: &str, value: &Value) -> Result<BTreeMap<String, Value>, String> {
        let Some(store) = &self.store else {
            return Err("this daemon has no settings store".to_owned());
        };
        store
            .set_setting(key, value)
            .map_err(|error| error.to_string())?;
        self.settings().map_err(|error| error.to_string())
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

    /// The daemon is on its way out: open cards can no longer be answered,
    /// and the provider must let its agents go — their processes belong to
    /// the provider, and this is what keeps none of them behind.
    pub fn shutdown(&self) {
        self.cancel_permissions();
        self.driver.shutdown();
    }

    pub fn driver_name(&self) -> &'static str {
        self.driver.name()
    }

    /// The agent's current settings for a session the client holds open.
    /// Spawning an agent takes a moment, so the daemon asks from its own
    /// thread; an unknown session reads as an error message.
    pub fn load_options(&self, session: &str) -> Result<Vec<ConfigOption>, String> {
        if !self.has_session(session) {
            return Err(format!("unknown session {session}"));
        }
        self.driver
            .options(session)
            .map_err(|error| error.to_string())
    }

    /// Applies the user's choice and returns the refreshed list, as the agent
    /// reports it.
    pub fn set_option(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Vec<ConfigOption>, String> {
        if !self.has_session(session) {
            return Err(format!("unknown session {session}"));
        }
        self.driver
            .set_option(session, id, value)
            .map_err(|error| error.to_string())
    }

    /// Opens a session and returns its id. The project is recorded but nothing
    /// is started yet: no process, no worktree.
    pub fn start_session(&self, project: Option<String>) -> String {
        let id = Uuid::new_v4().to_string();
        if let Some(store) = &self.store {
            // The title comes from the first prompt; a failed write must not
            // stop the session from running in memory.
            let _ = store.start_session(&id, project.as_deref(), self.driver.name());
        }
        self.open_sessions().push(Session {
            id: id.clone(),
            project,
        });
        id
    }

    pub fn has_session(&self, id: &str) -> bool {
        self.open_sessions().iter().any(|session| session.id == id)
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
        if let Some(store) = &self.store {
            let _ = store.start_turn(session, turn, prompt);
        }

        let part_session = session.to_owned();
        let mut position = 0;
        let mut parts = |part| {
            if let Some(store) = &self.store {
                // The transcript and the client see the same part in the same
                // order; a write failure must not interrupt the stream.
                let _ = store.append_part(session, turn, position, &part);
            }
            position += 1;

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

        // The key the provider handed back last turn is its way back in after
        // a restart; a write failure must not stop the turn from running.
        let resume = match &self.store {
            Some(store) => store.provider_session(session).ok().flatten(),
            None => None,
        };
        let result = self.driver.start_turn(
            SessionRef {
                id: session,
                resume: resume.as_deref(),
            },
            prompt,
            &mut asker,
            &mut parts,
        );

        // Whatever the provider handed back is what the next turn — maybe of
        // the next daemon — resumes with. `None` clears a stale key.
        if let Ok(key) = &result {
            if let Some(store) = &self.store {
                let _ = store.set_provider_session(session, key.as_deref());
            }
        }

        let send = |message| (emit.borrow_mut())(message);

        let reason = match result {
            Ok(_) => FinishReason::Completed,
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

    fn open_sessions(&self) -> std::sync::MutexGuard<'_, Vec<Session>> {
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
    use std::sync::Mutex;

    struct Echo;

    impl Driver for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn start_turn(
            &self,
            _: SessionRef<'_>,
            prompt: &str,
            _: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
            emit(Part::Text {
                delta: prompt.to_owned(),
            });
            Ok(None)
        }

        fn options(&self, _: &str) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Ok(Vec::new())
        }

        fn set_option(
            &self,
            _: &str,
            _id: &str,
            _value: &str,
        ) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Err(DriverError::new("this agent has no settings"))
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
            _: SessionRef<'_>,
            _: &str,
            ask: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
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
            Ok(None)
        }

        fn options(&self, _: &str) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Ok(Vec::new())
        }

        fn set_option(
            &self,
            _: &str,
            _id: &str,
            _value: &str,
        ) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Err(DriverError::new("this agent has no settings"))
        }
    }

    struct Broken;

    impl Driver for Broken {
        fn name(&self) -> &'static str {
            "broken"
        }

        fn start_turn(
            &self,
            _: SessionRef<'_>,
            _: &str,
            _: &mut dyn Asker,
            _: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
            Err(DriverError::new("no"))
        }

        fn options(&self, _: &str) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Ok(Vec::new())
        }

        fn set_option(
            &self,
            _: &str,
            _id: &str,
            _value: &str,
        ) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Err(DriverError::new("this agent has no settings"))
        }
    }

    /// A driver that remembers sessions: it records what it was asked to
    /// resume with and hands `give` back as its key, the way OpenCode would.
    struct Remembers {
        seen: Arc<Mutex<Vec<Option<String>>>>,
        give: Option<String>,
    }

    impl Remembers {
        fn giving(key: &str) -> (Self, Arc<Mutex<Vec<Option<String>>>>) {
            let seen = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    seen: Arc::clone(&seen),
                    give: Some(key.to_owned()),
                },
                seen,
            )
        }
    }

    impl Driver for Remembers {
        fn name(&self) -> &'static str {
            "remembers"
        }

        fn start_turn(
            &self,
            session: SessionRef<'_>,
            _: &str,
            _: &mut dyn Asker,
            _: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(session.resume.map(str::to_owned));
            Ok(self.give.clone())
        }

        fn options(&self, _: &str) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Ok(Vec::new())
        }

        fn set_option(
            &self,
            _: &str,
            _id: &str,
            _value: &str,
        ) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Err(DriverError::new("this agent has no settings"))
        }
    }

    fn core_with(driver: Arc<dyn Driver>) -> Core {
        Core::new(driver)
    }

    fn core_with_store(driver: Arc<dyn Driver>) -> Core {
        let store = Arc::new(circulo_persistence::Store::in_memory().expect("store"));
        Core::with_store(driver, Some(store))
    }

    #[test]
    fn options_need_an_open_session_and_follow_the_driver() {
        let core = core_with(Arc::new(Echo));
        assert!(core.load_options("missing").is_err());
        assert!(core.set_option("missing", "model", "x").is_err());

        let session = core.start_session(None);
        assert_eq!(
            core.load_options(&session).expect("options"),
            Vec::new(),
            "the echo agent has no settings"
        );
        assert!(
            core.set_option(&session, "model", "x").is_err(),
            "and nothing to set"
        );
    }

    #[test]
    fn a_core_with_a_store_writes_what_the_window_draws() {
        let core = core_with_store(Arc::new(Echo));
        let session = core.start_session(Some("/tmp/project".into()));

        core.run_turn(&session, "hello", |_| {});

        let sessions = core.sessions().expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session);
        assert_eq!(sessions[0].title, "hello", "the first prompt titles it");

        let turns = core.transcript(&session).expect("transcript");
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].prompt, "hello");
        assert_eq!(
            turns[0].parts,
            vec![Part::Text {
                delta: "hello".into()
            }]
        );
    }

    #[test]
    fn a_stored_session_can_be_reopened_and_keeps_its_turns() {
        // One database, two cores: the second is a daemon that restarted.
        let store = Arc::new(circulo_persistence::Store::in_memory().expect("store"));

        let core = Core::with_store(Arc::new(Echo), Some(Arc::clone(&store)));
        let session = core.start_session(None);
        core.run_turn(&session, "one", |_| {});

        let restarted = Core::with_store(Arc::new(Echo), Some(store));
        assert!(restarted.reopen_session(&session).expect("reopen"));
        assert!(!restarted.reopen_session("missing").expect("reopen"));
        assert_eq!(restarted.transcript(&session).expect("transcript").len(), 1);
    }

    #[test]
    fn a_provider_key_travels_from_turn_to_turn() {
        let (driver, seen) = Remembers::giving("k1");
        let core = core_with_store(Arc::new(driver));
        let session = core.start_session(None);

        core.run_turn(&session, "one", |_| {});
        core.run_turn(&session, "two", |_| {});

        let seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            seen.as_slice(),
            &[None, Some("k1".to_owned())],
            "the first turn has nothing to resume; the second carries the stored key"
        );
    }

    #[test]
    fn a_reopened_core_resumes_with_the_stored_key() {
        let store = Arc::new(circulo_persistence::Store::in_memory().expect("store"));
        let (driver, _seen) = Remembers::giving("k1");
        let core = Core::with_store(Arc::new(driver), Some(Arc::clone(&store)));
        let session = core.start_session(None);
        core.run_turn(&session, "one", |_| {});

        // A daemon restart: a fresh core over the same database.
        let (driver, seen_again) = Remembers::giving("k1");
        let restarted = Core::with_store(Arc::new(driver), Some(store));
        restarted.reopen_session(&session).expect("reopen");
        restarted.run_turn(&session, "two", |_| {});

        let seen = seen_again.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            seen.as_slice(),
            &[Some("k1".to_owned())],
            "the restart resumes the provider's session, not a fresh one"
        );
    }

    #[test]
    fn a_provider_that_loses_its_key_clears_it() {
        let store = Arc::new(circulo_persistence::Store::in_memory().expect("store"));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let driver = Remembers {
            seen: Arc::clone(&seen),
            give: None,
        };
        let core = Core::with_store(Arc::new(driver), Some(Arc::clone(&store)));
        let session = core.start_session(None);
        store
            .set_provider_session(&session, Some("stale"))
            .expect("seed a stale key");

        core.run_turn(&session, "one", |_| {});

        assert_eq!(
            store.provider_session(&session).expect("key"),
            None,
            "a provider with no session must not leave a stale key behind"
        );
        let seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(seen.first(), Some(&Some("stale".to_owned())));
    }

    #[test]
    fn settings_are_stored_and_refused_without_a_store() {
        let core = core_with(Arc::new(Echo));
        assert!(core.settings().expect("settings").is_empty());
        assert!(
            core.set_setting("favorites.model", &serde_json::json!(["a"]))
                .is_err()
        );

        let core = core_with_store(Arc::new(Echo));
        let settings = core
            .set_setting("favorites.model", &serde_json::json!(["a"]))
            .expect("set");
        assert_eq!(
            settings.get("favorites.model"),
            Some(&serde_json::json!(["a"])),
            "the answer carries the whole map, stored form"
        );
        assert_eq!(core.settings().expect("read").len(), 1);
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
            _: SessionRef<'_>,
            _: &str,
            ask: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
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
            Ok(None)
        }

        fn options(&self, _: &str) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Ok(Vec::new())
        }

        fn set_option(
            &self,
            _: &str,
            _id: &str,
            _value: &str,
        ) -> Result<Vec<circulo_protocol::ConfigOption>, DriverError> {
            Err(DriverError::new("this agent has no settings"))
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
