//! The Codex lifecycle against a scripted app server.
//!
//! The real CLI is not installed on every machine, and a real turn spends a
//! model call. This Python stand-in speaks the documented app-server protocol
//! and makes the lifecycle observable: a per-thread prompt counter proves the
//! same thread is reused, a liveness file proves the child was released, and
//! permission/question answers come back in the text the turn streams.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use circulo_protocol::{Part, PermissionRequest, Question, QuestionKind};
use circulo_provider::{Answer, Asker, Decision, Driver, SessionRef};
use circulo_provider_codex::Codex;

/// A fake `codex app-server`. `FAKE_CODEX_MODE` picks what a turn does:
/// plain, permission, question, cancel, error, cwd or options.
const AGENT: &str = r#"
import json, os, sys

MODE = os.environ.get("FAKE_CODEX_MODE", "plain")
LIVENESS = os.environ.get("FAKE_CODEX_LIVENESS")
STORE = os.environ.get("FAKE_CODEX_STORE")

if STORE and not os.path.isdir(STORE):
    os.makedirs(STORE)
if LIVENESS:
    with open(LIVENESS, "w") as alive:
        alive.write(str(os.getpid()))

THREADS = {}
NEXT_TURN = 0

def thread_path(tid):
    return os.path.join(STORE, tid + ".json")

def persist(tid):
    if STORE:
        with open(thread_path(tid), "w") as file:
            json.dump(THREADS[tid], file)

def new_thread(cwd, model):
    n = 1
    if STORE:
        n = len([name for name in os.listdir(STORE) if name.endswith(".json")]) + 1
    tid = "thr_%d" % n
    THREADS[tid] = {"prompts": 0, "cwd": cwd, "model": model or "gpt-test-one", "effort": "low"}
    persist(tid)
    return tid

def send(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()

def respond(message, result):
    send({"id": message["id"], "result": result})

def fail(message, text):
    send({"id": message["id"], "error": {"code": -32602, "message": text}})

def models():
    return {"data": [
        {"id": "gpt-test-one", "displayName": "GPT Test One", "isDefault": True,
         "defaultReasoningEffort": "low",
         "supportedReasoningEfforts": [{"reasoningEffort": "low"}, {"reasoningEffort": "high"}]},
        {"id": "gpt-test-two", "displayName": "GPT Test Two", "isDefault": False,
         "defaultReasoningEffort": "medium",
         "supportedReasoningEfforts": [{"reasoningEffort": "medium"}, {"reasoningEffort": "high"}]},
    ]}

def completed(tid, turn, status="completed", error=None):
    payload = {"threadId": tid, "turn": {"id": turn, "status": status, "items": []}}
    if error:
        payload["turn"]["error"] = {"message": error}
    send({"method": "turn/completed", "params": payload})

def text(tid, turn, item, delta):
    send({"method": "item/agentMessage/delta",
          "params": {"threadId": tid, "turnId": turn, "itemId": item, "delta": delta}})

def reasoning(tid, turn, item, delta):
    send({"method": "item/reasoning/summaryTextDelta",
          "params": {"threadId": tid, "turnId": turn, "itemId": item,
                     "delta": delta, "summaryIndex": 0}})

def run_turn(message):
    global NEXT_TURN
    params = message.get("params", {})
    tid = params.get("threadId")
    entry = THREADS.get(tid)
    if entry is None:
        fail(message, "unknown thread")
        return

    entry["prompts"] += 1
    if params.get("model"):
        entry["model"] = params["model"]
    if params.get("effort"):
        entry["effort"] = params["effort"]
    persist(tid)

    NEXT_TURN += 1
    turn = "turn_%d" % NEXT_TURN
    respond(message, {"turn": {"id": turn, "status": "inProgress", "items": []}})
    send({"method": "turn/started",
          "params": {"threadId": tid, "turn": {"id": turn, "status": "inProgress", "items": []}}})

    if MODE == "cancel":
        text(tid, turn, "m1", "working")
        while True:
            line = sys.stdin.readline()
            if not line:
                return
            incoming = json.loads(line)
            if incoming.get("method") == "turn/interrupt":
                respond(incoming, {})
                completed(tid, turn, "interrupted")
                return
        return

    if MODE == "permission":
        send({"id": 900, "method": "item/commandExecution/requestApproval",
              "params": {"threadId": tid, "turnId": turn, "itemId": "cmd1",
                         "command": "cargo test --workspace", "reason": "Run the tests"}})
        answer = json.loads(sys.stdin.readline())
        decision = answer.get("result", {}).get("decision", "?")
        text(tid, turn, "m1", "decision=" + decision)
        completed(tid, turn)
        return

    if MODE == "question":
        send({"id": 901, "method": "mcpServer/elicitation/request",
              "params": {"threadId": tid, "turnId": turn, "serverName": "fake",
                         "mode": "form", "message": "How should I approach this?",
                         "requestedSchema": {
                             "type": "object",
                             "properties": {
                                 "strategy": {"type": "string",
                                              "enum": ["conservative", "balanced"]},
                                 "note": {"type": "string", "title": "Note"}},
                             "required": ["strategy"]}}})
        answer = json.loads(sys.stdin.readline())
        result = answer.get("result", {})
        content = result.get("content") or {}
        text(tid, turn, "m1",
             "answer=" + result.get("action", "?") + ":" +
             str(content.get("strategy", "-")) + ":" + str(content.get("note", "-")))
        completed(tid, turn)
        return

    reasoning(tid, turn, "r1", "thinking")
    send({"method": "item/started",
          "params": {"threadId": tid, "turnId": turn,
                     "item": {"type": "commandExecution", "id": "cmd1",
                              "command": "cargo test", "status": "inProgress"}}})
    send({"method": "item/commandExecution/outputDelta",
          "params": {"threadId": tid, "turnId": turn, "itemId": "cmd1",
                     "delta": "test output"}})
    send({"method": "item/commandExecution/outputDelta",
          "params": {"threadId": tid, "turnId": turn, "itemId": "cmd1",
                     "delta": "\nall good"}})
    text(tid, turn, "m1", "hello ")
    text(tid, turn, "m1", "world")
    send({"method": "item/completed",
          "params": {"threadId": tid, "turnId": turn,
                     "item": {"type": "commandExecution", "id": "cmd1",
                              "command": "cargo test", "status": "completed"}}})

    if MODE == "error":
        completed(tid, turn, "failed", "the model said no")
    elif MODE == "cwd":
        text(tid, turn, "m2", "cwd=" + entry["cwd"])
        completed(tid, turn)
    elif MODE == "options":
        text(tid, turn, "m2", "n=%d model=%s effort=%s" % (
            entry["prompts"], entry["model"], entry["effort"]))
        completed(tid, turn)
    else:
        text(tid, turn, "m2", "n=%d" % entry["prompts"])
        completed(tid, turn)

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")

    if method == "initialize":
        respond(message, {"userAgent": "fake-codex", "platformFamily": "unix",
                          "platformOs": "macos"})
    elif method == "initialized":
        pass
    elif method == "model/list":
        respond(message, models())
    elif method == "thread/start":
        params = message.get("params", {})
        tid = new_thread(params.get("cwd", ""), params.get("model"))
        send({"method": "thread/started", "params": {"thread": {"id": tid}}})
        respond(message, {"thread": {"id": tid, "sessionId": tid},
                          "model": THREADS[tid]["model"],
                          "approvalPolicy": "on-request"})
    elif method == "thread/resume":
        params = message.get("params", {})
        tid = params.get("threadId")
        loaded = THREADS.get(tid)
        if loaded is None and STORE and tid and os.path.exists(thread_path(tid)):
            with open(thread_path(tid)) as file:
                loaded = json.load(file)
            THREADS[tid] = loaded
        if loaded is None:
            fail(message, "unknown thread")
        else:
            if params.get("model"):
                loaded["model"] = params["model"]
            persist(tid)
            respond(message, {"thread": {"id": tid, "sessionId": tid},
                              "model": loaded["model"],
                              "approvalPolicy": "on-request"})
    elif method == "turn/start":
        run_turn(message)
    elif method == "turn/interrupt":
        respond(message, {})
    elif message.get("id") is not None:
        send({"id": message["id"],
              "error": {"code": -32601, "message": "unknown method"}})

if LIVENESS:
    try:
        os.remove(LIVENESS)
    except OSError:
        pass
"#;

struct ScriptedAgent {
    directory: PathBuf,
}

impl ScriptedAgent {
    fn new(name: &str, mode: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("circulo-codex-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("project")).expect("temp dir");

        let agent = directory.join("agent.py");
        fs::write(&agent, AGENT).expect("write agent");

        let liveness = directory.join("alive");
        let store = directory.join("store");
        fs::write(
            directory.join("agent.sh"),
            format!(
                "#!/bin/sh\nFAKE_CODEX_MODE={mode} FAKE_CODEX_LIVENESS='{}' FAKE_CODEX_STORE='{}' exec python3 '{}'\n",
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

    fn binary(&self) -> PathBuf {
        self.directory.join("agent.sh")
    }

    fn project(&self) -> PathBuf {
        self.directory.join("project")
    }

    fn pid(&self) -> Option<u32> {
        fs::read_to_string(self.directory.join("alive"))
            .ok()
            .and_then(|pid| pid.trim().parse().ok())
    }

    fn alive(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

struct Refuse;

impl Asker for Refuse {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: Question) -> Answer {
        Answer::cancel()
    }
}

struct AllowOnce {
    seen: Arc<Mutex<Option<PermissionRequest>>>,
}

impl Asker for AllowOnce {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        *self.seen.lock().unwrap() = Some(request);
        Decision::Selected("accept".to_owned())
    }

    fn question(&mut self, _: Question) -> Answer {
        Answer::cancel()
    }
}

struct Balanced;

impl Asker for Balanced {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: Question) -> Answer {
        Answer::accept(BTreeMap::from([(
            "strategy".to_owned(),
            "balanced".to_owned(),
        )]))
    }
}

fn text(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect()
}

fn turn(driver: &Codex, session: &str, prompt: &str) -> (Option<String>, Vec<Part>) {
    let mut ask = Refuse;
    let mut parts = Vec::new();
    let key = driver
        .start_turn(SessionRef::new(session), prompt, &mut ask, &mut |part| {
            parts.push(part)
        })
        .expect("turn");
    (key, parts)
}

fn wait_until(mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn a_turn_streams_text_reasoning_and_tools() {
    let agent = ScriptedAgent::new("stream", "plain");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    let (key, parts) = turn(&driver, "s1", "hello");

    assert!(key.is_some());
    assert_eq!(text(&parts), "hello worldn=1");
    assert!(
        parts
            .iter()
            .any(|part| matches!(part, Part::Reasoning { delta } if delta == "thinking"))
    );
    assert!(parts.iter().any(|part| matches!(
        part,
        Part::Tool { name, state: circulo_protocol::ToolState::Running, .. } if name == "command"
    )));
    assert!(parts.iter().any(|part| matches!(
        part,
        Part::Tool { name, state: circulo_protocol::ToolState::Done, output: Some(output), .. }
            if name == "command" && output == "test output\nall good"
    )));
}

#[test]
fn a_permission_card_round_trips() {
    let agent = ScriptedAgent::new("permission", "permission");
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let seen = Arc::new(Mutex::new(None));
    let mut ask = AllowOnce { seen: seen.clone() };
    let mut parts = Vec::new();

    driver
        .start_turn(
            SessionRef::new("s1"),
            "run the tests",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("turn");

    let request = seen.lock().unwrap().clone().expect("a card");
    assert_eq!(request.title, "Run the tests");
    assert_eq!(request.detail, "cargo test --workspace");
    assert_eq!(
        request
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect::<Vec<_>>(),
        vec!["accept", "acceptForSession", "decline", "cancel"]
    );
    assert!(text(&parts).contains("decision=accept"));
}

#[test]
fn a_question_card_round_trips() {
    let agent = ScriptedAgent::new("question", "question");
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let mut ask = Balanced;
    let mut parts = Vec::new();

    driver
        .start_turn(SessionRef::new("s1"), "plan", &mut ask, &mut |part| {
            parts.push(part)
        })
        .expect("turn");

    assert!(
        text(&parts).contains("answer=accept:balanced:-"),
        "the accepted value went back: {}",
        text(&parts)
    );
}

#[test]
fn model_and_effort_options_apply_to_the_next_turn() {
    let agent = ScriptedAgent::new("options", "options");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    let options = driver.options("s1").expect("options");
    assert_eq!(options[0].id, "model");
    assert_eq!(options[0].current, "gpt-test-one");
    assert_eq!(options[1].id, "effort");
    assert_eq!(options[1].current, "low");
    assert_eq!(
        options[1]
            .options
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        vec!["low", "high"]
    );

    let options = driver
        .set_option("s1", "model", "gpt-test-two")
        .expect("model");
    assert_eq!(options[0].current, "gpt-test-two");
    assert_eq!(
        options[1].current, "medium",
        "a model switch reads the new model's default effort"
    );

    driver.set_option("s1", "effort", "high").expect("effort");
    let (_, parts) = turn(&driver, "s1", "go");
    assert!(text(&parts).contains("model=gpt-test-two effort=high"));
}

#[test]
fn a_live_session_shows_the_promised_effort_and_rejects_a_stale_one() {
    let agent = ScriptedAgent::new("refuse", "options");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    turn(&driver, "s1", "one");
    let options = driver
        .set_option("s1", "model", "gpt-test-two")
        .expect("model");
    assert_eq!(options[1].current, "medium");

    let error = driver
        .set_option("s1", "effort", "low")
        .expect_err("refused");
    assert!(error.to_string().contains("not an effort"));
}

#[test]
fn the_session_project_reaches_the_agent() {
    let agent = ScriptedAgent::new("cwd", "cwd");
    let project = agent.project();
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let mut ask = Refuse;
    let mut parts = Vec::new();

    driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume: None,
                cwd: Some(project.to_str().expect("utf-8 path")),
            },
            "where",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("turn");

    assert!(
        text(&parts).contains(&format!("cwd={}", project.to_string_lossy())),
        "{}",
        text(&parts)
    );
}

#[test]
fn two_turns_share_one_thread_and_one_child() {
    let agent = ScriptedAgent::new("continuity", "plain");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    let (first, _) = turn(&driver, "s1", "one");
    let pid = wait_until(|| agent.pid().is_some())
        .then(|| agent.pid())
        .flatten();
    let (second, parts) = turn(&driver, "s1", "two");

    assert_eq!(first, second);
    assert!(text(&parts).contains("n=2"));
    assert_eq!(agent.pid(), pid, "the same child served both turns");
}

#[test]
fn a_restart_resumes_the_thread() {
    let agent = ScriptedAgent::new("resume", "plain");
    let key = {
        let driver = Codex::with_binary(agent.project(), agent.binary());
        let (key, parts) = turn(&driver, "s1", "one");
        assert!(text(&parts).contains("n=1"));
        key.expect("key")
    };

    let driver = Codex::with_binary(agent.project(), agent.binary());
    let mut ask = Refuse;
    let mut parts = Vec::new();
    let resumed = driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume: Some(&key),
                cwd: None,
            },
            "two",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("resumed turn");

    assert_eq!(resumed.as_deref(), Some(key.as_str()));
    assert!(
        text(&parts).contains("n=2"),
        "the agent kept the thread's history: {}",
        text(&parts)
    );
}

#[test]
fn a_lost_key_starts_a_fresh_thread() {
    let agent = ScriptedAgent::new("lost", "plain");
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let mut ask = Refuse;
    let mut parts = Vec::new();

    let key = driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume: Some("thr_404"),
                cwd: None,
            },
            "one",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("fallback turn");

    assert_ne!(key.as_deref(), Some("thr_404"));
    assert!(text(&parts).contains("n=1"));
}

#[test]
fn a_cancel_interrupts_the_running_turn() {
    let agent = ScriptedAgent::new("cancel", "cancel");
    let driver = Arc::new(Codex::with_binary(agent.project(), agent.binary()));
    let (sender, receiver) = channel();

    let running = {
        let driver = driver.clone();
        thread::spawn(move || {
            let mut ask = Refuse;
            let mut emit = |part: Part| {
                let _ = sender.send(part);
            };
            driver.start_turn(SessionRef::new("s1"), "work", &mut ask, &mut emit)
        })
    };

    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the turn started");
    driver.cancel("s1").expect("cancel");

    let error = running.join().expect("thread").expect_err("aborted");
    assert!(error.is_cancelled());
}

#[test]
fn reset_drops_the_thread() {
    let agent = ScriptedAgent::new("reset", "plain");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    let (first, _) = turn(&driver, "s1", "one");
    driver.reset("s1");
    let (second, parts) = turn(&driver, "s1", "two");

    assert_ne!(first, second, "the next prompt started a new thread");
    assert!(text(&parts).contains("n=1"));
}

#[test]
fn shutdown_releases_the_child() {
    let agent = ScriptedAgent::new("shutdown", "plain");
    let driver = Codex::with_binary(agent.project(), agent.binary());

    turn(&driver, "s1", "one");
    let pid = agent.pid().expect("the agent ran");
    driver.shutdown();

    assert!(
        wait_until(|| !ScriptedAgent::alive(pid)),
        "the app server did not exit"
    );
}

#[test]
fn a_failed_turn_carries_the_agent_message() {
    let agent = ScriptedAgent::new("error", "error");
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let mut ask = Refuse;

    let error = driver
        .start_turn(SessionRef::new("s1"), "fail", &mut ask, &mut |_| {})
        .expect_err("failed turn");

    assert!(error.to_string().contains("the model said no"));
    assert!(!error.is_cancelled());
}

#[test]
fn a_question_field_is_a_choice_or_a_text_box() {
    let agent = ScriptedAgent::new("fields", "question");
    let driver = Codex::with_binary(agent.project(), agent.binary());
    let seen = Arc::new(Mutex::new(None));

    struct Capture(Arc<Mutex<Option<Question>>>);
    impl Asker for Capture {
        fn permission(&mut self, _: PermissionRequest) -> Decision {
            Decision::Cancelled
        }
        fn question(&mut self, question: Question) -> Answer {
            *self.0.lock().unwrap() = Some(question);
            Answer::cancel()
        }
    }

    let mut ask = Capture(seen.clone());
    let _ = driver.start_turn(SessionRef::new("s1"), "plan", &mut ask, &mut |_| {});

    let question = seen.lock().unwrap().clone().expect("a question");
    assert_eq!(question.fields.len(), 2);
    assert_eq!(
        question.fields[0].kind,
        QuestionKind::Choice {
            options: vec!["conservative".into(), "balanced".into()]
        }
    );
    assert_eq!(question.fields[1].kind, QuestionKind::Text);
    assert!(question.fields[0].required);
}
