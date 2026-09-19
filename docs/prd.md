# Circulo — Product Requirements Document

- **Status:** Draft v0.1 (2026-09-02)
- **Owner:** soycanopa
- **Related:** [TRD](trd.md) · [UX](ux.md) · [Flow](flow.md) · [UI](ui.md) · [Implementation Plan](implement.md)

## 1. Vision

Circulo is a local-first desktop orchestrator for coding-agent CLIs — the same product
category as [waku](https://github.com/egoist/waku), built on a different foundation:
**Wails v3 + React** instead of Rust/GPUI, and **plain HTTP + SSE** instead of ACP.

One window where a developer manages **projects** (folders), runs **sessions** against
agent CLIs, watches the agent work **live** (reasoning, answer, tool calls as separate,
streaming parts), and answers **permission requests** — without dropping into a terminal.

## 2. Target user

A developer who already uses agent CLIs (OpenCode first) and wants a GUI that:

- Keeps one session per task, grouped by project, with searchable history.
- Streams the agent's work as it happens — not a transcript dumped at the end.
- Surfaces tool calls and file edits as first-class UI, not as a wall of text.
- Never sends code or context anywhere except the agent process on localhost.

## 3. Scope

### In scope (v0)

| Area | Commitment |
|---|---|
| Adapters | **OpenCode only** (`opencode serve`, HTTP+SSE, API v1) |
| Transport | HTTP + SSE to the agent CLI server. **No ACP.** |
| Network | **Local only** (127.0.0.1). No LAN, no Tailscale/remote until the local protocol is stable |
| Platform | macOS (Apple Silicon) first; Wails v3 keeps Windows/Linux attainable |
| Sessions | Create, list (grouped by date), rename, delete, open multiple sequentially |
| Chat | Live streaming: reasoning, answer and tool calls rendered as separate parts |
| Markdown | GFM rendering with readable code blocks (language label, copy, wrapping) |
| Permissions | In-chat permission cards: Allow once / Always allow / Deny |
| Control | Abort a running turn; model/agent selection per prompt |
| History | Hydrated from the OpenCode server on session open; app stores no chat DB in v0 |

### Out of scope (v0) — explicit non-goals

- Remote access (Tailscale/LAN), multi-user, cloud sync.
- A second adapter (Claude, Codex, …) — the adapter interface must *exist*, adapter #2 waits.
- Checkpoints/rewind, worktrees, session forking (OpenCode supports forking; UI later).
- Telemetry. Nothing leaves the machine except what the agent CLI itself already does.

## 4. Functional requirements

### Projects

- **FR-1** — Add a project by picking a local folder. Circulo starts an OpenCode server
  rooted at that folder (or attaches to an already-running server — see FR-2).
- **FR-2** — Two connection modes per project: **managed** (app spawns/stops
  `opencode serve`) and **attach** (user started the server; app connects by URL).
- **FR-3** — Remove a project. A managed server is stopped with it.
- **FR-4** — Project list and mode persist across app restarts (`settings.json`).

### Sessions

- **FR-5** — New session per project with a title (auto-title from the agent when available).
- **FR-6** — Sidebar lists sessions grouped by day (Today / Yesterday / This week / Older).
- **FR-7** — Rename and delete sessions.
- **FR-8** — Opening a session hydrates full message/parts history from the server.
- **FR-9** — Sessions show live status: idle / working / retrying / error.

### Chat

- **FR-10** — Send a prompt (Enter to send, Shift+Enter newline); multi-line supported.
- **FR-11** — Live streaming of the assistant turn: text deltas appear as they arrive;
  reasoning streams in a separate collapsible block; tool calls render as their own cards
  with pending → running → completed/error states and expandable input/output.
- **FR-12** — Assistant answer renders as GitHub-flavored Markdown; code blocks show the
  language, a copy button, and wrap long lines.
- **FR-13** — Per-turn usage footer: tokens (in/out/reasoning/cache) and cost, from
  `step-finish`/final message.
- **FR-14** — Session history is rendered with the same part model as live streaming
  (one reducer for both — see TRD §6).
- **FR-15** — `@` file mention in the composer attaches a file part (stretch for v0;
  protocol supports it).

### Permissions & control

- **FR-16** — Permission requests surface as a card pinned above the composer:
  what is being asked (command/edit), with Allow once / Always allow / Deny.
- **FR-17** — Abort button stops the current turn (`POST /session/{id}/abort`).
- **FR-18** — Retry state (provider 429/backoff) is visible: "Retrying (attempt 2)… next in Ns".

### Errors & connection

- **FR-19** — Adapter lifecycle is visible per project: starting → running → stopped/error,
  with the underlying cause when it fails (binary missing, port taken, crash).
- **FR-20** — The app's SSE connection to its backend auto-reconnects; missed OpenCode
  events are tolerated because parts arrive as full replacements (verified on 1.18.25).
- **FR-21** — Session errors (`session.error`) render inline in the transcript as an error
  block, never as a silent stall.

## 5. Non-functional requirements

- **NFR-1 Streaming latency** — a text delta reaches the screen < 100 ms p95 after the
  backend receives it. Render batching must cap at ~15 Hz; never batch on a timer > 100 ms.
- **NFR-2 Throughput** — a 5k-token turn must not drop the UI below 30 fps while scrolled
  to bottom; markdown re-parses only the last changed block (message-level memo).
- **NFR-3 Robustness** — unknown OpenCode event types and unknown part types are ignored
  gracefully (forward compatibility), never crash the reducer.
- **NFR-4 Isolation** — OpenCode processes are children of the app and are killed on quit;
  no orphaned `opencode serve` after normal exit.
- **NFR-5 Security** — everything binds to 127.0.0.1. If `OPENCODE_SERVER_PASSWORD` is set,
  the adapter sends HTTP Basic auth.
- **NFR-6 Startup** — app window interactive < 2 s; a managed OpenCode server becomes
  ready-typed within 10 s or the project shows an error state.

## 6. Success metrics (v0)

- Send a prompt in a new project in < 3 interactions from cold start.
- First visible token of a turn streams without manual refresh, 100% of sessions.
- Zero orphaned `opencode serve` processes after 20 consecutive app quits.
- A one-hour session with 50+ tool calls stays scrollable at 60 fps.

## 7. Open questions

- **Q1** — Auto session titles: OpenCode has a `title` agent; decide whether Circulo
  triggers it (costs one LLM call) or lets OpenCode's own title flow handle it. *Deferred
  until FR-5 lands; default = accept server-provided title, inline rename always available.*
- **Q2** — Multiple windows (one per project) vs single window with project switcher.
  *v0 = single window, sidebar project switcher; windows are a v1 candidate.*
