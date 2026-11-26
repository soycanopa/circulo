//! One agent per session: the continuity contract, against a scripted agent.
//!
//! OpenCode 2.3.2 keeps a session's state in its own store, so the driver's
//! job is the lifecycle: keep one `opencode acp` child per Circulo session,
//! apply a mid-turn choice at the turn's end, release the child on shutdown,
//! and start over when the child dies. This scripted agent makes each of
//! those observable — a prompt counter proves the process is the same one,
//! an "alive" file proves it is gone.

#![forbid(unsafe_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use circulo_protocol::{ConfigOption, Part, PermissionRequest};
use circulo_provider::{Decision, Driver, SessionRef};

/// A fake agent that counts the prompts it has served in this process and
/// reports its current effort, so a test can read both back. When
/// `FAKE_ACP_LIVENESS` names a file, the agent creates it on start (with its
/// pid) and removes it when our stdin closes on it — EOF means the driver
/// let go, however that happened. When `FAKE_ACP_STORE` names a directory,
/// sessions survive the process there, the way OpenCode keeps its own store:
/// `session/load` restores the prompt count, an unknown one is refused.
const AGENT: &str = r#"
import json, os, sys

MODE = os.environ.get("FAKE_ACP_MODE", "plain")
LIVENESS = os.environ.get("FAKE_ACP_LIVENESS")
STORE = os.environ.get("FAKE_ACP_STORE")
if STORE and not os.path.isdir(STORE):
    os.makedirs(STORE)
if LIVENESS:
    with open(LIVENESS, "w") as alive:
        alive.write(str(os.getpid()))

CONFIG = {
    "effort": {"name": "Effort",
               "options": [("low", "Low"), ("high", "High")],
               "current": "low"},
}

PROMPTS = 0
CURRENT = None

def session_path(session_id):
    return os.path.join(STORE, session_id + ".json")

def remember():
    if STORE and CURRENT:
        with open(session_path(CURRENT), "w") as file:
            json.dump({"prompts": PROMPTS}, file)

def config_options():
    return [{"id": key, "name": entry["name"], "type": "select",
             "currentValue": entry["current"],
             "options": [{"value": value, "name": name}
                         for value, name in entry["options"]]}
            for key, entry in CONFIG.items()]

def send(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")

    if method == "initialize":
        send({"jsonrpc": "2.0", "id": message["id"],
              "result": {"protocolVersion": 1, "agentCapabilities": {}}})
    elif method == "session/new":
        CONFIG["effort"]["current"] = "low"
        PROMPTS = 0
        if STORE:
            CURRENT = "s%d" % (len([name for name in os.listdir(STORE)
                                    if name.endswith(".json")]) + 1)
            remember()
        else:
            CURRENT = "s1"
        send({"jsonrpc": "2.0", "id": message["id"],
              "result": {"sessionId": CURRENT, "configOptions": config_options()}})
    elif method == "session/load":
        wanted = message.get("params", {}).get("sessionId")
        if STORE and wanted and os.path.exists(session_path(wanted)):
            with open(session_path(wanted)) as file:
                PROMPTS = json.load(file)["prompts"]
            CURRENT = wanted
            send({"jsonrpc": "2.0", "id": message["id"],
                  "result": {"sessionId": CURRENT, "configOptions": config_options()}})
        else:
            send({"jsonrpc": "2.0", "id": message["id"],
                  "error": {"code": -32602, "message": "unknown session"}})
    elif method == "session/set_config_option":
        params = message.get("params", {})
        entry = CONFIG.get(params.get("configId"))
        values = [value for value, _ in entry["options"]] if entry else []
        if entry is None or params.get("value") not in values:
            send({"jsonrpc": "2.0", "id": message["id"],
                  "error": {"code": -32602, "message": "unknown option or value"}})
        else:
            entry["current"] = params["value"]
            send({"jsonrpc": "2.0", "id": message["id"],
                  "result": {"configOptions": config_options()}})
    elif method == "session/prompt":
        PROMPTS += 1
        remember()

        if MODE == "permission":
            send({"jsonrpc": "2.0", "id": 900, "method": "session/request_permission",
                  "params": {
                      "sessionId": CURRENT or "s1",
                      "toolCall": {"toolCallId": "call-1", "title": "Run cargo test",
                                   "kind": "execute",
                                   "rawInput": {"command": "cargo test"}},
                      "options": [
                          {"optionId": "allow-once", "name": "Allow once",
                           "kind": "allow_once"}]}})
            answer = json.loads(sys.stdin.readline())
            answer["result"]["outcome"]

        text = "n=%d effort=%s" % (PROMPTS, CONFIG["effort"]["current"])
        send({"jsonrpc": "2.0", "method": "session/update",
              "params": {"sessionId": CURRENT or "s1",
                         "update": {"sessionUpdate": "agent_message_chunk",
                                    "content": {"type": "text", "text": text}}}})
        send({"jsonrpc": "2.0", "id": message["id"],
              "result": {"stopReason": "end_turn"}})

if LIVENESS:
    os.remove(LIVENESS)
"#;

/// The scripted agent, its launcher and its liveness file, all in one
/// throwaway directory.
struct ScriptedAgent {
    directory: PathBuf,
}

impl ScriptedAgent {
    fn new(name: &str, mode: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("circulo-continuity-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("temp dir");

        let agent = directory.join("agent.py");
        fs::write(&agent, AGENT).expect("write agent");

        let liveness = directory.join("alive");
        let store = directory.join("store");
        fs::write(
            directory.join("agent.sh"),
            format!(
                "#!/bin/sh\nFAKE_ACP_MODE={mode} FAKE_ACP_LIVENESS='{}' FAKE_ACP_STORE='{}' exec python3 '{}'\n",
                liveness.to_string_lossy(),
                store.to_string_lossy(),
                agent.to_string_lossy()
            ),
        )
        .expect("write launcher");

        let launcher = directory.join("agent.sh");
        let mut permissions = fs::metadata(&launcher).expect("launcher").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&launcher, permissions).expect("chmod");

        Self { directory }
    }

    fn plain(name: &str) -> Self {
        Self::new(name, "plain")
    }

    fn binary(&self) -> PathBuf {
        self.directory.join("agent.sh")
    }

    /// The pid the agent reported, read from its liveness file.
    fn pid(&self) -> Option<u32> {
        fs::read_to_string(self.directory.join("alive"))
            .ok()
            .and_then(|pid| pid.trim().parse().ok())
    }

    /// Whether the agent process is still running, asked the same way the
    /// system would: a signal probe that is delivered to no one is an error.
    fn alive(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl Drop for ScriptedAgent {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

/// A user who is not there: out-of-turn requests never prompt, and these
/// tests ask for nothing on their own.
struct Refuse;

impl circulo_provider::Asker for Refuse {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: circulo_protocol::Question) -> circulo_provider::Answer {
        circulo_provider::Answer::cancel()
    }
}

/// A user who answers the first permission card from another thread, so a
/// test can act while the turn is parked on it.
struct Gate {
    asked: Sender<PermissionRequest>,
    proceed: Receiver<Decision>,
}

impl circulo_provider::Asker for Gate {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        let _ = self.asked.send(request);
        self.proceed.recv().unwrap_or(Decision::Cancelled)
    }

    fn question(&mut self, _: circulo_protocol::Question) -> circulo_provider::Answer {
        circulo_provider::Answer::cancel()
    }
}

fn run_turn(
    driver: &circulo_provider_opencode::OpenCode,
    session: &str,
    ask: &mut dyn circulo_provider::Asker,
) -> std::io::Result<String> {
    let mut parts = Vec::new();
    driver
        .start_turn(SessionRef::new(session), "go", ask, &mut |part: Part| {
            parts.push(part)
        })
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    Ok(parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect())
}

fn wait_process_gone(agent: &ScriptedAgent) -> bool {
    let Some(pid) = agent.pid() else {
        return true;
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !ScriptedAgent::alive(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn two_turns_share_one_agent_session() {
    let agent = ScriptedAgent::plain("share");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;

    let first = run_turn(&driver, "s1", &mut ask).expect("first turn");
    let second = run_turn(&driver, "s1", &mut ask).expect("second turn");

    assert!(first.contains("n=1"), "{first}");
    assert!(
        second.contains("n=2"),
        "the second turn kept the agent's memory: {second}"
    );
}

#[test]
fn the_session_key_rides_back_to_the_core() {
    let agent = ScriptedAgent::plain("key");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;

    let mut parts = Vec::new();
    let key = driver
        .start_turn(SessionRef::new("s1"), "go", &mut ask, &mut |part: Part| {
            parts.push(part)
        })
        .expect("turn");
    assert_eq!(key.as_deref(), Some("s1"), "the agent's session id");
}

#[test]
fn a_choice_applies_to_the_live_session() {
    let agent = ScriptedAgent::plain("live-choice");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;

    run_turn(&driver, "s1", &mut ask).expect("attach");

    let options = driver.set_option("s1", "effort", "high").expect("set");
    let effort = options
        .iter()
        .find(|option| option.id == "effort")
        .expect("effort");
    assert_eq!(effort.current, "high");

    let second = run_turn(&driver, "s1", &mut ask).expect("second turn");
    assert!(second.contains("n=2 effort=high"), "{second}");
}

#[test]
fn a_choice_made_mid_turn_lands_on_the_next_turn() {
    let agent = ScriptedAgent::new("mid-turn", "permission");
    let driver = std::sync::Arc::new(circulo_provider_opencode::OpenCode::with_binary(
        agent.directory.clone(),
        agent.binary(),
    ));

    let (asked_tx, asked_rx) = channel();
    let (proceed_tx, proceed_rx) = channel();
    let mut gate = Gate {
        asked: asked_tx,
        proceed: proceed_rx,
    };

    // The turn parks on its permission card.
    let turn = {
        let driver = std::sync::Arc::clone(&driver);
        thread::spawn(move || {
            let mut parts = Vec::new();
            let _ = driver.start_turn(
                circulo_provider::SessionRef::new("s1"),
                "go",
                &mut gate,
                &mut |part: Part| parts.push(part),
            );
            parts
                .iter()
                .filter_map(|part| match part {
                    Part::Text { delta } => Some(delta.clone()),
                    _ => None,
                })
                .collect::<String>()
        })
    };

    let _card = asked_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("the card arrived");

    // The session is busy: the choice queues instead of refusing.
    let options = driver.set_option("s1", "effort", "high").expect("set");
    let effort = options
        .iter()
        .find(|option| option.id == "effort")
        .expect("effort");
    assert_eq!(
        effort.current, "high",
        "the picker is told what it asked for"
    );

    // Answer the card, and the turn resumes with the agent's own, still-low
    // value — the queued choice has not been sent yet.
    proceed_tx
        .send(Decision::Selected("allow-once".to_owned()))
        .expect("answer the card");
    let first = turn.join().expect("turn thread");
    assert!(first.contains("n=1 effort=low"), "{first}");

    // The recall at the turn's end applied the choice: this turn reads high.
    let mut ask = Refuse;
    let second = run_turn(&driver, "s1", &mut ask).expect("second turn");
    assert!(second.contains("n=2 effort=high"), "{second}");

    drop(proceed_tx);
}

#[test]
fn shutdown_releases_the_agent_process() {
    let agent = ScriptedAgent::plain("shutdown");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;
    run_turn(&driver, "s1", &mut ask).expect("attach");
    let pid = agent.pid().expect("the agent is alive");
    assert!(ScriptedAgent::alive(pid), "the agent process is running");

    driver.shutdown();

    assert!(
        wait_process_gone(&agent),
        "the agent process survived the shutdown"
    );
}

#[test]
fn a_dead_agent_is_dropped_and_the_next_turn_starts_fresh() {
    let agent = ScriptedAgent::plain("dead");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;
    run_turn(&driver, "s1", &mut ask).expect("attach");

    // The agent dies under us, the way a crash would.
    let pid = agent.pid().expect("the agent's pid");
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .expect("kill the agent");

    let failed = run_turn(&driver, "s1", &mut ask);
    assert!(failed.is_err(), "a turn on a dead agent cannot succeed");

    let fresh = run_turn(&driver, "s1", &mut ask).expect("fresh turn");
    assert!(
        fresh.contains("n=1"),
        "a new agent process took over: {fresh}"
    );
}

#[test]
fn options_read_the_live_session_without_spawning() {
    let agent = ScriptedAgent::plain("live-list");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;
    run_turn(&driver, "s1", &mut ask).expect("attach");

    let options: Vec<ConfigOption> = driver.options("s1").expect("options");
    let effort = options
        .iter()
        .find(|option| option.id == "effort")
        .expect("effort");
    assert_eq!(effort.current, "low");
}

#[test]
fn a_restarted_driver_resumes_the_stored_session() {
    let agent = ScriptedAgent::plain("resume");
    // The first life of the session: one turn, then the daemon is gone —
    // the driver drop releases its child, memory included. Only the key
    // the driver handed back survives, the way the core's database would.
    let key = {
        let driver = circulo_provider_opencode::OpenCode::with_binary(
            agent.directory.clone(),
            agent.binary(),
        );
        let mut ask = Refuse;
        let mut parts = Vec::new();
        let key = driver
            .start_turn(SessionRef::new("s1"), "go", &mut ask, &mut |part: Part| {
                parts.push(part)
            })
            .expect("first life")
            .expect("the session key");
        let first: String = parts
            .iter()
            .filter_map(|part| match part {
                Part::Text { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert!(first.contains("n=1"), "{first}");
        key
    };

    // The second life: a fresh driver, the stored key, the same history.
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;
    let mut parts = Vec::new();
    let key = driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume: Some(&key),
                cwd: None,
                attachments: &[],
            },
            "go",
            &mut ask,
            &mut |part: Part| parts.push(part),
        )
        .expect("resumed turn");

    let resumed: String = parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        resumed.contains("n=2"),
        "the resumed agent continued where it left off: {resumed}"
    );
    assert_eq!(key.as_deref(), Some("s1"), "the same session, kept");
}

#[test]
fn a_lost_session_starts_fresh_rather_than_failing() {
    let agent = ScriptedAgent::plain("lost");
    let driver =
        circulo_provider_opencode::OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut ask = Refuse;

    // No such session in the agent's store: the driver starts over instead
    // of failing the turn, and the fresh key comes back to heal the store.
    let mut parts = Vec::new();
    let key = driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume: Some("gone"),
                cwd: None,
                attachments: &[],
            },
            "go",
            &mut ask,
            &mut |part: Part| parts.push(part),
        )
        .expect("turn");

    let fresh: String = parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert!(fresh.contains("n=1"), "a new conversation began: {fresh}");
    assert_eq!(key.as_deref(), Some("s1"), "the fresh session id");
}
