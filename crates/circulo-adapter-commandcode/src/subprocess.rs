//! Spawn and drive a `cmd -p ... --output-format json` subprocess.
//!
//! The lifecycle:
//! 1. Build the command line: `cmd -p <query> --output-format json [flags]`
//! 2. Spawn the child with stdout piped.
//! 3. Read stdout line-by-line in a blocking thread; each line is JSON.
//! 4. The final `result` frame drives `SessionBound` + `Completed` /
//!    `Failed`. Any non-zero exit before a `result` frame is mapped to
//!    an `AdapterError` from the exit code.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use circulo_adapter::{AdapterError, AdapterEvent, ErrorReason, GenerateRequest};
use circulo_core::ComposerPermissionMode;

pub enum ProbeOutcome {
    Available,
    AuthRequired,
    Missing,
    Other(String),
}

pub fn probe(binary: &Path) -> ProbeOutcome {
    let output = match Command::new(binary).arg("--version").output() {
        Ok(o) => o,
        Err(_) => return ProbeOutcome::Missing,
    };
    if output.status.success() {
        ProbeOutcome::Available
    } else {
        match output.status.code() {
            Some(3) => ProbeOutcome::AuthRequired,
            _ => ProbeOutcome::Other(format!(
                "Command Code exited with status {}.",
                output.status
            )),
        }
    }
}

/// Wraps a running child so callers can kill it from another thread
/// (e.g. `abort_turn`). `kill_wait` is idempotent.
pub struct ChildHandle {
    child: Option<std::process::Child>,
    #[allow(dead_code)]
    stderr: Option<String>,
}

impl ChildHandle {
    /// Kill the child and reap it. Idempotent. Public so `lib.rs` can
    /// invoke it from `abort_turn` after taking the child from the slot.
    pub fn kill_wait(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        self.kill_wait();
    }
}

/// Bundles a spawned child with everything the driver needs: the live
/// process handle (kept around for abort), and the drain reader for stdout.
pub struct StartedTurn {
    /// The `agent_session_id` Circulo uses to look up the running child
    /// when aborting. Empty until we see a `result.sessionId` frame.
    pub session_id: String,
    /// Shared handle to the live child; lets the lib kill it on
    /// `abort_turn` even while the driver thread is reading stdout.
    pub handle: Option<Arc<Mutex<ChildHandle>>>,
    /// Blocking reader for stdout, line by line.
    pub stdout: BufReader<std::process::ChildStdout>,
    /// Buffer for stderr so we can include it in failure messages.
    pub stderr_buffer: Arc<Mutex<String>>,
}

impl StartedTurn {
    /// Build the argv for a turn. Exposed for tests.
    pub fn build_argv(request: &GenerateRequest) -> Vec<String> {
        let mut argv: Vec<String> = vec![
            "-p".to_string(),
            request.user_text.clone(),
            "--output-format".to_string(),
            "json".to_string(),
        ];
        if matches!(
            request.composer_permission_mode,
            Some(ComposerPermissionMode::FullAccess)
                | Some(ComposerPermissionMode::AutoAcceptEdits)
        ) {
            argv.push("--yolo".to_string());
        }
        argv
    }

    /// Spawn the subprocess. Returns a started turn with a live handle
    /// for `abort_turn`. Errors here come from binary missing or exec
    /// failing.
    pub fn start(binary: PathBuf, request: &GenerateRequest) -> Result<Self, AdapterError> {
        let mut cmd = Command::new(&binary);
        cmd.args(Self::build_argv(request));
        if let Some(cwd) = request.working_directory.as_deref() {
            if !cwd.as_os_str().is_empty() {
                cmd.current_dir(cwd);
            }
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|err| {
            AdapterError::failed(
                ErrorReason::BinaryMissing,
                format!("Failed to spawn `cmd`: {err}"),
            )
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AdapterError::failed(ErrorReason::StreamFailed, "Missing stdout pipe."))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AdapterError::failed(ErrorReason::StreamFailed, "Missing stderr pipe."))?;
        let stderr_buffer = Arc::new(Mutex::new(String::new()));
        // Drain stderr in a background thread so it doesn't block the child.
        {
            let stderr_buffer = Arc::clone(&stderr_buffer);
            thread::spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut buf = String::new();
                let _ = reader.read_to_string(&mut buf);
                if let Ok(mut guard) = stderr_buffer.lock() {
                    *guard = buf;
                }
            });
        }
        let handle = Arc::new(Mutex::new(ChildHandle {
            child: Some(child),
            stderr: None,
        }));
        Ok(StartedTurn {
            session_id: String::new(),
            handle: Some(handle),
            stdout: BufReader::new(stdout),
            stderr_buffer,
        })
    }

    /// Read stdout line-by-line, dispatch events via the mapping, and
    /// return the final outcome. Drains the child before returning.
    pub fn drive(mut self, emit: &mut dyn FnMut(AdapterEvent)) -> Result<(), AdapterError> {
        let mut session_id = String::new();
        let mut buf = String::new();
        loop {
            buf.clear();
            let read = match self.stdout.read_line(&mut buf) {
                Ok(n) => n,
                Err(err) => {
                    return Err(AdapterError::failed(
                        ErrorReason::StreamFailed,
                        format!("Failed to read from `cmd` stdout: {err}"),
                    ));
                }
            };
            if read == 0 {
                break;
            }
            let line = buf.trim();
            if line.is_empty() {
                continue;
            }
            match crate::mapping::map_ndjson_line(line) {
                crate::mapping::MappingOutcome::Emitted(event) => {
                    if let AdapterEvent::SessionBound { agent_session_id } = &event {
                        session_id = agent_session_id.clone();
                    }
                    emit(event);
                }
                crate::mapping::MappingOutcome::Ignored => {}
                crate::mapping::MappingOutcome::Failed(err) => {
                    self.take_child_handle();
                    return Err(err);
                }
            }
        }
        // After EOF on stdout, the child has exited (the OS closes the
        // pipe). Reap it and read its exit code.
        let exit_code = self.reap();
        if !session_id.is_empty() {
            self.session_id = session_id;
        }
        match exit_code {
            Some(0) | None => Ok(()),
            Some(code) => {
                let stderr = self
                    .stderr_buffer
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default();
                Err(crate::mapping::map_exit_code(code, &stderr))
            }
        }
    }

    fn take_child_handle(&mut self) -> Option<Arc<Mutex<ChildHandle>>> {
        self.handle.take()
    }

    fn reap(&mut self) -> Option<i32> {
        let arc = self.handle.take()?;
        let mut guard = arc.lock().ok()?;
        let mut child = guard.child.take()?;
        match child.wait() {
            Ok(status) => status.code(),
            Err(_) => None,
        }
    }
}
