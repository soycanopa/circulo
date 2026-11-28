//! The Codex provider.
//!
//! It speaks the **Codex app-server** protocol: JSON-RPC 2.0 over stdin and
//! stdout, one message per line, with the `jsonrpc` header omitted on the wire.
//! The app server is the surface Codex ships for rich clients (its own VS Code
//! extension uses it), and it is the only documented one that carries
//! approvals, interruption and thread resume. `codex exec --json` was the other
//! candidate: a one-shot batch stream with no client-side approval and no
//! interrupt, so it cannot answer a card or stop a turn.
//!
//! One `codex app-server` child per Circulo session keeps the thread live
//! across turns. The thread id is the continuity key the core stores; after a
//! restart `thread/resume` reopens it, and a key the agent no longer knows
//! starts a fresh thread instead of failing the turn.
//!
//! Approvals arrive as server-initiated JSON-RPC requests
//! (`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`)
//! and become permission cards; the answer goes back as the documented
//! `{"decision": …}` payload. Questions arrive as
//! `mcpServer/elicitation/request` forms, which need the documented
//! `experimentalApi` capability.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use circulo_protocol::{
    ConfigChoice, ConfigOption, Part, PermissionOption, PermissionOptionKind, PermissionRequest,
    Question, QuestionField, QuestionKind, ToolState, Usage,
};
use circulo_provider::{Answer, Asker, Attachment, Decision, Driver, DriverError, SessionRef};
use serde_json::{Value, json};

/// The agent works in the project and asks before it leaves it. `never` would
/// silence the cards, which are the product's permission model.
const APPROVAL_POLICY: &str = "on-request";
/// Writes inside the session's working directory; anything wider needs a
/// decision from the user.
const SANDBOX: &str = "workspace-write";
const SERVICE_NAME: &str = "circulo";

/// Interrupt requests need an id of their own: `cancel` runs on another thread
/// while the turn thread owns the agent, so it cannot allocate from that
/// connection's counter.
static CANCEL_ID: AtomicU64 = AtomicU64::new(1_000_000);

pub struct Codex {
    cwd: PathBuf,
    binary: PathBuf,
    /// Choices the user made, keyed by option id. The next turn carries them.
    overrides: Mutex<BTreeMap<String, String>>,
    /// One live app server per Circulo session.
    live: Mutex<HashMap<String, Arc<Mutex<Option<Attached>>>>>,
    /// What a cancel needs while a turn holds its slot.
    cancels: Mutex<HashMap<String, CancelHandle>>,
    /// The last `model/list` a connection reported. Kept so a mid-turn choice
    /// can refresh the options without spawning another agent.
    catalog: Mutex<Option<Vec<ModelInfo>>>,
}

/// The agent side of one Circulo session.
struct Attached {
    agent: Agent,
    /// The app-server thread id. It is also the key the core stores.
    thread: String,
    /// The options list the last connection reported.
    options: Vec<ConfigOption>,
}

/// What a cancel needs while the turn owns the slot.
#[derive(Clone)]
struct CancelHandle {
    stdin: Arc<Mutex<ChildStdin>>,
    thread: String,
    /// The turn currently running, once `turn/start` answered.
    turn: Arc<Mutex<Option<String>>>,
    /// A cancel arrived before the turn id did.
    requested: Arc<AtomicBool>,
}

impl CancelHandle {
    fn cancel(&self) -> Result<(), DriverError> {
        self.requested.store(true, Ordering::SeqCst);
        let turn = self
            .turn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        // No id yet: the turn thread sends the interrupt as soon as the
        // `turn/start` response arrives.
        let Some(turn) = turn else {
            return Ok(());
        };

        let id = CANCEL_ID.fetch_add(1, Ordering::Relaxed);
        write_line(
            &self.stdin,
            &json!({
                "id": id,
                "method": "turn/interrupt",
                "params": {"threadId": self.thread, "turnId": turn},
            }),
        )
    }
}

/// One model from `model/list`, reduced to what the pickers need.
#[derive(Clone)]
struct ModelInfo {
    id: String,
    label: String,
    is_default: bool,
    default_effort: String,
    efforts: Vec<String>,
}

/// What opening or resuming a thread reported back.
struct Opened {
    thread: String,
    model: Option<String>,
    effort: Option<String>,
}

impl Codex {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self::with_binary(cwd, "codex")
    }

    /// Points the driver at another binary: a pinned build, or the scripted
    /// agent the tests run.
    pub fn with_binary(cwd: impl Into<PathBuf>, binary: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            binary: binary.into(),
            overrides: Mutex::new(BTreeMap::new()),
            live: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            catalog: Mutex::new(None),
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

    fn slot_for(&self, id: &str) -> Arc<Mutex<Option<Attached>>> {
        self.live()
            .entry(id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    /// The slot's contents, recovered if a turn panicked mid-way; `None` when
    /// nothing is attached or a turn holds the lock.
    fn peek(slot: &Mutex<Option<Attached>>) -> Option<std::sync::MutexGuard<'_, Option<Attached>>> {
        match slot.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    fn cached_catalog(&self) -> Option<Vec<ModelInfo>> {
        self.catalog
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The models this connection reports. A fresh connection asks a
    /// short-lived app server; the result is cached for the pickers.
    fn catalog(&self) -> Result<Vec<ModelInfo>, DriverError> {
        if let Some(models) = self.cached_catalog() {
            return Ok(models);
        }

        let mut agent = Agent::spawn(&self.binary, &self.cwd)?;
        let mut refuse = RefuseAll;
        handshake(&mut agent, &mut refuse)?;
        let models = read_catalog(&mut agent, &mut refuse)?;
        self.remember(&models);
        Ok(models)
    }

    fn remember(&self, models: &[ModelInfo]) {
        *self
            .catalog
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(models.to_vec());
    }

    /// The two select options the agent supports: model and effort. Effort
    /// follows the selected model, and a stored effort the model does not
    /// support is not shown.
    fn options_for(&self, models: &[ModelInfo]) -> Vec<ConfigOption> {
        let overrides = self.overrides().clone();
        let Some(selected) = current_model(models, &overrides) else {
            return Vec::new();
        };

        let effort = overrides
            .get("effort")
            .filter(|value| selected.efforts.iter().any(|option| option == *value))
            .cloned()
            .unwrap_or_else(|| selected.default_effort.clone());

        let mut options = vec![ConfigOption {
            id: "model".to_owned(),
            label: "Model".to_owned(),
            current: selected.id.clone(),
            options: models
                .iter()
                .map(|model| ConfigChoice {
                    id: model.id.clone(),
                    label: model.label.clone(),
                    provider: String::new(),
                })
                .collect(),
        }];

        if !selected.efforts.is_empty() {
            options.push(ConfigOption {
                id: "effort".to_owned(),
                label: "Effort".to_owned(),
                current: effort,
                options: selected
                    .efforts
                    .iter()
                    .map(|effort| ConfigChoice {
                        id: effort.clone(),
                        label: humanize(effort),
                        provider: String::new(),
                    })
                    .collect(),
            });
        }

        options
    }

    /// Spawns the agent for a session, opens or resumes its thread, and reads
    /// the options the connection reports.
    fn attach(
        &self,
        resume: Option<&str>,
        cwd: Option<&str>,
        ask: &mut dyn Asker,
    ) -> Result<Attached, DriverError> {
        let cwd = cwd.map(PathBuf::from).unwrap_or_else(|| self.cwd.clone());
        let mut agent = Agent::spawn(&self.binary, &cwd)?;
        agent.model_override = self.overrides().get("model").cloned();
        handshake(&mut agent, ask)?;

        let opened = match resume {
            Some(key) => match resume_thread(&mut agent, key, &cwd, ask) {
                Ok(opened) => opened,
                // A key the agent no longer knows starts a fresh conversation;
                // the new id comes back and heals the store.
                Err(_) => start_thread(&mut agent, &cwd, ask)?,
            },
            None => start_thread(&mut agent, &cwd, ask)?,
        };

        let models = read_catalog(&mut agent, ask).unwrap_or_default();
        if !models.is_empty() {
            self.remember(&models);

            // The picker should show what this thread really uses, not the
            // catalog default. The reported values become the stored choice
            // only when the user has not made one already.
            if let Some(reported) = &opened.model
                && models.iter().any(|model| &model.id == reported)
            {
                let mut overrides = self.overrides();
                overrides
                    .entry("model".to_owned())
                    .or_insert_with(|| reported.clone());
                if let Some(effort) = &opened.effort
                    && models
                        .iter()
                        .find(|model| &model.id == reported)
                        .is_some_and(|model| model.efforts.iter().any(|option| option == effort))
                {
                    overrides
                        .entry("effort".to_owned())
                        .or_insert_with(|| effort.clone());
                }
            }
        }
        let options = self.options_for(&models);

        Ok(Attached {
            agent,
            thread: opened.thread,
            options,
        })
    }
}

impl Driver for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn start_turn(
        &self,
        session_ref: SessionRef<'_>,
        prompt: &str,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<Option<String>, DriverError> {
        let slot = self.slot_for(session_ref.id);
        let input = turn_input(prompt, session_ref.attachments)?;

        // Held for the whole turn: the agent answers one prompt at a time.
        let mut slot = slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if slot.is_none() {
            let attached = self.attach(session_ref.resume, session_ref.cwd, ask)?;
            self.cancels().insert(
                session_ref.id.to_owned(),
                CancelHandle {
                    stdin: Arc::clone(&attached.agent.stdin),
                    thread: attached.thread.clone(),
                    turn: Arc::new(Mutex::new(None)),
                    requested: Arc::new(AtomicBool::new(false)),
                },
            );
            *slot = Some(attached);
        }

        let handle = self.cancels().get(session_ref.id).cloned();
        let attached = slot.as_mut().expect("attached just above");
        let overrides = self.overrides().clone();

        let run = attached.agent.run_turn(
            &attached.thread,
            input,
            &overrides,
            handle.as_ref(),
            ask,
            emit,
        );

        if let Some(handle) = &handle {
            *handle
                .turn
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }

        match run {
            Ok(()) => {
                // A choice made mid-turn lands at the turn's end, so the next
                // `options` call (and the next turn) sees it.
                if let Some(models) = self.cached_catalog() {
                    attached.options = self.options_for(&models);
                }
                Ok(Some(attached.thread.clone()))
            }
            Err(error) => {
                // A dead process cannot serve the next turn; dropping it lets
                // the stored key bring the thread back on the next one.
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
        if let Some(slot) = self.live().get(session).cloned()
            && let Some(attached) = Self::peek(&slot)
            && let Some(live) = attached.as_ref()
        {
            return Ok(live.options.clone());
        }

        let models = self.catalog()?;
        Ok(self.options_for(&models))
    }

    fn set_option(
        &self,
        session: &str,
        id: &str,
        value: &str,
    ) -> Result<Vec<ConfigOption>, DriverError> {
        let models = self.catalog()?;

        {
            let mut overrides = self.overrides();
            match id {
                "model" => {
                    let model = models
                        .iter()
                        .find(|model| model.id == value)
                        .ok_or_else(|| DriverError::new(format!("unknown model {value}")))?;
                    overrides.insert(id.to_owned(), value.to_owned());
                    // A stale effort the new model does not support would be a
                    // picker value the agent refuses; drop it instead.
                    if let Some(effort) = overrides.get("effort")
                        && !model.efforts.iter().any(|option| option == effort)
                    {
                        overrides.remove("effort");
                    }
                }
                "effort" => {
                    let Some(model) = current_model(&models, &overrides) else {
                        return Err(DriverError::new("this agent reports no model"));
                    };
                    if !model.efforts.iter().any(|option| option == value) {
                        return Err(DriverError::new(format!(
                            "{value} is not an effort of {}",
                            model.id
                        )));
                    }
                    overrides.insert(id.to_owned(), value.to_owned());
                }
                other => return Err(DriverError::new(format!("unknown option {other}"))),
            }
        }

        let options = self.options_for(&models);

        // A live, idle session shows the promised value at once; the turn
        // itself reads it from the overrides.
        if let Some(slot) = self.live().get(session).cloned()
            && let Some(mut attached) = Self::peek(&slot)
            && let Some(live) = attached.as_mut()
        {
            live.options = options.clone();
        }

        Ok(options)
    }

    fn shutdown(&self) {
        // A slot whose turn is still streaming keeps its Arc inside the turn
        // thread; its child dies when the turn ends, or with us anyway: the
        // app server exits on stdin EOF.
        self.cancels().clear();
        let mut live = self.live();
        for slot in live.values() {
            if let Ok(mut attached) = slot.try_lock() {
                *attached = None;
            }
        }
        live.clear();
    }

    fn usage(&self, session: &str) -> Option<Usage> {
        let slot = self.live().get(session).cloned()?;
        let attached = Self::peek(&slot)?;
        attached.as_ref().and_then(|live| live.agent.usage.clone())
    }
}

/// Out-of-turn calls never prompt, so a request here belongs to no turn that
/// could answer it.
struct RefuseAll;

impl Asker for RefuseAll {
    fn permission(&mut self, _: PermissionRequest) -> Decision {
        Decision::Cancelled
    }

    fn question(&mut self, _: Question) -> Answer {
        Answer::cancel()
    }
}

fn current_model<'a>(
    models: &'a [ModelInfo],
    overrides: &BTreeMap<String, String>,
) -> Option<&'a ModelInfo> {
    overrides
        .get("model")
        .and_then(|id| models.iter().find(|model| &model.id == id))
        .or_else(|| models.iter().find(|model| model.is_default))
        .or_else(|| models.first())
}

/// `initialize` plus `initialized`, the app server's mandatory handshake.
fn handshake(agent: &mut Agent, ask: &mut dyn Asker) -> Result<(), DriverError> {
    agent.request(
        "initialize",
        json!({
            "clientInfo": {
                "name": "circulo",
                "title": "Circulo",
                "version": env!("CARGO_PKG_VERSION"),
            },
            // Only for the documented `mcpServer/elicitation/request`: it is
            // how Codex asks the client a question. No experimental method is
            // called by this driver.
            "capabilities": {"experimentalApi": true},
        }),
        ask,
    )?;
    agent.notify("initialized", json!({}))?;
    Ok(())
}

fn start_thread(agent: &mut Agent, cwd: &Path, ask: &mut dyn Asker) -> Result<Opened, DriverError> {
    let mut params = json!({
        "cwd": cwd.to_string_lossy(),
        "approvalPolicy": APPROVAL_POLICY,
        "sandbox": SANDBOX,
        "serviceName": SERVICE_NAME,
    });
    let model = agent.model_override.clone();
    if let Some(model) = model {
        params["model"] = json!(model);
    }

    let result = agent.request("thread/start", params, ask)?;
    opened(&result, "codex did not return a thread id")
}

fn resume_thread(
    agent: &mut Agent,
    key: &str,
    cwd: &Path,
    ask: &mut dyn Asker,
) -> Result<Opened, DriverError> {
    let mut params = json!({
        "threadId": key,
        // The transcript already lives in SQLite; replaying the thread's
        // turns would only cost time.
        "excludeTurns": true,
        "cwd": cwd.to_string_lossy(),
        "approvalPolicy": APPROVAL_POLICY,
        "sandbox": SANDBOX,
    });
    let model = agent.model_override.clone();
    if let Some(model) = model {
        params["model"] = json!(model);
    }

    let result = agent.request("thread/resume", params, ask)?;
    opened(&result, "codex did not return the resumed thread")
}

fn opened(result: &Value, missing: &str) -> Result<Opened, DriverError> {
    Ok(Opened {
        thread: thread_id(result, missing)?,
        model: result
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_owned),
        effort: result
            .get("reasoningEffort")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

/// Codex accepts text and local images in a turn. An image becomes
/// `localImage`; a text file is inlined (capped) because the protocol has no
/// file-content input. The blob path is local to the daemon, where the agent
/// runs.
const MAX_INLINE_TEXT: usize = 256 * 1024;
const TEXT_TRUNCATED: &str = "\n… [truncated]";

fn turn_input(prompt: &str, attachments: &[Attachment]) -> Result<Vec<Value>, DriverError> {
    let mut input = vec![json!({"type": "text", "text": prompt})];

    for attachment in attachments {
        if attachment.mime.starts_with("image/") {
            input.push(json!({
                "type": "localImage",
                "path": attachment.path.to_string_lossy(),
            }));
        } else {
            let bytes = std::fs::read(&attachment.path).map_err(|error| {
                DriverError::new(format!("cannot read {}: {error}", attachment.name))
            })?;
            input.push(json!({
                "type": "text",
                "text": format!(
                    "Attached file: {}\n```\n{}\n```",
                    attachment.name,
                    inline_text(&bytes)
                ),
            }));
        }
    }

    Ok(input)
}

fn inline_text(bytes: &[u8]) -> String {
    let end = bytes.len().min(MAX_INLINE_TEXT);
    let mut text = String::from_utf8_lossy(&bytes[..end]).into_owned();
    if end < bytes.len() {
        text.push_str(TEXT_TRUNCATED);
    }
    text
}

/// Codex reports cumulative token counts, not the current context: `total` is
/// the whole session, `last` is one turn. Showing a context ratio from either
/// would lie, so the gauge shows the cumulative tokens it really knows.
fn usage_from(params: &Value) -> Option<Usage> {
    let total = params.pointer("/tokenUsage/total")?;

    Some(Usage {
        total_tokens: token(total, "totalTokens"),
        input_tokens: token(total, "inputTokens"),
        output_tokens: token(total, "outputTokens"),
        cached_tokens: token(total, "cachedInputTokens"),
        ..Usage::default()
    })
}

fn token(value: &Value, key: &str) -> Option<u64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|count| u64::try_from(count).ok())
}

fn thread_id(result: &Value, missing: &str) -> Result<String, DriverError> {
    result
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| DriverError::new(missing))
}

fn read_catalog(agent: &mut Agent, ask: &mut dyn Asker) -> Result<Vec<ModelInfo>, DriverError> {
    let result = agent.request(
        "model/list",
        json!({"limit": 100, "includeHidden": false}),
        ask,
    )?;
    Ok(models_from(&result))
}

fn models_from(result: &Value) -> Vec<ModelInfo> {
    result
        .get("data")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter(|model| model.get("hidden").and_then(Value::as_bool) != Some(true))
                .filter_map(|model| {
                    let id = model.get("id").and_then(Value::as_str)?.to_owned();
                    let label = model
                        .get("displayName")
                        .and_then(Value::as_str)
                        .unwrap_or(&id)
                        .to_owned();
                    let efforts: Vec<String> = model
                        .get("supportedReasoningEfforts")
                        .and_then(Value::as_array)
                        .map(|efforts| {
                            efforts
                                .iter()
                                .filter_map(|effort| {
                                    effort
                                        .get("reasoningEffort")
                                        .and_then(Value::as_str)
                                        .map(str::to_owned)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let default_effort = model
                        .get("defaultReasoningEffort")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| efforts.first().cloned())
                        .unwrap_or_default();

                    Some(ModelInfo {
                        id,
                        label,
                        is_default: model
                            .get("isDefault")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        default_effort,
                        efforts,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One `codex app-server` process: a JSON-RPC peer on stdin and stdout.
struct Agent {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    /// The model choice a thread should start or resume with.
    model_override: Option<String>,
    /// I/O with this process failed; it serves no further turns.
    dead: bool,
    /// The latest `thread/tokenUsage/updated` this connection sent.
    usage: Option<Usage>,
}

impl Agent {
    fn spawn(binary: &Path, cwd: &Path) -> Result<Self, DriverError> {
        let mut child = Command::new(binary)
            .arg("app-server")
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
            .ok_or_else(|| DriverError::new("codex app-server has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DriverError::new("codex app-server has no stdout"))?;

        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: BufReader::new(stdout),
            next_id: 1,
            model_override: None,
            dead: false,
            usage: None,
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
        self.send(json!({"id": id, "method": method, "params": params}))?;

        loop {
            let message = self.receive()?;

            // A server request has both an id and a method; a response has
            // only the id.
            if message.get("method").is_some() {
                if message.get("id").is_some() {
                    self.answer(&message, ask)?;
                }
                continue;
            }

            if message.get("id").and_then(Value::as_u64) == Some(id) {
                return response(message);
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), DriverError> {
        self.send(json!({"method": method, "params": params}))
    }

    /// Runs one turn and streams its notifications until `turn/completed`.
    fn run_turn(
        &mut self,
        thread: &str,
        input: Vec<Value>,
        overrides: &BTreeMap<String, String>,
        handle: Option<&CancelHandle>,
        ask: &mut dyn Asker,
        emit: &mut dyn FnMut(Part),
    ) -> Result<(), DriverError> {
        let id = self.next_id;
        self.next_id += 1;

        let mut params = json!({
            "threadId": thread,
            "input": input,
        });
        if let Some(model) = overrides.get("model") {
            params["model"] = json!(model);
        }
        if let Some(effort) = overrides.get("effort") {
            params["effort"] = json!(effort);
        }
        self.send(json!({"id": id, "method": "turn/start", "params": params}))?;

        let mut turn_id: Option<String> = None;
        let mut tools = HashMap::new();
        let mut last_error: Option<String> = None;

        loop {
            let message = self.receive()?;

            let Some(method) = message
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_owned)
            else {
                // A response. The one this turn waits for carries the turn id;
                // interrupt responses and stale ids are ignored.
                if message.get("id").and_then(Value::as_u64) == Some(id) {
                    let result = response(message)?;
                    let returned = result
                        .pointer("/turn/id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| DriverError::new("codex did not return a turn id"))?;
                    if turn_id.is_none() {
                        turn_id = Some(returned.to_owned());
                        register_turn(self, handle, returned)?;
                    }
                }
                continue;
            };

            if message.get("id").is_some() {
                self.answer(&message, ask)?;
                continue;
            }

            let params = message.get("params").cloned().unwrap_or(Value::Null);
            if !for_thread(&params, thread) {
                continue;
            }

            match method.as_str() {
                "turn/started" => {
                    if let Some(started) = params.pointer("/turn/id").and_then(Value::as_str)
                        && turn_id.is_none()
                    {
                        turn_id = Some(started.to_owned());
                        register_turn(self, handle, started)?;
                    }
                }
                "item/agentMessage/delta" => {
                    if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                        emit(Part::Text {
                            delta: delta.to_owned(),
                        });
                    }
                }
                "item/reasoning/summaryTextDelta" => {
                    if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                        emit(Part::Reasoning {
                            delta: delta.to_owned(),
                        });
                    }
                }
                "item/commandExecution/outputDelta" => {
                    if let (Some(item_id), Some(delta)) = (
                        params.get("itemId").and_then(Value::as_str),
                        params.get("delta").and_then(Value::as_str),
                    ) {
                        let tool = tools.entry(item_id.to_owned()).or_insert_with(|| Tool {
                            name: "command".to_owned(),
                            detail: String::new(),
                            output: String::new(),
                        });
                        append_output(&mut tool.output, delta);
                    }
                }
                "item/started" => {
                    if let Some(item) = params.get("item") {
                        apply_item(item, false, &mut tools, emit);
                    }
                }
                "item/completed" => {
                    if let Some(item) = params.get("item") {
                        apply_item(item, true, &mut tools, emit);
                    }
                }
                "error" => {
                    last_error = params
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                }
                "thread/tokenUsage/updated" => {
                    if let Some(usage) = usage_from(&params) {
                        self.usage = Some(usage);
                    }
                }
                "turn/completed" => {
                    let completed = params.pointer("/turn/id").and_then(Value::as_str);
                    if let (Some(current), Some(completed)) = (&turn_id, completed)
                        && current != completed
                    {
                        continue;
                    }

                    let status = params
                        .pointer("/turn/status")
                        .and_then(Value::as_str)
                        .unwrap_or("failed");

                    return match status {
                        "completed" => Ok(()),
                        "interrupted" => Err(DriverError::cancelled("the user stopped the turn")),
                        _ => {
                            let message = params
                                .pointer("/turn/error/message")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                                .or(last_error)
                                .unwrap_or_else(|| "the turn failed".to_owned());
                            Err(DriverError::new(message))
                        }
                    };
                }
                _ => {}
            }
        }
    }

    /// Replies to a request the agent made. Approvals and form elicitations
    /// become cards and block the turn until the user answers; anything else
    /// is refused as unsupported.
    fn answer(&mut self, message: &Value, ask: &mut dyn Asker) -> Result<(), DriverError> {
        let Some(id) = message.get("id") else {
            return Ok(());
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                let request = approval_request(method, &params);
                let decision = match ask.permission(request) {
                    Decision::Selected(option) => option,
                    // The turn is already being abandoned; `cancel` also
                    // interrupts it, which is the honest answer.
                    Decision::Cancelled => "cancel".to_owned(),
                };
                self.send(json!({"id": id, "result": {"decision": decision}}))
            }
            "mcpServer/elicitation/request" => {
                let mode = params.get("mode").and_then(Value::as_str);
                let result = match mode {
                    Some("form" | "openai/form" | "openaiForm") => {
                        let answer = ask.question(question_from(&params));
                        match answer.action {
                            circulo_protocol::QuestionAction::Accept => {
                                json!({"action": "accept", "content": answer.values})
                            }
                            circulo_protocol::QuestionAction::Decline => {
                                json!({"action": "decline", "content": null})
                            }
                            circulo_protocol::QuestionAction::Cancel => {
                                json!({"action": "cancel", "content": null})
                            }
                        }
                    }
                    // URL and device-verification prompts need a browser flow
                    // this client does not have.
                    _ => json!({"action": "decline", "content": null}),
                };
                self.send(json!({"id": id, "result": result}))
            }
            _ => self.send(json!({
                "id": id,
                "error": {"code": -32601, "message": "not supported by this client"},
            })),
        }
    }

    fn send(&mut self, message: Value) -> Result<(), DriverError> {
        write_line(&self.stdin, &message).inspect_err(|_| self.dead = true)
    }

    fn receive(&mut self) -> Result<Value, DriverError> {
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).map_err(|error| {
                self.dead = true;
                DriverError::new(format!("codex app-server: {error}"))
            })?;

            if read == 0 {
                self.dead = true;
                return Err(DriverError::new("codex app-server closed the connection"));
            }

            if line.trim().is_empty() {
                continue;
            }

            return serde_json::from_str(&line).map_err(|error| {
                DriverError::new(format!("codex app-server sent a bad message: {error}"))
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

/// One JSON line on the child's stdin. The mutex is what lets a cancel write
/// while the turn thread is blocked reading the child's stdout.
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
        .map_err(|error| DriverError::new(format!("codex app-server: {error}")))
}

/// Sends `turn/interrupt` for one turn on this connection.
fn send_interrupt(agent: &mut Agent, thread: &str, turn: &str) -> Result<(), DriverError> {
    let id = CANCEL_ID.fetch_add(1, Ordering::Relaxed);
    agent.send(json!({
        "id": id,
        "method": "turn/interrupt",
        "params": {"threadId": thread, "turnId": turn},
    }))
}

/// Remembers the turn id the cancel handle needs and, when a cancel arrived
/// first, sends the interrupt now.
fn register_turn(
    agent: &mut Agent,
    handle: Option<&CancelHandle>,
    turn: &str,
) -> Result<(), DriverError> {
    let Some(handle) = handle else {
        return Ok(());
    };
    *handle
        .turn
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(turn.to_owned());

    if handle.requested.load(Ordering::SeqCst) {
        send_interrupt(agent, &handle.thread, turn)?;
    }
    Ok(())
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

fn for_thread(params: &Value, thread: &str) -> bool {
    params
        .get("threadId")
        .and_then(Value::as_str)
        .is_none_or(|id| id == thread)
}

/// What we remember about one item so its later lifecycle keeps the same row.
struct Tool {
    name: String,
    detail: String,
    /// What the command printed, accumulated from its output deltas.
    output: String,
}

/// Tool output can be megabytes; 64 KiB reads a failing test and leaves the
/// row usable. The rest is marked, not silently dropped.
const MAX_TOOL_OUTPUT: usize = 64 * 1024;
const TRUNCATED: &str = "\n… [output truncated]";

fn append_output(output: &mut String, delta: &str) {
    if output.len() >= MAX_TOOL_OUTPUT {
        return;
    }
    let remaining = MAX_TOOL_OUTPUT - output.len();
    if delta.len() <= remaining {
        output.push_str(delta);
        return;
    }
    // Never split a UTF-8 character.
    let mut end = remaining;
    while end > 0 && !delta.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&delta[..end]);
    output.push_str(TRUNCATED);
}

/// Maps one `item/started` or `item/completed` item to a tool row. Text and
/// reasoning items are skipped: their deltas already streamed the content.
fn apply_item(
    item: &Value,
    completed: bool,
    tools: &mut HashMap<String, Tool>,
    emit: &mut dyn FnMut(Part),
) {
    let kind = item.get("type").and_then(Value::as_str).unwrap_or_default();
    let id = item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let (name, detail) = match kind {
        "commandExecution" => (
            "command".to_owned(),
            item.get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        "fileChange" => ("edit".to_owned(), changed_paths(item)),
        "mcpToolCall" => (
            item.get("tool")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_owned(),
            item.get("server")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        "webSearch" => (
            "web search".to_owned(),
            item.get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        "imageView" => (
            "image view".to_owned(),
            item.get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        ),
        // `agentMessage`, `reasoning`, `plan` and anything new: no row.
        _ => return,
    };

    let entry = tools.entry(id.clone()).or_insert_with(|| Tool {
        name: name.clone(),
        detail: detail.clone(),
        output: String::new(),
    });
    if entry.detail.is_empty() && !detail.is_empty() {
        entry.detail = detail;
    }
    if !name.is_empty() {
        entry.name = name;
    }

    // A completed command may carry the whole output even when no delta did;
    // the deltas win when both exist.
    if completed
        && kind == "commandExecution"
        && entry.output.is_empty()
        && let Some(aggregated) = item.get("aggregatedOutput").and_then(Value::as_str)
    {
        append_output(&mut entry.output, aggregated);
    }

    let status = item.get("status").and_then(Value::as_str);
    let state = if completed || matches!(status, Some("completed" | "failed" | "declined")) {
        ToolState::Done
    } else {
        ToolState::Running
    };
    let detail = match status {
        Some("failed") => format!("{} · failed", entry.detail),
        Some("declined") => format!("{} · declined", entry.detail),
        _ => entry.detail.clone(),
    };
    // Output travels once, with the finished row.
    let output = if completed && !entry.output.is_empty() {
        Some(entry.output.clone())
    } else {
        None
    };

    emit(Part::Tool {
        id,
        name: entry.name.clone(),
        detail,
        state,
        output,
    });
}

fn changed_paths(item: &Value) -> String {
    item.get("changes")
        .and_then(Value::as_array)
        .map(|changes| {
            changes
                .iter()
                .filter_map(|change| change.get("path").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// The four decisions the app server documents for a command or file change.
fn approval_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption {
            id: "accept".to_owned(),
            label: "Allow once".to_owned(),
            kind: PermissionOptionKind::AllowOnce,
        },
        PermissionOption {
            id: "acceptForSession".to_owned(),
            label: "Allow for session".to_owned(),
            kind: PermissionOptionKind::AllowAlways,
        },
        PermissionOption {
            id: "decline".to_owned(),
            label: "Reject".to_owned(),
            kind: PermissionOptionKind::RejectOnce,
        },
        PermissionOption {
            id: "cancel".to_owned(),
            label: "Stop the turn".to_owned(),
            kind: PermissionOptionKind::RejectAlways,
        },
    ]
}

/// The card, built from the app server's approval request. The title is the
/// agent's reason when it gives one; the detail is the command in full.
fn approval_request(method: &str, params: &Value) -> PermissionRequest {
    let is_file = method == "item/fileChange/requestApproval";

    let title = if params.get("networkApprovalContext").is_some() {
        "The agent is asking for network access".to_owned()
    } else if is_file {
        "The agent wants to edit files".to_owned()
    } else {
        params
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("The agent wants to run a command")
            .to_owned()
    };

    let detail = params
        .get("command")
        .and_then(Value::as_str)
        .or_else(|| params.get("cwd").and_then(Value::as_str))
        .or_else(|| params.get("grantRoot").and_then(Value::as_str))
        .or_else(|| params.get("reason").and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned();

    PermissionRequest {
        title,
        detail,
        options: approval_options(),
    }
}

/// A question from an MCP elicitation form. The schema is a flat object; each
/// property becomes one field, an enum becomes a choice, anything else needs a
/// text answer.
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

/// `code_style` becomes `Code style`; an effort id becomes a label.
fn humanize(id: &str) -> String {
    let mut label = String::with_capacity(id.len());
    for (index, word) in id
        .split(['_', '-', ' '])
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn collect(items: &[(Value, bool)]) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut tools = HashMap::new();
        for (item, completed) in items {
            apply_item(item, *completed, &mut tools, &mut |part| parts.push(part));
        }
        parts
    }

    #[test]
    fn a_command_item_becomes_a_tool_row() {
        let parts = collect(&[(
            json!({
                "type": "commandExecution",
                "id": "item_1",
                "command": "cargo test --workspace",
                "status": "inProgress"
            }),
            false,
        )]);

        assert_eq!(
            parts,
            vec![Part::Tool {
                id: "item_1".into(),
                name: "command".into(),
                detail: "cargo test --workspace".into(),
                state: ToolState::Running,
                output: None,
            }]
        );
    }

    #[test]
    fn a_finished_command_says_so_and_failure_is_marked() {
        let parts = collect(&[
            (
                json!({
                    "type": "commandExecution",
                    "id": "item_2",
                    "command": "cargo test",
                    "status": "failed"
                }),
                true,
            ),
            (
                json!({
                    "type": "fileChange",
                    "id": "item_3",
                    "changes": [{"path": "src/lib.rs"}, {"path": "README.md"}],
                    "status": "completed"
                }),
                true,
            ),
        ]);

        assert!(
            matches!(&parts[0], Part::Tool { state: ToolState::Done, detail, .. }
                if detail.ends_with("· failed"))
        );
        assert!(matches!(&parts[1], Part::Tool { name, detail, .. }
                if name == "edit" && detail == "src/lib.rs, README.md"));
    }

    #[test]
    fn token_usage_becomes_cumulative_tokens() {
        let usage = usage_from(&json!({
            "tokenUsage": {
                "total": {
                    "totalTokens": 1500,
                    "inputTokens": 1000,
                    "cachedInputTokens": 200,
                    "outputTokens": 500
                },
                "last": {"totalTokens": 300},
                "modelContextWindow": 200000
            }
        }))
        .expect("usage");

        assert_eq!(usage.total_tokens, Some(1500));
        assert_eq!(usage.input_tokens, Some(1000));
        assert_eq!(usage.output_tokens, Some(500));
        assert_eq!(usage.cached_tokens, Some(200));
        assert_eq!(usage.context_used, None, "Codex reports no current context");
    }

    #[test]
    fn a_completed_command_carries_its_aggregated_output() {
        let parts = collect(&[(
            json!({
                "type": "commandExecution",
                "id": "item_4",
                "command": "cargo test",
                "status": "completed",
                "aggregatedOutput": "ok\n"
            }),
            true,
        )]);

        assert!(matches!(
            &parts[0],
            Part::Tool {
                output: Some(output),
                ..
            } if output == "ok\n"
        ));
    }

    #[test]
    fn output_is_capped_and_marked() {
        let mut output = String::new();
        append_output(&mut output, &"a".repeat(MAX_TOOL_OUTPUT + 10));

        assert!(output.ends_with(TRUNCATED));
        assert!(output.len() <= MAX_TOOL_OUTPUT + TRUNCATED.len());
    }

    #[test]
    fn text_reasoning_and_unknown_items_are_ignored_by_the_item_mapper() {
        let parts = collect(&[
            (
                json!({"type": "agentMessage", "id": "m1", "text": "done"}),
                true,
            ),
            (
                json!({"type": "reasoning", "id": "r1", "summary": []}),
                true,
            ),
            (json!({"type": "contextCompaction", "id": "c1"}), true),
        ]);
        assert!(parts.is_empty());
    }

    #[test]
    fn an_approval_card_offers_the_four_documented_decisions() {
        let request = approval_request(
            "item/commandExecution/requestApproval",
            &json!({"command": "rm -rf build", "reason": "clean the build"}),
        );

        assert_eq!(request.title, "clean the build");
        assert_eq!(request.detail, "rm -rf build");
        assert_eq!(
            request
                .options
                .iter()
                .map(|option| option.id.as_str())
                .collect::<Vec<_>>(),
            vec!["accept", "acceptForSession", "decline", "cancel"]
        );
    }

    #[test]
    fn a_file_change_approval_uses_the_root_and_its_own_title() {
        let request = approval_request(
            "item/fileChange/requestApproval",
            &json!({"reason": "needs to write", "grantRoot": "/tmp/project"}),
        );
        assert_eq!(request.title, "The agent wants to edit files");
        assert_eq!(request.detail, "/tmp/project");
    }

    #[test]
    fn a_form_elicitation_becomes_a_question_with_its_field_order() {
        let question = question_from(&json!({
            "message": "How should I approach this?",
            "requestedSchema": {
                "type": "object",
                "properties": {
                    "strategy": {"type": "string", "enum": ["conservative", "balanced"]},
                    "note": {"type": "string", "title": "Note"}
                },
                "required": ["strategy"]
            }
        }));

        assert_eq!(question.message, "How should I approach this?");
        assert_eq!(question.fields.len(), 2);
        assert_eq!(question.fields[0].id, "strategy");
        assert!(question.fields[0].required);
        assert_eq!(
            question.fields[0].kind,
            QuestionKind::Choice {
                options: vec!["conservative".into(), "balanced".into()]
            }
        );
        assert_eq!(question.fields[1].label, "Note");
        assert!(!question.fields[1].required);
    }

    #[test]
    fn models_become_model_and_effort_options() {
        let models = models_from(&json!({
            "data": [
                {
                    "id": "gpt-a",
                    "displayName": "GPT A",
                    "isDefault": true,
                    "defaultReasoningEffort": "low",
                    "supportedReasoningEfforts": [
                        {"reasoningEffort": "low"},
                        {"reasoningEffort": "high"}
                    ]
                },
                {
                    "id": "gpt-b",
                    "displayName": "GPT B",
                    "isDefault": false,
                    "defaultReasoningEffort": "medium",
                    "supportedReasoningEfforts": [{"reasoningEffort": "medium"}]
                }
            ]
        }));

        let codex = Codex::new(".");
        let options = codex.options_for(&models);

        assert_eq!(options[0].id, "model");
        assert_eq!(options[0].current, "gpt-a");
        assert_eq!(options[0].options.len(), 2);
        assert_eq!(options[1].id, "effort");
        assert_eq!(options[1].current, "low");
        assert_eq!(
            options[1]
                .options
                .iter()
                .map(|choice| choice.id.as_str())
                .collect::<Vec<_>>(),
            vec!["low", "high"]
        );
    }

    #[test]
    fn a_fresh_thread_starts_in_the_project_and_asks() {
        let params = {
            let mut params = json!({
                "cwd": "/tmp/project",
                "approvalPolicy": APPROVAL_POLICY,
                "sandbox": SANDBOX,
                "serviceName": SERVICE_NAME,
            });
            let model: Option<&str> = Some("gpt-a");
            if let Some(model) = model {
                params["model"] = json!(model);
            }
            params
        };

        assert_eq!(params["approvalPolicy"], "on-request");
        assert_eq!(params["sandbox"], "workspace-write");
        assert_eq!(params["model"], "gpt-a");
    }
}
