//! The Codex lifecycle against the installed CLI.
//!
//! These spend a model call and need a logged-in `codex`, so they stay behind
//! `--ignored`, the same way OpenCode's real tests do:
//!
//! ```sh
//! cargo test -p circulo-provider-codex --test real -- --ignored
//! ```

#![forbid(unsafe_code)]

use circulo_protocol::{Part, PermissionRequest, Question};
use circulo_provider::{Answer, Asker, Decision, Driver, SessionRef};
use circulo_provider_codex::Codex;

struct Refuse;

impl Asker for Refuse {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: Question) -> Answer {
        Answer::cancel()
    }
}

#[test]
#[ignore = "spawns the real codex CLI and spends a model call"]
fn a_real_turn_streams_and_remembers_across_turns() {
    let driver = Codex::new(std::env::current_dir().expect("cwd"));
    let mut ask = Refuse;
    let mut parts = Vec::new();

    let key = driver
        .start_turn(
            SessionRef::new("circulo-real-1"),
            "Reply with the single word: ok",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("turn");

    assert!(key.is_some(), "the driver returned a thread id");
    assert!(
        parts
            .iter()
            .any(|part| matches!(part, Part::Text { delta } if !delta.is_empty())),
        "the turn streamed text"
    );

    let mut parts = Vec::new();
    driver
        .start_turn(
            SessionRef {
                id: "circulo-real-1",
                resume: key.as_deref(),
                cwd: None,
            },
            "Reply with the single word: again",
            &mut ask,
            &mut |part| parts.push(part),
        )
        .expect("second turn");
}

#[test]
#[ignore = "needs a logged-in codex CLI"]
fn the_real_catalog_lists_models_and_efforts() {
    let driver = Codex::new(std::env::current_dir().expect("cwd"));
    let options = driver.options("circulo-real-2").expect("options");

    assert!(!options.is_empty(), "the agent reports at least one option");
    assert_eq!(options[0].id, "model");
}
