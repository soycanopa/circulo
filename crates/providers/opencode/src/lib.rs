//! The OpenCode provider.
//!
//! It speaks **Agent Client Protocol** to `opencode acp` over stdio: JSON-RPC
//! 2.0, one message per line. That is the agent-client surface the CLI ships for
//! editors and apps, so there is no port to open and no token to broker.
//!
//! Why not the v2 HTTP API: `opencode serve` requires pairing or a service
//! token, and its auth flow is not documented for third-party clients. ACP is
//! documented, structured and covers the same turn — verified against OpenCode
//! 2.3.2: `initialize`, `session/new`, `session/prompt`, with
//! `session/update` notifications carrying message chunks, reasoning chunks and
//! tool calls.
//!
//! A turn spawns one agent process. That is deliberate for the first slice: no
//! shared mutable process, no half-dead session to recover. Keeping one agent
//! per session (ACP `session/load`) lands when sessions are persisted.
//!
//! Permissions: a `session/request_permission` becomes a card in the client.
//! The driver blocks on the answer, then tells the agent which option the user
//! chose; a client that goes away answers as cancelled.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use circulo_protocol::{
    ConfigChoice, ConfigOption, Part, PermissionOption, PermissionOptionKind, PermissionRequest,
    Question, QuestionField, QuestionKind, ToolState,
};
use circulo_provider::{Asker, Decision, Driver, DriverError, SessionRef};
use serde_json::{Value, json};

/// ACP version this driver speaks. `initialize` returns the same number or the
/// agent refuses the client.
const ACP_VERSION: u64 = 1;

pub struct OpenCode {
    cwd: PathBuf,
    binary: PathBuf,
    /// Choices the user made, keyed by option id. A set only lasts for the
    /// agent session that received it — the probe showed a fresh `session/new`
    /// resets `effort` to `default` — so every new session re-applies these.
    overrides: Mutex<BTreeMap<String, String>>,
}

impl OpenCode {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            binary: PathBuf::from("opencode"),
            overrides: Mutex::new(BTreeMap::new()),
        }
    }

    /// Points the driver at another ACP agent: a pinned build, or the scripted
    /// agent a test uses to exercise the permission round trip.
    pub fn with_binary(cwd: impl Into<PathBuf>, binary: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            binary: binary.into(),
            overrides: Mutex::new(BTreeMap::new()),
        }
    }

    fn overrides(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.overrides
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Re-applies the user's choices to a fresh agent session and returns the
    /// list the agent reports afterwards. A choice the agent refuses is
    /// dropped: the option is gone, and the rest still apply.
    fn recall(&self, agent: &mut Agent, session: &str, ask: &mut dyn Asker) -> Vec<ConfigOption> {
        let mut options = config_options(&agent.last_config);
        let mut stored = self.overrides().clone();

        stored.retain(|id, value| {
            match agent.request(
                "session/set_config_option",
                json!({
                    "sessionId": session,
                    "configId": id,
                    "value": value,
                }),
                ask,
            ) {
                Ok(result) => {
                    options = config_options(&result);
                    true
                }
                Err(_) => false,
            }
        });

        *self.overrides() = stored;
        options
    }
}

impl Driver for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn start_turn(
        &self,
        _: SessionRef<'_>,
        prompt: &str,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<Option<String>, DriverError> {
        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let session = open_session(&mut agent, &self.cwd, ask)?;
        self.recall(&mut agent, &session, ask);

        let mut tools = HashMap::new();
        agent.prompt(&session, prompt, &mut tools, ask, emit)?;
        // Per-turn processes for now; the session key rides along when the
        // driver keeps its agents alive.
        Ok(None)
    }

    fn options(&self, _: &str) -> Result<Vec<ConfigOption>, DriverError> {
        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let mut refuse = RefuseAll;
        let session = open_session(&mut agent, &self.cwd, &mut refuse)?;
        Ok(self.recall(&mut agent, &session, &mut refuse))
    }

    fn set_option(&self, _: &str, id: &str, value: &str) -> Result<Vec<ConfigOption>, DriverError> {
        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let mut refuse = RefuseAll;
        let session = open_session(&mut agent, &self.cwd, &mut refuse)?;

        // Check against what the agent just reported, so a stale picker gets a
        // clear error instead of a refused call.
        let fresh = config_options(&agent.last_config);
        let option = fresh
            .iter()
            .find(|option| option.id == id)
            .ok_or_else(|| DriverError::new(format!("unknown option {id}")))?;
        if !option.options.iter().any(|choice| choice.id == value) {
            return Err(DriverError::new(format!("{value} is not a {id}")));
        }

        let result = agent.request(
            "session/set_config_option",
            json!({
                "sessionId": session,
                "configId": id,
                "value": value,
            }),
            &mut refuse,
        )?;
        let options = config_options(&result);
        self.overrides().insert(id.to_owned(), value.to_owned());
        Ok(options)
    }
}

/// The agent refused nothing: out-of-turn calls never prompt, so a permission
/// or elicitation request here means the agent is confused, and the turn it
/// belongs to does not exist to answer it.
struct RefuseAll;

impl Asker for RefuseAll {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: Question) -> circulo_provider::Answer {
        circulo_provider::Answer::cancel()
    }
}

/// `initialize` plus `session/new`, checked the way a turn checks them.
/// Returns the agent's session id and remembers the reported config list on
/// the agent for the caller to validate against.
fn open_session(agent: &mut Agent, cwd: &Path, ask: &mut dyn Asker) -> Result<String, DriverError> {
    let welcome = agent.request(
        "initialize",
        json!({
            "protocolVersion": ACP_VERSION,
            "clientCapabilities": {
                "fs": {"readTextFile": false, "writeTextFile": false},
                // Form questions are supported; an agent must not ask for
                // a mode the client has not advertised.
                "elicitation": {"form": {}}
            }
        }),
        ask,
    )?;

    let version = welcome.get("protocolVersion").and_then(Value::as_u64);
    if version != Some(ACP_VERSION) {
        return Err(DriverError::new(format!(
            "opencode speaks ACP version {version:?}, this driver speaks {ACP_VERSION}"
        )));
    }

    let session = agent.request(
        "session/new",
        json!({
            "cwd": cwd.to_string_lossy(),
            "mcpServers": []
        }),
        ask,
    )?;
    agent.last_config = session.clone();
    let session = session
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| DriverError::new("opencode did not return a session id"))?
        .to_owned();
    Ok(session)
}

/// The agent's settings as `session/new` or `session/set_config_option`
/// reported them. Only `select` lists become options; anything else is a
/// setting this client has no control for, and is skipped the way unknown
/// updates are.
fn config_options(value: &Value) -> Vec<ConfigOption> {
    value
        .get("configOptions")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    if option.get("type").and_then(Value::as_str) != Some("select") {
                        return None;
                    }
                    let choices = option
                        .get("options")
                        .and_then(Value::as_array)?
                        .iter()
                        .filter_map(|choice| {
                            Some(ConfigChoice {
                                id: choice.get("value")?.as_str()?.to_owned(),
                                label: choice.get("name")?.as_str()?.to_owned(),
                            })
                        })
                        .collect::<Vec<_>>();
                    Some(ConfigOption {
                        id: option.get("id")?.as_str()?.to_owned(),
                        label: option
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        current: option
                            .get("currentValue")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                        options: choices,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One `opencode acp` process: a JSON-RPC peer on stdin and stdout.
struct Agent {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    /// The last `session/new` result, so a caller can validate a choice
    /// against what the agent reported.
    last_config: Value,
}

impl Agent {
    fn spawn(binary: &Path, cwd: &Path) -> Result<Self, DriverError> {
        let mut child = Command::new(binary)
            .arg("acp")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // The agent's own diagnostics belong on our stderr.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                DriverError::new(format!("cannot start `{}`: {error}", binary.display()))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DriverError::new("opencode acp has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DriverError::new("opencode acp has no stdout"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            last_config: Value::Null,
        })
    }

    /// Sends a request and waits for its response, answering the agent's own
    /// requests in between.
    fn request(
        &mut self,
        method: &str,
        params: Value,
        ask: &mut dyn Asker,
    ) -> Result<Value, DriverError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;

        loop {
            let message = self.receive()?;

            if message.get("id").and_then(Value::as_u64) == Some(id) {
                return response(message);
            }

            if message.get("method").is_some() {
                self.answer(&message, ask)?;
            }
        }
    }

    /// Sends the prompt and streams its updates until the agent answers.
    fn prompt(
        &mut self,
        session: &str,
        text: &str,
        tools: &mut HashMap<String, Tool>,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<(), DriverError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "session/prompt",
            "params": {
                "sessionId": session,
                "prompt": [{"type": "text", "text": text}]
            }
        }))?;

        loop {
            let message = self.receive()?;

            if message.get("id").and_then(Value::as_u64) == Some(id) {
                let result = response(message)?;
                return match result.get("stopReason").and_then(Value::as_str) {
                    None | Some("end_turn") => Ok(()),
                    Some(reason) => Err(DriverError::new(format!("the agent stopped: {reason}"))),
                };
            }

            match message.get("method").and_then(Value::as_str) {
                Some("session/update") => {
                    if let Some(update) = message.pointer("/params/update") {
                        apply_update(update, tools, emit);
                    }
                }
                Some(_) => self.answer(&message, ask)?,
                None => {}
            }
        }
    }

    /// Replies to a request the agent made. A permission request becomes a
    /// card and blocks the turn until the user answers; anything else is
    /// refused as unsupported.
    fn answer(&mut self, message: &Value, ask: &mut dyn Asker) -> Result<(), DriverError> {
        let Some(id) = message.get("id") else {
            return Ok(());
        };

        if message.get("method").and_then(Value::as_str) == Some("elicitation/create") {
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            let mode = params.get("mode").and_then(Value::as_str);

            // Only form mode is advertised (`clientCapabilities.elicitation`).
            if mode != Some("form") {
                self.send(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32602, "message": "only form elicitation is supported"}
                }))?;
                return Ok(());
            }

            let answer = ask.question(question_from(&params));
            let result = match answer.action {
                circulo_protocol::QuestionAction::Accept => {
                    json!({"action": "accept", "content": answer.values})
                }
                circulo_protocol::QuestionAction::Decline => json!({"action": "decline"}),
                circulo_protocol::QuestionAction::Cancel => json!({"action": "cancel"}),
            };
            self.send(json!({"jsonrpc": "2.0", "id": id, "result": result}))?;
            return Ok(());
        }

        if message.get("method").and_then(Value::as_str) == Some("session/request_permission") {
            let request = permission_request(message.get("params"));

            // No options means there is nothing to choose; answering cancelled
            // is better than a card with no buttons.
            let outcome = if request.options.is_empty() {
                Decision::Cancelled
            } else {
                ask.permission(request)
            };

            let result = match outcome {
                Decision::Selected(option) => json!({
                    "outcome": {"outcome": "selected", "optionId": option}
                }),
                Decision::Cancelled => json!({"outcome": {"outcome": "cancelled"}}),
            };
            self.send(json!({"jsonrpc": "2.0", "id": id, "result": result}))?;
        } else {
            self.send(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "not supported by this client"}
            }))?;
        }

        Ok(())
    }

    fn send(&mut self, message: Value) -> Result<(), DriverError> {
        let mut line =
            serde_json::to_string(&message).map_err(|error| DriverError::new(error.to_string()))?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.flush())
            .map_err(|error| DriverError::new(format!("opencode acp: {error}")))
    }

    fn receive(&mut self) -> Result<Value, DriverError> {
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .map_err(|error| DriverError::new(format!("opencode acp: {error}")))?;

            if read == 0 {
                return Err(DriverError::new("opencode acp closed the connection"));
            }

            if line.trim().is_empty() {
                continue;
            }

            return serde_json::from_str(&line).map_err(|error| {
                DriverError::new(format!("opencode acp sent a bad message: {error}"))
            });
        }
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn response(message: Value) -> Result<Value, DriverError> {
    if let Some(error) = message.get("error") {
        let text = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(DriverError::new(text));
    }

    Ok(message.get("result").cloned().unwrap_or(Value::Null))
}

/// The card, built from the ACP request. The title is the agent's own wording;
/// the detail is the command or path in full, because the decision is about what
/// will actually run.
fn permission_request(params: Option<&Value>) -> PermissionRequest {
    let params = params.cloned().unwrap_or(Value::Null);
    let tool = params.get("toolCall").cloned().unwrap_or(Value::Null);

    let title = tool
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("The agent wants to run a tool")
        .to_owned();
    let detail = tool
        .pointer("/rawInput/command")
        .and_then(Value::as_str)
        .or_else(|| tool.pointer("/locations/0/path").and_then(Value::as_str))
        .or_else(|| tool.pointer("/rawInput/path").and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned();

    let options = params
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    Some(PermissionOption {
                        id: option.get("optionId").and_then(Value::as_str)?.to_owned(),
                        label: option.get("name").and_then(Value::as_str)?.to_owned(),
                        kind: permission_kind(option.get("kind").and_then(Value::as_str)),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    PermissionRequest {
        title,
        detail,
        options,
    }
}

/// A question from an ACP form elicitation. The schema is a flat object; each
/// property becomes one field, an enum becomes a choice, anything else needs a
/// text field this client does not have yet.
fn question_from(params: &Value) -> Question {
    let message = params
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("The agent is asking a question")
        .to_owned();

    let required: Vec<&str> = params
        .pointer("/requestedSchema/required")
        .and_then(Value::as_array)
        .map(|required| required.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let fields = params
        .pointer("/requestedSchema/properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .map(|(id, schema)| {
                    let label = schema
                        .get("title")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| humanize(id));

                    let options: Vec<String> = schema
                        .get("enum")
                        .and_then(Value::as_array)
                        .map(|options| {
                            options
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();

                    let kind = if options.is_empty() {
                        QuestionKind::Text
                    } else {
                        QuestionKind::Choice { options }
                    };

                    QuestionField {
                        id: id.clone(),
                        label,
                        kind,
                        required: required.contains(&id.as_str()),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Question { message, fields }
}

/// `code_style` becomes `Code style`: agents write schema property names, and
/// a card should not show a raw identifier.
fn humanize(id: &str) -> String {
    let mut label = String::with_capacity(id.len());
    for (index, word) in id
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .enumerate()
    {
        if index > 0 {
            label.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            label.extend(first.to_uppercase());
            label.push_str(chars.as_str());
        }
    }
    if label.is_empty() {
        id.to_owned()
    } else {
        label
    }
}

fn permission_kind(kind: Option<&str>) -> PermissionOptionKind {
    match kind {
        Some("allow_once") => PermissionOptionKind::AllowOnce,
        Some("allow_always") => PermissionOptionKind::AllowAlways,
        Some("reject_once") => PermissionOptionKind::RejectOnce,
        Some("reject_always") => PermissionOptionKind::RejectAlways,
        _ => PermissionOptionKind::Other,
    }
}

/// What we remember about a tool call so its updates keep the same row.
struct Tool {
    name: String,
    detail: String,
}

/// Maps one ACP `session/update` to zero or more protocol parts. Unknown update
/// kinds are ignored: ACP adds them over time.
fn apply_update(update: &Value, tools: &mut HashMap<String, Tool>, emit: &mut dyn FnMut(Part)) {
    match update.get("sessionUpdate").and_then(Value::as_str) {
        Some("agent_message_chunk") => {
            if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                emit(Part::Text {
                    delta: text.to_owned(),
                });
            }
        }
        Some("agent_thought_chunk") => {
            if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                emit(Part::Reasoning {
                    delta: text.to_owned(),
                });
            }
        }
        Some("tool_call" | "tool_call_update") => {
            let id = update
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let name = update
                .get("title")
                .and_then(Value::as_str)
                .or_else(|| update.get("name").and_then(Value::as_str))
                .unwrap_or("tool")
                .to_owned();
            let detail = update
                .pointer("/locations/0/path")
                .and_then(Value::as_str)
                .or_else(|| update.pointer("/rawInput/path").and_then(Value::as_str))
                .unwrap_or_default()
                .to_owned();

            let tool = tools.entry(id.clone()).or_insert_with(|| Tool {
                name: name.clone(),
                detail: detail.clone(),
            });
            if tool.detail.is_empty() && !detail.is_empty() {
                tool.detail = detail;
            }

            let status = update.get("status").and_then(Value::as_str);
            let state = match status {
                Some("completed" | "failed") => ToolState::Done,
                _ => ToolState::Running,
            };
            let detail = match status {
                Some("failed") => format!("{} · failed", tool.detail),
                _ => tool.detail.clone(),
            };

            emit(Part::Tool {
                id,
                name: tool.name.clone(),
                detail,
                state,
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(updates: &[Value]) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut tools = HashMap::new();
        for update in updates {
            apply_update(update, &mut tools, &mut |part| parts.push(part));
        }
        parts
    }

    #[test]
    fn text_and_reasoning_chunks_become_parts() {
        let parts = collect(&[
            json!({"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": "thinking"}}),
            json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "ready"}}),
        ]);

        assert_eq!(
            parts,
            vec![
                Part::Reasoning {
                    delta: "thinking".into()
                },
                Part::Text {
                    delta: "ready".into()
                }
            ]
        );
    }

    #[test]
    fn a_tool_call_and_its_updates_keep_one_row() {
        let parts = collect(&[
            json!({
                "sessionUpdate": "tool_call",
                "toolCallId": "call_1",
                "title": "read",
                "kind": "read",
                "status": "pending"
            }),
            json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "call_1",
                "status": "in_progress",
                "locations": [{"path": "/tmp/project"}]
            }),
            json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "call_1",
                "status": "completed",
                "locations": [{"path": "/tmp/project"}]
            }),
        ]);

        assert_eq!(parts.len(), 3);
        for part in &parts {
            match part {
                Part::Tool { id, name, .. } => {
                    assert_eq!(id, "call_1", "every update carries the call id");
                    assert_eq!(name, "read");
                }
                other => panic!("expected tool parts, got {other:?}"),
            }
        }
        // The first report has no target yet; the updates fill the same row.
        assert!(matches!(&parts[0], Part::Tool { detail, .. } if detail.is_empty()));
        assert!(
            matches!(&parts[1], Part::Tool { detail, .. } if detail == "/tmp/project"),
            "the path arrives with the update"
        );
        assert!(matches!(
            parts[0],
            Part::Tool {
                state: ToolState::Running,
                ..
            }
        ));
        assert!(matches!(
            parts[2],
            Part::Tool {
                state: ToolState::Done,
                ..
            }
        ));
    }

    #[test]
    fn a_failed_tool_says_so() {
        let parts = collect(&[
            json!({
                "sessionUpdate": "tool_call",
                "toolCallId": "call_2",
                "title": "bash",
                "rawInput": {"path": "cargo test"}
            }),
            json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "call_2",
                "status": "failed"
            }),
        ]);

        match &parts[1] {
            Part::Tool {
                state: ToolState::Done,
                detail,
                ..
            } => assert!(detail.ends_with("· failed")),
            other => panic!("expected a tool row, got {other:?}"),
        }
    }

    #[test]
    fn unknown_updates_are_ignored() {
        let parts = collect(&[
            json!({"sessionUpdate": "available_commands_update", "availableCommands": []}),
            json!({"sessionUpdate": "usage_update"}),
        ]);

        assert!(parts.is_empty());
    }
}
