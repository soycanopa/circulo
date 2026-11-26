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
//! One agent process per Circulo session: the driver keeps every session's
//! `opencode acp` child alive, so the agent remembers the conversation across
//! turns — history is no longer display-only. The child dies with the driver,
//! when the daemon winds down; a restarted daemon hands the stored session id
//! back to the agent (ACP `session/load`) instead of starting from nothing.
//!
//! Permissions: a `session/request_permission` becomes a card in the client.
//! The driver blocks on the answer, then tells the agent which option the user
//! chose; a client that goes away answers as cancelled.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

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
    /// Choices the user made, keyed by option id. They are the defaults for
    /// every session this driver opens, and the queue for a choice made while
    /// a turn holds the session.
    overrides: Mutex<BTreeMap<String, String>>,
    /// One live agent per Circulo session, keyed by the session's wire id.
    /// Each slot locks on its own: a long turn parks its session without
    /// parking anyone else's, and two turns on one session queue the way an
    /// agent requires — one prompt at a time.
    live: Mutex<HashMap<String, Arc<Mutex<Option<Attached>>>>>,
    /// What a cancel needs while a turn holds its slot: the child's stdin and
    /// the ACP session id. The slot itself is locked for the whole turn, so
    /// this is the handle that lets another thread send `session/cancel`.
    cancels: Mutex<HashMap<String, CancelHandle>>,
}

/// The agent side of one Circulo session: the process, its ACP session and
/// what it has reported so far.
struct Attached {
    agent: Agent,
    /// The ACP session id inside that process. It is also the key the core
    /// stores, so a restarted daemon can `session/load` it back.
    session: String,
    /// The options list from the last `session/new`, set or recall.
    options: Vec<ConfigOption>,
    /// The overrides this attach has already applied. A choice made after it
    /// — mid-turn — differs from this snapshot, and the end of the turn is
    /// when it lands: the next turn reads the refreshed session.
    applied: BTreeMap<String, String>,
}

/// What a cancel needs while the turn owns the slot.
#[derive(Clone)]
struct CancelHandle {
    stdin: Arc<Mutex<ChildStdin>>,
    /// The ACP session id inside that child.
    session: String,
}

impl CancelHandle {
    fn cancel(&self) -> Result<(), DriverError> {
        write_line(
            &self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "session/cancel",
                "params": {"sessionId": self.session},
            }),
        )
    }
}

impl OpenCode {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            binary: PathBuf::from("opencode"),
            overrides: Mutex::new(BTreeMap::new()),
            live: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
        }
    }

    /// Points the driver at another ACP agent: a pinned build, or the scripted
    /// agent a test uses to exercise the permission round trip.
    pub fn with_binary(cwd: impl Into<PathBuf>, binary: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            binary: binary.into(),
            overrides: Mutex::new(BTreeMap::new()),
            live: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
        }
    }

    fn overrides(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.overrides
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn live(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<Mutex<Option<Attached>>>>> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn cancels(&self) -> std::sync::MutexGuard<'_, HashMap<String, CancelHandle>> {
        self.cancels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The session's slot, created empty on first sight. An empty slot means
    /// "known, not attached": the next turn or listing attaches it.
    fn slot_for(&self, id: &str) -> Arc<Mutex<Option<Attached>>> {
        self.live()
            .entry(id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    /// The slot's contents, recovered if a turn panicked mid-way; `None` when
    /// nothing is attached or a turn holds the lock right now.
    fn peek(slot: &Mutex<Option<Attached>>) -> Option<std::sync::MutexGuard<'_, Option<Attached>>> {
        match slot.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Spawns the agent and opens its session, applying the user's choices.
    /// A stored key resumes the agent's own session — that is what makes a
    /// restarted daemon a continuation instead of a reset. A key the agent
    /// no longer knows starts a fresh conversation rather than failing the
    /// turn: the new key comes back, the store heals, the transcript stays.
    ///
    /// The session's project folder decides the agent's working directory; a
    /// projectless session uses the driver's configured fallback.
    fn attach(
        &self,
        resume: Option<&str>,
        cwd: Option<&str>,
        ask: &mut dyn Asker,
    ) -> Result<Attached, DriverError> {
        let cwd = cwd.map(PathBuf::from).unwrap_or_else(|| self.cwd.clone());
        let mut agent = Agent::spawn(&self.binary, &cwd)?;
        let session = match resume {
            Some(key) => match load_session(&mut agent, key, &cwd, ask) {
                Ok(session) => session,
                Err(_) => open_session(&mut agent, &cwd, ask)?,
            },
            None => open_session(&mut agent, &cwd, ask)?,
        };
        let applied = self.overrides().clone();
        let options = self.recall(&mut agent, &session, ask);

        Ok(Attached {
            agent,
            session,
            options,
            applied,
        })
    }

    /// Applies an override to whichever agent can take it right now. A live,
    /// idle session gets it directly; a busy one queues it for its turn's
    /// end; no session at all takes a short-lived one. Either way the choice
    /// is in `overrides`, which is what every future attach re-applies.
    fn apply_choice(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Vec<ConfigOption>, DriverError> {
        let slot = self.live().get(session).cloned();
        if let Some(slot) = &slot {
            if let Some(mut guard) = Self::peek(slot) {
                if let Some(live) = guard.as_mut() {
                    // A stale picker gets a clear error instead of a refused
                    // call, checked against what this agent last reported.
                    let option = option_in(&live.options, id)?;
                    check_choice(option, id, value)?;

                    let mut refuse = RefuseAll;
                    let result = live.agent.request(
                        "session/set_config_option",
                        json!({
                            "sessionId": live.session,
                            "configId": id,
                            "value": value,
                        }),
                        &mut refuse,
                    )?;
                    let options = config_options(&result);
                    live.options = options.clone();
                    live.applied.insert(id.to_owned(), value.to_owned());
                    self.overrides().insert(id.to_owned(), value.to_owned());
                    return Ok(options);
                }
            } else {
                // A turn holds the session. The choice lands when it ends —
                // the queued override differs from the applied snapshot, and
                // that is the recall trigger. Until then the picker shows
                // the value it asked for.
                let fresh = self.options(session)?;
                let option = option_in(&fresh, id)?;
                check_choice(option, id, value)?;
                self.overrides().insert(id.to_owned(), value.to_owned());

                let mut promised = fresh;
                if let Some(option) = promised.iter_mut().find(|option| option.id == id) {
                    option.current = value.to_owned();
                }
                return Ok(promised);
            }
        }

        // Nothing live: a short-lived agent takes the choice the way the
        // per-turn driver always did.
        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let mut refuse = RefuseAll;
        let session = open_session(&mut agent, &self.cwd, &mut refuse)?;

        let fresh = config_options(&agent.last_config);
        let option = option_in(&fresh, id)?;
        check_choice(option, id, value)?;

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

    /// Re-applies the user's choices to an agent session and returns the list
    /// the agent reports afterwards. A choice the agent refuses is dropped:
    /// the option is gone, and the rest still apply.
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
        session_ref: SessionRef<'_>,
        prompt: &str,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<Option<String>, DriverError> {
        let slot = self.slot_for(session_ref.id);

        // Held for the whole turn: the agent answers one prompt at a time,
        // and a second turn on this session waits instead of interleaving.
        let mut slot = slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.is_none() {
            let attached = self.attach(session_ref.resume, session_ref.cwd, ask)?;
            // The cancel handle points at the same child, so another thread
            // can ask the agent to stop while this turn holds the slot.
            self.cancels().insert(
                session_ref.id.to_owned(),
                CancelHandle {
                    stdin: Arc::clone(&attached.agent.stdin),
                    session: attached.session.clone(),
                },
            );
            *slot = Some(attached);
        }
        let attached = slot.as_mut().expect("attached just above");

        let mut tools = HashMap::new();
        let run = attached
            .agent
            .prompt(&attached.session, prompt, &mut tools, ask, emit);

        match run {
            Ok(()) => {
                // A choice made mid-turn is still queued; this is where it
                // lands, so the next turn reads the refreshed session.
                if *self.overrides() != attached.applied {
                    attached.options = self.recall(&mut attached.agent, &attached.session, ask);
                    attached.applied = self.overrides().clone();
                }
                Ok(Some(attached.session.clone()))
            }
            Err(error) => {
                // A dead process cannot serve the next turn; dropping it lets
                // the stored key bring the session back on the next one.
                if attached.agent.dead {
                    *slot = None;
                    self.cancels().remove(session_ref.id);
                }
                Err(error)
            }
        }
    }

    fn cancel(&self, session: &str) -> Result<(), DriverError> {
        let handle = self.cancels().get(session).cloned();
        let Some(handle) = handle else {
            return Err(DriverError::new(format!(
                "no agent is attached to session {session}"
            )));
        };
        handle.cancel()
    }

    fn reset(&self, session: &str) {
        // The next prompt must not continue the conversation whose files and
        // transcript were rewound. Dropping the slot kills the child; the core
        // refuses a rewind while a turn runs, so the lock is free.
        self.cancels().remove(session);
        let slot = self.live().remove(session);
        if let Some(slot) = slot {
            let mut attached = slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *attached = None;
        }
    }

    fn options(&self, session: &str) -> Result<Vec<ConfigOption>, DriverError> {
        let slot = self.live().get(session).cloned();
        if let Some(slot) = &slot
            && let Some(attached) = Self::peek(slot)
            && let Some(live) = attached.as_ref()
        {
            return Ok(live.options.clone());
        }

        // Nothing live: what a fresh session reports is what there is.
        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let mut refuse = RefuseAll;
        let session = open_session(&mut agent, &self.cwd, &mut refuse)?;
        Ok(self.recall(&mut agent, &session, &mut refuse))
    }

    fn set_option(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Vec<ConfigOption>, DriverError> {
        self.apply_choice(session, id, value)
    }

    fn shutdown(&self) {
        // Release every agent this driver holds. A slot whose turn is still
        // streaming keeps its Arc inside the turn thread and its child dies
        // when the turn ends — or with us, since the agent reads our stdin
        // and closes on EOF either way.
        self.cancels().clear();
        let mut live = self.live();
        for slot in live.values() {
            if let Ok(mut attached) = slot.try_lock() {
                *attached = None;
            }
        }
        live.clear();
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
    initialize(agent, ask)?;

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

fn initialize(agent: &mut Agent, ask: &mut dyn Asker) -> Result<(), DriverError> {
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
    Ok(())
}

/// Reattaches to a session the agent still remembers. The conversation is
/// replayed as `session/update` notifications before the response arrives;
/// `request` skips notifications, and rightly so — the transcript already
/// lives in the core's database, and drawing the replay again would double
/// it on screen. The response carries the session's config but, as probed
/// on OpenCode 2.3.2, no id: the loaded session is the one that was asked
/// for, so that is what comes back.
fn load_session(
    agent: &mut Agent,
    key: &str,
    cwd: &Path,
    ask: &mut dyn Asker,
) -> Result<String, DriverError> {
    initialize(agent, ask)?;

    let loaded = agent.request(
        "session/load",
        json!({
            "sessionId": key,
            "cwd": cwd.to_string_lossy(),
            "mcpServers": []
        }),
        ask,
    )?;
    agent.last_config = loaded;
    Ok(key.to_owned())
}

/// The agent's settings as `session/new` or `session/set_config_option`
/// reported them. Only `select` lists become options; anything else is a
/// setting this client has no control for, and is skipped the way unknown
/// updates are.
fn option_in<'a>(options: &'a [ConfigOption], id: &str) -> Result<&'a ConfigOption, DriverError> {
    options
        .iter()
        .find(|option| option.id == id)
        .ok_or_else(|| DriverError::new(format!("unknown option {id}")))
}

fn check_choice(option: &ConfigOption, id: &str, value: &str) -> Result<(), DriverError> {
    if option.options.iter().any(|choice| choice.id == value) {
        Ok(())
    } else {
        Err(DriverError::new(format!("{value} is not a {id}")))
    }
}

/// Who serves a choice, and what it is called without the vendor tag. ACP
/// reports models as `deepseek/deepseek-v4-pro` named
/// `deepseek/DeepSeek V4 Pro`; the picker wants `DeepSeek V4 Pro` under a
/// `DeepSeek` heading. The prefix only counts as the vendor when the label
/// repeats it — anything else is shown as the agent wrote it.
fn split_provider(id: &str, label: String) -> (String, String) {
    match id.split_once('/') {
        Some((vendor, _))
            if label.starts_with(vendor) && label[vendor.len()..].starts_with('/') =>
        {
            (vendor.to_owned(), label[vendor.len() + 1..].to_owned())
        }
        _ => (String::new(), label),
    }
}
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
                            let id = choice.get("value")?.as_str()?.to_owned();
                            let label = choice.get("name")?.as_str()?.to_owned();
                            let (provider, label) = split_provider(&id, label);
                            Some(ConfigChoice {
                                id,
                                label,
                                provider,
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
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    /// The last `session/new` result, so a caller can validate a choice
    /// against what the agent reported.
    last_config: Value,
    /// I/O with this process failed; it serves no further turns and its
    /// holder drops it rather than hand work to a corpse.
    dead: bool,
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
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: BufReader::new(stdout),
            next_id: 1,
            last_config: Value::Null,
            dead: false,
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
                    // The user asked for it through `session/cancel`; that is
                    // an abort, not a provider failure.
                    Some("cancelled") => Err(DriverError::cancelled("the user stopped the turn")),
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
        write_line(&self.stdin, &message).inspect_err(|_| self.dead = true)
    }

    fn receive(&mut self) -> Result<Value, DriverError> {
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).map_err(|error| {
                self.dead = true;
                DriverError::new(format!("opencode acp: {error}"))
            })?;

            if read == 0 {
                self.dead = true;
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

/// One JSON-RPC line on the child's stdin. The mutex is what lets a cancel
/// write while the turn thread is blocked reading the child's stdout; both
/// sides share the same handle.
fn write_line(stdin: &Mutex<ChildStdin>, message: &Value) -> Result<(), DriverError> {
    let mut line =
        serde_json::to_string(message).map_err(|error| DriverError::new(error.to_string()))?;
    line.push('\n');

    let mut stdin = stdin
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    stdin
        .write_all(line.as_bytes())
        .and_then(|()| stdin.flush())
        .map_err(|error| DriverError::new(format!("opencode acp: {error}")))
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

    #[test]
    fn a_model_arrives_without_its_vendor_tag() {
        assert_eq!(
            split_provider(
                "deepseek/deepseek-v4-pro",
                "deepseek/DeepSeek V4 Pro".into()
            ),
            ("deepseek".into(), "DeepSeek V4 Pro".into())
        );
        // The vendor prefix must repeat in the label, or nothing is stripped.
        assert_eq!(
            split_provider("deepseek/deepseek-v4-pro", "DeepSeek V4 Pro".into()),
            (String::new(), "DeepSeek V4 Pro".into())
        );
        // No vendor in the id at all.
        assert_eq!(
            split_provider("beam-free", "Beam Free".into()),
            (String::new(), "Beam Free".into())
        );
    }

    fn collect(updates: &[Value]) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut tools = HashMap::new();
        for update in updates {
            apply_update(update, &mut tools, &mut |part| parts.push(part));
        }
        parts
    }

    /// The real agent's load contract, without a model call: open a session,
    /// drop the process, and hand the stored id to a fresh one. What this
    /// pins down is the method name, the params and the response shape of
    /// `session/load` against the installed OpenCode.
    #[test]
    #[ignore = "spawns the real opencode CLI"]
    fn a_real_session_loads_without_a_model_call() {
        let cwd = std::env::temp_dir();
        let binary = std::path::PathBuf::from("opencode");

        let key = {
            let mut agent = Agent::spawn(&binary, &cwd).expect("spawn");
            let session = open_session(&mut agent, &cwd, &mut RefuseAll).expect("open");
            drop(agent);
            session
        };

        let mut agent = Agent::spawn(&binary, &cwd).expect("respawn");
        let loaded = load_session(&mut agent, &key, &cwd, &mut RefuseAll).expect("load");
        assert_eq!(loaded, key, "the agent answered with the same session");
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
