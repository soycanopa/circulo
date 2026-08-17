//! Daemon-owned lifecycle for a local `opencode serve` process.
//!
//! Circulo never attaches to a server it did not start: it probes a dedicated
//! loopback port, reuses a healthy Circulo-side server, and spawns one when the
//! port is vacant and an `opencode` binary can be located.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use circulo_adapter::{AdapterError, ErrorReason};

pub const DEFAULT_OPENCODE_PORT: u16 = 7433;

const PROBE_PATH: &str = "/doc";
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    /// Explicit `opencode` binary override (`CIRCULO_OPENCODE_CMD`). When set it
    /// is used even if the file does not exist, so spawn failures surface.
    pub command: Option<PathBuf>,
    pub cwd: PathBuf,
    pub startup_timeout: Duration,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let port = std::env::var("CIRCULO_OPENCODE_PORT")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .unwrap_or(DEFAULT_OPENCODE_PORT);
        let command = std::env::var_os("CIRCULO_OPENCODE_CMD").map(PathBuf::from);
        let cwd = std::env::var_os("CIRCULO_OPENCODE_CWD")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            port,
            command,
            cwd,
            startup_timeout: Duration::from_secs(10),
        }
    }
}

enum Probe {
    /// A real OpenCode server answered on the port.
    OpenCode,
    /// Something is listening but it is not an OpenCode server.
    Occupied,
    /// Nothing is listening.
    Vacant,
}

pub struct ServerManager {
    config: ServerConfig,
    child: Mutex<Option<Child>>,
    #[allow(clippy::type_complexity)]
    binary_resolver: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>,
}

impl ServerManager {
    pub fn new(config: ServerConfig) -> Self {
        Self::with_binary_resolver(config, find_opencode_binary)
    }

    /// Test seam: controls how the `opencode` binary is located when no
    /// explicit command is configured.
    pub fn with_binary_resolver(
        config: ServerConfig,
        resolver: fn() -> Option<PathBuf>,
    ) -> Self {
        Self {
            config,
            child: Mutex::new(None),
            binary_resolver: Box::new(resolver),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Returns Ok once a healthy OpenCode server answers on the dedicated port,
    /// spawning one if needed. Holds the lock for the duration of a spawn so
    /// concurrent callers cannot race into double spawns.
    pub fn ensure_running(&self) -> Result<(), AdapterError> {
        let mut child = self.child.lock().map_err(|_| lock_poisoned())?;
        match probe(self.config.port) {
            Probe::OpenCode => return Ok(()),
            Probe::Occupied => {
                return Err(AdapterError::unavailable(
                    ErrorReason::PortOccupied,
                    format!(
                        "Port {} is used by something that is not an OpenCode server.",
                        self.config.port
                    ),
                ));
            }
            Probe::Vacant => {}
        }

        let binary = match &self.config.command {
            Some(path) => path.clone(),
            None => (self.binary_resolver)().ok_or_else(|| {
                AdapterError::unavailable(
                    ErrorReason::BinaryMissing,
                    "The opencode command was not found on this Mac.",
                )
            })?,
        };

        let mut spawned = spawn_server(&binary, &self.config).map_err(|err| {
            AdapterError::unavailable(
                ErrorReason::StartFailed,
                format!("OpenCode could not be started: {err}"),
            )
        })?;

        let deadline = std::time::Instant::now() + self.config.startup_timeout;
        loop {
            match probe(self.config.port) {
                Probe::OpenCode => {
                    *child = Some(spawned);
                    return Ok(());
                }
                Probe::Occupied => {
                    let _ = spawned.kill();
                    let _ = spawned.wait();
                    return Err(AdapterError::unavailable(
                        ErrorReason::PortOccupied,
                        format!(
                            "Port {} is used by something that is not an OpenCode server.",
                            self.config.port
                        ),
                    ));
                }
                Probe::Vacant => {}
            }
            if std::time::Instant::now() >= deadline {
                let _ = spawned.kill();
                let _ = spawned.wait();
                return Err(AdapterError::unavailable(
                    ErrorReason::StartFailed,
                    "OpenCode did not become ready in time.",
                ));
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for ServerManager {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut child) = child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn lock_poisoned() -> AdapterError {
    AdapterError::failed(
        ErrorReason::Internal,
        "The OpenCode server manager state is unavailable.",
    )
}

fn spawn_server(binary: &PathBuf, config: &ServerConfig) -> std::io::Result<Child> {
    Command::new(binary)
        .arg("serve")
        .arg("--port")
        .arg(config.port.to_string())
        .arg("--hostname")
        .arg("127.0.0.1")
        .current_dir(&config.cwd)
        // We own this loopback server; keep it passwordless even if the user's
        // shell exports credentials for their own interactive servers.
        .env_remove("OPENCODE_SERVER_PASSWORD")
        .env_remove("OPENCODE_SERVER_USERNAME")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

fn probe(port: u16) -> Probe {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&addr, PROBE_TIMEOUT) {
        Ok(stream) => stream,
        // Only a clean refusal proves the port is vacant; a timeout means the
        // port is wedged or busy, and we must not spawn over it.
        Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
            return Probe::Vacant
        }
        Err(_) => return Probe::Occupied,
    };
    let _ = stream.set_read_timeout(Some(PROBE_TIMEOUT));
    let request = format!("GET {PROBE_PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return Probe::Occupied;
    }
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    // Read until end of headers; enough to identify an OpenAPI document.
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                head.extend_from_slice(&buf[..n]);
                if head.len() > 64 * 1024 {
                    break;
                }
            }
            Err(_) => return Probe::Occupied,
        }
    }
    let text = String::from_utf8_lossy(&head);
    let looks_openapi = text.starts_with("HTTP/") && text.contains("\"openapi\"");
    if looks_openapi {
        Probe::OpenCode
    } else {
        Probe::Occupied
    }
}

fn find_opencode_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("opencode");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let known = [
        home.join(".opencode/bin/opencode"),
        PathBuf::from("/opt/homebrew/bin/opencode"),
        PathBuf::from("/usr/local/bin/opencode"),
    ];
    known.into_iter().find(|path| path.is_file())
}
