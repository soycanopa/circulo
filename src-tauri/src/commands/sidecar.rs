use serde::Serialize;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

pub const SIDECAR_PORT: u16 = 21337;
pub const SIDECAR_URL: &str = "http://127.0.0.1:21337";

pub struct SidecarState {
    child: Mutex<Option<CommandChild>>,
    port: u16,
}

impl SidecarState {
    fn new() -> Self {
        Self {
            child: Mutex::new(None),
            port: SIDECAR_PORT,
        }
    }
}

#[derive(Serialize)]
pub struct SidecarStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

fn workspace_root(app: &AppHandle) -> PathBuf {
    if let Ok(resource) = app.path().resource_dir() {
        let candidate = resource.join("..").join("..").join("..");
        if candidate.join("package.json").exists() {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn sidecar_script(root: &PathBuf) -> PathBuf {
    root.join("sidecar").join("entry.ts")
}

/// Arranca el sidecar Node (tRPC + servicios que aún no están en Rust).
#[tauri::command]
pub async fn sidecar_start(app: AppHandle, state: State<'_, SidecarState>) -> Result<SidecarStatus, String> {
    {
        let guard = state.child.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Ok(sidecar_status(&state, None));
        }
    }

    let root = workspace_root(&app);
    let script = sidecar_script(&root);
    if !script.exists() {
        return Err(format!("No se encontró el sidecar en {}", script.display()));
    }

    let bun = which::which("bun").map_err(|_| "bun no está en PATH".to_string())?;

    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    let sidecar = app
        .shell()
        .command(bun.to_string_lossy().to_string())
        .arg(script.to_string_lossy().to_string())
        .env("CIRCULO_USER_DATA", &user_data)
        .env("CIRCULO_SIDECAR_PORT", state.port.to_string())
        .env("CIRCULO_WORKSPACE", root.to_string_lossy().as_ref())
        .env(
            "CIRCULO_MIGRATIONS_PATH",
            root.join("drizzle").to_string_lossy().to_string(),
        );

    let (mut rx, child) = sidecar.spawn().map_err(|e| e.to_string())?;

    {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => println!("[sidecar] {}", String::from_utf8_lossy(&line)),
                CommandEvent::Stderr(line) => eprintln!("[sidecar] {}", String::from_utf8_lossy(&line)),
                CommandEvent::Terminated(payload) => {
                    println!("[sidecar] terminado: {:?}", payload);
                    if let Some(state) = app_handle.try_state::<SidecarState>() {
                        if let Ok(mut guard) = state.child.lock() {
                            *guard = None;
                        }
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(sidecar_status(&state, None))
}

#[tauri::command]
pub async fn sidecar_stop(state: State<'_, SidecarState>) -> Result<SidecarStatus, String> {
    let child = {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(child) = child {
        child.kill().map_err(|e| e.to_string())?;
    }
    Ok(sidecar_status(&state, None))
}

#[tauri::command]
pub fn sidecar_get_status(state: State<'_, SidecarState>) -> SidecarStatus {
    sidecar_status(&state, None)
}

fn sidecar_status(state: &SidecarState, pid: Option<u32>) -> SidecarStatus {
    let running = state
        .child
        .lock()
        .ok()
        .map(|g| g.is_some())
        .unwrap_or(false);
    SidecarStatus {
        running,
        port: state.port,
        url: format!("http://127.0.0.1:{}", state.port),
        pid,
        error: None,
    }
}

pub fn init_state() -> SidecarState {
    SidecarState::new()
}