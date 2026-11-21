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

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use circulo_core::Core;
use circulo_protocol::{ClientRequest, Hello, PROTOCOL_VERSION, ProtocolError, ServerMessage};
use circulo_provider_demo::DemoDriver;
use tungstenite::{Message, WebSocket};
use uuid::Uuid;

pub struct Daemon {
    listener: TcpListener,
    token: String,
    core: Arc<Core>,
}

impl Daemon {
    /// A daemon with the demo provider, on an ephemeral loopback port.
    pub fn bind() -> Result<Self, ProtocolError> {
        Self::with_core(Arc::new(Core::new(Arc::new(DemoDriver::new()))))
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
    /// cannot block the next client.
    pub fn serve(self) -> ! {
        loop {
            match self.listener.accept() {
                Ok((stream, _peer)) => {
                    let token = self.token.clone();
                    let core = Arc::clone(&self.core);

                    thread::spawn(move || {
                        if let Err(error) = handle(stream, &token, &core) {
                            eprintln!("circulo-daemon: connection closed: {error}");
                        }
                    });
                }
                Err(error) => eprintln!("circulo-daemon: accept failed: {error}"),
            }
        }
    }
}

fn handle(stream: TcpStream, token: &str, core: &Core) -> Result<(), ConnectionError> {
    let mut socket = tungstenite::accept(stream)
        .map_err(|error| ConnectionError::Handshake(error.to_string()))?;

    // Handshake: protocol version first, then the token.
    let Some(line) = next(&mut socket)? else {
        return Ok(());
    };
    let hello = Hello::decode(&line)?;

    if hello.protocol != PROTOCOL_VERSION {
        send(
            &mut socket,
            &ServerMessage::Error {
                message: format!(
                    "protocol version {} does not match {}",
                    hello.protocol, PROTOCOL_VERSION
                ),
            },
        )?;
        return Ok(());
    }

    if hello.token != token {
        send(
            &mut socket,
            &ServerMessage::Error {
                message: "bad token".into(),
            },
        )?;
        return Ok(());
    }

    send(
        &mut socket,
        &ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            daemon: core.driver_name().to_owned(),
        },
    )?;

    while let Some(line) = next(&mut socket)? {
        match ClientRequest::decode(&line)? {
            Some(ClientRequest::StartSession { project }) => {
                let session = core.start_session(project);
                send(&mut socket, &ServerMessage::SessionStarted { session })?;
            }
            Some(ClientRequest::Prompt { session, text }) => {
                core.run_turn(&session, &text, |message| {
                    // A client that walks away mid-turn ends the turn for this
                    // connection only; the write error is reported there.
                    let _ = send(&mut socket, &message);
                });
            }
            None => {
                send(
                    &mut socket,
                    &ServerMessage::Error {
                        message: "unknown request".into(),
                    },
                )?;
            }
        }
    }

    Ok(())
}

fn send(socket: &mut WebSocket<TcpStream>, message: &ServerMessage) -> Result<(), ConnectionError> {
    Ok(socket.send(Message::text(message.encode()?))?)
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
    #[error("handshake: {0}")]
    Handshake(String),
}
