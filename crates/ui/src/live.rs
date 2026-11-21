//! The live transcript: what the daemon has streamed for the open session.
//!
//! This is presentation state, not conversation storage. It is rebuilt from the
//! daemon's history when that exists; for now it grows from the messages of one
//! turn at a time and holds nothing else.

use std::collections::VecDeque;

use circulo_protocol::{
    FinishReason, Part, PermissionOption, QuestionField, ServerMessage, ToolState,
};

#[derive(Default)]
pub struct Transcript {
    pub turns: Vec<Turn>,
    pub status: Status,
    /// The text the window just sent. `TurnStarted` carries no prompt, so it is
    /// held here until the daemon reports the turn it belongs to.
    pending_prompt: Option<String>,
    /// Cards waiting for an answer, oldest first: a permission and a question
    /// share the slot above the composer, so one queue keeps their order.
    pub cards: VecDeque<Card>,
}

#[derive(Clone)]
pub enum Card {
    Permission(PendingPermission),
    Question(PendingQuestion),
}

impl Card {
    pub fn id(&self) -> &str {
        match self {
            Card::Permission(request) => &request.id,
            Card::Question(request) => &request.id,
        }
    }
}

/// One open question, as the agent described it.
#[derive(Clone)]
pub struct PendingQuestion {
    pub id: String,
    pub message: String,
    pub fields: Vec<QuestionField>,
}

/// One open permission request, as the daemon described it.
#[derive(Clone)]
pub struct PendingPermission {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub options: Vec<PermissionOption>,
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
    /// What the user sent, shown above the turn. Empty for a turn the window
    /// did not start.
    pub prompt: String,
    pub parts: Vec<Segment>,
}

/// One rendered piece of a turn, in the order it arrived. Text and reasoning
/// accumulate; tools keep their row and update it when they finish.
pub enum Segment {
    Reasoning(String),
    Tool {
        /// The provider's call id. Updates of a call find this row by it, even
        /// when another tool started in between; an empty id falls back to the
        /// last open row with the same name.
        id: String,
        name: String,
        detail: String,
        done: bool,
    },
    Text {
        text: String,
        /// The parsed markdown, rebuilt only when the text grows. It lives with
        /// the segment, so it cannot leak across sessions.
        parsed: crate::markdown::Cached,
    },
}

impl Transcript {
    pub fn apply(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::TurnStarted { .. } => {
                self.turns.push(Turn {
                    prompt: self.pending_prompt.take().unwrap_or_default(),
                    parts: Vec::new(),
                });
                self.status.running = true;
                self.status.error = None;
                self.status.finished = None;
            }
            ServerMessage::Part { part, .. } => {
                if let Some(turn) = self.turns.last_mut() {
                    turn.push(part);
                }
            }
            ServerMessage::PermissionRequested { id, request, .. } => {
                self.cards.push_back(Card::Permission(PendingPermission {
                    id,
                    title: request.title,
                    detail: request.detail,
                    options: request.options,
                }));
            }
            ServerMessage::QuestionAsked { id, question, .. } => {
                self.cards.push_back(Card::Question(PendingQuestion {
                    id,
                    message: question.message,
                    fields: question.fields,
                }));
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

    /// Remembers the prompt the window is about to send, so the turn can show
    /// it above the answer.
    pub fn expect_prompt(&mut self, prompt: String) {
        self.pending_prompt = Some(prompt);
    }
}

impl Turn {
    fn push(&mut self, part: Part) {
        match part {
            Part::Text { delta } => match self.parts.last_mut() {
                Some(Segment::Text { text, .. }) => text.push_str(&delta),
                _ => self.parts.push(Segment::Text {
                    text: delta,
                    parsed: crate::markdown::Cached::default(),
                }),
            },
            Part::Reasoning { delta } => match self.parts.last_mut() {
                Some(Segment::Reasoning(text)) => text.push_str(&delta),
                _ => self.parts.push(Segment::Reasoning(delta)),
            },
            Part::Tool {
                id,
                name,
                detail,
                state,
            } => {
                // A tool reports itself several times: pending, in progress,
                // then done. The call id finds the row even when another tool
                // started in between, which is the usual shape of a real turn.
                let open = self.parts.iter_mut().rev().find(|segment| match segment {
                    Segment::Tool {
                        id: open_id,
                        name: open_name,
                        done: false,
                        ..
                    } => {
                        if id.is_empty() {
                            *open_name == name
                        } else {
                            *open_id == id
                        }
                    }
                    _ => false,
                });

                if let Some(Segment::Tool {
                    name: open_name,
                    detail: open_detail,
                    done: open_done,
                    ..
                }) = open
                {
                    // The same call reports itself several times and only then
                    // knows its target; keep one row and let the detail fill in.
                    if !detail.is_empty() {
                        *open_detail = detail;
                    }
                    *open_name = name;
                    *open_done = state == ToolState::Done;
                    return;
                }

                self.parts.push(Segment::Tool {
                    id,
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
    fn a_sent_prompt_is_shown_on_the_turn_it_belongs_to() {
        let mut transcript = Transcript::default();
        transcript.expect_prompt("hello".into());
        transcript.apply(started(1));

        assert_eq!(transcript.turns[0].prompt, "hello");
        assert!(transcript.turns[0].parts.is_empty());
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
            Segment::Text { text, .. } => assert_eq!(text, "Hello world"),
            _ => panic!("expected text"),
        }
        assert!(transcript.status.running);
    }

    #[test]
    fn a_tool_row_updates_in_place() {
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(part(Part::Tool {
            id: "call-1".into(),
            name: "Read".into(),
            detail: String::new(),
            state: ToolState::Running,
        }));
        transcript.apply(part(Part::Tool {
            id: "call-1".into(),
            name: "Read".into(),
            detail: "src/lib.rs".into(),
            state: ToolState::Running,
        }));
        transcript.apply(part(Part::Tool {
            id: "call-1".into(),
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
    fn a_tool_update_finds_its_row_after_another_tool_starts() {
        // The shape of a real turn: call A opens, call B opens, then A finishes.
        // Matching only the last row would leave A spinning and open a duplicate
        // for it. That is the bug a real OpenCode run showed.
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(part(tool("call-a", "Read", "", ToolState::Running)));
        transcript.apply(part(tool(
            "call-b",
            "Bash",
            "cargo test",
            ToolState::Running,
        )));
        transcript.apply(part(tool("call-a", "Read", "src/lib.rs", ToolState::Done)));
        transcript.apply(part(tool("call-b", "Bash", "cargo test", ToolState::Done)));

        let parts = &transcript.turns[0].parts;
        assert_eq!(parts.len(), 2, "one row per call, not one per update");

        match &parts[0] {
            Segment::Tool {
                name, detail, done, ..
            } => {
                assert_eq!(name, "Read");
                assert_eq!(detail, "src/lib.rs");
                assert!(done, "the finished call leaves no spinner behind");
            }
            _ => panic!("expected a tool row"),
        }
        match &parts[1] {
            Segment::Tool { name, done, .. } => {
                assert_eq!(name, "Bash");
                assert!(done);
            }
            _ => panic!("expected a tool row"),
        }
    }

    #[test]
    fn a_tool_without_an_id_falls_back_to_the_last_open_row() {
        // A provider without call ids still reports the same call twice.
        let mut transcript = Transcript::default();
        transcript.apply(started(1));
        transcript.apply(part(tool("", "Read", "", ToolState::Running)));
        transcript.apply(part(tool("", "Read", "src/lib.rs", ToolState::Done)));

        let parts = &transcript.turns[0].parts;
        assert_eq!(parts.len(), 1);
        assert!(matches!(&parts[0], Segment::Tool { done: true, .. }));
    }

    fn tool(id: &str, name: &str, detail: &str, state: ToolState) -> Part {
        Part::Tool {
            id: id.into(),
            name: name.into(),
            detail: detail.into(),
            state,
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
