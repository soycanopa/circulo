# circuloGo — Implementation Plan

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

## Outstanding before Phase 7 (manual passes, consolidated)

1. v2 handoff: owner E2E checklist (§3 of
   [opencode-v2-phase6-e2e.md](opencode-v2-phase6-e2e.md)) plus the §5 decisions
   still awaiting the owner.
2. Full E2E manual checklist pass (below) — the project gate in AGENTS.md for
   opening remote/adapter #2 work. (The phase-5 leftover — prompt verified inside
   the webview — is exercised in daily use; the formal checklist pass is what
   remains.)
3. Re-sync [ui.md](ui.md) with the shipped Circulo replica, terminals and app-bar
   actions (debt recorded in both amendments above).

## Phase 7 — Polish + packaging (branch `feature/release-v0`)

- Perf pass (NFR-2 profiling with 50+ tool-call session), a11y pass (UX §8), light theme
  check, icon + app name, `wails3 build` + `wails3 package` (.app), orphan-process audit
  (NFR-4), version pinning audit.
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

## Out-of-scope backlog (v1+, in priority order)

1. v2 granular OpenCode events (`message.part.delta` everywhere, tool input streaming).
2. `@` file mentions + image attachments in composer (protocol already has file parts).
3. Syntax highlighting behind MarkdownView; diff syntax tone.
4. Second adapter (Claude CLI stream-json) — proves the neutral protocol.
5. Checkpoints/revert UI (OC supports revert/unrevert), session fork.
6. Remote access (Tailcat) — design frozen in [remote.md](remote.md); implementation blocked by project rule until local protocol is stable.
