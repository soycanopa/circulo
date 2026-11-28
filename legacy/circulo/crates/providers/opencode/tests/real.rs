//! A real turn against the installed OpenCode CLI.
//!
//! Ignored by default: it spawns `opencode acp`, needs a logged-in provider and
//! spends a model call. Run it with `cargo test -p circulo-provider-opencode --
//! --ignored` when touching the wire mapping.

use circulo_protocol::Part;
use circulo_protocol::{PermissionRequest, Question};
use circulo_provider::{Answer, Asker, Decision, Driver, SessionRef};
use circulo_provider_opencode::OpenCode;

/// A real turn that stops for approval has no user here. Refuse loudly instead
/// of hanging: the test prompt is a plain reply, so this should never happen.
struct NoAsker;

impl Asker for NoAsker {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        panic!("the agent asked for permission: {}", request.title);
    }

    fn question(&mut self, question: Question) -> Answer {
        panic!("the agent asked a question: {}", question.message);
    }
}

/// Lists and switches the real agent's config without prompting: no model
/// call, but it needs the installed CLI.
#[test]
#[ignore = "spawns the real opencode CLI"]
fn a_real_switch_applies_and_holds() {
    let driver = OpenCode::new(std::env::temp_dir());

    let options = driver.options("s1").expect("options");
    assert!(
        options.iter().any(|option| option.id == "model"),
        "the agent reports its models"
    );

    let options = driver.set_option("s1", "effort", "low").expect("set");
    let current = options
        .iter()
        .find(|option| option.id == "effort")
        .map(|option| option.current.as_str());
    assert_eq!(current, Some("low"));

    // A fresh session resets the agent; the driver re-applies the choice.
    let options = driver.options("s1").expect("options again");
    let current = options
        .iter()
        .find(|option| option.id == "effort")
        .map(|option| option.current.as_str());
    assert_eq!(current, Some("low"));
}

#[test]
#[ignore = "spawns the real opencode CLI and spends a model call"]
fn a_real_turn_streams_text() {
    let driver = OpenCode::new(std::env::temp_dir());

    let mut parts = Vec::new();
    driver
        .start_turn(
            SessionRef::new("s1"),
            "Reply with exactly: ready",
            &mut NoAsker,
            &mut |part| parts.push(part),
        )
        .expect("turn");

    let text: String = parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        text.to_lowercase().contains("ready"),
        "the agent answered: {text:?}"
    );
}

/// The answer to the agent's memory of a word planted one turn earlier.
fn recall(driver: &OpenCode, resume: Option<&str>) -> String {
    let mut parts = Vec::new();
    let key = driver
        .start_turn(
            SessionRef {
                id: "s1",
                resume,
                cwd: None,
                attachments: &[],
            },
            "Which fruit did I ask you to remember? Reply with exactly that one word.",
            &mut NoAsker,
            &mut |part| parts.push(part),
        )
        .expect("turn");

    let text: String = parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { delta } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert!(key.is_some(), "the driver keeps the session key");
    text
}

#[test]
#[ignore = "spawns the real opencode CLI and spends two model calls"]
fn a_real_session_remembers_across_turns() {
    let driver = OpenCode::new(std::env::temp_dir());

    let mut parts = Vec::new();
    driver
        .start_turn(
            SessionRef::new("s1"),
            "Remember this fruit for later: pineapple. Reply with exactly: noted.",
            &mut NoAsker,
            &mut |part| parts.push(part),
        )
        .expect("first turn");

    let answer = recall(&driver, None);
    assert!(
        answer.to_lowercase().contains("pineapple"),
        "the second turn knew the first: {answer:?}"
    );
}

#[test]
#[ignore = "spawns the real opencode CLI and spends two model calls"]
fn a_real_session_resumes_after_a_restart() {
    let key = {
        // The first life: plant the word, then drop the driver, which kills
        // the agent process — a restart in miniature.
        let driver = OpenCode::new(std::env::temp_dir());
        let mut parts = Vec::new();
        driver
            .start_turn(
                SessionRef::new("s1"),
                "Remember this fruit for later: pineapple. Reply with exactly: noted.",
                &mut NoAsker,
                &mut |part| parts.push(part),
            )
            .expect("first turn")
            .expect("the agent's session key")
    };

    let driver = OpenCode::new(std::env::temp_dir());
    let answer = recall(&driver, Some(&key));
    assert!(
        answer.to_lowercase().contains("pineapple"),
        "the loaded session remembered: {answer:?}"
    );
}
