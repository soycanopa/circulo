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

const PROMPT: &str = "Give me a quick overview of this workspace.";

/// The composer is not typeable yet, so the app sends one fixed prompt. The
/// override exists so a real provider can be exercised with a different ask
/// (a table, a code block) without a rebuild.
fn prompt() -> String {
    std::env::var("CIRCULO_PROMPT").unwrap_or_else(|_| PROMPT.to_owned())
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

    let session = match client.start_session(None) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("circulo: cannot open a session: {error}");
            return ExitCode::FAILURE;
        }
    };

    let prompt = prompt();
    if let Err(error) = client.prompt(&session, &prompt) {
        eprintln!("circulo: cannot send the prompt: {error}");
        return ExitCode::FAILURE;
    }

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
            let sent = match request {
                ClientRequest::AnswerPermission { id, option } => {
                    writer.answer_permission(&id, option.as_deref())
                }
                ClientRequest::AnswerQuestion {
                    id,
                    action,
                    answers,
                } => writer.answer_question(&id, action, answers),
                _ => Ok(()),
            };
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
                    cx.new(|cx| {
                        Shell::new(cx, receiver.clone(), answers_tx.clone(), prompt.clone())
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
