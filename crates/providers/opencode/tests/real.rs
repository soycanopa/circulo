//! A real turn against the installed OpenCode CLI.
//!
//! Ignored by default: it spawns `opencode acp`, needs a logged-in provider and
//! spends a model call. Run it with `cargo test -p circulo-provider-opencode --
//! --ignored` when touching the wire mapping.

use circulo_protocol::Part;
use circulo_protocol::PermissionRequest;
use circulo_provider::{Asker, Decision, Driver};
use circulo_provider_opencode::OpenCode;

/// A real turn that stops for approval has no user here. Refuse loudly instead
/// of hanging: the test prompt is a plain reply, so this should never happen.
struct NoAsker;

impl Asker for NoAsker {
    fn permission(&mut self, request: PermissionRequest) -> Decision {
        panic!("the agent asked for permission: {}", request.title);
    }
}

#[test]
#[ignore = "spawns the real opencode CLI and spends a model call"]
fn a_real_turn_streams_text() {
    let driver = OpenCode::new(std::env::temp_dir());

    let mut parts = Vec::new();
    driver
        .start_turn("Reply with exactly: ready", &mut NoAsker, &mut |part| {
            parts.push(part)
        })
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
