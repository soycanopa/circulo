//! Interactive embedded shell (PTY) for the bottom terminal drawer.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputPayload {
    tab_id: String,
    data: String,
}

pub struct UserTerminalState {
    sessions: AsyncMutex<HashMap<String, ActiveSession>>,
}

impl Default for UserTerminalState {
    fn default() -> Self {
        Self {
            sessions: AsyncMutex::new(HashMap::new()),
        }
    }
}

struct ActiveSession {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    suppress_exit: Arc<AtomicBool>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| {
        #[cfg(target_os = "macos")]
        {
            "/bin/zsh".to_string()
        }
        #[cfg(not(target_os = "macos"))]
        {
            "/bin/sh".to_string()
        }
    })
}

fn resolve_cwd(project_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(project_path.trim());
    if !path.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    path.canonicalize()
        .map_err(|err| format!("Invalid project path: {err}"))
}

#[tauri::command]
pub async fn spawn_user_terminal(
    app: AppHandle,
    state: State<'_, UserTerminalState>,
    tab_id: String,
    project_path: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if tab_id.trim().is_empty() {
        return Err("tabId must not be empty".to_string());
    }

    let mut guard = state.sessions.lock().await;
    if let Some(session) = guard.get(&tab_id) {
        let master = session
            .master
            .lock()
            .map_err(|_| "PTY lock poisoned".to_string())?;
        master
            .resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(2),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("Failed to resize terminal: {err}"))?;
        return Ok(());
    }

    let cwd = resolve_cwd(&project_path)?;
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("Failed to open PTY: {err}"))?;

    let shell = default_shell();
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|err| format!("Failed to spawn shell: {err}"))?;
    drop(pair.slave);

    let master: Arc<Mutex<Box<dyn MasterPty + Send>>> =
        Arc::new(Mutex::new(pair.master));
    let writer = master
        .lock()
        .map_err(|_| "PTY lock poisoned".to_string())?
        .take_writer()
        .map_err(|err| format!("Failed to open PTY writer: {err}"))?;
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(writer));
    let suppress_exit = Arc::new(AtomicBool::new(false));

    let reader = master
        .lock()
        .map_err(|_| "PTY lock poisoned".to_string())?
        .try_clone_reader()
        .map_err(|err| format!("Failed to open PTY reader: {err}"))?;

    let app_for_reader = app.clone();
    let tab_for_reader = tab_id.clone();
    let suppress_for_reader = suppress_exit.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buffer[..n]).into_owned();
                    let _ = app_for_reader.emit(
                        "user_terminal_output",
                        TerminalOutputPayload {
                            tab_id: tab_for_reader.clone(),
                            data: chunk,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        if !suppress_for_reader.load(Ordering::Relaxed) {
            let _ = app_for_reader.emit(
                "user_terminal_exit",
                TerminalOutputPayload {
                    tab_id: tab_for_reader,
                    data: String::new(),
                },
            );
        }
    });

    info!(
        tab_id = %tab_id,
        shell = %shell,
        cwd = %cwd.display(),
        "Embedded user terminal spawned"
    );

    guard.insert(
        tab_id,
        ActiveSession {
            master,
            writer,
            suppress_exit,
            _child: child,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn write_user_terminal(
    state: State<'_, UserTerminalState>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    let guard = state.sessions.lock().await;
    let session = guard
        .get(&tab_id)
        .ok_or_else(|| format!("Terminal tab not running: {tab_id}"))?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "PTY writer lock poisoned".to_string())?;
    writer
        .write_all(data.as_bytes())
        .map_err(|err| format!("Failed to write to terminal: {err}"))?;
    writer
        .flush()
        .map_err(|err| format!("Failed to flush terminal: {err}"))?;
    Ok(())
}

#[tauri::command]
pub async fn resize_user_terminal(
    state: State<'_, UserTerminalState>,
    tab_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let guard = state.sessions.lock().await;
    let session = guard
        .get(&tab_id)
        .ok_or_else(|| format!("Terminal tab not running: {tab_id}"))?;
    let master = session
        .master
        .lock()
        .map_err(|_| "PTY lock poisoned".to_string())?;
    master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("Failed to resize terminal: {err}"))?;
    Ok(())
}

#[tauri::command]
pub async fn close_user_terminal(
    state: State<'_, UserTerminalState>,
    tab_id: String,
) -> Result<(), String> {
    let mut guard = state.sessions.lock().await;
    if let Some(session) = guard.remove(&tab_id) {
        session.suppress_exit.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub async fn close_all_user_terminals(state: State<'_, UserTerminalState>) -> Result<(), String> {
    let mut guard = state.sessions.lock().await;
    for (_, session) in guard.drain() {
        session.suppress_exit.store(true, Ordering::Relaxed);
    }
    Ok(())
}
