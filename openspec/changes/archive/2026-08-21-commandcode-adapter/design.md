# Design: commandcode-adapter

## D1 — Crate shape

`crates/circulo-adapter-commandcode/Cargo.toml`:

```toml
[package]
name = "circulo-adapter-commandcode"
# workspace inheritance

[dependencies]
circulo-adapter.workspace = true
circulo-core.workspace = true
serde.workspace = true
serde_json.workspace = true
```

Workspace `Cargo.toml` adds the new member.

Layout:

```
crates/circulo-adapter-commandcode/
├── Cargo.toml
├── src/
│   ├── lib.rs          # public API: CommandCodeAdapter, CommandCodeAdapter::from_env
│   ├── discovery.rs    # locate the `cmd` binary
│   ├── subprocess.rs   # spawn child, manage lifecycle, read NDJSON stdout
│   └── mapping.rs      # NDJSON frames -> AdapterEvent; exit codes -> AdapterError
└── tests/
    ├── fixtures/
    │   ├── turn-text.ndjson
    │   ├── turn-tool-call.ndjson
    │   ├── turn-error.ndjson
    │   └── turn-auth-error.ndjson
    └── mapping.rs
```

## D2 — Binary discovery

`crates/circulo-adapter-commandcode/src/discovery.rs`:

```rust
pub fn discover_commandcode_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("COMMANDCODE_BIN") {
        let p = PathBuf::from(path);
        if p.is_file() { return Some(p); }
    }
    // PATH lookup
    let path_var = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_var) {
        let candidate = entry.join("cmd");
        if candidate.is_file() { return Some(candidate); }
    }
    None
}
```

On macOS the npm-global bin directory is usually `~/.npm-global/bin` or `/usr/local/bin`; both are on PATH for most users. The `COMMANDCODE_BIN` override covers custom installs.

## D3 — Subprocess lifecycle

`crates/circulo-adapter-commandcode/src/subprocess.rs`:

The adapter spawns one child per turn:

```rust
let mut cmd = std::process::Command::new(&binary);
cmd.arg("-p").arg(query);
cmd.arg("--output-format").arg("json");
if session.cwd.is_some() { cmd.current_dir(session.cwd); }
if session.resume_id.is_some() { cmd.arg("--resume").arg(...); }
if session.continue_recent { cmd.arg("--continue"); }
if !session.permission_allow_tools { cmd.arg("--yolo"); }
cmd.stdin(Stdio::null());
cmd.stdout(Stdio::piped());
cmd.stderr(Stdio::piped());
let child = cmd.spawn().ok_or(...)?;
```

The child handle is stored in an `Arc<Mutex<Option<Child>>>` keyed by the session id (one child per session at a time). `abort_turn` calls `child.kill()` and waits.

The stdout is read line-by-line in a blocking task. Each line is parsed as `serde_json::Value` and dispatched via the mapping module. The reader stops when the child exits and the stdout pipe closes.

The stderr is drained into a buffer for use in error mapping.

## D4 — NDJSON mapping

`crates/circulo-adapter-commandcode/src/mapping.rs`:

Command Code's NDJSON has two frame shapes:

1. **Event frames**, one per `AgentEvent`:
   ```json
   {"type":"event","event":{"type":"...","...":"..."}}
   ```
2. **One final result line**, always last:
   ```json
   {"type":"result","subtype":"success|error|max_turns","sessionId":"...","stopReason":"...","usage":{...},"durationMs":...,"finalText":"..."}
   ```

Event types observed in the docs (incomplete; the docs only show a few):
- `tool_running`, `tool_complete`
- `text` (or similar) → text deltas
- `session_title` → title updates
- `todo_updated` → task list updates

The mapping is defensive: unknown event types are logged at debug and otherwise ignored. Forward compatibility is built in.

| CommandCode event | AdapterEvent |
| --- | --- |
| `event.type == "tool_running"` | `ToolCallStarted { id, name, status: Running, input: description }` |
| `event.type == "tool_complete"` | `ToolCallCompleted { id, output }` |
| `event.type == "text"` (or any event with `content`) | `TextDelta { content }` |
| `event.type == "session_title"` | `SessionTitleUpdated { title }` |
| `event.type == "todo_updated"` | `TaskListUpdated { tasks }` (best-effort parse) |
| `result.subtype == "success"` + `sessionId` | `SessionBound { agent_session_id }` |
| `result.subtype == "success"` | `Completed` |
| `result.subtype == "error"` or non-zero exit | `Failed { reason, message }` mapped from exit code or `error` field |
| `result.subtype == "max_turns"` | `Failed { reason: Cancelled, message: "max turns reached" }` |

The mapping is tested against NDJSON fixtures; unknown event types get a `MappingOutcome::Ignored` and never reach the daemon.

## D5 — Exit code mapping

`mapping.rs` also maps exit codes from the Command Code docs:

| Exit code | AdapterError |
| --- | --- |
| 0 | (success — result frame already handled) |
| 3 (`EXIT_AUTH_ERROR`) | `unavailable(Unauthorized, "Sign in required. Run cmd login.")` |
| 4 (`EXIT_PERMISSION_DENIED`) | `failed(PermissionDenied, ...)` |
| 5 (`EXIT_RATE_LIMITED`) | `unavailable(RateLimited, ...)` |
| 6 (`EXIT_CONNECTION_ERROR`) | `unavailable(StreamFailed, ...)` |
| 7 (`EXIT_SERVER_ERROR`) | `unavailable(StreamFailed, ...)` |
| 8 (`EXIT_MAX_TURNS_REACHED`) | `failed(Cancelled, "max turns reached")` |
| 9 (`EXIT_NO_RESPONSE`) | `failed(ProviderFailed, ...)` |
| 10 (`EXIT_INSUFFICIENT_CREDITS`) | `unavailable(Unauthorized, "Insufficient credits")` |
| 130 (`EXIT_INTERRUPTED`) | `failed(Cancelled, "interrupted")` |
| any other | `failed(Internal, stderr)` |

These map to existing `ErrorReason` variants where possible. New variants would need a separate change; the current set covers all observed codes.

## D6 — `impl AgentAdapter`

`crates/circulo-adapter-commandcode/src/lib.rs`:

```rust
pub struct CommandCodeAdapter {
    binary: PathBuf,
}

impl CommandCodeAdapter {
    pub fn from_env() -> Option<Self> {
        discover_commandcode_binary().map(|binary| Self { binary })
    }
    pub fn with_binary(binary: PathBuf) -> Self { Self { binary } }
}

impl AgentAdapter for CommandCodeAdapter {
    fn name(&self) -> &'static str { "commandcode" }
    fn probe(&self) -> AdapterHealth {
        // Run `cmd --version` with a 2s timeout; if exit 0 → Available.
        // If exit 3 → Unavailable(Unauthorized). Else → Unavailable(Missing).
    }
    fn generate(&self, req: GenerateRequest, emit: &mut dyn FnMut(AdapterEvent)) -> Result<(), AdapterError> {
        // Spawn the headless subprocess, pipe stdout to the mapping,
        // emit AdapterEvent for each frame. The result frame drives
        // SessionBound + Completed/Failed.
    }
    fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> { Ok(Vec::new()) }
    fn abort_turn(&self, agent_session_id: &str, working_directory: Option<&Path>) -> Result<(), AdapterError> {
        // Look up the running child for agent_session_id and kill it.
    }
    fn delete_agent_session(&self, _id: &str, _cwd: Option<&Path>) -> Result<(), AdapterError> { Ok(()) }
    fn opencode_health(&self) -> Option<OpenCodeHealth> { None }  // not applicable
}
```

`probe()` shells out to `cmd --version` with a 2s timeout. Exit 0 → `Available`; exit 3 → `Unavailable(Unauthorized)`; anything else → `Unavailable(Missing)`. This is best-effort and never panics.

`generate()` is the core. It spawns the subprocess, reads stdout line-by-line, dispatches each via the mapping function, and emits the events. If the subprocess can't be spawned (binary not found at runtime despite discovery succeeding earlier), returns `AdapterError::failed(BinaryMissing, "Command Code binary not found.")`. If the child exits non-zero without a `result` frame, reads stderr and maps to an `AdapterError`.

`abort_turn()` looks up the running child by `agent_session_id` in a `Mutex<HashMap<String, Arc<Mutex<Child>>>>` and kills it. Best-effort; returns `Ok(())` if no child is registered (the turn may have already finished).

## D7 — Workspace + daemon wiring

`Cargo.toml` (workspace root) adds the new member.

`crates/circulo-daemon/src/adapter_registry.rs`:

```rust
pub struct AdapterRegistry {
    opencode: Arc<dyn AgentAdapter>,
    commandcode: Option<Arc<dyn AgentAdapter>>,
}

impl AdapterRegistry {
    pub fn build() -> Self {
        let opencode: Arc<dyn AgentAdapter> = ...;
        let commandcode = circulo_adapter_commandcode::CommandCodeAdapter::from_env()
            .map(|a| Arc::new(a) as Arc<dyn AgentAdapter>);
        Self { opencode, commandcode }
    }
    pub fn for_agent(&self, agent: AgentType) -> Option<Arc<dyn AgentAdapter>> {
        match agent {
            AgentType::OpenCode => Some(Arc::clone(&self.opencode)),
            AgentType::CommandCode => self.commandcode.as_ref().map(Arc::clone),
        }
    }
    pub fn list(&self) -> Vec<AgentDescriptor> {
        // opencode + commandcode (when registered)
    }
}
```

`AdapterRegistry::with_opencode` (test constructor) sets `commandcode = None` so the test daemon still works.

## D8 — Settings → General

`crates/circulo-app/src/settings/general.rs`:

Today the panel shows daemon + OpenCode. The settings refresh handler already runs `health()`. We extend it to also call `list_agents()`. The general panel adds a row "Command Code" with the descriptor's `available` + `version?` and the locale copy for `unavailable` / `auth_required` / `install_hint`.

The OpenCode row stays as-is; the Command Code row sits below it. If `available = false` and the binary wasn't discovered, the row shows the install hint. If `available = false` but the binary IS installed, the row shows the auth hint (auth failed). Distinguishing "binary missing" from "binary present but auth failed" requires the adapter's `probe()` to communicate both, which it does via exit code 3 (auth) vs other (missing).

The Settings refresh handler already exists; we just enrich the `HealthResponse` consumption with a `list_agents` fetch and a new "agents" field. The `SettingsSection::General` panel renders both.

## D9 — i18n

`crates/circulo-i18n/locales/en.json`:

```json
"settings.commandcode.title": "Command Code",
"settings.commandcode.unavailable": "Not installed",
"settings.commandcode.auth_required": "Sign in required. Run cmd login in your terminal.",
"settings.commandcode.install_hint": "Install with npm i -g command-code, then sign in with cmd login."
```

## D10 — Spec deltas

### `openspec/specs/agent-adapter/spec.md`

Add a sibling requirement alongside the registry dispatch:

> ### Requirement: Command Code adapter maps NDJSON to normalized events
>
> The Command Code adapter MUST run `cmd -p <query> --output-format json`, parse the resulting NDJSON into `AdapterEvent`s, and surface auth errors (exit code 3) as `AdapterError::unavailable(Unauthorized, ...)`. The adapter MUST accept a `COMMANDCODE_BIN` env override and fall back to `cmd` on `PATH`.
>
> #### Scenario: Successful turn emits Completed
> - **GIVEN** a Command Code binary is present and authenticated
> - **WHEN** the adapter runs a turn
> - **THEN** the event stream ends with `SessionBound` (if a `sessionId` is reported) and `Completed`
>
> #### Scenario: Auth failure surfaces as Unavailable
> - **GIVEN** the binary is present but the user is not authenticated
> - **WHEN** the adapter runs a turn
> - **THEN** the result is `AdapterError::unavailable(Unauthorized, ...)` with a human message

### `openspec/specs/local-daemon-api/spec.md`

Already documents `/v1/agents` returning one descriptor per registered provider. This change proves the second case works; no spec change needed.

## D11 — Files

| File | Change |
| --- | --- |
| `Cargo.toml` (workspace) | + member `crates/circulo-adapter-commandcode` |
| `crates/circulo-adapter-commandcode/Cargo.toml` | NEW |
| `crates/circulo-adapter-commandcode/src/lib.rs` | NEW: `CommandCodeAdapter` + `impl AgentAdapter` |
| `crates/circulo-adapter-commandcode/src/discovery.rs` | NEW: `discover_commandcode_binary` |
| `crates/circulo-adapter-commandcode/src/subprocess.rs` | NEW: spawn child, manage lifecycle |
| `crates/circulo-adapter-commandcode/src/mapping.rs` | NEW: NDJSON → events, exit codes → errors |
| `crates/circulo-adapter-commandcode/tests/fixtures/turn-*.ndjson` | NEW fixtures |
| `crates/circulo-adapter-commandcode/tests/mapping.rs` | NEW tests |
| `crates/circulo-daemon/Cargo.toml` | + dep on `circulo-adapter-commandcode` |
| `crates/circulo-daemon/src/adapter_registry.rs` | + `commandcode` field + wiring |
| `crates/circulo-app/src/settings/general.rs` | + Command Code row |
| `crates/circulo-i18n/locales/en.json` | + 4 keys |
| `openspec/specs/agent-adapter/spec.md` | + Command Code mapping requirement |

## D12 — Verification

| Step | Expected |
| --- | --- |
| `cargo check --workspace` | 0 warnings introduced |
| `cargo test --workspace` | 154 tests pass + new mapping tests |
| `python3 scripts/check-crate-boundaries.py` | clean |
| Manual | With `cmd` installed: `cmd -p "echo hi" --output-format json` returns NDJSON; a Circulo session with `agent = command_code` streams the same events. Without auth: 503 / auth copy. |

## D13 — Commit strategy

```
docs(openspec): add commandcode-adapter change artifacts
docs(specs): CommandCode mapping in agent-adapter
chore(i18n): CommandCode copy in Settings
feat(app): CommandCode row in Settings -> General
feat(daemon): register CommandCode in AdapterRegistry
feat(adapter-commandcode): NDJSON mapping with fixtures
feat(adapter-commandcode): impl AgentAdapter with subprocess
feat(adapter-commandcode): crate skeleton and binary discovery
feat(workspace): add circulo-adapter-commandcode member
```

8 commits. The first is the artifacts; the rest are the implementation in dependency order (workspace → crate skeleton → discovery → subprocess → mapping → impl → tests → daemon wiring → UI → i18n → spec).

## D14 — Manual pass (user's responsibility)

Per `AGENTS.md §1.8`, the user runs the manual verification:

1. Install `cmd`: `npm i -g command-code`
2. `cmd login` (or skip — auth path must produce 503 + auth copy)
3. Open Circulo, create a session with agent = Command Code (AgentSelector visible)
4. Send a query; verify text streams back, tool calls render as cards, the response completes
5. Kill the session mid-turn (Stop); verify `abort_turn` kills the child
6. Run `cmd logout` and try again; verify auth path