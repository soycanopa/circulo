use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use circulo_core::Uuid;
use circulo_protocol::ApiError;

const PERMISSION_TIMEOUT: Duration = Duration::from_secs(120);

struct PendingPermission {
    tx: mpsc::Sender<bool>,
}

#[derive(Clone)]
pub struct PermissionWaiter {
    pending: Arc<Mutex<HashMap<(Uuid, String), PendingPermission>>>,
    timeout: Duration,
}

impl PermissionWaiter {
    pub fn new() -> Self {
        Self::with_timeout(PERMISSION_TIMEOUT)
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            timeout,
        }
    }

    /// Blocks until the app replies or the timeout elapses. Timeout → deny.
    pub fn wait(&self, session_id: Uuid, permission_id: String) -> bool {
        let (tx, rx) = mpsc::channel();
        self.pending.lock().expect("permission waiter lock").insert(
            (session_id, permission_id.clone()),
            PendingPermission { tx },
        );
        let allow = matches!(rx.recv_timeout(self.timeout), Ok(true));
        self.pending
            .lock()
            .expect("permission waiter lock")
            .remove(&(session_id, permission_id));
        allow
    }

    pub fn reply(&self, session_id: Uuid, permission_id: &str, allow: bool) -> Result<(), ApiError> {
        let key = (session_id, permission_id.to_owned());
        let guard = self.pending.lock().map_err(|_| ApiError::internal())?;
        let Some(pending) = guard.get(&key) else {
            return Err(ApiError::not_found("No pending permission request."));
        };
        pending
            .tx
            .send(allow)
            .map_err(|_| ApiError::internal())?;
        Ok(())
    }
}

impl Default for PermissionWaiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_times_out_as_deny() {
        let waiter = PermissionWaiter::with_timeout(Duration::from_millis(30));
        let session_id = Uuid::new_v4();
        assert!(!waiter.wait(session_id, "perm_1".into()));
    }

    #[test]
    fn reply_unblocks_waiter() {
        let waiter = PermissionWaiter::with_timeout(Duration::from_secs(5));
        let session_id = Uuid::new_v4();
        let shared = waiter.clone();
        let handle = std::thread::spawn(move || shared.wait(session_id, "perm_2".into()));
        std::thread::sleep(Duration::from_millis(20));
        waiter.reply(session_id, "perm_2", true).unwrap();
        assert!(handle.join().unwrap());
    }
}
