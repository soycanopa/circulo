//! The daemon: the core behind a local listener.
//!
//! The daemon owns the process the clients talk to. It binds loopback, prints
//! its address and token once so a supervisor can hand them over, then serves
//! connections: one thread per connection, the protocol on the wire, the core
//! behind it.
//!
//! Loopback only for now. Remote will add WebSocket + TLS in front of the same
//! protocol and the same core.

#![forbid(unsafe_code)]

use std::io::BufReader;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use circulo_core::Core;
use circulo_protocol::{
    ClientRequest, Hello, PROTOCOL_VERSION, ProtocolError, ServerMessage, read_line, write_message,
};
use circulo_provider_demo::DemoDriver;
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

    /// One line a supervisor can parse to learn where the daemon listens and
    /// which token to present.
    pub fn announcement(&self) -> String {
        format!(
            r#"{{"address":"{}","token":"{}"}}"#,
            self.local_addr(),
            self.token
        )
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

fn handle(stream: TcpStream, token: &str, core: &Core) -> Result<(), ProtocolError> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    // Handshake: protocol version first, then the token.
    let Some(line) = read_line(&mut reader)? else {
        return Ok(());
    };
    let hello = Hello::decode(&line)?;

    if hello.protocol != PROTOCOL_VERSION {
        write_message(
            &mut writer,
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
        write_message(
            &mut writer,
            &ServerMessage::Error {
                message: "bad token".into(),
            },
        )?;
        return Ok(());
    }

    write_message(
        &mut writer,
        &ServerMessage::Welcome {
            protocol: PROTOCOL_VERSION,
            daemon: core.driver_name().to_owned(),
        },
    )?;

    while let Some(line) = read_line(&mut reader)? {
        match ClientRequest::decode(&line)? {
            Some(ClientRequest::StartSession { project }) => {
                let session = core.start_session(project);
                write_message(&mut writer, &ServerMessage::SessionStarted { session })?;
            }
            Some(ClientRequest::Prompt { session, text }) => {
                core.run_turn(&session, &text, |message| {
                    // A client that walks away mid-turn ends the turn for this
                    // connection only; the write error is reported there.
                    let _ = write_message(&mut writer, &message);
                });
            }
            None => {
                write_message(
                    &mut writer,
                    &ServerMessage::Error {
                        message: "unknown request".into(),
                    },
                )?;
            }
        }
    }

    Ok(())
}
