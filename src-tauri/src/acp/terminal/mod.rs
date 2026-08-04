//! ACP `terminal/*` client handlers — subprocess host with captured output.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::v1::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, TerminalExitStatus, TerminalId,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse,
};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify};
use tracing::{info, warn};

const DEFAULT_OUTPUT_BYTE_LIMIT: u64 = 256 * 1024;

struct TerminalEntry {
    session_id: String,
    label: String,
    output: Arc<Mutex<String>>,
    truncated: Arc<Mutex<bool>>,
    exit_status: Arc<Mutex<Option<TerminalExitStatus>>>,
    done: Arc<Notify>,
    child: Arc<Mutex<Option<Child>>>,
    output_byte_limit: u64,
    released: Arc<Mutex<bool>>,
}

pub struct TerminalManager {
    project_root: PathBuf,
    app: AppHandle,
    generation: u64,
    terminals: HashMap<String, TerminalEntry>,
}

impl TerminalManager {
    pub fn new(project_root: PathBuf, app: AppHandle, generation: u64) -> Self {
        Self {
            project_root,
            app,
            generation,
            terminals: HashMap::new(),
        }
    }

    pub async fn create(
        &mut self,
        request: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse, String> {
        let session_id = request.session_id.to_string();
        let cwd = resolve_terminal_cwd(&self.project_root, request.cwd.as_deref())?;
        let limit = request
            .output_byte_limit
            .unwrap_or(DEFAULT_OUTPUT_BYTE_LIMIT);

        let mut command = Command::new(&request.command);
        command
            .args(&request.args)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        for env in &request.env {
            command.env(&env.name, &env.value);
        }

        let mut child = command
            .spawn()
            .map_err(|err| format!("Failed to spawn terminal command: {err}"))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let terminal_id = format!("term_{}", uuid::Uuid::new_v4());
        let label = format!(
            "{} {}",
            request.command,
            request.args.join(" ")
        )
        .trim()
        .to_string();

        let output = Arc::new(Mutex::new(String::new()));
        let truncated = Arc::new(Mutex::new(false));
        let exit_status = Arc::new(Mutex::new(None));
        let done = Arc::new(Notify::new());
        let child_handle = Arc::new(Mutex::new(Some(child)));
        let released = Arc::new(Mutex::new(false));

        let entry = TerminalEntry {
            session_id: session_id.clone(),
            label: label.clone(),
            output: output.clone(),
            truncated: truncated.clone(),
            exit_status: exit_status.clone(),
            done: done.clone(),
            child: child_handle.clone(),
            output_byte_limit: limit,
            released: released.clone(),
        };
        self.terminals.insert(terminal_id.clone(), entry);

        info!(
            terminal_id = %terminal_id,
            command = %request.command,
            cwd = %cwd.display(),
            "ACP terminal/create"
        );

        self.emit_snapshot(
            &terminal_id,
            &session_id,
            &label,
            "",
            false,
            None,
            true,
        );

        let app = self.app.clone();
        let gen = self.generation;
        let tid = terminal_id.clone();
        let sid = session_id.clone();
        let lbl = label.clone();

        if let Some(stdout) = stdout {
            spawn_output_pump(
                stdout,
                output.clone(),
                truncated.clone(),
                limit,
                app.clone(),
                tid.clone(),
                sid.clone(),
                lbl.clone(),
                gen,
            );
        }
        if let Some(stderr) = stderr {
            spawn_output_pump(
                stderr,
                output,
                truncated,
                limit,
                app.clone(),
                tid.clone(),
                sid.clone(),
                lbl,
                gen,
            );
        }

        let wait_app = self.app.clone();
        let wait_tid = terminal_id.clone();
        let wait_sid = session_id;
        let wait_gen = self.generation;
        let wait_output = self
            .terminals
            .get(&terminal_id)
            .map(|e| e.output.clone())
            .unwrap();
        let wait_truncated = self
            .terminals
            .get(&terminal_id)
            .map(|e| e.truncated.clone())
            .unwrap();
        let wait_exit = self
            .terminals
            .get(&terminal_id)
            .map(|e| e.exit_status.clone())
            .unwrap();
        let wait_done = self
            .terminals
            .get(&terminal_id)
            .map(|e| e.done.clone())
            .unwrap();
        let wait_child = child_handle;
        let wait_label = label;

        tokio::spawn(async move {
            let status = {
                let mut guard = wait_child.lock().await;
                if let Some(child) = guard.as_mut() {
                    child.wait().await.ok()
                } else {
                    None
                }
            };

            let exit = status.map(|s| {
                if let Some(code) = s.code() {
                    TerminalExitStatus::new().exit_code(code as u32)
                } else {
                    TerminalExitStatus::new().signal("SIGTERM".to_string())
                }
            });

            if let Some(ref exit_status) = exit {
                *wait_exit.lock().await = Some(exit_status.clone());
            }

            let out = wait_output.lock().await.clone();
            let tr = *wait_truncated.lock().await;
            emit_terminal_event(
                &wait_app,
                &wait_tid,
                &wait_sid,
                &wait_label,
                &out,
                tr,
                exit.clone(),
                false,
                wait_gen,
            );
            wait_done.notify_waiters();
        });

        Ok(CreateTerminalResponse::new(TerminalId::new(terminal_id)))
    }

    pub async fn output(
        &self,
        request: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse, String> {
        let terminal_id = request.terminal_id.to_string();
        let entry = self
            .terminals
            .get(&terminal_id)
            .ok_or_else(|| format!("Unknown terminal: {terminal_id}"))?;
        self.assert_session(&terminal_id, &request.session_id.to_string())?;

        let output = entry.output.lock().await.clone();
        let truncated = *entry.truncated.lock().await;
        let exit_status = entry.exit_status.lock().await.clone();

        Ok(TerminalOutputResponse::new(output, truncated).exit_status(exit_status))
    }

    pub async fn wait_for_exit(
        &self,
        request: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse, String> {
        let terminal_id = request.terminal_id.to_string();
        let entry = self
            .terminals
            .get(&terminal_id)
            .ok_or_else(|| format!("Unknown terminal: {terminal_id}"))?;
        self.assert_session(&terminal_id, &request.session_id.to_string())?;

        entry.done.notified().await;

        let exit = entry
            .exit_status
            .lock()
            .await
            .clone()
            .unwrap_or_default();
        Ok(WaitForTerminalExitResponse::new(exit))
    }

    pub async fn kill(&mut self, request: KillTerminalRequest) -> Result<KillTerminalResponse, String> {
        let terminal_id = request.terminal_id.to_string();
        self.assert_session(&terminal_id, &request.session_id.to_string())?;
        self.kill_child(&terminal_id).await;
        Ok(KillTerminalResponse::new())
    }

    pub async fn release(
        &mut self,
        request: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse, String> {
        let terminal_id = request.terminal_id.to_string();
        self.assert_session(&terminal_id, &request.session_id.to_string())?;
        self.kill_child(&terminal_id).await;
        if let Some(entry) = self.terminals.remove(&terminal_id) {
            *entry.released.lock().await = true;
        }
        Ok(ReleaseTerminalResponse::new())
    }

    pub async fn release_session(&mut self, session_id: &str) {
        let ids: Vec<String> = self
            .terminals
            .iter()
            .filter(|(_, e)| e.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.kill_child(&id).await;
            self.terminals.remove(&id);
        }
    }

    async fn kill_child(&mut self, terminal_id: &str) {
        if let Some(entry) = self.terminals.get(terminal_id) {
            let mut guard = entry.child.lock().await;
            if let Some(child) = guard.as_mut() {
                if let Err(err) = child.kill().await {
                    warn!(terminal_id, %err, "terminal kill failed");
                }
            }
        }
    }

    fn assert_session(&self, terminal_id: &str, session_id: &str) -> Result<(), String> {
        let entry = self
            .terminals
            .get(terminal_id)
            .ok_or_else(|| format!("Unknown terminal: {terminal_id}"))?;
        if entry.session_id != session_id {
            return Err("Terminal session mismatch".to_string());
        }
        Ok(())
    }

    fn emit_snapshot(
        &self,
        terminal_id: &str,
        session_id: &str,
        label: &str,
        output: &str,
        truncated: bool,
        exit_status: Option<TerminalExitStatus>,
        running: bool,
    ) {
        emit_terminal_event(
            &self.app,
            terminal_id,
            session_id,
            label,
            output,
            truncated,
            exit_status,
            running,
            self.generation,
        );
    }
}

fn resolve_terminal_cwd(project_root: &Path, cwd: Option<&Path>) -> Result<PathBuf, String> {
    let cwd = cwd
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_root.to_path_buf());
    let canonical_root = project_root
        .canonicalize()
        .map_err(|err| format!("Invalid project path: {err}"))?;
    let canonical_cwd = cwd
        .canonicalize()
        .map_err(|err| format!("Invalid terminal cwd: {err}"))?;
    if !canonical_cwd.starts_with(&canonical_root) {
        return Err("Terminal cwd escapes project root".to_string());
    }
    Ok(canonical_cwd)
}

fn append_with_limit(buffer: &mut String, truncated: &mut bool, chunk: &str, limit: u64) {
    if chunk.is_empty() {
        return;
    }
    buffer.push_str(chunk);
    let limit = limit as usize;
    while buffer.as_bytes().len() > limit {
        let mut cut = 0usize;
        for (idx, _) in buffer.char_indices() {
            if idx == 0 {
                continue;
            }
            if buffer.as_bytes()[cut..].len() <= limit {
                break;
            }
            cut = idx;
        }
        if cut == 0 {
            buffer.clear();
            break;
        }
        buffer.drain(..cut);
        *truncated = true;
    }
}

fn spawn_output_pump<R>(
    reader: R,
    output: Arc<Mutex<String>>,
    truncated: Arc<Mutex<bool>>,
    limit: u64,
    app: AppHandle,
    terminal_id: String,
    session_id: String,
    label: String,
    generation: u64,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader);
        let mut buf = vec![0u8; 4096];
        loop {
            match lines.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    let (snapshot, tr, running) = {
                        let mut out = output.lock().await;
                        let mut tr_flag = truncated.lock().await;
                        append_with_limit(&mut out, &mut tr_flag, &chunk, limit);
                        (out.clone(), *tr_flag, true)
                    };
                    emit_terminal_event(
                        &app,
                        &terminal_id,
                        &session_id,
                        &label,
                        &snapshot,
                        tr,
                        None,
                        running,
                        generation,
                    );
                }
                Err(_) => break,
            }
        }
    });
}

fn emit_terminal_event(
    app: &AppHandle,
    terminal_id: &str,
    session_id: &str,
    label: &str,
    output: &str,
    truncated: bool,
    exit_status: Option<TerminalExitStatus>,
    running: bool,
    generation: u64,
) {
    let payload = serde_json::json!({
        "terminalId": terminal_id,
        "sessionId": session_id,
        "label": label,
        "output": output,
        "truncated": truncated,
        "running": running,
        "exitStatus": exit_status,
        "connectionGeneration": generation,
    });
    let _ = app.emit("acp:terminal_output", payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_from_start_at_char_boundary() {
        let mut buf = "hello world".to_string();
        let mut truncated = false;
        append_with_limit(&mut buf, &mut truncated, "!!!!", 8);
        assert!(truncated);
        assert!(buf.as_bytes().len() <= 8);
    }

    #[test]
    fn rejects_cwd_outside_project() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let err = resolve_terminal_cwd(&project, Some(outside.as_path())).unwrap_err();
        assert!(err.contains("escapes"));
    }
}
