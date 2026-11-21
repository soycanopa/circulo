//! The live transcript: what the daemon has streamed for the open session.
//!
//! This is presentation state, not conversation storage. It is rebuilt from the
//! daemon's history when that exists; for now it grows from the messages of one
//! turn at a time and holds nothing else.

use circulo_protocol::{FinishReason, Part, ServerMessage, ToolState};

#[derive(Default)]
pub struct Transcript {
    pub turns: Vec<Turn>,
    pub status: Status,
}

#[derive(Default)]
pub struct Status {
    /// A turn is streaming.
    pub running: bool,
    /// The last turn ended for a reason other than success.
    pub error: Option<String>,
    /// How the last turn ended, once it has.
    pub finished: Option<FinishReason>,
}

#[derive(Default)]
pub struct Turn {
    pub parts: Vec<Segment>,
}

/// One rendered piece of a turn, in the order it arrived. Text and reasoning
/// accumulate; tools keep their row and update it when they finish.
pub enum Segment {
    Reasoning(String),
    Tool {
        name: String,
        detail: String,
        done: bool,
    },
    Text(String),
}

impl Transcript {
    pub fn apply(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::TurnStarted { .. } => {
                self.turns.push(Turn::default());
                self.status.running = true;
                self.status.error = None;
                self.status.finished = None;
            }
            ServerMessage::Part { part, .. } => {
                if let Some(turn) = self.turns.last_mut() {
                    turn.push(part);
                }
            }
            ServerMessage::TurnFinished { reason, .. } => {
                self.status.running = false;
                self.status.finished = Some(reason);
            }
            ServerMessage::Error { message } => {
                self.status.error = Some(message);
            }
            // Welcome and session messages are handled before a transcript
            // exists, or are not transcript content.
            ServerMessage::Welcome { .. } | ServerMessage::SessionStarted { .. } => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.turns.iter().all(|turn| turn.parts.is_empty())
    }
}

impl Turn {
    fn push(&mut self, part: Part) {
        match part {
            Part::Text { delta } => match self.parts.last_mut() {
                Some(Segment::Text(text)) => text.push_str(&delta),
                _ => self.parts.push(Segment::Text(delta)),
            },
            Part::Reasoning { delta } => match self.parts.last_mut() {
                Some(Segment::Reasoning(text)) => text.push_str(&delta),
                _ => self.parts.push(Segment::Reasoning(delta)),
            },
            Part::Tool {
                name,
                detail,
                state,
            } => {
                // A tool reports itself running and then done. That is the same
                // row, not a second one.
                if let Some(Segment::Tool {
                    name: open_name,
                    detail: open_detail,
                    done: open_done,
                }) = self.parts.last_mut()
                    && *open_name == name
                    && !*open_done
                {
                    // The same call reports itself several times and only then
                    // knows its target; keep one row and let the detail fill in.
                    *open_detail = detail;
                    *open_done = state == ToolState::Done;
                    return;
                }

                self.parts.push(Segment::Tool {
                    name,
                    detail,
                    done: state == ToolState::Done,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(turn: u64) -> ServerMessage {
        ServerMessage::TurnStarted {
            session: "s1".into(),
            turn,
        }
    }

    fn part(part: Part) -> ServerMessage {
        ServerMessage::Part {
            session: "s1".into(),
            turn: 1,
            part,
        }
    }

    fn text(delta: &str) -> ServerMessage {
        part(Part::Text {
            delta: delta.into(),
        })
    }

    #[test]
    fn deltas_accumulate_into_one_segment() {
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(text("Hello"));
        transcript.apply(text(" world"));

        let parts = &transcript.turns[0].parts;
        assert_eq!(parts.len(), 1);
        match &parts[0] {
            Segment::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("expected text"),
        }
        assert!(transcript.status.running);
    }

    #[test]
    fn a_tool_row_updates_in_place() {
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(part(Part::Tool {
            name: "Read".into(),
            detail: String::new(),
            state: ToolState::Running,
        }));
        transcript.apply(part(Part::Tool {
            name: "Read".into(),
            detail: "src/lib.rs".into(),
            state: ToolState::Running,
        }));
        transcript.apply(part(Part::Tool {
            name: "Read".into(),
            detail: "src/lib.rs".into(),
            state: ToolState::Done,
        }));

        let parts = &transcript.turns[0].parts;
        assert_eq!(parts.len(), 1, "all reports are the same row");
        match &parts[0] {
            Segment::Tool { done, detail, .. } => {
                assert!(done);
                assert_eq!(detail, "src/lib.rs", "the target fills in");
            }
            _ => panic!("expected a tool row"),
        }
    }

    #[test]
    fn finishing_closes_the_turn() {
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(text("done"));
        transcript.apply(ServerMessage::TurnFinished {
            session: "s1".into(),
            turn: 1,
            reason: FinishReason::Completed,
        });

        assert!(!transcript.status.running);
        assert_eq!(transcript.status.finished, Some(FinishReason::Completed));
    }

    #[test]
    fn an_error_is_kept_for_the_header() {
        let mut transcript = Transcript::default();
        transcript.apply(ServerMessage::Error {
            message: "no such session".into(),
        });

        assert_eq!(transcript.status.error.as_deref(), Some("no such session"));
    }
}
