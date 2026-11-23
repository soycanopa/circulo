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

use std::collections::BTreeMap;

use circulo_protocol::{
    ClientRequest, Hello, HistoryTurn, PROTOCOL_VERSION, ProtocolError, QuestionAction,
    ServerMessage, SessionSummary,
};
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

    /// Lists the stored sessions, newest first.
    pub fn list_sessions(&mut self) -> Result<Vec<SessionSummary>, ClientError> {
        send(&mut self.socket, &ClientRequest::ListSessions.encode()?)?;

        loop {
            match self.next_message()? {
                Some(ServerMessage::Sessions { sessions }) => return Ok(sessions),
                Some(ServerMessage::Error { message }) => {
                    return Err(ClientError::Refused(message));
                }
                Some(_) => continue,
                None => return Err(ClientError::Closed),
            }
        }
    }

    /// Opens a stored session and returns its transcript.
    pub fn open_session(&mut self, session: &str) -> Result<Vec<HistoryTurn>, ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::OpenSession {
                session: session.to_owned(),
            }
            .encode()?,
        )?;

        loop {
            match self.next_message()? {
                Some(ServerMessage::History { turns, .. }) => return Ok(turns),
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

    /// Applies one of the agent's settings. The daemon answers with the
    /// refreshed `Options`, or an `Error` for an unknown session or a choice
    /// the agent refuses.
    pub fn set_option(&mut self, session: &str, id: &str, value: &str) -> Result<(), ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::SetOption {
                session: session.to_owned(),
                id: id.to_owned(),
                value: value.to_owned(),
            }
            .encode()?,
        )
    }

    /// Answers a permission request. `option` is one of the ids the daemon
    /// offered; `None` cancels, which is only right when the turn is going
    /// away.
    pub fn answer_permission(&mut self, id: &str, option: Option<&str>) -> Result<(), ClientError> {
        send(&mut self.socket, &answer(id, option).encode()?)
    }

    /// Asks for the client's own settings. The daemon answers with
    /// `Settings` carrying the whole map.
    pub fn get_settings(&mut self) -> Result<(), ClientError> {
        send(&mut self.socket, &ClientRequest::GetSettings.encode()?)
    }

    /// Stores one client setting. The daemon answers with `Settings`.
    pub fn set_setting(&mut self, key: &str, value: serde_json::Value) -> Result<(), ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::SetSetting {
                key: key.to_owned(),
                value,
            }
            .encode()?,
        )
    }

    /// Splits the connection so a reader can block while a writer sends: a
    /// turn asks a question and the answer has to reach the daemon without
    /// waiting for the turn to end. One instance only reads, the other only
    /// writes, over the same TCP stream.
    pub fn split(self) -> Result<(Self, ClientWriter), ClientError> {
        // Loopback is what ships, and a plain stream is the one that can be
        // cloned. A TLS connection is split when remote lands, with the client
        // rewritten around it.
        let stream = match self.socket.get_ref() {
            MaybeTlsStream::Plain(stream) => MaybeTlsStream::Plain(stream.try_clone()?),
            _ => {
                return Err(ClientError::Unexpected(
                    "only a plain connection can be split".into(),
                ));
            }
        };
        let writer = WebSocket::from_raw_socket(
            stream,
            tungstenite::protocol::Role::Client,
            Some(tungstenite::protocol::WebSocketConfig::default()),
        );
        Ok((self, ClientWriter { socket: writer }))
    }

    /// Reads the next message. `None` means the daemon closed the connection.
    pub fn next_message(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        next(&mut self.socket)
    }
}

/// The other half of a split connection: sends requests, never reads.
pub struct ClientWriter {
    socket: Socket,
}

impl ClientWriter {
    /// Sends any request over the writer half of a split. The UI needs prompts,
    /// card answers and the session requests, and one entry point keeps a new
    /// request from being silently dropped.
    pub fn send_request(&mut self, request: &ClientRequest) -> Result<(), ClientError> {
        send(&mut self.socket, &request.encode()?)
    }

    /// Sends a prompt over the writer half of a split, so the composer can send
    /// while the reader is blocked on the previous turn.
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

    /// Answers a permission request over the writer half of a split.
    pub fn answer_permission(&mut self, id: &str, option: Option<&str>) -> Result<(), ClientError> {
        send(&mut self.socket, &answer(id, option).encode()?)
    }

    /// Answers a question over the writer half of a split.
    pub fn answer_question(
        &mut self,
        id: &str,
        action: QuestionAction,
        answers: BTreeMap<String, String>,
    ) -> Result<(), ClientError> {
        send(
            &mut self.socket,
            &ClientRequest::AnswerQuestion {
                id: id.to_owned(),
                action,
                answers,
            }
            .encode()?,
        )
    }
}

fn answer(id: &str, option: Option<&str>) -> ClientRequest {
    ClientRequest::AnswerPermission {
        id: id.to_owned(),
        option: option.map(str::to_owned),
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
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("the daemon refused the connection: {0}")]
    Refused(String),
    #[error("the daemon closed the connection")]
    Closed,
    #[error("unexpected message: {0}")]
    Unexpected(String),
}
