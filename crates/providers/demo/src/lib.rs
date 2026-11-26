//! A provider that does nothing real: it streams a canned turn.
//!
//! It exists so the daemon, the protocol and the client can be proven end to
//! end before a real CLI is spawned. Its parts use the same shapes a real
//! provider produces — reasoning, a tool call, then text — so the transcript
//! renderer is exercised against realistic traffic.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use circulo_protocol::{ConfigChoice, ConfigOption, Part, ToolState};
use circulo_provider::{Asker, Driver, DriverError, SessionRef};

pub struct DemoDriver {
    chunk_delay: Duration,
    /// Choices the window made, re-applied to every turn the way a real
    /// provider re-applies what its agent forgets.
    overrides: Mutex<BTreeMap<String, String>>,
}

impl DemoDriver {
    /// A readable pace for a human watching the window.
    pub fn new() -> Self {
        Self {
            chunk_delay: Duration::from_millis(18),
            overrides: Mutex::new(BTreeMap::new()),
        }
    }

    /// No delay, for tests.
    pub fn instant() -> Self {
        Self {
            chunk_delay: Duration::ZERO,
            overrides: Mutex::new(BTreeMap::new()),
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

    fn start_turn(
        &self,
        _: SessionRef<'_>,
        prompt: &str,
        _: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<Option<String>, DriverError> {
        emit(Part::Reasoning {
            delta: "Reading the request and picking a shape for the answer.\n\n\
                    The thought arrives with paragraphs of its own, the way a real agent's \
                    does, and the collapsed row must stay one line."
                .into(),
        });

        emit(Part::Tool {
            id: "demo-read".into(),
            name: "Read".into(),
            detail: "crates/ui/src/transcript.rs".into(),
            state: ToolState::Running,
            output: None,
        });
        thread::sleep(self.chunk_delay);
        emit(Part::Tool {
            id: "demo-read".into(),
            name: "Read".into(),
            detail: "crates/ui/src/transcript.rs".into(),
            state: ToolState::Done,
            output: Some(
                "1  //! The transcript: header, and the rows the daemon streams.\n2  //!".into(),
            ),
        });
        thread::sleep(self.chunk_delay);
        emit(Part::Tool {
            id: "demo-bash".into(),
            name: "Bash".into(),
            detail: "cargo test --workspace\ncargo clippy --workspace --all-targets".into(),
            state: ToolState::Done,
            output: Some("test result: ok. 99 passed; 0 failed\nFinished `dev` profile".into()),
        });

        let answer = format!(
            "The daemon received: {prompt}\n\n\
             This turn travelled from the provider, through the core, over the protocol, and \
             into the client. The text is streamed in deltas, the reasoning and the tool call \
             are separate parts, and the turn ends with a finish reason.\n\n\
             ## What the transcript renders\n\n\
             - `**bold**` and `*italic*` become weight and slant\n\
             - `inline code` becomes a mono run on a chip\n\
             - fenced blocks keep their lines\n\n\
             ```rust\n\
             let part = Part::Text {{ delta }};\n\
             emit(part);\n\
             ```\n\n\
             > A quote is indented behind a rule.\n\n\
             That is the whole markdown subset for now."
        );

        for word in answer.split_inclusive(' ') {
            emit(Part::Text {
                delta: word.to_owned(),
            });
            thread::sleep(self.chunk_delay);
        }

        // The demo keeps no session of its own; every turn starts fresh.
        Ok(None)
    }

    fn options(&self, _: &str) -> Result<Vec<ConfigOption>, DriverError> {
        Ok(demo_options(&self.overrides()))
    }

    fn set_option(&self, _: &str, id: &str, value: &str) -> Result<Vec<ConfigOption>, DriverError> {
        let options = demo_options(&self.overrides());
        let option = options
            .iter()
            .find(|option| option.id == id)
            .ok_or_else(|| DriverError::new(format!("unknown option {id}")))?;
        if !option.options.iter().any(|choice| choice.id == value) {
            return Err(DriverError::new(format!("{value} is not a {id}")));
        }

        self.overrides().insert(id.to_owned(), value.to_owned());
        Ok(demo_options(&self.overrides()))
    }
}

impl DemoDriver {
    fn overrides(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.overrides
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The demo's two settings, with the window's choices applied. The ids match
/// the composer's pickers the way the real agent's categories do.
fn demo_options(overrides: &BTreeMap<String, String>) -> Vec<ConfigOption> {
    vec![
        ConfigOption {
            id: "model".into(),
            label: "Model".into(),
            current: overrides
                .get("model")
                .cloned()
                .unwrap_or_else(|| "demo-fast".into()),
            options: vec![
                ConfigChoice {
                    id: "demo-fast".into(),
                    label: "Demo Fast".into(),
                    provider: "demo".into(),
                },
                ConfigChoice {
                    id: "demo-deep".into(),
                    label: "Demo Deep".into(),
                    provider: "demo".into(),
                },
            ],
        },
        ConfigOption {
            id: "effort".into(),
            label: "Effort".into(),
            current: overrides
                .get("effort")
                .cloned()
                .unwrap_or_else(|| "medium".into()),
            options: vec![
                ConfigChoice {
                    id: "low".into(),
                    label: "Low".into(),
                    provider: String::new(),
                },
                ConfigChoice {
                    id: "medium".into(),
                    label: "Medium".into(),
                    provider: String::new(),
                },
                ConfigChoice {
                    id: "high".into(),
                    label: "High".into(),
                    provider: String::new(),
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_protocol::{PermissionRequest, Question};
    use circulo_provider::{Answer, Decision};

    #[test]
    fn a_turn_streams_reasoning_a_tool_and_text() {
        struct NoAsker;
        impl Asker for NoAsker {
            fn permission(&mut self, _: PermissionRequest) -> Decision {
                Decision::Cancelled
            }

            fn question(&mut self, _: Question) -> Answer {
                Answer::decline()
            }
        }

        let mut parts = Vec::new();
        DemoDriver::instant()
            .start_turn(SessionRef::new("s1"), "hello", &mut NoAsker, &mut |part| {
                parts.push(part)
            })
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
        // The canned turn is also the renderer's fixture: it must carry the
        // markdown shapes the transcript styles.
        assert!(text.contains("## What the transcript renders"));
        assert!(text.contains("```rust"));
        assert!(text.contains("- `inline code`"));
    }

    #[test]
    fn a_choice_survives_and_a_bad_one_is_refused() {
        let driver = DemoDriver::instant();

        let options = driver.options("s1").expect("options");
        assert_eq!(options.len(), 2);

        let options = driver.set_option("s1", "effort", "high").expect("set");
        let effort = options
            .iter()
            .find(|option| option.id == "effort")
            .expect("effort");
        assert_eq!(effort.current, "high");

        // The choice is still there on the next read: this is what lets a
        // per-turn process keep the user's pick.
        let options = driver.options("s1").expect("options again");
        let effort = options
            .iter()
            .find(|option| option.id == "effort")
            .expect("effort");
        assert_eq!(effort.current, "high");

        assert!(driver.set_option("s1", "effort", "ludicrous").is_err());
        assert!(driver.set_option("s1", "model", "high").is_err());
        assert!(driver.set_option("s1", "nope", "high").is_err());
    }
}
