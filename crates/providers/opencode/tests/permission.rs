//! The permission and question round trips, against a scripted ACP agent.
//!
//! OpenCode 2.3.2 in ACP mode approves tools on its own, so a real session
//! never asks. This test stands an agent in front of the driver that does ask,
//! and checks the whole path: the ACP request becomes a `PermissionRequest`,
//! the answer goes back as the chosen `optionId`, and a cancelled request reads
//! as cancelled on the agent's side.
//!
//! The agent is a Python script because the test only needs JSON-RPC on stdio;
//! nothing from it ships. `FAKE_ACP_MODE` picks what it asks for.

#![forbid(unsafe_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use std::collections::BTreeMap;

use circulo_protocol::{Part, PermissionRequest, Question};
use circulo_provider::{Answer, Asker, Decision, Driver};
use circulo_provider_opencode::OpenCode;

/// A fake agent that asks for permission before it answers the prompt.
const AGENT: &str = r#"
import json, os, sys

MODE = os.environ.get("FAKE_ACP_MODE", "permission")

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
        send({"jsonrpc": "2.0", "id": message["id"], "result": {"sessionId": "s1"}})
    elif method == "session/prompt" and MODE == "question":
        send({"jsonrpc": "2.0", "id": 901, "method": "elicitation/create",
              "params": {
                  "sessionId": "s1",
                  "mode": "form",
                  "message": "How should I approach this?",
                  "requestedSchema": {
                      "type": "object",
                      "properties": {
                          "strategy": {"type": "string",
                                       "enum": ["conservative", "balanced"]},
                          "note": {"type": "string", "title": "Note"}},
                      "required": ["strategy"]}}})

        answer = json.loads(sys.stdin.readline())
        result = answer["result"]
        content = result.get("content", {}) or {}
        text = ("answer=" + result.get("action", "?")
                + ":" + str(content.get("strategy", "-"))
                + ":" + str(content.get("note", "-")))

        send({"jsonrpc": "2.0", "method": "session/update",
              "params": {"sessionId": "s1",
                         "update": {"sessionUpdate": "agent_message_chunk",
                                    "content": {"type": "text", "text": text}}}})
        send({"jsonrpc": "2.0", "id": message["id"],
              "result": {"stopReason": "end_turn"}})
    elif method == "session/prompt":
        send({"jsonrpc": "2.0", "id": 900, "method": "session/request_permission",
              "params": {
                  "sessionId": "s1",
                  "toolCall": {"toolCallId": "call-1", "title": "Run cargo test",
                               "kind": "execute",
                               "rawInput": {"command": "cargo test --workspace"}},
                  "options": [
                      {"optionId": "allow-once", "name": "Allow once", "kind": "allow_once"},
                      {"optionId": "reject-once", "name": "Reject", "kind": "reject_once"}]}})

        answer = json.loads(sys.stdin.readline())
        outcome = answer["result"]["outcome"]
        chosen = outcome.get("optionId") or outcome.get("outcome")

        send({"jsonrpc": "2.0", "method": "session/update",
              "params": {"sessionId": "s1",
                         "update": {"sessionUpdate": "agent_message_chunk",
                                    "content": {"type": "text", "text": "chosen=" + chosen}}}})
        send({"jsonrpc": "2.0", "id": message["id"],
              "result": {"stopReason": "end_turn"}})
"#;

struct ScriptedAgent {
    directory: PathBuf,
}

impl ScriptedAgent {
    fn new(name: &str) -> Self {
        Self::with_mode(name, "permission")
    }

    fn with_mode(name: &str, mode: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("circulo-acp-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("temp dir");

        let agent = directory.join("agent.py");
        fs::write(&agent, AGENT).expect("write agent");
        fs::write(
            directory.join("agent.sh"),
            format!(
                "#!/bin/sh\nFAKE_ACP_MODE={mode} exec python3 '{}'\n",
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
}

impl Drop for ScriptedAgent {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

/// A user who picks `option`, or the first one when it is `None`.
struct Picks {
    /// The permission option to pick, or cancelled when `None`.
    option: Option<&'static str>,
    /// The answer to a choice question, keyed for `strategy`; `None` declines.
    choice: Option<&'static str>,
    /// The free-text answer, keyed for `note`. `None` leaves it out.
    note: Option<&'static str>,
    asked: Vec<PermissionRequest>,
    questions: Vec<Question>,
}

impl Asker for Picks {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        self.asked.push(request);
        match self.option {
            Some(option) => Decision::Selected(option.to_owned()),
            None => Decision::Cancelled,
        }
    }

    fn question(&mut self, question: Question) -> Answer {
        self.questions.push(question);
        let Some(value) = self.choice else {
            return Answer::decline();
        };

        let mut answers = BTreeMap::from([("strategy".to_string(), value.to_string())]);
        if let Some(note) = self.note {
            answers.insert("note".to_string(), note.to_string());
        }
        Answer::accept(answers)
    }
}

fn run(agent: &ScriptedAgent, option: Option<&'static str>) -> (Vec<Part>, Picks) {
    run_answering(agent, option, None)
}

fn run_answering(
    agent: &ScriptedAgent,
    option: Option<&'static str>,
    choice: Option<&'static str>,
) -> (Vec<Part>, Picks) {
    run_with_note(agent, option, choice, None)
}

fn run_with_note(
    agent: &ScriptedAgent,
    option: Option<&'static str>,
    choice: Option<&'static str>,
    note: Option<&'static str>,
) -> (Vec<Part>, Picks) {
    let driver = OpenCode::with_binary(agent.directory.clone(), agent.binary());
    let mut asker = Picks {
        option,
        choice,
        note,
        asked: Vec::new(),
        questions: Vec::new(),
    };
    let mut parts = Vec::new();

    driver
        .start_turn("go", &mut asker, &mut |part| parts.push(part))
        .expect("turn");

    (parts, asker)
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

#[test]
fn the_agent_gets_the_option_the_user_chose() {
    let agent = ScriptedAgent::new("allow");
    let (parts, asker) = run(&agent, Some("allow-once"));

    // The card, as the driver understood the ACP request.
    assert_eq!(asker.asked.len(), 1);
    let request = &asker.asked[0];
    assert_eq!(request.title, "Run cargo test");
    assert_eq!(request.detail, "cargo test --workspace");
    assert_eq!(
        request
            .options
            .iter()
            .map(|o| o.id.as_str())
            .collect::<Vec<_>>(),
        vec!["allow-once", "reject-once"]
    );

    // And the answer went back verbatim.
    assert_eq!(text(&parts), "chosen=allow-once");
}

#[test]
fn a_question_reaches_the_card_and_both_kinds_of_answer_go_back() {
    let agent = ScriptedAgent::with_mode("question", "question");
    let (parts, asker) = run_with_note(&agent, None, Some("balanced"), Some("because I said so"));

    assert_eq!(asker.questions.len(), 1);
    let question = &asker.questions[0];
    assert_eq!(question.message, "How should I approach this?");
    assert_eq!(question.fields.len(), 2);

    // The enum property becomes a choice, the plain string a text field.
    assert_eq!(
        question.fields[0].kind,
        circulo_protocol::QuestionKind::Choice {
            options: vec!["conservative".into(), "balanced".into()]
        }
    );
    assert_eq!(question.fields[0].label, "Strategy");
    assert!(question.fields[0].required);

    assert_eq!(question.fields[1].id, "note");
    assert_eq!(question.fields[1].label, "Note");
    assert_eq!(
        question.fields[1].kind,
        circulo_protocol::QuestionKind::Text
    );
    assert!(!question.fields[1].required);

    // Both values reach the agent, keyed by their schema property.
    assert_eq!(text(&parts), "answer=accept:balanced:because I said so");
}

#[test]
fn a_declined_question_tells_the_agent_so_over_acp() {
    let agent = ScriptedAgent::with_mode("question-decline", "question");
    let (parts, _) = run_answering(&agent, None, None);

    assert_eq!(text(&parts), "answer=decline:-:-");
}

#[test]
fn a_cancelled_request_reads_as_cancelled_on_the_agent_side() {
    let agent = ScriptedAgent::new("cancel");
    let (parts, _) = run(&agent, None);

    assert_eq!(text(&parts), "chosen=cancelled");
}
