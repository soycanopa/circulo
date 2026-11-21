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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use circulo_protocol::{FinishReason, ServerMessage};
use circulo_provider::Driver;
use uuid::Uuid;

pub struct Session {
    pub id: String,
    pub project: Option<String>,
}

pub struct Core {
    driver: Arc<dyn Driver>,
    sessions: Mutex<Vec<Session>>,
    turns: AtomicU64,
}

impl Core {
    pub fn new(driver: Arc<dyn Driver>) -> Self {
        Self {
            driver,
            sessions: Mutex::new(Vec::new()),
            turns: AtomicU64::new(0),
        }
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

        let part_session = session.to_owned();
        let result = self.driver.start_turn(prompt, &mut |part| {
            emit(ServerMessage::Part {
                session: part_session.clone(),
                turn,
                part,
            })
        });

        let reason = match result {
            Ok(()) => FinishReason::Completed,
            Err(error) => {
                emit(ServerMessage::Error {
                    message: error.to_string(),
                });
                FinishReason::Failed
            }
        };

        emit(ServerMessage::TurnFinished {
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

        fn start_turn(&self, prompt: &str, emit: &mut dyn FnMut(Part)) -> Result<(), DriverError> {
            emit(Part::Text {
                delta: prompt.to_owned(),
            });
            Ok(())
        }
    }

    struct Broken;

    impl Driver for Broken {
        fn name(&self) -> &'static str {
            "broken"
        }

        fn start_turn(&self, _: &str, _: &mut dyn FnMut(Part)) -> Result<(), DriverError> {
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
