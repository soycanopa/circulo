//! End to end: a real daemon, a real client, the real protocol.

use std::sync::Arc;
use std::thread;

use circulo_client::{Client, ClientError};
use circulo_core::Core;
use circulo_daemon::Daemon;
use circulo_protocol::{ConfigOption, FinishReason, Part, ServerMessage, ToolState};
use circulo_provider_demo::DemoDriver;

fn daemon() -> (Daemon, String, String) {
    let core = Arc::new(Core::new(Arc::new(DemoDriver::instant())));
    let daemon = Daemon::with_core(core).expect("bind the daemon");
    let url = daemon.url();
    let token = daemon.token().to_owned();
    (daemon, url, token)
}

#[test]
fn a_client_streams_a_turn_from_the_daemon() {
    let (daemon, url, token) = daemon();
    thread::spawn(move || daemon.serve());

    let mut client = Client::connect(&url, &token).expect("handshake");
    let session = client
        .start_session(Some("/tmp/project".into()))
        .expect("session");
    client.prompt(&session, "hello daemon").expect("prompt");

    let mut text = String::new();
    let mut tools = 0;
    let mut finished = None;

    while let Some(message) = client.next_message().expect("message") {
        match message {
            ServerMessage::Part {
                part: Part::Text { delta },
                ..
            } => text.push_str(&delta),
            ServerMessage::Part {
                part: Part::Tool { .. },
                ..
            } => tools += 1,
            ServerMessage::TurnFinished { reason, .. } => {
                finished = Some(reason);
                break;
            }
            _ => {}
        }
    }

    assert!(
        tools >= 2,
        "the demo turn reports a running and a done tool"
    );
    assert!(
        text.contains("hello daemon"),
        "the prompt reaches the daemon"
    );
    assert!(text.contains("streamed in deltas"));
    assert_eq!(finished, Some(FinishReason::Completed));
}

/// Reads until the daemon reports the session's options. The list is pushed
/// from the daemon's own thread, so it lands after the synchronous answers.
fn wait_options(client: &mut Client, session: &str) -> Vec<ConfigOption> {
    for _ in 0..100 {
        match client.next_message().expect("message") {
            Some(ServerMessage::Options {
                session: got,
                options,
            }) if got == session => return options,
            Some(_) => continue,
            None => panic!("the daemon closed the connection"),
        }
    }
    panic!("no options for {session}");
}

#[test]
fn options_round_trip_through_the_daemon() {
    let (daemon, url, token) = daemon();
    thread::spawn(move || daemon.serve());

    let mut client = Client::connect(&url, &token).expect("handshake");
    let session = client.start_session(None).expect("session");

    // The daemon pushes the list after the session opens.
    let options = wait_options(&mut client, &session);
    assert!(options.iter().any(|option| option.id == "model"));
    let effort = options
        .iter()
        .find(|option| option.id == "effort")
        .expect("effort");
    assert_eq!(effort.current, "medium");

    // A choice comes back refreshed, the way the pickers redraw.
    client.set_option(&session, "effort", "high").expect("set");
    let options = wait_options(&mut client, &session);
    let effort = options
        .iter()
        .find(|option| option.id == "effort")
        .expect("effort");
    assert_eq!(effort.current, "high");

    // An unknown session reads as an error, not a hang.
    client
        .set_option("missing", "effort", "high")
        .expect("send");
    for _ in 0..100 {
        match client.next_message().expect("message") {
            Some(ServerMessage::Error { message }) => {
                assert!(message.contains("unknown session"), "{message}");
                return;
            }
            Some(_) => continue,
            None => panic!("the daemon closed the connection"),
        }
    }
    panic!("no error for a missing session");
}

#[test]
fn the_wrong_token_is_refused() {
    let (daemon, url, _) = daemon();
    thread::spawn(move || daemon.serve());

    match Client::connect(&url, "not-the-token") {
        Err(ClientError::Refused(message)) => assert!(message.contains("token")),
        Err(other) => panic!("expected a refusal, got {other:?}"),
        Ok(_) => panic!("expected a refusal, got a connection"),
    }
}

#[test]
fn a_shutdown_stops_the_listener_and_frees_the_port() {
    use std::net::TcpStream;
    use std::sync::mpsc::channel;

    let (daemon, url, token) = daemon();
    let (fire, stopped) = channel();
    thread::spawn(move || daemon.serve_until(stopped));

    // A turn goes through first: the shutdown must not disturb served work.
    let mut client = Client::connect(&url, &token).expect("handshake");
    let session = client.start_session(None).expect("session");
    client.prompt(&session, "before the end").expect("prompt");
    while let Some(message) = client.next_message().expect("message") {
        if matches!(message, ServerMessage::TurnFinished { .. }) {
            break;
        }
    }

    // The supervisor's stdin closed: the daemon winds down and returns,
    // destructors included.
    fire.send(()).expect("fire");

    let host_port = url.trim_start_matches("ws://").to_owned();
    let mut refused = false;
    for _ in 0..100 {
        match TcpStream::connect(&host_port) {
            // The connection may still land on a socket that accept already
            // drained; only a hard refusal proves the listener is closed.
            Err(_) => {
                refused = true;
                break;
            }
            Ok(_) => thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    assert!(refused, "the daemon kept listening after the shutdown");
}

#[test]
fn an_unknown_session_comes_back_as_an_error() {
    let (daemon, url, token) = daemon();
    thread::spawn(move || daemon.serve());

    let mut client = Client::connect(&url, &token).expect("handshake");
    client.prompt("missing-session", "hello").expect("prompt");

    assert!(matches!(
        client.next_message().expect("message"),
        Some(ServerMessage::Error { .. })
    ));
}

#[test]
fn the_announcement_carries_the_url_and_the_token() {
    let (daemon, url, token) = daemon();
    let announcement = daemon.announcement();

    assert!(url.starts_with("ws://"), "the transport is a WebSocket");
    assert!(announcement.contains(&url));
    assert!(announcement.contains(&token));
}

/// Keeps the demo turn's shapes honest for the transcript renderer.
#[test]
fn the_turn_reaches_the_client_in_order() {
    let (daemon, url, token) = daemon();
    thread::spawn(move || daemon.serve());

    let mut client = Client::connect(&url, &token).expect("handshake");
    let session = client.start_session(None).expect("session");
    client.prompt(&session, "order").expect("prompt");

    let mut seen_done_tool = false;
    let mut seen_text = false;

    while let Some(message) = client.next_message().expect("message") {
        match message {
            ServerMessage::Part {
                part:
                    Part::Tool {
                        state: ToolState::Done,
                        ..
                    },
                ..
            } => seen_done_tool = true,
            ServerMessage::Part {
                part: Part::Text { .. },
                ..
            } => {
                if !seen_text {
                    // A done tool must be reported before the answer starts.
                    assert!(seen_done_tool, "the answer starts after the tool finishes");
                    seen_text = true;
                }
            }
            ServerMessage::TurnFinished { .. } => break,
            _ => {}
        }
    }

    assert!(seen_text);
}
