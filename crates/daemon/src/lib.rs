//! The daemon: the core behind a local WebSocket listener.
//!
//! The daemon owns the process the clients talk to. It binds loopback, prints
//! its address and token once so a supervisor can hand them over, then serves
//! connections: one thread per connection, the protocol on the wire, the core
//! behind it.
//!
//! Loopback only for now. A tailnet or a reverse proxy can front it for remote
//! without touching this code, and TLS terminates before the WebSocket upgrade.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use std::path::PathBuf;

use circulo_core::{Core, QueueState};
use circulo_persistence::Store;
use circulo_protocol::{ClientRequest, Hello, PROTOCOL_VERSION, ProtocolError, ServerMessage};
use circulo_provider::Driver;
use circulo_provider_codex::Codex;
use circulo_provider_demo::DemoDriver;
use circulo_provider_opencode::OpenCode;
use tungstenite::protocol::{Role, WebSocketConfig};
use tungstenite::{Message, WebSocket};
use uuid::Uuid;

pub struct Daemon {
    listener: TcpListener,
    token: String,
    core: Arc<Core>,
}

impl Daemon {
    /// A daemon on an ephemeral loopback port, with the provider named by
    /// `CIRCULO_PROVIDER` (`opencode` today) or the demo provider otherwise.
    pub fn bind() -> Result<Self, ProtocolError> {
        Self::with_core(Arc::new(Core::with_store(driver(), store())))
    }

    pub fn with_core(core: Arc<Core>) -> Result<Self, ProtocolError> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        Ok(Self {
            listener,
            token: Uuid::new_v4().to_string(),
            core,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("a bound listener has an address")
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// The URL a client connects to.
    pub fn url(&self) -> String {
        format!("ws://{}", self.local_addr())
    }

    /// One line a supervisor can parse to learn where the daemon listens and
    /// which token to present.
    pub fn announcement(&self) -> String {
        format!(r#"{{"url":"{}","token":"{}"}}"#, self.url(), self.token)
    }

    /// Accepts forever. Each connection gets its own thread so a long turn
    /// cannot block the next client. Tests use this: nothing stops it.
    pub fn serve(self) -> ! {
        let (_hold, stopped) = channel();
        self.serve_until(stopped);
        unreachable!("serve_until only returns when its shutdown fires")
    }

    /// Serves until `shutdown` fires — the supervisor's stdin reached EOF,
    /// meaning the app that owns us is gone. Then the core is released: open
    /// cards are cancelled and the provider's agents go home, which is what
    /// keeps no orphaned agent process behind. Returns so the process can
    /// exit cleanly, destructors included.
    pub fn serve_until(self, shutdown: Receiver<()>) {
        // A blocking accept cannot observe the shutdown, so the listener polls
        // and the wait doubles as the loop's rest between attempts.
        let _ = self.listener.set_nonblocking(true);
        loop {
            match self.listener.accept() {
                Ok((stream, peer)) => {
                    let token = self.token.clone();
                    let core = Arc::clone(&self.core);

                    // The accepted socket inherits the listener's nonblocking
                    // flag, and the WebSocket handshake blocks on purpose.
                    let _ = stream.set_nonblocking(false);

                    thread::spawn(move || {
                        if let Err(error) = handle(stream, peer, &token, core) {
                            eprintln!("circulo-daemon: {peer}: {error}");
                        }
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    match shutdown.recv_timeout(std::time::Duration::from_millis(50)) {
                        Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
                Err(error) => eprintln!("circulo-daemon: accept failed: {error}"),
            }
        }

        self.core.shutdown();
    }
}

/// The database, opened where `crates/persistence` decides. A daemon that cannot
/// open it still serves turns; it just has no history to offer.
fn store() -> Option<Arc<Store>> {
    match circulo_persistence::default_path().and_then(|path| Store::open(&path)) {
        Ok(store) => Some(Arc::new(store)),
        Err(error) => {
            eprintln!("circulo-daemon: no history database: {error}");
            None
        }
    }
}

/// Which provider turns prompts into parts. The project directory comes from
/// `CIRCULO_PROJECT`, or the daemon's own directory until projects are real.
fn driver() -> Arc<dyn Driver> {
    match std::env::var("CIRCULO_PROVIDER").as_deref() {
        Ok("opencode") => {
            let cwd = std::env::var("CIRCULO_PROJECT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_else(|_| PathBuf::from("."));
            // A development hook: point the driver at another ACP agent, such
            // as the scripted one that asks for permission.
            match std::env::var("CIRCULO_OPENCODE_BIN") {
                Ok(binary) => Arc::new(OpenCode::with_binary(cwd, binary)),
                Err(_) => Arc::new(OpenCode::new(cwd)),
            }
        }
        Ok("codex") => {
            let cwd = std::env::var("CIRCULO_PROJECT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_else(|_| PathBuf::from("."));
            // The same development hook as OpenCode: a scripted app server,
            // or a pinned Codex build.
            match std::env::var("CIRCULO_CODEX_BIN") {
                Ok(binary) => Arc::new(Codex::with_binary(cwd, binary)),
                Err(_) => Arc::new(Codex::new(cwd)),
            }
        }
        _ => Arc::new(DemoDriver::new()),
    }
}

fn handle(
    stream: TcpStream,
    _peer: SocketAddr,
    token: &str,
    core: Arc<Core>,
) -> Result<(), ConnectionError> {
    // Two instances over one TCP stream: this thread only reads, the writer
    // thread only writes. A turn can then ask a question and keep reading the
    // answer, which a single blocking loop could not do.
    let writer_stream = stream.try_clone()?;
    let mut socket = tungstenite::accept(stream)
        .map_err(|error| ConnectionError::Handshake(error.to_string()))?;
    let mut writer = WebSocket::from_raw_socket(
        writer_stream,
        Role::Server,
        Some(WebSocketConfig::default()),
    );

    let (outbound, inbox) = channel::<ServerMessage>();
    thread::spawn(move || {
        while let Ok(message) = inbox.recv() {
            let Ok(line) = message.encode() else {
                continue;
            };
            if writer.send(Message::text(line)).is_err() {
                break;
            }
        }
    });

    // Handshake: protocol version first, then the token.
    let Some(line) = next(&mut socket)? else {
        return Ok(());
    };
    let hello = Hello::decode(&line)?;

    if hello.protocol != PROTOCOL_VERSION {
        emit(
            &outbound,
            ServerMessage::Error {
                message: format!(
                    "protocol version {} does not match {}",
                    hello.protocol, PROTOCOL_VERSION
                ),
            },
        );
        return Ok(());
    }

    if hello.token != token {
        emit(
            &outbound,
            ServerMessage::Error {
                message: "bad token".into(),
            },
        );
        return Ok(());
    }

    emit(
        &outbound,
        ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            daemon: core.driver_name().to_owned(),
        },
    );

    // The sessions this connection has sent prompts to. When it goes away,
    // only their cards are cancelled; another client's turn keeps running.
    let mut prompted = HashSet::new();

    while let Some(line) = next(&mut socket)? {
        match ClientRequest::decode(&line)? {
            Some(ClientRequest::StartSession { project }) => {
                let session = core.start_session(project);
                emit(
                    &outbound,
                    ServerMessage::SessionStarted {
                        session: session.clone(),
                    },
                );
                emit(
                    &outbound,
                    ServerMessage::Queue {
                        session: session.clone(),
                        items: Vec::new(),
                    },
                );
                // Asking the agent takes a moment; the read loop stays free.
                push_repo_status(Arc::clone(&core), outbound.clone(), session.clone());
                push_options(Arc::clone(&core), outbound.clone(), session);
            }
            Some(ClientRequest::Prompt {
                session,
                text,
                attachments,
            }) => match core.enqueue_with(&session, &text, attachments) {
                Ok(state) => {
                    prompted.insert(session.clone());
                    start_queue(Arc::clone(&core), outbound.clone(), &session, state);
                }
                Err(message) => emit(&outbound, ServerMessage::Error { message }),
            },
            Some(ClientRequest::Steer {
                session,
                text,
                attachments,
            }) => {
                match core.steer_with(&session, &text, attachments) {
                    Ok(state) => {
                        // The steer prompt is in the queue and the running turn
                        // is being cancelled; the worker starts it next.
                        prompted.insert(session.clone());
                        start_queue(Arc::clone(&core), outbound.clone(), &session, state);
                    }
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                }
            }
            Some(ClientRequest::RemoveQueued { session, id }) => {
                match core.remove_queued(&session, &id) {
                    Ok(items) => emit(&outbound, ServerMessage::Queue { session, items }),
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                }
            }
            Some(ClientRequest::CancelTurn { session }) => {
                if let Err(message) = core.cancel_turn(&session) {
                    emit(&outbound, ServerMessage::Error { message });
                }
            }
            Some(ClientRequest::Rewind { session, turn, to }) => {
                // Restoring files can take a moment; the read loop stays free.
                let core = Arc::clone(&core);
                let outbound = outbound.clone();
                thread::spawn(move || match core.rewind(&session, turn, to) {
                    Ok(turns) => {
                        emit(
                            &outbound,
                            ServerMessage::Queue {
                                session: session.clone(),
                                items: Vec::new(),
                            },
                        );
                        emit(
                            &outbound,
                            ServerMessage::History {
                                session: session.clone(),
                                turns,
                            },
                        );
                        push_repo_status(Arc::clone(&core), outbound, session);
                    }
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                });
            }
            Some(ClientRequest::AnswerPermission { id, option }) => {
                if !core.answer_permission(&id, option) {
                    emit(
                        &outbound,
                        ServerMessage::Error {
                            message: format!("no permission request {id}"),
                        },
                    );
                }
            }
            Some(ClientRequest::AnswerQuestion {
                id,
                action,
                answers,
            }) => {
                if !core.answer_question(&id, action, answers) {
                    emit(
                        &outbound,
                        ServerMessage::Error {
                            message: format!("no question {id}"),
                        },
                    );
                }
            }
            Some(ClientRequest::ListSessions) => match core.sessions() {
                Ok(sessions) => emit(&outbound, ServerMessage::Sessions { sessions }),
                Err(error) => emit(
                    &outbound,
                    ServerMessage::Error {
                        message: error.to_string(),
                    },
                ),
            },
            Some(ClientRequest::ListProjects) => match core.projects() {
                Ok(projects) => emit(&outbound, ServerMessage::Projects { projects }),
                Err(error) => emit(
                    &outbound,
                    ServerMessage::Error {
                        message: error.to_string(),
                    },
                ),
            },
            Some(ClientRequest::AddProject { path }) => {
                // Canonicalizing can touch a network volume; the read loop
                // stays free while it happens.
                let core = Arc::clone(&core);
                let outbound = outbound.clone();
                thread::spawn(move || match core.add_project(&path) {
                    Ok((canonical, projects)) => {
                        // The picker's path and the stored path can differ
                        // (symlinks, `/tmp`): tell the client which one a
                        // session must carry before the list.
                        emit(&outbound, ServerMessage::ProjectAdded { path: canonical });
                        emit(&outbound, ServerMessage::Projects { projects });
                    }
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                });
            }
            Some(ClientRequest::RemoveProject { path }) => match core.remove_project(&path) {
                Ok(projects) => emit(&outbound, ServerMessage::Projects { projects }),
                Err(message) => emit(&outbound, ServerMessage::Error { message }),
            },
            Some(ClientRequest::OpenSession { session }) => {
                let reopened = core.reopen_session(&session);
                match (reopened, core.transcript(&session)) {
                    (Ok(true), Ok(turns)) => {
                        emit(
                            &outbound,
                            ServerMessage::History {
                                session: session.clone(),
                                turns,
                            },
                        );
                        emit(
                            &outbound,
                            ServerMessage::Queue {
                                session: session.clone(),
                                items: core.queued(&session),
                            },
                        );
                        push_options(Arc::clone(&core), outbound.clone(), session.clone());
                        push_repo_status(Arc::clone(&core), outbound.clone(), session);
                    }
                    (Ok(false), _) => emit(
                        &outbound,
                        ServerMessage::Error {
                            message: format!("unknown session {session}"),
                        },
                    ),
                    (Err(error), _) | (_, Err(error)) => emit(
                        &outbound,
                        ServerMessage::Error {
                            message: error.to_string(),
                        },
                    ),
                }
            }
            Some(ClientRequest::SetOption { session, id, value }) => {
                // The driver spawns an agent for this; the read loop stays
                // free, and the refreshed list lands as `Options`.
                let core = Arc::clone(&core);
                let outbound = outbound.clone();
                thread::spawn(move || match core.set_option(&session, &id, &value) {
                    Ok(options) => emit(&outbound, ServerMessage::Options { session, options }),
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                });
            }
            Some(ClientRequest::GetSettings) => match core.settings() {
                Ok(settings) => emit(&outbound, ServerMessage::Settings { settings }),
                Err(error) => emit(
                    &outbound,
                    ServerMessage::Error {
                        message: error.to_string(),
                    },
                ),
            },
            Some(ClientRequest::SetSetting { key, value }) => {
                // The store takes the write; the answer is the whole map, the
                // way the client redraws.
                let core = Arc::clone(&core);
                let outbound = outbound.clone();
                thread::spawn(move || match core.set_setting(&key, &value) {
                    Ok(settings) => emit(&outbound, ServerMessage::Settings { settings }),
                    Err(message) => emit(&outbound, ServerMessage::Error { message }),
                });
            }
            None => {
                emit(
                    &outbound,
                    ServerMessage::Error {
                        message: "unknown request".into(),
                    },
                );
            }
        }
    }

    // The connection is going away. The prompts this client queued will never
    // run for anyone, and a turn it started should not keep the agent working
    // for nobody. Other sessions keep theirs.
    for session in prompted {
        core.clear_queue(&session);
        let _ = core.cancel_turn(&session);
    }
    Ok(())
}

fn emit(outbound: &Sender<ServerMessage>, message: ServerMessage) {
    let _ = outbound.send(message);
}

/// Shows the refreshed queue and starts the session's worker when nobody owns
/// it. The worker drains the whole queue on its own thread, so the read loop
/// stays free to answer a permission card.
fn start_queue(core: Arc<Core>, outbound: Sender<ServerMessage>, session: &str, state: QueueState) {
    emit(
        &outbound,
        ServerMessage::Queue {
            session: session.to_owned(),
            items: state.items,
        },
    );
    if !state.start_worker {
        return;
    }

    let session = session.to_owned();
    thread::spawn(move || {
        core.drain(&session, |message| {
            // A client that walks away ends the turn; the writer thread
            // notices and stops. The close handler clears what was queued.
            let _ = outbound.send(message);
        });
    });
}

/// Asks the agent for its settings on its own thread and pushes what it
/// reports. The daemon owns the thread; the core owns the session check.
fn push_options(core: Arc<Core>, outbound: Sender<ServerMessage>, session: String) {
    thread::spawn(move || match core.load_options(&session) {
        Ok(options) => emit(&outbound, ServerMessage::Options { session, options }),
        Err(message) => emit(&outbound, ServerMessage::Error { message }),
    });
}

/// Reads the session's Git facts off the read loop: the folder may be on a
/// network volume, and the branch can change under a turn.
fn push_repo_status(core: Arc<Core>, outbound: Sender<ServerMessage>, session: String) {
    thread::spawn(move || {
        let (repo, branch) = core.repo_status(&session);
        emit(
            &outbound,
            ServerMessage::RepoStatus {
                session,
                repo,
                branch,
            },
        );
    });
}

fn next(socket: &mut WebSocket<TcpStream>) -> Result<Option<String>, ConnectionError> {
    loop {
        match socket.read() {
            Ok(Message::Text(text)) => return Ok(Some(text.to_string())),
            Ok(Message::Close(_)) => return Ok(None),
            // Ping and pong are handled by the WebSocket layer.
            Ok(_) => continue,
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ConnectionError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("websocket: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("handshake: {0}")]
    Handshake(String),
}
