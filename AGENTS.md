# AGENTS.md — Circulo

Rules for any agent (or human) working in this repository. The original rule set was
written for a Rust/crate project; adjustments for this Go + React/Wails codebase are
marked **[adjusted]**. Everything else is verbatim intent from the project owner.

## Working agreement

- Be direct and technical.
- Explain the *why* with documentation (OpenCode, Wails and more) when you propose
  something. Primary sources: [opencode.ai/docs](https://opencode.ai/docs/) and the
  OpenAPI spec the running server serves at **`GET /openapi.json`** (on v2 `/doc` is
  the webview HTML, not the spec); [v3.wails.io](https://v3.wails.io/).
- If a human decision contradicts best practice or the quality bar, raise your hand.
  Do not comply just to please. Build it solid.
- If you have a better idea, propose it and explain why we should pick it.
- **Do not invent APIs.** If it is not in the harness's official docs, it does not
  exist. Read the docs before writing code. **[adjusted]** For OpenCode the source of
  truth is the OpenAPI 3.1 spec served by the running server (`GET /openapi.json`) plus
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
- **Reference projects: ideas yes, code never.** [waku](https://github.com/egoist/waku)
  is GPL-3.0-only. Learn its UX patterns and protocol ideas from it — the neutral
  protocol, the per-turn git-ref checkpoints, the sequenced envelope — but never
  copy a file, a function or a type from it, or Circulo inherits GPL-3.0. Its
  daemon split is also *not* a template: it exists to drive 9 providers plus a
  browser client and remote access, and adopting it here would break the
  single-composition-root invariant for no proven need.

## Architecture invariants **[adjusted]**

Original "Always modular: one responsibility per crate" maps to this codebase as:

- One responsibility per **Go package** and per **TS module**.
- Dependency direction is one-way:
  `frontend → internal/agent/protocol (contract) ← internal/orchestrator ← {internal/opencode, internal/omp}`.
  The orchestrator holds N adapters behind one `agent.Adapter` interface; a third
  adapter slots in beside them without touching the two above.
- The UI does not import adapters: frontend code knows only the neutral protocol
  (`frontend/src/lib/agent/protocol.ts`) and the relay API (`/agent/...`). It must
  never see OpenCode or omp types, ports, or URLs. The provider name
  (`"opencode" | "omp"`) is the only provider vocabulary that crosses this line,
  and only for *identity* — product name, logo, which "add project" entry to
  show. **Behavior must branch on `protocol.Capabilities`, never on the name.**
  A UI check like `provider === "omp"` is correct for identity and wrong for
  anything else: omp's `SetBranch` is a no-op and its permission round-trip does
  not exist, so a name check produces a control that does nothing.
- Adapters do not know the UI: `internal/opencode` and `internal/omp` must not
  import `wails` packages, `internal/relay`, or `internal/orchestrator`. Each
  exposes an `agent.AgentAdapter` and a channel of `agent/protocol` events —
  nothing else.
- The two adapters differ in transport on purpose: OpenCode is HTTP+SSE against
  a spawned `opencode serve`, omp is newline-delimited JSON over a child's stdio
  (`omp --mode rpc`). Nothing above them may learn that difference — if a change
  makes the orchestrator or the UI aware of *how* an adapter talks, the neutral
  protocol has been breached.
- `internal/relay` translates HTTP ⇄ orchestrator calls and holds **no business logic**.
- `main.go` is the only composition root: it is the one place that maps a
  `store.Project.Provider` to a concrete adapter. Adding adapter #3 means editing
  that switch and nothing else.

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
  - **The Wails signal handler routes correctly but does not finish in time.**
    It handles SIGINT+SIGTERM and reaches `ServiceShutdown()` — that part was
    verified by reading the code and it is true. But the graceful teardown does
    not complete before the process exits, so the managed agent servers survive
    as orphans still holding their ports. Measured twice on 2026-09-29 with
    opencode 2.0.18: after SIGTERM the app was gone and `opencode serve` was
    still listening. `main.go` therefore installs `shutdownOnSignal`, which
    stops the adapters synchronously and then exits. Do not "simplify" it away by
    trusting the default handler — that is what this note is for.
  - `frontend/bindings/` is generated. Never hand-edit; regenerate via
    `wails3 generate bindings` / build.
  - `Events.Off(name)` removes **all** listeners for that name — keep and call the
    unsubscribe function returned by `Events.On` instead.
  - The webview origin is `wails://localhost` (macOS/Linux), so cross-origin fetch to
    `http://127.0.0.1` needs the `/agent` route-mounted relay — not direct webview calls.
- **OpenCode traps** — the adapter speaks **v2 only** (`internal/opencode`,
  contract verified against **2.0.18** on 2026-09-29):
  - Events are `session.*`, never v1's `message.*`. Text streams as
    `session.text.{started,delta,ended}` (and `session.reasoning.*`), tools as
    `session.tool.{called,input.started,input.ended,progress,success,error}`,
    accounting in `session.step.ended` / `session.usage.updated`. **There is no
    `message.part.updated` and no `message.part.delta`** — do not write code
    against v1 names.
  - **Part identity is synthesized, not server-given.** Text and reasoning parts
    use `<assistantMessageID>:text` and `:reasoning` (`streamPart` in
    `translate_v2.go`); tool parts carry the server's own id. Hydration and the
    live stream must agree on these ids or reopening a session silently loses the
    transcript.
  - Updates are full replacements; deltas are an optimization only. The reducer
    upserts by part id, which is what makes a missed frame self-heal (FR-20).
  - Responses are envelope-wrapped: single objects arrive as `{"data": …}`.
    `GET /api/session/{id}/message` items carry **`type`** (`user|assistant|system|idle`),
    not `role`; assistant text lives in `content[]`, user text at top-level `text`.
  - The prompt body is `{text, files[]}` — **`parts[]` is rejected**. Each file
    must be an absolute `file://` URI; relative paths fail server-side.
  - Every route needs Basic auth: user `opencode`, password from the
    `server password <random>` line the managed server prints at boot.
  - **There is no `/api/health` in GA.** Beta had it; GA answers 404 and the
    replacement is `GET /api/info` → `{version, pid, …}`. A probe that requires
    `{healthy, true}` fails silently and leaves the catalog empty — that is
    exactly how waku 0.1.19 ended up with an empty model picker on opencode
    2.0.6+ (their issue #242, still open). The GA binary is a single `opencode`,
    not a separate `opencode2`. Likewise: the command body is `{name, text}`
    (beta's `{command, text}` gets a 400) and rename is `PATCH /api/session/{id}
    {title}` (the dedicated endpoint is 404).
  - Unknown events are forward compatibility, not errors (NFR-3): the server emits
    a constant stream of `*.updated`, `mcp.*`, `model.*`, `provider.*` noise. Skip
    them silently — but remember that a silently skipped event is a part the user
    never sees, so when something goes missing, check the event name against the
    spec before suspecting the UI.
  - **Fixtures cannot detect drift.** `internal/opencode/testdata/*.sse` are real
    captures with the version in the header comment; they freeze a shape, they do
    not tell you a newer server moved it. That is what
    `CIRCULOGO_OPENCODE_LIVE=1 go test ./internal/opencode/ -run TestLiveSmoke`
    is for — run it after any opencode version bump, before trusting the fixtures.
- **omp traps** (`internal/omp`, verified against **18.4.2**):
  - Frames are NDJSON over the child's stdio, and protocol **v2** frames are
    chunked — oversized stdout arrives as reassembled chunks against an advertised
    ceiling, not as one line. Validate the full chunk, do not trust a prefix.
  - The wire carries **no message id** and streamed content grows, so message ids
    are chronological ordinals per role (`a1`, `u1`, …). They must stay identical
    across live frames, `get_messages` hydration, and adapter restarts — same
    invariant as the OpenCode side, different mechanism.
  - **No permission round-trip exists over RPC.** Tool approvals resolve inside
    omp per its `tools.approvalMode`, headless prompts fail closed, and
    `ReplyPermission` returns an explicit error. Do not "fix" this by inventing a
    neutral event; the gap is upstream and is recorded in trd.md §3A.
  - Live smoke: `CIRCULOGO_OMP_LIVE=1 go test ./internal/omp/ -run TestLiveSmoke`.
- **[adjusted]** Original rule "UI does not import adapters. Adapters do not UI."
  is enforced per the Architecture invariants above.

## Project map

- `docs/` — PRD, TRD, UX, Flow, UI spec, implementation plan. Change docs in the same
  PR as the behavior they describe.
- `internal/agent/protocol` — the neutral wire contract. Changing it means changing
  `frontend/src/lib/agent/protocol.ts` and reducer tests in the same commit.
- Every `protocol.Envelope` carries `seq` and `epoch`, assigned by the orchestrator
  as it fans events in. **They are not decoration.** `seq` restarts at 1 on every
  process start, so a client that trusted a bare sequence would accept a fresh run's
  event 1 as the continuation of an old run's event 1; `epoch` is what makes a
  resume across a restart detectable. The relay answers a resume with a `replay`
  control frame (`full` / `resumed` / `stale` / `gap`) *before* any event, and the
  client refetches only when the status is not `resumed`. A cursor older than the
  ring replays nothing — a partial tail that looks complete is worse than none.
  Corollary: **full-part updates are the self-healing mechanism for part content
  only.** A dropped `permission.asked`, `session.status`, `adapter.status`, retry
  or form request strands the UI no matter how idempotent the parts are.
- `internal/agent` — the `Adapter` interface both backends implement. It is
  deliberately the smallest useful surface; grow it only when a real backend needs it.
- `internal/opencode/` — adapter #1. The only place allowed to speak OpenCode.
  HTTP+SSE, v2 API only.
- `internal/omp/` — adapter #2 (`omp --mode rpc`, stdio NDJSON). The only place
  allowed to speak omp. Same contract, different transport.
- `internal/orchestrator/` — project registry, adapter lifecycle fan-in, event hub.
  Knows adapters only as `agent.Adapter`.
- `internal/relay/` — the `/agent` HTTP+SSE surface the webview talks to.
- `internal/store/` — `settings.json` persistence (projects, provider, mode). Atomic writes.
- `internal/term/` — PTY terminals (creack/pty), one per project. I/O rides SSE +
  write/resize POSTs; it is transport-agnostic to the provider, not to the relay.
- `internal/checkpoint/` — per-turn workspace states stored as git refs under
  `refs/circulo/`, so a turn can be rewound without asking the backend to
  cooperate (omp has no equivalent of OpenCode's `revert`). Two invariants the
  tests protect: a capture must not touch the user's index, worktree or stash
  list, and a rewind must restore the **worktree** without moving HEAD.
