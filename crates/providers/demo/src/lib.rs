//! A provider that does nothing real: it streams a canned turn.
//!
//! It exists so the daemon, the protocol and the client can be proven end to
//! end before a real CLI is spawned. Its parts use the same shapes a real
//! provider produces — reasoning, a tool call, then text — so the transcript
//! renderer is exercised against realistic traffic.

#![forbid(unsafe_code)]

use std::thread;
use std::time::Duration;

use circulo_protocol::{Part, ToolState};
use circulo_provider::{Driver, DriverError};

pub struct DemoDriver {
    chunk_delay: Duration,
}

impl DemoDriver {
    /// A readable pace for a human watching the window.
    pub fn new() -> Self {
        Self {
            chunk_delay: Duration::from_millis(18),
        }
    }

    /// No delay, for tests.
    pub fn instant() -> Self {
        Self {
            chunk_delay: Duration::ZERO,
        }
    }
}

impl Default for DemoDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for DemoDriver {
    fn name(&self) -> &'static str {
        "demo"
    }

    fn start_turn(&self, prompt: &str, emit: &mut dyn FnMut(Part)) -> Result<(), DriverError> {
        emit(Part::Reasoning {
            delta: "Reading the request and picking a shape for the answer.".into(),
        });

        emit(Part::Tool {
            name: "Read".into(),
            detail: "crates/ui/src/transcript.rs".into(),
            state: ToolState::Running,
        });
        thread::sleep(self.chunk_delay);
        emit(Part::Tool {
            name: "Read".into(),
            detail: "crates/ui/src/transcript.rs".into(),
            state: ToolState::Done,
        });

        let answer = format!(
            "The daemon received: {prompt}\n\nThis turn travelled from the provider, through the \
             core, over the protocol, and into the client. The text is streamed in deltas, the \
             reasoning and the tool call are separate parts, and the turn ends with a finish \
             reason."
        );

        for word in answer.split_inclusive(' ') {
            emit(Part::Text {
                delta: word.to_owned(),
            });
            thread::sleep(self.chunk_delay);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_streams_reasoning_a_tool_and_text() {
        let mut parts = Vec::new();
        DemoDriver::instant()
            .start_turn("hello", &mut |part| parts.push(part))
            .expect("turn");

        assert!(matches!(parts.first(), Some(Part::Reasoning { .. })));
        assert!(parts.iter().any(|part| matches!(
            part,
            Part::Tool {
                state: ToolState::Running,
                ..
            }
        )));
        assert!(parts.iter().any(|part| matches!(
            part,
            Part::Tool {
                state: ToolState::Done,
                ..
            }
        )));

        let text: String = parts
            .iter()
            .filter_map(|part| match part {
                Part::Text { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert!(text.contains("hello"), "the prompt is acknowledged");
        assert!(text.contains("streamed in deltas"));
    }
}
