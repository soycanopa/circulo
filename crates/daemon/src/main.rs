//! The daemon process: bind, announce, serve.

use std::io::{self, Write};
use std::process::ExitCode;

use circulo_daemon::Daemon;

fn main() -> ExitCode {
    let daemon = match Daemon::bind() {
        Ok(daemon) => daemon,
        Err(error) => {
            eprintln!("circulo-daemon: cannot bind: {error}");
            return ExitCode::FAILURE;
        }
    };

    // The supervisor (today the desktop) parses this line.
    println!("{}", daemon.announcement());
    let _ = io::stdout().flush();

    daemon.serve();
}
