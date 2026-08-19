# Tasks: commandcode-adapter

## 1. Workspace

- [x] 1.1 Add `crates/circulo-adapter-commandcode` to the workspace `Cargo.toml` members

## 2. Crate skeleton

- [x] 2.1 `crates/circulo-adapter-commandcode/Cargo.toml` (deps: circulo-adapter, circulo-core, serde, serde_json)
- [x] 2.2 `lib.rs`: `CommandCodeAdapter` struct + `name()` + `from_env()` returning `Option<Self>`
- [x] 2.3 `discovery.rs`: `discover_commandcode_binary()` honoring `COMMANDCODE_BIN` then PATH

## 3. Subprocess

- [x] 3.1 `subprocess.rs`: spawn `cmd -p ... --output-format json` with cwd, resume/continue flags, optional `--yolo`
- [x] 3.2 Track running children by `agent_session_id` for `abort_turn`
- [x] 3.3 Read stdout line-by-line, parse each line as JSON

## 4. Mapping

- [x] 4.1 `mapping.rs`: NDJSON frames → `AdapterEvent` (tool_running, tool_complete, text, session_title, todo_updated, result)
- [x] 4.2 Exit code → `AdapterError` mapping (3 = Unauthorized, 4 = PermissionDenied, 5 = RateLimited, 6/7 = StreamFailed, 8 = max_turns, 9 = ProviderFailed, 10 = InsufficientCredits, 130 = Cancelled)
- [x] 4.3 `SessionBound` from `result.sessionId`

## 5. Trait impl

- [x] 5.1 `probe()` runs `cmd --version` with 2s timeout; maps exit 0 → Available, exit 3 → Unavailable(Unauthorized), other → Unavailable(Missing)
- [x] 5.2 `generate()` spawns subprocess, drives mapping, returns `AdapterError` on non-zero exit
- [x] 5.3 `list_models()` returns `Ok(Vec::new())` (no programmatic catalog)
- [x] 5.4 `abort_turn()` kills the running child for the given session id
- [x] 5.5 `delete_agent_session()` returns `Ok(())` (no remote handle)
- [x] 5.6 `opencode_health()` returns `None`

## 6. Tests

- [x] 6.1 `tests/fixtures/turn-text.ndjson`: text-only successful turn
- [x] 6.2 `tests/fixtures/turn-tool-call.ndjson`: turn with tool_running + tool_complete
- [x] 6.3 `tests/fixtures/turn-error.ndjson`: result.subtype=error
- [x] 6.4 `tests/fixtures/turn-auth-error.ndjson`: exit 3
- [x] 6.5 `tests/mapping.rs`: feed each fixture through the mapping; assert the event sequence

## 7. Daemon wiring

- [x] 7.1 `circulo-daemon/Cargo.toml`: add `circulo-adapter-commandcode` dep
- [x] 7.2 `AdapterRegistry::build()` calls `CommandCodeAdapter::from_env()` and stores as `Option`
- [x] 7.3 `AdapterRegistry::for_agent(CommandCode)` returns the registered adapter
- [x] 7.4 `AdapterRegistry::list()` includes the CommandCode descriptor when registered
- [x] 7.5 `AdapterRegistry::with_opencode()` (test constructor) sets `commandcode = None`

## 8. Settings UI

- [x] 8.1 `settings/general.rs`: add Command Code row using `GET /v1/agents` data
- [x] 8.2 Wire the refresh handler to also call `list_agents()`
- [x] 8.3 Show install hint when the binary is missing, auth hint when present but not authenticated

## 9. i18n

- [x] 9.1 `settings.commandcode.title`, `unavailable`, `auth_required`, `install_hint` in `en.json`

## 10. Specs

- [x] 10.1 `agent-adapter`: Command Code mapping requirement + scenarios

## 11. OpenSpec artifacts

- [x] 11.1 `proposal.md`, `design.md`, `tasks.md` in `openspec/changes/2026-08-21-commandcode-adapter/`

## 12. Verification

- [x] 12.1 `cargo check --workspace` clean
- [x] 12.2 `cargo test --workspace` — 154 + new tests pass
- [x] 12.3 `python3 scripts/check-crate-boundaries.py` clean
- [x] 12.4 Manual pass (user): install `cmd`, login, create a Command Code session, send a query, verify the stream