//! The composition root: it wires the pieces together, owns the window and
//! supervises the daemon.
//!
//! The app assembles the UI, starts the daemon, connects the client to it and
//! streams a turn into the window. It holds no product logic — if something
//! here grows beyond wiring, it belongs in `crates/ui` or `crates/core`.

#![forbid(unsafe_code)]

mod supervisor;

use std::process::ExitCode;
use std::thread;

use circulo_client::Client;
use circulo_protocol::ClientRequest;
use circulo_ui::{Assets, Shell};
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px,
    size,
};

/// A draft the composer starts with. It exists so a real provider can be
/// exercised without typing, and so a development session can start from a
/// fixed ask. The window sends nothing on its own.
fn draft() -> String {
    std::env::var("CIRCULO_PROMPT").unwrap_or_default()
}

fn main() -> ExitCode {
    let daemon = match supervisor::Daemon::start() {
        Ok(daemon) => daemon,
        Err(error) => {
            eprintln!("circulo: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut client = match Client::connect(daemon.url(), daemon.token()) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("circulo: cannot reach the daemon: {error}");
            return ExitCode::FAILURE;
        }
    };

    let draft = draft();
    let daemon_name = client.daemon().to_owned();

    // Reopen the newest stored session, or start one. A database that cannot be
    // read is not fatal: the window still works, it just starts empty.
    let (session, history, sessions) = match client.list_sessions() {
        Ok(sessions) if !sessions.is_empty() => match client.open_session(&sessions[0].id) {
            Ok(history) => (sessions[0].id.clone(), history, sessions),
            Err(error) => {
                eprintln!("circulo: cannot reopen the last session: {error}");
                match client.start_session(None) {
                    Ok(session) => (session, Vec::new(), Vec::new()),
                    Err(error) => {
                        eprintln!("circulo: cannot open a session: {error}");
                        return ExitCode::FAILURE;
                    }
                }
            }
        },
        _ => match client.start_session(None) {
            Ok(session) => (session, Vec::new(), Vec::new()),
            Err(error) => {
                eprintln!("circulo: cannot open a session: {error}");
                return ExitCode::FAILURE;
            }
        },
    };

    // The client is blocking, so it lives on its own thread and hands messages
    // to the UI. Dropping the receiver ends the thread with the window.
    let (sender, receiver) = async_channel::unbounded();

    // A turn can block on a permission card, and the answer has to reach the
    // daemon while the reader is blocked, so the connection splits in two.
    let (answers_tx, answers_rx) = async_channel::unbounded::<ClientRequest>();
    let (mut client, mut writer) = match client.split() {
        Ok(halves) => halves,
        Err(error) => {
            eprintln!("circulo: cannot split the connection: {error}");
            return ExitCode::FAILURE;
        }
    };

    thread::spawn(move || {
        while let Ok(Some(message)) = client.next_message() {
            if sender.send_blocking(message).is_err() {
                break;
            }
        }
    });

    thread::spawn(move || {
        while let Ok(request) = answers_rx.recv_blocking() {
            let sent = writer.send_request(&request);
            if sent.is_err() {
                break;
            }
        }
    });

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1000.), px(740.)), cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Circulo".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(16.), px(18.))),
                    }),
                    window_min_size: Some(size(px(980.), px(680.))),
                    ..Default::default()
                },
                |_, cx| {
                    let session = session.clone();
                    let history = history.clone();
                    let daemon = daemon_name.clone();
                    cx.new(|cx| {
                        Shell::new(
                            cx,
                            receiver.clone(),
                            answers_tx.clone(),
                            session,
                            draft.clone(),
                            history,
                            sessions,
                            daemon,
                        )
                    })
                },
            )
            .expect("the Circulo window should open");

            cx.activate(true);
        });

    // The daemon is killed here: one lifecycle, nothing left listening.
    drop(daemon);
    ExitCode::SUCCESS
}
