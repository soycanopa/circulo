//! The daemon process, owned by the app.
//!
//! One supervisor, one lifecycle: the app starts `circulo-daemon` next to its
//! own binary, reads the announcement line it prints, and lets go of its stdin
//! on quit, which winds the daemon down cleanly — only a daemon that refuses
//! to die is killed. The tailcat tunnel will be started the same way, from the
//! same place, when remote lands.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
struct Announcement {
    url: String,
    token: String,
}

pub struct Daemon {
    child: Child,
    /// Held open for as long as the app lives. When it closes — a crash, a
    /// SIGKILL, a normal quit — the daemon sees EOF and shuts down, so no
    /// listener and no agent process outlives us.
    stdin: Option<ChildStdin>,
    url: String,
    token: String,
}

impl Daemon {
    pub fn start() -> Result<Self, SupervisorError> {
        let program = daemon_program()?;

        let mut child = Command::new(&program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // The daemon's diagnostics belong on our stderr, not in the UI.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|source| SupervisorError::Spawn {
                program: program.display().to_string(),
                source,
            })?;

        let stdin = child.stdin.take().ok_or(SupervisorError::NoAnnouncement)?;
        let stdout = child.stdout.take().ok_or(SupervisorError::NoAnnouncement)?;
        let line = BufReader::new(stdout)
            .lines()
            .next()
            .ok_or(SupervisorError::NoAnnouncement)??;
        let Announcement { url, token } = parse(&line)?;

        Ok(Self {
            child,
            stdin: Some(stdin),
            url,
            token,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // Closing stdin is the polite ask: the daemon winds down and takes
        // its agent processes with it. It gets a moment to do that; only a
        // daemon that refuses to die is killed outright.
        drop(self.stdin.take());
        for _ in 0..40 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn parse(line: &str) -> Result<Announcement, SupervisorError> {
    serde_json::from_str(line).map_err(SupervisorError::Announcement)
}

/// The daemon sits next to the app in `target/debug`, and next to the bundle in
/// a release. `CIRCULO_DAEMON` overrides both for testing.
fn daemon_program() -> Result<PathBuf, SupervisorError> {
    if let Some(path) = std::env::var_os("CIRCULO_DAEMON") {
        return Ok(PathBuf::from(path));
    }

    let exe = std::env::current_exe()?;
    let directory = exe.parent().ok_or(SupervisorError::NoDaemon)?;
    let name = if cfg!(windows) {
        "circulo-daemon.exe"
    } else {
        "circulo-daemon"
    };
    let program = directory.join(name);

    if !program.exists() {
        return Err(SupervisorError::NoDaemon);
    }

    Ok(program)
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot find circulo-daemon next to the app; build the workspace first")]
    NoDaemon,
    #[error("circulo-daemon did not announce itself")]
    NoAnnouncement,
    #[error("cannot read the daemon announcement: {0}")]
    Announcement(serde_json::Error),
    #[error("cannot start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_announcement() {
        let announcement = parse(r#"{"url":"ws://127.0.0.1:54591","token":"abc"}"#).expect("parse");

        assert_eq!(announcement.url, "ws://127.0.0.1:54591");
        assert_eq!(announcement.token, "abc");
    }

    #[test]
    fn a_bad_line_is_an_error() {
        assert!(parse("circulo-daemon: something went wrong").is_err());
    }
}
