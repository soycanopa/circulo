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
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use circulo_git::CheckpointKind;
use circulo_persistence::{Store, StoreError};
use circulo_protocol::{
    ConfigOption, FinishReason, HistoryTurn, PermissionRequest, ProjectSummary, Question,
    QuestionAction, QueuedPrompt, ServerMessage, SessionSummary,
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
    /// The turn running for each session. `run_turn` registers it before the
    /// driver starts, so a cancel can find the cards that turn owns instead of
    /// closing every session's cards at once.
    in_flight: Mutex<HashMap<String, u64>>,
    /// Prompts waiting for their turn, one FIFO per session. The session's
    /// worker drains them; nothing here survives a daemon restart, because a
    /// prompt that never started is not part of the transcript.
    queues: Mutex<HashMap<String, SessionQueue>>,
    permissions: Broker<Decision>,
    questions: Broker<Answer>,
}

/// One session's pending prompts and the worker that drains them.
#[derive(Default)]
struct SessionQueue {
    pending: VecDeque<QueuedPrompt>,
    /// A worker thread is draining this session.
    worker: bool,
    /// The client that fed the queue is gone: finish the current turn, then
    /// stop instead of starting the next prompt.
    stopped: bool,
}

impl SessionQueue {
    fn items(&self) -> Vec<QueuedPrompt> {
        self.pending.iter().cloned().collect()
    }
}

/// What a caller learns after a prompt joins the queue: the whole list, and
/// whether it has to start the session's worker.
pub struct QueueState {
    pub items: Vec<QueuedPrompt>,
    pub start_worker: bool,
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
            in_flight: Mutex::new(HashMap::new()),
            queues: Mutex::new(HashMap::new()),
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

    /// The folders the user added, as the sidebar draws them. A memory-only
    /// core has none.
    pub fn projects(&self) -> Result<Vec<ProjectSummary>, StoreError> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };
        Ok(store
            .projects()?
            .into_iter()
            .map(|project| ProjectSummary {
                path: project.path,
                name: project.name,
            })
            .collect())
    }

    /// Adds a folder as a project. The daemon owns the filesystem, so the path
    /// is canonicalized and checked here; a path that is not a directory is an
    /// honest error to the client. Returns the whole list, the way the wire
    /// answers.
    pub fn add_project(&self, path: &str) -> Result<(String, Vec<ProjectSummary>), String> {
        let Some(store) = &self.store else {
            return Err("this daemon has no project store".to_owned());
        };
        let canonical =
            std::fs::canonicalize(path).map_err(|error| format!("cannot open {path}: {error}"))?;
        if !canonical.is_dir() {
            return Err(format!("{} is not a folder", canonical.display()));
        }
        let name = canonical
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| canonical.to_string_lossy().into_owned());
        let canonical = canonical.to_string_lossy().into_owned();
        store
            .add_project(&canonical, &name)
            .map_err(|error| error.to_string())?;
        let projects = self.projects().map_err(|error| error.to_string())?;
        Ok((canonical, projects))
    }

    /// Removes a project from the list. The folder and its sessions stay:
    /// this is a sidebar entry, not a delete.
    pub fn remove_project(&self, path: &str) -> Result<Vec<ProjectSummary>, String> {
        let Some(store) = &self.store else {
            return Err("this daemon has no project store".to_owned());
        };
        store
            .remove_project(path)
            .map_err(|error| error.to_string())?;
        self.projects().map_err(|error| error.to_string())
    }

    /// The Git facts for a session: whether its project is a repository and,
    /// if so, the branch it is on. A projectless session or a plain folder is
    /// `(false, None)`; the window hides the branch chip then.
    pub fn repo_status(&self, session: &str) -> (bool, Option<String>) {
        let project = self
            .open_sessions()
            .iter()
            .find(|open| open.id == session)
            .and_then(|open| open.project.clone());
        let Some(project) = project else {
            return (false, None);
        };

        let path = std::path::Path::new(&project);
        let repo = circulo_git::is_repo(path);
        let branch = if repo {
            circulo_git::branch(path)
        } else {
            None
        };
        (repo, branch)
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

        let project = store.session_project(id)?;
        let mut open = self.open_sessions();
        if !open.iter().any(|session| session.id == id) {
            open.push(Session {
                id: id.to_owned(),
                // The project is part of the session, not of the window that
                // happened to create it: a restart has to bring it back too.
                project,
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

    /// Stops the turn running in `session`: its open cards are cancelled (the
    /// blocked asker reads `Cancelled`) and the provider is asked to abort. The
    /// turn ends as `Aborted`. Prompts already queued stay queued. A session
    /// with no turn in flight is an error, so a stale client gets an honest
    /// answer instead of a silent no-op.
    pub fn cancel_turn(&self, session: &str) -> Result<(), String> {
        let Some(turn) = self.in_flight().get(session).copied() else {
            return Err(format!("no turn is running in {session}"));
        };
        self.permissions.cancel_turn(session, turn);
        self.questions.cancel_turn(session, turn);
        self.driver
            .cancel(session)
            .map_err(|error| error.to_string())
    }

    /// Adds `text` to the session's queue. `start_worker` is true when the
    /// caller has to run `drain` on a thread: no worker owns the session.
    pub fn enqueue(&self, session: &str, text: &str) -> Result<QueueState, String> {
        if !self.has_session(session) {
            return Err(format!("unknown session {session}"));
        }
        let mut queues = self.queues();
        let queue = queues.entry(session.to_owned()).or_default();
        queue.pending.push_back(QueuedPrompt {
            id: Uuid::new_v4().to_string(),
            text: text.to_owned(),
        });
        // A live client means the worker may run again.
        queue.stopped = false;
        let start_worker = !queue.worker;
        queue.worker = true;
        Ok(QueueState {
            items: queue.items(),
            start_worker,
        })
    }

    /// Puts `text` at the front and aborts the running turn, so it starts next
    /// with the provider session's context intact. With no turn running it is
    /// just a prompt at the front. A provider that cannot abort falls back to
    /// running it after the current turn: the text is never lost.
    pub fn steer(&self, session: &str, text: &str) -> Result<QueueState, String> {
        if !self.has_session(session) {
            return Err(format!("unknown session {session}"));
        }
        let state = {
            let mut queues = self.queues();
            let queue = queues.entry(session.to_owned()).or_default();
            queue.pending.push_front(QueuedPrompt {
                id: Uuid::new_v4().to_string(),
                text: text.to_owned(),
            });
            queue.stopped = false;
            let start_worker = !queue.worker;
            queue.worker = true;
            QueueState {
                items: queue.items(),
                start_worker,
            }
        };

        // Outside the queue lock: the worker takes it between turns, and the
        // driver may take its time. "No turn in flight" is not a failure here:
        // the item is already at the front.
        let _ = self.cancel_turn(session);
        Ok(state)
    }

    /// Drops one prompt that has not started. Returns the refreshed list.
    pub fn remove_queued(&self, session: &str, id: &str) -> Result<Vec<QueuedPrompt>, String> {
        let mut queues = self.queues();
        let Some(queue) = queues.get_mut(session) else {
            return Err(format!("unknown session {session}"));
        };
        // The item may have started between the click and this request. A
        // stale removal is a no-op, and the refreshed list is the honest
        // answer; an error banner would only blame the user for a race.
        queue.pending.retain(|item| item.id != id);
        Ok(queue.items())
    }

    /// The prompts waiting in `session`, oldest first.
    pub fn queued(&self, session: &str) -> Vec<QueuedPrompt> {
        self.queues()
            .get(session)
            .map(SessionQueue::items)
            .unwrap_or_default()
    }

    /// Drops everything waiting and tells the worker not to start another
    /// prompt. The caller cancels the turn in flight separately.
    pub fn clear_queue(&self, session: &str) {
        let mut queues = self.queues();
        if let Some(queue) = queues.get_mut(session) {
            queue.pending.clear();
            queue.stopped = true;
        }
    }

    /// Runs the session's queue, one turn at a time, until it is empty or the
    /// client that fed it is gone. The caller owns a thread and the emit side.
    pub fn drain(&self, session: &str, mut emit: impl FnMut(ServerMessage)) {
        loop {
            let Some(item) = self.next_queued(session) else {
                return;
            };
            emit(ServerMessage::Queue {
                session: session.to_owned(),
                items: self.queued(session),
            });
            self.run_turn(session, &item.text, &mut emit);
        }
    }

    /// Pops the next prompt, or releases the worker when the queue is empty.
    /// Claiming and releasing happen under one lock, so a prompt that arrives
    /// just as the worker stops always gets a new worker.
    fn next_queued(&self, session: &str) -> Option<QueuedPrompt> {
        let mut queues = self.queues();
        let queue = queues.get_mut(session)?;
        if queue.stopped {
            queue.worker = false;
            return None;
        }
        match queue.pending.pop_front() {
            Some(item) => Some(item),
            None => {
                queue.worker = false;
                None
            }
        }
    }

    fn queues(&self) -> std::sync::MutexGuard<'_, HashMap<String, SessionQueue>> {
        self.queues.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Drops the cards of the turn running in `session`, so the driver's
    /// blocked asker reads `Cancelled` and the turn can end. A session with no
    /// turn in flight is untouched, and so is every other session.
    pub fn cancel_turn_requests(&self, session: &str) -> bool {
        let Some(turn) = self.in_flight().get(session).copied() else {
            return false;
        };
        self.permissions.cancel_turn(session, turn);
        self.questions.cancel_turn(session, turn);
        true
    }

    /// Drops every open request: the daemon is going away, so nothing can
    /// answer. The waiting drivers read that as cancelled and end their turns.
    pub fn cancel_all_requests(&self) {
        self.permissions.cancel_all();
        self.questions.cancel_all();
    }

    /// The daemon is on its way out: open cards can no longer be answered,
    /// and the provider must let its agents go — their processes belong to
    /// the provider, and this is what keeps none of them behind.
    pub fn shutdown(&self) {
        self.cancel_all_requests();
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
        // Registered before the client hears the turn started: a cancel that
        // arrives right after that message has to find it.
        self.in_flight().insert(session.to_owned(), turn);
        emit(ServerMessage::TurnStarted {
            session: session.to_owned(),
            turn,
            prompt: prompt.to_owned(),
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
        // The project is the folder the agent runs in. A session without one
        // leaves the provider on its configured fallback.
        let project = self
            .open_sessions()
            .iter()
            .find(|open| open.id == session)
            .and_then(|open| open.project.clone());
        let checkpoint_session = Uuid::parse_str(session).ok();
        let checkpoint = |kind| {
            if let (Some(path), Some(id)) = (project.as_deref(), checkpoint_session) {
                // A checkpoint is a safety net, never a reason a turn fails.
                let _ = circulo_git::capture(std::path::Path::new(path), id, turn, kind);
            }
        };

        checkpoint(CheckpointKind::Start);
        let result = self.driver.start_turn(
            SessionRef {
                id: session,
                resume: resume.as_deref(),
                cwd: project.as_deref(),
            },
            prompt,
            &mut asker,
            &mut parts,
        );
        checkpoint(CheckpointKind::End);

        // Whatever the provider handed back is what the next turn — maybe of
        // the next daemon — resumes with. `None` clears a stale key.
        if let Ok(key) = &result
            && let Some(store) = &self.store
        {
            let _ = store.set_provider_session(session, key.as_deref());
        }

        let send = |message| (emit.borrow_mut())(message);

        let reason = match result {
            Ok(_) => FinishReason::Completed,
            // The user stopped the turn; that is an outcome, not a failure.
            Err(error) if error.is_cancelled() => FinishReason::Aborted,
            Err(error) => {
                send(ServerMessage::Error {
                    message: error.to_string(),
                });
                FinishReason::Failed
            }
        };

        // Only this turn's registration is cleared: a newer turn on the same
        // session owns the slot by now. It goes before `TurnFinished` so a
        // client that cancels right after seeing it cannot hit a dead turn.
        let mut in_flight = self.in_flight();
        if in_flight.get(session) == Some(&turn) {
            in_flight.remove(session);
        }
        drop(in_flight);

        send(ServerMessage::TurnFinished {
            session: session.to_owned(),
            turn,
            reason,
        });

        let (repo, branch) = self.repo_status(session);
        send(ServerMessage::RepoStatus {
            session: session.to_owned(),
            repo,
            branch,
        });
    }

    fn in_flight(&self) -> std::sync::MutexGuard<'_, HashMap<String, u64>> {
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
        let (id, answer) = self.permissions.open(self.session, self.turn);

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
        let (id, answer) = self.questions.open(self.session, self.turn);

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
/// permissions and questions apart on the wire ("p-3", "q-4"), and each entry
/// remembers which turn owns it: cancelling one turn must leave the cards of
/// every other session alone.
struct Broker<T> {
    prefix: &'static str,
    next: AtomicU64,
    pending: Mutex<HashMap<String, Pending<T>>>,
}

struct Pending<T> {
    session: String,
    turn: u64,
    sender: Sender<T>,
}

impl<T> Broker<T> {
    fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
        }
    }

    fn open(&self, session: &str, turn: u64) -> (String, Receiver<T>) {
        let id = format!(
            "{}-{}",
            self.prefix,
            self.next.fetch_add(1, Ordering::Relaxed) + 1
        );
        let (send, receive) = channel();
        self.pending().insert(
            id.clone(),
            Pending {
                session: session.to_owned(),
                turn,
                sender: send,
            },
        );
        (id, receive)
    }

    fn answer(&self, id: &str, answer: T) -> bool {
        let sender = self.pending().remove(id);
        sender.is_some_and(|pending| pending.sender.send(answer).is_ok())
    }

    fn close(&self, id: &str) {
        self.pending().remove(id);
    }

    /// Drops the requests owned by one turn. Dropping the senders is the
    /// cancellation: the waiting askers wake with a closed channel and answer
    /// the provider with cancelled.
    fn cancel_turn(&self, session: &str, turn: u64) {
        self.pending()
            .retain(|_, pending| pending.session != session || pending.turn != turn);
    }

    fn cancel_all(&self) {
        self.pending().clear();
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, HashMap<String, Pending<T>>> {
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

    /// A driver that records the folder each turn was given.
    struct SeesCwd {
        seen: Arc<Mutex<Vec<Option<String>>>>,
    }

    impl Driver for SeesCwd {
        fn name(&self) -> &'static str {
            "sees-cwd"
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
                .push(session.cwd.map(str::to_owned));
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

    /// A driver whose turn blocks on a permission card and reports an abort
    /// when that card is cancelled, the way a real provider does.
    struct Stops;

    impl Driver for Stops {
        fn name(&self) -> &'static str {
            "stops"
        }

        fn start_turn(
            &self,
            _: SessionRef<'_>,
            _: &str,
            ask: &mut dyn Asker,
            emit: &mut dyn FnMut(Part),
        ) -> Result<Option<String>, DriverError> {
            let decision = ask.permission(PermissionRequest {
                title: "Keep going?".into(),
                detail: String::new(),
                options: vec![circulo_protocol::PermissionOption {
                    id: "allow".into(),
                    label: "Allow".into(),
                    kind: circulo_protocol::PermissionOptionKind::AllowOnce,
                }],
            });

            match decision {
                Decision::Selected(_) => {
                    emit(Part::Text {
                        delta: "finished".into(),
                    });
                    Ok(None)
                }
                Decision::Cancelled => Err(DriverError::cancelled("the user stopped the turn")),
            }
        }

        fn cancel(&self, _: &str) -> Result<(), DriverError> {
            Ok(())
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
        let session = core.start_session(Some("/tmp/project".into()));
        core.run_turn(&session, "one", |_| {});

        let restarted = Core::with_store(Arc::new(Echo), Some(store));
        assert!(restarted.reopen_session(&session).expect("reopen"));
        assert!(!restarted.reopen_session("missing").expect("reopen"));
        assert_eq!(restarted.transcript(&session).expect("transcript").len(), 1);

        let open = restarted.open_sessions();
        assert_eq!(
            open[0].project.as_deref(),
            Some("/tmp/project"),
            "a reopened session keeps the project it was created in"
        );
    }

    #[test]
    fn a_repo_project_gets_checkpoints_and_a_branch() {
        let Some(directory) = git_repo() else {
            return;
        };
        let core = core_with(Arc::new(Echo));
        let session = core.start_session(Some(directory.to_string_lossy().into_owned()));

        core.run_turn(&session, "go", |_| {});

        let id = Uuid::parse_str(&session).expect("the core mints a UUID");
        assert!(
            circulo_git::has_checkpoint(&directory, id, 1, CheckpointKind::Start),
            "the state the turn started from is captured"
        );
        assert!(
            circulo_git::has_checkpoint(&directory, id, 1, CheckpointKind::End),
            "the state the turn left is captured"
        );
        assert_eq!(
            core.repo_status(&session),
            (true, Some("test".to_owned())),
            "the window can show the real branch"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_plain_folder_has_no_checkpoints_and_no_branch() {
        let directory = std::env::temp_dir().join(format!("circulo-core-plain-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("temp");
        let core = core_with(Arc::new(Echo));
        let session = core.start_session(Some(directory.to_string_lossy().into_owned()));

        core.run_turn(&session, "go", |_| {});

        let id = Uuid::parse_str(&session).expect("the core mints a UUID");
        assert!(!circulo_git::has_checkpoint(
            &directory,
            id,
            1,
            CheckpointKind::Start
        ));
        assert_eq!(core.repo_status(&session), (false, None));

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A real repository with one commit on a `test` branch, or `None` when
    /// this machine has no `git`.
    fn git_repo() -> Option<std::path::PathBuf> {
        if !std::process::Command::new("git")
            .arg("--version")
            .output()
            .ok()?
            .status
            .success()
        {
            return None;
        }

        let directory = std::env::temp_dir().join(format!("circulo-core-git-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).ok()?;
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .current_dir(&directory)
                .args(args)
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        };
        if !run(&["init", "-q"]) {
            return None;
        }
        run(&["config", "user.name", "Test"]);
        run(&["config", "user.email", "test@example.com"]);
        std::fs::write(directory.join("a.txt"), "one").ok()?;
        run(&["add", "a.txt"]);
        run(&["commit", "-qm", "init"]);
        run(&["checkout", "-q", "-b", "test"]);
        Some(directory)
    }

    #[test]
    fn the_session_project_is_the_drivers_cwd() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let core = core_with(Arc::new(SeesCwd {
            seen: Arc::clone(&seen),
        }));

        let project = core.start_session(Some("/tmp/project".into()));
        core.run_turn(&project, "go", |_| {});
        let projectless = core.start_session(None);
        core.run_turn(&projectless, "go", |_| {});

        let seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            seen.as_slice(),
            &[Some("/tmp/project".to_owned()), None],
            "the provider sees the project folder, and no folder when there is none"
        );
    }

    #[test]
    fn projects_are_added_with_a_real_folder_and_removed() {
        let directory =
            std::env::temp_dir().join(format!("circulo-core-project-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("temp project");
        let canonical = std::fs::canonicalize(&directory).expect("canonical");

        let store = Arc::new(circulo_persistence::Store::in_memory().expect("store"));
        let core = Core::with_store(Arc::new(Echo), Some(store));

        let (added, projects) = core
            .add_project(directory.to_str().expect("utf-8"))
            .expect("add");
        assert_eq!(added, canonical.to_string_lossy());
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, canonical.to_string_lossy());
        assert_eq!(
            projects[0].name,
            directory.file_name().expect("name").to_string_lossy()
        );

        // A path that is not a folder, and a missing one, are refused.
        let file = directory.join("file.txt");
        std::fs::write(&file, "x").expect("file");
        assert!(core.add_project(file.to_str().expect("utf-8")).is_err());
        assert!(core.add_project("/definitely/not/here").is_err());

        core.remove_project(&projects[0].path).expect("remove");
        assert!(core.projects().expect("list").is_empty());
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn a_memory_only_core_refuses_to_store_a_project() {
        let core = Core::new(Arc::new(Echo));
        assert!(core.add_project("/tmp").is_err());
        assert!(core.projects().expect("list").is_empty());
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
        let finished = messages
            .iter()
            .rev()
            .find(|message| matches!(message, ServerMessage::TurnFinished { .. }))
            .expect("the turn finished");
        assert!(matches!(
            finished,
            ServerMessage::TurnFinished {
                turn: 1,
                reason: FinishReason::Completed,
                ..
            }
        ));
        assert!(
            matches!(messages.last(), Some(ServerMessage::RepoStatus { .. })),
            "the branch status follows the turn"
        );
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
        let messages = run_in_background(Arc::clone(&core), session.clone());

        let mut text = None;
        while let Ok(message) = messages.recv() {
            match message {
                // The client is gone the moment the card would have appeared.
                ServerMessage::PermissionRequested { .. } => {
                    core.cancel_turn_requests(&session);
                }
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
    fn cancelling_one_session_leaves_another_turn_alone() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let first = core.start_session(None);
        let second = core.start_session(None);
        let first_messages = run_in_background(Arc::clone(&core), first.clone());
        let second_messages = run_in_background(Arc::clone(&core), second.clone());

        // Both turns are blocked on their own card.
        next_card(&first_messages);
        let second_id = next_card(&second_messages);

        assert!(
            core.cancel_turn_requests(&first),
            "the first turn was running"
        );
        assert_eq!(outcome(&first_messages), "cancelled");

        // The other session never noticed: its card is still open and answerable.
        assert!(core.answer_permission(&second_id, Some("allow-once".into())));
        assert_eq!(outcome(&second_messages), "chose allow-once");
        assert!(!core.cancel_turn_requests("missing"));
    }

    /// Enqueues the first prompt and starts the session worker, the way the
    /// daemon does. The returned channel carries everything the turn emits.
    fn start_worker(
        core: &Arc<Core>,
        session: &str,
        first: &str,
    ) -> std::sync::mpsc::Receiver<ServerMessage> {
        let state = core.enqueue(session, first).expect("enqueue");
        assert!(state.start_worker, "nothing owned the session yet");

        let (send, receive) = std::sync::mpsc::channel();
        let core = Arc::clone(core);
        let session = session.to_owned();
        std::thread::spawn(move || {
            core.drain(&session, move |message| {
                let _ = send.send(message);
            });
        });
        receive
    }

    #[test]
    fn prompts_queue_in_order_behind_the_running_turn() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let session = core.start_session(None);
        let messages = start_worker(&core, &session, "one");

        let mut started = Vec::new();
        let mut cards = 0;
        let mut finished = 0;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::TurnStarted { turn, prompt, .. } => started.push((turn, prompt)),
                ServerMessage::PermissionRequested { id, .. } => {
                    cards += 1;
                    if cards == 1 {
                        // The running turn owns the session; the next prompt waits.
                        let state = core.enqueue(&session, "two").expect("queue");
                        assert!(!state.start_worker, "the worker already owns it");
                        assert_eq!(
                            state
                                .items
                                .iter()
                                .map(|item| item.text.as_str())
                                .collect::<Vec<_>>(),
                            vec!["two"]
                        );
                    }
                    assert!(core.answer_permission(&id, Some("allow-once".into())));
                }
                ServerMessage::TurnFinished { .. } => finished += 1,
                _ => {}
            }
        }

        assert_eq!(finished, 2);
        assert_eq!(
            started,
            vec![(1, "one".to_owned()), (2, "two".to_owned())],
            "the second prompt starts only after the first turn ends"
        );
    }

    #[test]
    fn removing_a_queued_prompt_keeps_it_from_running() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let session = core.start_session(None);
        let messages = start_worker(&core, &session, "one");

        let mut started = Vec::new();
        let mut cards = 0;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::TurnStarted { turn, prompt, .. } => started.push((turn, prompt)),
                ServerMessage::PermissionRequested { id, .. } => {
                    cards += 1;
                    if cards == 1 {
                        let queued = core.enqueue(&session, "two").expect("queue");
                        let id = queued.items[0].id.clone();
                        assert_eq!(
                            core.remove_queued(&session, &id).expect("remove"),
                            Vec::new()
                        );
                        core.enqueue(&session, "three").expect("queue again");
                    }
                    assert!(core.answer_permission(&id, Some("allow-once".into())));
                }
                _ => {}
            }
        }

        assert_eq!(
            started,
            vec![(1, "one".to_owned()), (2, "three".to_owned())],
            "the removed prompt never runs"
        );
    }

    #[test]
    fn a_cleared_queue_does_not_run_after_the_current_turn() {
        let core = Arc::new(core_with(Arc::new(AsksOnce)));
        let session = core.start_session(None);
        let messages = start_worker(&core, &session, "one");

        let mut started = Vec::new();
        let mut first_card = true;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::TurnStarted { turn, prompt, .. } => started.push((turn, prompt)),
                ServerMessage::PermissionRequested { id, .. } => {
                    if first_card {
                        first_card = false;
                        core.enqueue(&session, "two").expect("queue");
                        core.clear_queue(&session);
                        assert!(core.queued(&session).is_empty());
                    }
                    assert!(core.answer_permission(&id, Some("allow-once".into())));
                }
                _ => {}
            }
        }

        assert_eq!(
            started,
            vec![(1, "one".to_owned())],
            "the client left; nothing waiting may start"
        );
    }

    #[test]
    fn steering_aborts_the_running_turn_and_starts_the_new_prompt_next() {
        let core = Arc::new(core_with(Arc::new(Stops)));
        let session = core.start_session(None);
        let messages = start_worker(&core, &session, "one");

        // The first turn is blocked on its card when the steer arrives.
        let mut started = Vec::new();
        loop {
            match messages.recv().expect("the turn is alive") {
                ServerMessage::TurnStarted { turn, prompt, .. } => started.push((turn, prompt)),
                ServerMessage::PermissionRequested { .. } => break,
                _ => {}
            }
        }

        let state = core.steer(&session, "two").expect("steer");
        assert!(!state.start_worker, "the worker already owns it");
        assert_eq!(state.items[0].text, "two", "the steer leads the queue");

        let mut finished = Vec::new();
        let mut cards = 0;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::TurnStarted { turn, prompt, .. } => started.push((turn, prompt)),
                ServerMessage::PermissionRequested { id, .. } => {
                    cards += 1;
                    assert!(core.answer_permission(&id, Some("allow-once".into())));
                }
                ServerMessage::TurnFinished { reason, .. } => finished.push(reason),
                _ => {}
            }
        }

        assert_eq!(
            started,
            vec![(1, "one".to_owned()), (2, "two".to_owned())],
            "the steer starts right after the aborted turn"
        );
        assert_eq!(
            finished,
            vec![FinishReason::Aborted, FinishReason::Completed],
            "the running turn aborts; the steer completes"
        );
        assert_eq!(cards, 1, "only the steer's turn asks");
    }

    #[test]
    fn aborting_a_turn_ends_it_as_aborted_without_an_error() {
        let core = Arc::new(core_with(Arc::new(Stops)));
        let session = core.start_session(None);
        let messages = run_in_background(Arc::clone(&core), session.clone());

        // The turn is blocked on its card; this is the state the stop button
        // sees. The card goes away and the driver reports the abort.
        next_card(&messages);
        core.cancel_turn(&session).expect("the turn was running");

        let mut reason = None;
        let mut error = None;
        while let Ok(message) = messages.recv() {
            match message {
                ServerMessage::TurnFinished {
                    reason: finished, ..
                } => {
                    reason = Some(finished);
                    break;
                }
                ServerMessage::Error { message } => error = Some(message),
                _ => {}
            }
        }

        assert_eq!(reason, Some(FinishReason::Aborted));
        assert_eq!(error, None, "an abort is not a failure");
        assert_eq!(
            core.cancel_turn(&session),
            Err(format!("no turn is running in {session}"))
        );
    }

    /// Waits for the turn's permission card and returns its id.
    fn next_card(messages: &std::sync::mpsc::Receiver<ServerMessage>) -> String {
        loop {
            if let ServerMessage::PermissionRequested { id, .. } =
                messages.recv().expect("the turn is alive")
            {
                return id;
            }
        }
    }

    /// Waits for the turn's answer text.
    fn outcome(messages: &std::sync::mpsc::Receiver<ServerMessage>) -> String {
        loop {
            match messages.recv().expect("the turn is alive") {
                ServerMessage::Part {
                    part: Part::Text { delta },
                    ..
                } => return delta,
                ServerMessage::TurnFinished { .. } => panic!("the turn ended without text"),
                _ => {}
            }
        }
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
        let finished = messages
            .iter()
            .rev()
            .find(|message| matches!(message, ServerMessage::TurnFinished { .. }))
            .expect("the turn finished");
        assert!(matches!(
            finished,
            ServerMessage::TurnFinished {
                reason: FinishReason::Failed,
                ..
            }
        ));
    }
}
