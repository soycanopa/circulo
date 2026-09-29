# Circulo — Implementation Plan

- **Status:** Draft v0.1 (2026-09-02); amended 2026-09-18 (Circulo pivot + outstanding list, below)
- **Related:** [TRD](trd.md) · [PRD](prd.md)
- **Workflow:** every phase = one branch `feature/<phase>` off `main`, granular commits
  (one logical change each), tests land with the code they cover. Never commit to `main`
  directly (project rule).

## Phase 0 — Foundation docs ✅ (this commit series)

`docs/{prd,trd,ux,flow,ui,implement}.md` + `AGENTS.md`. DoD: docs review-able, AGENTS.md
contains the adjusted rule set.

## Phase 1 — Scaffold ✅ (merged)

| # | Commit | Content | DoD |
|---|---|---|---|
| 1.1 | `chore: git init + ignore` | .gitignore (Go, node, wails build, .scratch) | clean status |
| 1.2 | `chore: wails3 react-ts scaffold` | `wails3 init -n circulogo -t react` output, `go mod tidy`, window options (1280×800, dark bg, mac hidden-inset titlebar) | `wails3 dev` opens window with template page |
| 1.3 | `feat(frontend): tailwind theme tokens + shadcn init` | shadcn init, theme vars from UI.md §1, dark default | Button renders themed |
| 1.4 | `test: vitest + go test wiring` | vitest config + sample, `make test` target (go test ./... + vitest run) | both suites green |
| 1.5 | `docs: AGENTS.md guardrails in template` | ensure lint configs reflect rules | lint passes |

## Phase 2 — Protocol + OpenCode adapter ✅ (merged) **tests first**

| # | Commit | Content |
|---|---|---|
| 2.1 | `feat(protocol): neutral events + parts` | `internal/agent/protocol`: envelope, event types, Part union, JSON camelCase tags, decode-and-skip-unknown helpers |
| 2.2 | `test(opencode): captured SSE fixtures` | `internal/opencode/testdata/*.sse` (real events from 1.18.25 smoke test: server.connected, session.created/updated/status busy+retry, message.updated, part.updated for text/reasoning/tool ×4 states/step-start/step-finish/patch, message.part.delta, permission.asked/replied, session.error) |
| 2.3 | `feat(opencode): SSE frame parser` | frame reader (data-only lines, comments, multi-line safety) + tests |
| 2.4 | `feat(opencode): wire types + decoder` | OC event/message/part structs (from `GET /doc` of 1.18.25), unknown-tolerant decoding + fixture tests |
| 2.5 | `feat(opencode): translate to neutral` | translation table (TRD §5.1) + table-driven tests |
| 2.6 | `feat(opencode): HTTP client` | create/list/rename/delete session, history, prompt_async, abort, permission reply, agents, providers, health; Basic-auth option; `httptest` server tests |
| 2.7 | `feat(opencode): process manager` | spawn managed server (free port, cwd, health poll ≤10s), attach mode, kill on stop, stderr ring buffer; lifecycle test (skip if no `opencode` binary) |
| 2.8 | `feat(agent): AgentAdapter interface` | `internal/agent`: interface (Start/Stop/Sessions/CreateSession/Prompt/Abort/ReplyPermission/Meta + Events() channel), compile-time check that opencode.Adapter satisfies it |

**DoD Phase 2:** `go test ./...` green with no opencode binary present (fixtures +
httptest); with binary present, lifecycle test green. No Wails imports in
`internal/{agent,opencode}` (assert via `go list -deps` in test or CI grep).

## Phase 3 — Orchestrator + relay ✅ (merged; E2E verified with real opencode serve via CIRCULOGO_DEBUG_ADDR)

| # | Commit | Content |
|---|---|---|
| 3.1 | `feat(store): settings persistence` | projects CRUD → settings.json (OS config dir), atomic write |
| 3.2 | `feat(orchestrator): project registry` | AddProject (managed/attach), status fan-in, event hub (subscribers, per-client buffer), graceful Shutdown |
| 3.3 | `feat(relay): /agent HTTP API` | endpoints per TRD §4, SSE writer (flush per event, heartbeat comment 15s), JSON errors |
| 3.4 | `feat(main): wire services` | register orchestrator service + relay route in Wails app; single-instance; window options |
| 3.5 | `test: relay integration` | httptest E2E: fake OC server → adapter → relay SSE → assert neutral events; manual smoke with real `opencode serve` |

**DoD Phase 3:** `curl -N localhost…/agent/sse` (from a debug listener or in-app test)
shows neutral events for a real prompt; `wails3 dev` window still loads; fallback to
`app.Event.Emit` decided/documented if route interception fails in dev.

## Phase 4 — UI shell ✅ (merged into `feature/frontend`)

| # | Commit | Content |
|---|---|---|
| 4.1 | `feat(frontend): protocol.ts + sse client` | TS mirror of protocol, EventSource wrapper w/ backoff reconnect (Flow §7), zustand stores |
| 4.2 | `feat(frontend): reducer + tests` | events→SessionState per TRD §6 + golden-stream vitest suite |
| 4.3 | `feat(sidebar): projects + sessions` | ProjectSwitcher, SessionList (date groups, status icons, rename/delete), empty state (FR-1/3/5/6/7) |
| 4.4 | `feat(composer): input + pickers` | textarea, send, model/agent pickers from /meta, optimistic user message (FR-10) |
| 4.5 | `feat(app-shell): layout + states` | AppShell, TitleBar, adapter status dots + banners (FR-19) |

**DoD Phase 4:** add real project via dialog → server starts (dot green) → session list
populates; creating a session works; no chat rendering yet.

## Phase 5 — Chat: parts + live streaming ✅ (merged into `feature/frontend`; streaming verified via relay E2E, in-window prompt verification pending manual pass)

| # | Commit | Content |
|---|---|---|
| 5.1 | `feat(chat): transcript + message list` | hydration on open (FR-8), pin-to-bottom + pill (UX §5) |
| 5.2 | `feat(chat): markdown view` | MarkdownView per UI.md §4, code header + copy + wrap |
| 5.3 | `feat(chat): part renderers` | ReasoningPart (live-open/collapse), ToolCard(+group fold), PatchCard, SubtaskPill, TurnFooter (FR-11/12/13) |
| 5.4 | `feat(chat): live streaming polish` | 15 Hz flush, working timer, reduced-motion, streaming memo (NFR-1/2) |
| 5.5 | `test(chat): reducer→render golden cases` | hydration+live overlap, delta-then-full, out-of-order |

**DoD Phase 5:** real prompt streams text+reasoning+tools live; history renders
identically to live; usage footer correct.

## Phase 6 — Permissions, abort, errors ✅ (merged into `feature/frontend`; needs the manual E2E checklist pass)

| # | Commit | Content |
|---|---|---|
| 6.1 | `feat(permissions): cards + replies` | PermissionCard stack, 3 actions, resolved-by-event removal (FR-16) |
| 6.2 | `feat(control): abort + retry state` | Stop button/Esc, retry banner from session.status retry (FR-17/18) |
| 6.3 | `feat(errors): inline error blocks + reconnect` | session.error rendering (FR-21), bridge reconnect resync (Flow §7) |
| 6.4 | `feat(sessions): session switch race safety` | hydration/live interleave tests (Flow §5) |

**DoD Phase 6:** full PRD functional checklist passes (below).

## Amendment 2026-09-18 — Circulo design pivot (recorded retroactively)

After Phases 5–6 merged, the visual layer pivoted to replicate the **Circulo
Paper** design (`feature/circulo-design` + app-bar/polish branches, merged to
`main`): theme tokens, sidebar geometry, an app bar, and chat metrics.

- [ui.md](ui.md) is **partially superseded**: the layout skeleton and component
  inventory still apply, but palette/geometry follow the Circulo replica.
  Re-sync ui.md before starting Phase 7 (it still says "neutral zinc, violet
  rejected" — no longer true of the code).
- `AppBar` (session breadcrumb + close action) exists but was never specified
  in ui.md.
- Behavior contracts ([ux.md](ux.md), [flow.md](flow.md)) are unaffected.

## Amendment 2026-09-18 (II) — OpenCode v2 migration + post-pivot feature wave

Recorded retroactively; both shipped on the branch stack now pushed to origin
(canonical repo: `github.com/soycanopa/circulo` — the project that previously lived
there is preserved under `legacy/*`).

**OpenCode v1 → v2 migration — code-complete.** Phases 0–5 of
[opencode-v2-migration.md](opencode-v2-migration.md): the adapter speaks only v2
(Basic auth captured from `serve` output, `/openapi.json` as the spec source, new
endpoint/event map) and the app runs against **2.0.8** via
`CIRCULOGO_OPENCODE_BIN`. Handoff state and owner gates live in
[opencode-v2-phase6-e2e.md](opencode-v2-phase6-e2e.md).

**Feature wave on top of the Circulo replica** (details in the commit history of
`feature/opencode-v2`):

- PTY terminals per project — `internal/term` (creack/pty), one tabbed xterm.js
  surface below the chat/composer cards; I/O over SSE + write/resize POSTs.
- `question` tool → ApprovalCard: one question at a time, 1/N odometer, radio
  auto-advance (v2 Forms events, not permissions).
- Thinking trace: one collapsible per work type (Reasoning/Search/Coding/Tools)
  with pixel-loader headers.
- CodePanel: fenced code with line numbers + syntax coloring; unified `diff`
  with old/new gutters and word-level pairing.
- `circulogo-flow` blocks render as the dotted flowchart canvas.
- AssistantText streams word-by-word (55ms reveal) and settles to copyable text.
- Session targeting: always-visible strip; new-session mode selects project +
  branch up front and the session materializes on the first message.
- Resizable sidebar (200–480px, persisted), animated pixel-glow texture,
  transcript edge fades, white Send/Stop pair.
- Per-session markdown formatting instruction via the instructions-entries API
  (`circulogo-format`); the v2 `instructions` config key is not read by the server.

The ui.md re-sync debt from the pivot amendment still stands and now also covers
the terminal surface and app-bar actions.

## Phase 7 — omp provider (branch `feature/omp-provider`) ✅ code-complete

Adapter #2, built on the owner's explicit go-ahead (the AGENTS.md adapter-#2 gate —
E2E checklist pass — is hereby overridden by owner decision for omp; the checklist
below still stands for a formal release pass). Target: [omp](https://omp.sh)
(can1357/oh-my-pi), which speaks `omp --mode rpc` over stdio NDJSON instead of
HTTP+SSE — proving the neutral protocol holds for a fundamentally different
transport, not just a second HTTP API.

| # | Commit | Content |
|---|---|---|
| 7.1 | `feat(projects): plumb provider selection` | `store.Project.Provider` (empty = opencode, old settings valid), AddProject validation (opencode\|omp; omp rejects attach), ProjectView/`protocol.ts` mirror, `api.addProject` provider arg |
| 7.2 | `feat(omp): adapter` | stdio RPC client (ready handshake, protocol v2 chunk reassembly, id correlation), event translation (deltas, tool parts, finish accounting, retry status, extension-UI forms), ordinal message identity, disk session discovery, fixtures + tests (`internal/omp/`), provider switch in `main.go` (`CIRCULOGO_OMP_BIN`) |
| 7.3 | `feat(ui): provider identity` | omp π mark, provider-aware banner copy, model-picker rail from the project provider, New-project provider menu, null-list hardening in `loadMeta`, gated live smoke (`CIRCULOGO_OMP_LIVE=1`) |

**Capability map (omp → Circulo).** Sessions (list/create/rename/delete), history
hydration with live-identical part ids, prompt/abort with steering when busy,
thinking-level variants, 264-model catalog with the composer default from omp's own
state, git VCS read locally, retry status, extension-UI forms (select/confirm/input).
**Known gaps:** no permission round-trip over RPC (approvals follow omp's
`tools.approvalMode`; headless prompts fail closed — `ReplyPermission` errors), no
named agents, no branch pinning (no instruction channel), compaction/subagent frames
have no neutral surface yet. Details in [trd.md §3A](trd.md).

**Verification.** `go test ./internal/omp/` (fixtures from real omp 18.2.8 captures,
no binary needed) + `CIRCULOGO_OMP_LIVE=1 go test ./internal/omp/ -run TestLiveSmoke`
(real child: handshake → catalog → prompt stream → hydration → abort → clean stop);
browser E2E against the debug listener (add omp project → banner → streaming
transcript → model picker) run on 2026-09-22.

## Amendment 2026-09-29 — provider version audit (branch `feature/version-audit`)

The adapters were verified against **opencode 2.0.8** and **omp 18.2.8** captures,
but the binaries the app actually spawns had moved on. Ten opencode minors and two
omp minors is enough to move an event or a field, and the adapters tolerate unknown
input by design (NFR-3), so drift would surface as a silently empty transcript
rather than an error. Audited on 2026-09-29: **no drift**.

- **omp 18.4.2** — `CIRCULOGO_OMP_LIVE=1 go test ./internal/omp/ -run TestLiveSmoke`
  green end to end (276-model catalog, default `zai/glm-5.3-flash`, real turn,
  hydration, abort, clean stop). Doc said 264 models: only the catalog grew.
- **opencode 2.0.18** — all **19** endpoints the adapter calls are present with
  identical method + path in the 2.0.18 spec (138 operations total); the streaming
  path, the usage accounting and the hydration shape are unchanged. Additive only:
  `session.inbox.delivered`, `session.step.streamed` and
  `session.instructions.updated` are new and correctly ignored, as is the usual
  mcp/model/provider noise. The prompt body is still `{text, files}` — `parts[]` is
  rejected — and `files[]` now carries `skills`, `delivery`, `resume`, `metadata`
  that the adapter does not use yet.
- **Guard added.** `internal/opencode/live_test.go` mirrors the omp smoke
  (`CIRCULOGO_OPENCODE_LIVE=1`): readiness, a real turn, and the §3.3
  hydration-parity assertion, logging the server version it ran against. The
  managed integration test now resolves the binary from PATH (what `main.go`
  spawns) and skips with a reason when the major is not 2, instead of silently
  passing against the pinned 2.0.8 side-by-side install.
- **Spec URL corrected.** On v2 `GET /doc` serves the webview HTML; the machine
  spec is `GET /openapi.json`. AGENTS.md pointed at `/doc`.

## Outstanding before Phase 8 (manual passes, consolidated)

1. v2 handoff: owner E2E checklist (§3 of
   [opencode-v2-phase6-e2e.md](opencode-v2-phase6-e2e.md)). Of the five §5
   decisions, **only §5.3 remains open** (import v1 sessions or start clean):
   §5.1 done (AGENTS.md rewritten for v2), §5.2 retracted (Wails already routes
   SIGTERM to `ServiceShutdown` — no handler needed), §5.4 moot (the global CLI
   is v2), §5.5 done (the stack is on `main`).
2. Full E2E manual checklist pass (below) — the project gate in AGENTS.md for
   remote work; the adapter-#2 clause was consumed by the omp provider (Phase 7).
   The provider half of it is now covered by the two live smokes above; what
   remains is the window-level pass (cards, banners, terminal, reconnect).
3. ~~Re-sync [ui.md](ui.md) with the shipped Circulo replica, terminals and app-bar
   actions.~~ **Done** (2026-09-29) — rewritten from the build as v0.2, split into
   durable contract (§2–§4) and dated inventory (§5). Debt from both earlier
   amendments is closed.

## Phase 8 — Polish + packaging (branch `feature/release-v0`)

- Perf pass (NFR-2 profiling with 50+ tool-call session), a11y pass (UX §8 —
  the open items are sidebar ↑↓/Enter session navigation and a WCAG AA contrast
  audit of the dark theme), icon + app name, `wails3 build` + `wails3 package`
  (.app), orphan-process audit (NFR-4).
- **No light theme in v0** (owner decision 2026-09-29): the build is deliberately
  dark-first against the Circulo Paper tokens and [ux.md](ux.md) §8 was corrected
  to stop promising one. A light theme is a v1 candidate.
- **v0 exit checklist:** all FR-1…FR-21 demonstrated once in a recorded E2E session
  (prompt → stream → permission → abort → error → history).

## E2E manual checklist (Phase 3+, repeat per release)

1. `mkdir /tmp/e2e-proj && cd /tmp/e2e-proj && git init` → add as project in app.
2. New session → prompt "list files, then reply DONE" → observe: tool card runs, text streams.
3. Prompt "create notes.txt with 'hi'" → approve edit permission via card.
4. Abort mid-turn on a long prompt → partial content stays, status idle.
5. Kill `opencode serve` externally → project dot red with detail → Retry works.
6. Quit app → `pgrep opencode` empty (no orphans).
7. Reopen → history hydrates for the same session.

## Amendment 2026-09-29 (II) — gap analysis against waku

Circulo and [waku](https://github.com/egoist/waku) sit in the same product
category, so waku is a legitimate reference. Three constraints on that reference,
decided by the owner on 2026-09-29:

1. **waku is GPL-3.0-only. Ideas and UX patterns yes, code never** — copying a
   file would force Circulo under GPL-3.0. This is now a rule in AGENTS.md.
2. **waku's architecture is not portable.** Its `waku-daemon` / `waku-protocol` /
   `waku-core` split exists because it drives 9 providers plus a browser client
   and remote access. Circulo is one process with a route-mounted relay, and the
   neutral protocol already covers what the daemon covers there. A daemon would
   break the single-composition-root invariant (main.go) with no proven need.
   WebSocket RPC is likewise the wrong call for a one-way event stream.
3. **The gaps below were each verified against the code**, not inferred from
   waku's README. Where Circulo already has something, it says so.

Also checked and **no action needed**: waku's oversized-catalog bug (its omp probe
skipped chunk reassembly, PR #153). Circulo's `Meta` reads through the same
reassembling framed reader, and the 18.4.2 live smoke returns a 276-model catalog
against the real binary.

And a position worth keeping: waku 0.1.19 is **broken against OpenCode GA** —
issue #242, open 11 days, 0 reactions, unfixed in main. Its probe hits
`GET /api/health`, which does not exist in GA, so the model picker silently
empties. Verified against the live 2.0.18 spec: `/api/health` is absent,
`/api/info` is present, the command body requires `{name, text}` and rename is a
`PATCH` — Circulo already does the right thing on all four counts, and the live
smoke added in the version audit now keeps it that way.

## Phase 9 — Stream integrity + checkpoints (post-E2E, before calling v0.1 stable)

Scope line set by the owner 2026-09-29: P0–P4 land before v0.1; P5–P7 are v1.
Sequenced P0 first because the later ones that touch the protocol ride on it.

**P0 — Sequenced envelopes with a replay cursor.** ✅ done (branch
`feature/stream-integrity`). The reason this was not cosmetic: the only defense
against a dropped frame was that parts are full replacements, so the reducer
self-heals (FR-20) — but that covers *part content only*. It does not cover
`permission.asked`, `session.status`, `adapter.status`, retry or form requests,
and those are the events that strand a UI. What shipped:
- `protocol.Envelope` carries `seq` + `epoch`; `protocol.ts` and the SSE client
  changed with it, per the AGENTS.md contract rule.
- The orchestrator stamps both, and `Subscribe(protocol.Cursor)` returns a
  `ReplayStatus` (`full` / `resumed` / `stale` / `gap`) with the channel.
- The relay sends that status as the first SSE control frame and writes an `id:`
  per event, so a native `EventSource` reconnect also carries `Last-Event-ID`.
- The client owns the cursor and rebuilds the `EventSource` on reconnect, dedupes
  on the `(epoch, seq)` pair rather than `seq` alone, and refetches **only** when
  the status is not `resumed`. A one-second blip no longer costs four REST round
  trips including the model catalog.
- **A hole the cursor exposed, now closed:** a subscriber that stopped reading had
  its events silently dropped. It is now dropped as a subscription — the channel
  closes, the relay ends the response, and the client reconnects with a cursor
  that recovers the exact tail.
- Verified end to end over HTTP, not just per-unit: connect, cut, emit three
  events while away, reconnect, receive exactly those three.

**P1 — Checkpoints as per-turn git refs.** ✅ done. Replaced backlog item 5
rather than adding to it: that item assumed OpenCode's `revert`/`unrevert`, which
ties checkpoints to one provider. Refs work identically for omp, where no such
support exists, and need nothing from the harness.
- `internal/checkpoint` owns the git plumbing. Refs live under `refs/circulo/`,
  outside `refs/heads` and `refs/tags`, named `session-<id>-turn-<n>-<start|end>`.
- Snapshots go through an **alternate index**, not `git stash create`: stash
  leaves everything alone but omits untracked files, and the files an agent
  creates are the whole point. The user's index, worktree and stash list are
  asserted unchanged after a capture.
- Turn boundaries are detected in the orchestrator from `session.status`
  (busy→idle), because the relay hands prompts straight to the adapter — the
  orchestrator never sees them — while status is the one event both backends
  translate identically.
- **Rewind restores the worktree and leaves HEAD alone.** Moving the branch
  would discard commits the user made during the session. The pre-rewind state
  is checkpointed as a `guard` first, so the action is undoable.
- The relay splits preview from action: `rewind-preview` returns the diff and
  whether uncommitted work would be lost; `rewind` acts and requires an explicit
  turn number so it can never default to "the newest one". Both routes are on
  the project, not the adapter, so rewind keeps working when the backend is down.
- 21 package tests against real git repositories, 9 wiring tests, 6 relay tests,
  8 frontend tests. One bug the wiring test caught: files an agent creates are
  untracked and `git read-tree` does not remove them, so a rewind that only
  restored tracked files would have undone almost nothing.

**P2 — Declared capabilities on the runtime.** `supports_steer`,
`supports_checkpoints`, `supports_branch_pinning` and friends, advertised by the
adapter in its runtime response. Today behavior is hardcoded per provider and the
UI branches on the provider name in places. This is *stricter* than the current
invariant — it removes the provider name as crossing vocabulary — and it is what
lets queue/steer work for OpenCode the way it already does for omp.

**P3 — Composer drafts per target.** Keyed by conversation target and restored on
return, so switching sessions no longer discards what was being typed.

**P4 — Scope on slash commands.** The useful idea is that **presentation order and
resolution precedence are two different orderings**: the picker ranks builtin →
project → user → skill, while project and user commands may still override a
less-specific command of the same name. Circulo's slash commands have no scope.

## Out-of-scope backlog (v1+, in priority order)

1. omp tool-call granularity. The OpenCode side is done — the adapter already
   handles `session.tool.input.started/ended` and `session.tool.progress`, and
   the neutral protocol has a `part.delta` fast path behind a full `part.updated`.
   omp only emits `toolcall_end`, so arguments appear when the call completes;
   incremental input depends on what omp's RPC actually exposes.
2. `@` file mentions + image attachments in composer (protocol already has file parts).
3. Syntax highlighting behind MarkdownView; diff syntax tone.
4. Third adapter (Claude CLI stream-json) — the omp adapter already proved the neutral protocol across transports (stdio vs HTTP+SSE).
5. ~~Checkpoints/revert UI~~ → **shipped as P1** in Phase 9: per-turn git refs
   instead of OpenCode's provider-specific `revert`/`unrevert`. Session fork stays
   here (OpenCode now takes `{before: msgID}` on `POST /api/session/{id}/fork`).
6. Remote access (Tailcat) — design frozen in [remote.md](remote.md); implementation blocked by project rule until local protocol is stable.
7. **Projectless tasks** (waku P5): auto-create a workspace under a local dated
   directory for a quick question, instead of demanding a folder up front. Fits
   the local-first model; Circulo has no such path today.
8. **Search within session history** (waku P6): message search, complementing the
   existing `@` file search.
9. **Plan quota / usage history** (waku P7): subscription-remaining display. Circulo
   shows per-turn usage only.
10. Auto-update (waku ships signed .dmg + updater). Packaging exists; the updater is
    a separate concern, after v0.1.
