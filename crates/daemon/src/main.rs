//! The daemon process: bind, announce, serve.

use std::io::{self, Read, Write};
use std::process::ExitCode;
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

    // One lifecycle: when the supervisor goes away, its end of our stdin closes
    // and we exit with it. This survives a SIGKILL of the parent, where no
    // graceful shutdown would run.
    thread::spawn(|| {
        let mut stdin = io::stdin();
        let mut buffer = [0u8; 64];
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        std::process::exit(0);
    });

    daemon.serve();
}
