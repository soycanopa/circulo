//! The protocol client.
//!
//! This crate exists on its own so a client does not have to be the desktop: a
//! future mobile app or remote tunnel links the same crate, speaks the same
//! handshake and receives the same messages. Nothing here knows about GPUI, the
//! core or a provider.
//!
//! The connection is a WebSocket. `ws://` on loopback today; `wss://` works as
//! soon as the daemon is reachable over TLS, which remote will provide.
//!
//! The connection is blocking. The desktop will drive it from a GPUI background
//! task; tests drive it directly.

#![forbid(unsafe_code)]

use std::net::TcpStream;

use circulo_protocol::{ClientRequest, Hello, PROTOCOL_VERSION, ProtocolError, ServerMessage};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

pub struct Client {
    socket: Socket,
}

impl Client {
    /// Connects and performs the versioned handshake. The daemon must welcome
    /// us; anything else is a refusal with its reason.
    pub fn connect(url: &str, token: &str) -> Result<Self, ClientError> {
        let (mut socket, _response) = tungstenite::connect(url)?;

        send(
            &mut socket,
            &Hello {
                protocol: PROTOCOL_VERSION,
                token: token.to_owned(),
            }
            .encode()?,
        )?;

        match next(&mut socket)? {
            Some(ServerMessage::Welcome { .. }) => Ok(Self { socket }),
            Some(ServerMessage::Error { message }) => Err(ClientError::Refused(message)),
            other => Err(ClientError::Unexpected(format!("{other:?}"))),
        }
    }

    /// Opens a session and returns its id.
    pub fn start_session(&mut self, project: Option<String>) -> Result<String, ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::StartSession { project }.encode()?,
        )?;

        loop {
            match self.next_message()? {
                Some(ServerMessage::SessionStarted { session }) => return Ok(session),
                Some(ServerMessage::Error { message }) => {
                    return Err(ClientError::Refused(message));
                }
                Some(_) => continue,
                None => return Err(ClientError::Closed),
            }
        }
    }

    /// Sends a prompt. The turn arrives through [`Client::next_message`].
    pub fn prompt(&mut self, session: &str, text: &str) -> Result<(), ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::Prompt {
                session: session.to_owned(),
                text: text.to_owned(),
            }
            .encode()?,
        )
    }

    /// Reads the next message. `None` means the daemon closed the connection.
    pub fn next_message(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        next(&mut self.socket)
    }
}

fn send(socket: &mut Socket, line: &str) -> Result<(), ClientError> {
    Ok(socket.send(Message::text(line))?)
}

fn next(socket: &mut Socket) -> Result<Option<ServerMessage>, ClientError> {
    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                // A newer daemon may send messages this client does not know.
                // Skip them instead of tearing the connection down.
                if let Some(message) = ServerMessage::decode(text.as_str())? {
                    return Ok(Some(message));
                }
            }
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
pub enum ClientError {
    #[error("transport: {0}")]
    Transport(#[from] tungstenite::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("the daemon refused the connection: {0}")]
    Refused(String),
    #[error("the daemon closed the connection")]
    Closed,
    #[error("unexpected message: {0}")]
    Unexpected(String),
}
