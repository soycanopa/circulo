use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use circulo_adapter::{QuestionAnswer, QuestionResponse};
use circulo_core::Uuid;
use circulo_protocol::ApiError;

const QUESTION_TIMEOUT: Duration = Duration::from_secs(300);

struct PendingQuestion {
    tx: mpsc::Sender<QuestionResponse>,
}

#[derive(Clone)]
pub struct QuestionWaiter {
    pending: Arc<Mutex<HashMap<(Uuid, String), PendingQuestion>>>,
    timeout: Duration,
}

impl QuestionWaiter {
    pub fn new() -> Self {
        Self::with_timeout(QUESTION_TIMEOUT)
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            timeout,
        }
    }

    /// Blocks until the app replies or the timeout elapses. Timeout → empty answers.
    pub fn wait(&self, session_id: Uuid, request_id: String) -> QuestionResponse {
        let (tx, rx) = mpsc::channel();
        self.pending.lock().expect("question waiter lock").insert(
            (session_id, request_id.clone()),
            PendingQuestion { tx },
        );
        let response = rx.recv_timeout(self.timeout).unwrap_or(QuestionResponse {
            answers: Vec::new(),
        });
        self.pending
            .lock()
            .expect("question waiter lock")
            .remove(&(session_id, request_id));
        response
    }

    pub fn reply(
        &self,
        session_id: Uuid,
        request_id: &str,
        answers: Vec<QuestionAnswer>,
    ) -> Result<(), ApiError> {
        let key = (session_id, request_id.to_owned());
        let guard = self.pending.lock().map_err(|_| ApiError::internal())?;
        let Some(pending) = guard.get(&key) else {
            return Err(ApiError::not_found("No pending question request."));
        };
        pending
            .tx
            .send(QuestionResponse { answers })
            .map_err(|_| ApiError::internal())?;
        Ok(())
    }
}

impl Default for QuestionWaiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_times_out_with_empty_answers() {
        let waiter = QuestionWaiter::with_timeout(Duration::from_millis(30));
        let session_id = Uuid::new_v4();
        assert!(waiter.wait(session_id, "q1".into()).answers.is_empty());
    }

    #[test]
    fn reply_unblocks_waiter() {
        let waiter = QuestionWaiter::with_timeout(Duration::from_secs(5));
        let session_id = Uuid::new_v4();
        let shared = waiter.clone();
        let handle = std::thread::spawn(move || shared.wait(session_id, "q2".into()));
        std::thread::sleep(Duration::from_millis(20));
        waiter
            .reply(
                session_id,
                "q2",
                vec![QuestionAnswer {
                    question_id: "question-0".into(),
                    answers: vec!["Yes".into()],
                }],
            )
            .unwrap();
        assert_eq!(handle.join().unwrap().answers[0].answers, vec!["Yes"]);
    }
}
