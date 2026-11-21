//! The protocol client.
//!
//! This crate exists on its own so a client does not have to be the desktop: a
//! future mobile app or remote tunnel links the same crate, speaks the same
//! handshake and receives the same messages. Nothing here knows about GPUI, the
//! core or a provider.
//!
//! The connection is blocking. The desktop will drive it from a GPUI background
//! task; tests drive it directly.

#![forbid(unsafe_code)]

use std::io::BufReader;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use circulo_protocol::{
    ClientRequest, Hello, PROTOCOL_VERSION, ServerMessage, read_line, write_message,
};

pub struct Client {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Client {
    /// Connects, then performs the versioned handshake. The daemon must welcome
    /// us; anything else is a refusal with its reason.
    pub fn connect(address: SocketAddr, token: &str) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(address)?;
        stream.set_nodelay(true)?;

        let mut writer = stream.try_clone()?;
        let mut reader = BufReader::new(stream);

        write_message(
            &mut writer,
            &Hello {
                protocol: PROTOCOL_VERSION,
                token: token.to_owned(),
            },
        )?;

        match read(&mut reader)? {
            Some(ServerMessage::Welcome { .. }) => Ok(Self { writer, reader }),
            Some(ServerMessage::Error { message }) => Err(ClientError::Refused(message)),
            other => Err(ClientError::Unexpected(format!("{other:?}"))),
        }
    }

    /// Opens a session and returns its id.
    pub fn start_session(&mut self, project: Option<String>) -> Result<String, ClientError> {
        write_message(&mut self.writer, &ClientRequest::StartSession { project })?;

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
        write_message(
            &mut self.writer,
            &ClientRequest::Prompt {
                session: session.to_owned(),
                text: text.to_owned(),
            },
        )?;
        Ok(())
    }

    /// Reads the next message. `None` means the daemon closed the connection.
    pub fn next_message(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        read(&mut self.reader)
    }

    /// How long a read may block. Used by a client that polls instead of
    /// dedicating a thread.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), ClientError> {
        Ok(self.reader.get_ref().set_read_timeout(timeout)?)
    }
}

fn read(reader: &mut BufReader<TcpStream>) -> Result<Option<ServerMessage>, ClientError> {
    loop {
        let Some(line) = read_line(reader)? else {
            return Ok(None);
        };

        // A newer daemon may send messages this client does not know. Skip them
        // instead of tearing the connection down.
        if let Some(message) = ServerMessage::decode(&line)? {
            return Ok(Some(message));
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] circulo_protocol::ProtocolError),
    #[error("the daemon refused the connection: {0}")]
    Refused(String),
    #[error("the daemon closed the connection")]
    Closed,
    #[error("unexpected message: {0}")]
    Unexpected(String),
}
