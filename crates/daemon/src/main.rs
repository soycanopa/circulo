//! The daemon process: bind, announce, serve.

use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::sync::mpsc::channel;
use std::thread;

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

    // One lifecycle: when the supervisor goes away, its end of our stdin
    // closes and the serve loop winds down — cards cancelled, agents released
    // by the driver, destructors run. This survives a SIGKILL of the parent,
    // where no graceful shutdown would run.
    let (fire, stopped) = channel();
    thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut buffer = [0u8; 64];
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        let _ = fire.send(());
    });

    daemon.serve_until(stopped);
    ExitCode::SUCCESS
}
