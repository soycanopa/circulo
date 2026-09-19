# AGENTS.md — Circulo

Rules for any agent (or human) working in this repository. The original rule set was
written for a Rust/crate project; adjustments for this Go + React/Wails codebase are
marked **[adjusted]**. Everything else is verbatim intent from the project owner.

## Working agreement

- Be direct and technical.
- Explain the *why* with documentation (OpenCode, Wails and more) when you propose
  something. Primary sources: [opencode.ai/docs](https://opencode.ai/docs/) and the
  OpenAPI spec the server itself serves at `GET /doc`; [v3.wails.io](https://v3.wails.io/).
- If a human decision contradicts best practice or the quality bar, raise your hand.
  Do not comply just to please. Build it solid.
- If you have a better idea, propose it and explain why we should pick it.
- **Do not invent APIs.** If it is not in the harness's official docs, it does not
  exist. Read the docs before writing code. **[adjusted]** For OpenCode the source of
  truth is the OpenAPI 3.1 spec served by the running server (`GET /doc`) plus
  opencode.ai/docs — re-verify endpoints/schemas whenever the pinned `opencode`
  version changes. For Wails, v3.wails.io only; v2 docs do not apply.
- **Do not assume. Ask.** If product, protocol, or permission is unclear, ask before
  implementing.
- **No changes without a plan** (short is fine: issue or commit message). Large work →
  written plan under `docs/`.
- **Do not loop.** If an approach fails twice, stop, write what you tried, and ask or
  change strategy.
- **Do not implement remote access or a second adapter until the local protocol is
  stable.** **[adjusted]** Original: "no remote/Tailcat". Concretely: no Tailcat tunnel,
  no remote listener, no adapter #2 (Claude, Codex, …) until the neutral protocol
  (`internal/agent/protocol`) is proven stable by the OpenCode adapter in daily use
  (gate: the E2E checklist in docs/implement.md). The remote design is frozen in
  docs/remote.md — build from there when the gate opens, do not redesign.

## Architecture invariants **[adjusted]**

Original "Always modular: one responsibility per crate" maps to this codebase as:

- One responsibility per **Go package** and per **TS module**.
- Dependency direction is one-way:
  `frontend → internal/agent/protocol (contract) ← internal/orchestrator ← internal/opencode`.
- The UI does not import adapters: frontend code knows only the neutral protocol
  (`frontend/src/lib/agent/protocol.ts`) and the relay API (`/agent/...`). It must never
  see OpenCode types, ports, or URLs.
- Adapters do not know the UI: `internal/opencode` (and every future
  `internal/<provider>`) must not import `wails` packages, `internal/relay`, or
  `internal/orchestrator`. It exposes an `agent.AgentAdapter` and a channel of
  `agent/protocol` events — nothing else.
- `internal/relay` translates HTTP ⇄ orchestrator calls and holds **no business logic**.

## Git & delivery

- All work happens on branches (`feature/<phase-or-topic>`). Never commit straight
  to `main`.
- Granular commits: one logical change each. Commit messages: `type(scope): summary`
  (`feat(protocol): …`, `fix(opencode): …`, `test: …`, `docs: …`, `chore: …`).
- Tests are part of the work, not an extra. **Parsers and protocol first**: when
  touching `internal/opencode` or the reducer, the fixtures/tables land in the same
  commit or the one before — never after.
- CI-lite (local): `go test ./...` and `pnpm vitest run` must pass before every push.

## Code standards

- Clean code standards. Small functions, explicit errors (Go: wrap with context,
  never `_ = err` where failure matters), no premature abstraction.
- Comments: non-obvious *why*, invariants, harness traps. Do not narrate what the code
  already says.
- **Wails v3 traps (beta — pinned, currently v3.0.0-beta.16):**
  - `ServiceShutdown() error` — no `context.Context` parameter, or it is silently
    never called.
  - `frontend/bindings/` is generated. Never hand-edit; regenerate via
    `wails3 generate bindings` / build.
  - `Events.Off(name)` removes **all** listeners for that name — keep and call the
    unsubscribe function returned by `Events.On` instead.
  - The webview origin is `wails://localhost` (macOS/Linux), so cross-origin fetch to
    `http://127.0.0.1` needs the `/agent` route-mounted relay — not direct webview calls.
- **OpenCode traps:**
  - `message.part.updated` carries the **full part** — treat every update as replace,
    use `message.part.delta` only as an optimization.
  - Ignore-list events (`server.heartbeat`, `plugin.added`, `catalog.updated`, …) must
    be skipped, never crash: unknown event/part types are forward compatibility.
  - Fixture files in `internal/opencode/testdata/` are real captured SSE from a
    specific opencode version — record the version in the fixture header comment.
- **[adjusted]** Original rule "UI does not import adapters. Adapters do not UI."
  is enforced per the Architecture invariants above.

## Project map

- `docs/` — PRD, TRD, UX, Flow, UI spec, implementation plan. Change docs in the same
  PR as the behavior they describe.
- `internal/agent/protocol` — the neutral wire contract. Changing it means changing
  `frontend/src/lib/agent/protocol.ts` and reducer tests in the same commit.
- `internal/opencode/` — the only place allowed to speak OpenCode.
