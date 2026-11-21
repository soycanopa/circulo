# Circulo — Product Requirements Document (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Owner:** soycanopa
- **Related:** [TRD](trd.md) · [UX](ux.md) · [Flow](flow.md) · [UI](ui.md) · [Implementation Plan](implement.md)
- **Reference:** [egoist/waku](https://github.com/egoist/waku) — feature parity goal, no code copy (GPL-3.0)

## 1. Vision

Circulo is a local-first native desktop app for working with coding-agent CLIs.  
v1.0 goal: reach **feature parity with Waku’s initial capabilities** while being built entirely in **MyGo** (Go, native UI).

One native window where a developer manages projects, runs independent agent sessions, watches work live (streaming transcripts, tool calls, reasoning), switches models/reasoning effort, steers or queues messages mid-turn, and rewinds Git-backed work with conversation-aware checkpoints — all staying on the machine.

## 2. Target user

A developer who already uses one or more agent CLIs (Claude Code, Codex, OpenCode, Cursor CLI, Amp, Grok Build, Pi, etc.) and wants a single native control surface instead of living in terminals or separate web UIs.

## 3. Scope — v1.0 (Waku parity)

### In scope

| Area | Commitment |
|---|---|
| Agents | Multi-provider support with native protocols and session continuity (at least OpenCode, Claude Code, Codex CLI; architecture ready for the rest of Waku’s list) |
| Projects & sessions | Keep projects and independent agent sessions in one app; git worktrees per session where useful |
| Chat | Live streaming transcripts with beautiful markdown, tool activities, reasoning blocks |
| Control | Switch models, reasoning effort, and access modes from a shared interface |
| Steering & queue | Queue follow-up messages or steer an agent mid-turn while it keeps working |
| Git | Rewind Git-backed tasks with conversation-aware checkpoints |
| Local-first | Everything stored locally (no account, no remote service required) |
| Architecture | Strong modular separation (UI / core / providers / protocol / persistence as independent puzzle pieces) |
| Platform | Desktop first (macOS primary, Linux/Windows via MyGo). Mobile after v1.0 |

### Explicit non-goals for v1.0

- Mobile (iOS/Android) — comes after v1.0
- Full remote/Tailscale access (design later)
- Computer Use / browser automation (macOS-only in Waku; evaluate later)
- Exact pixel-perfect clone (we copy design intent and layout; owner will adjust later)

## 4. Functional requirements (high-level)

### Projects & sessions
- Add/remove projects by folder picker
- Multiple independent sessions per project
- Session list with status, grouping, rename, delete
- Git worktrees or equivalent isolation when beneficial

### Chat & streaming
- Send prompts; live streaming of text, reasoning, tool calls as distinct parts
- Markdown rendering with good code blocks
- Usage/token/cost footers where providers supply them

### Control surface
- Shared model / reasoning effort / access mode pickers
- Message queue + mid-turn steering
- Abort current turn

### Git & checkpoints
- Conversation-aware checkpoints and rewind for Git-backed tasks

### Persistence
- Local storage for projects, sessions, transcripts, attachments, settings
- No cloud dependency

## 5. Non-functional requirements

- **Streaming latency**: text deltas reach the UI quickly (< 100 ms p95 after receipt)
- **Smoothness**: long transcripts stay scrollable at high frame rates
- **Local-only**: binds to loopback; no telemetry
- **Startup**: interactive window in a few seconds
- **Robustness**: unknown provider events ignored gracefully

## 6. Success criteria for v1.0

- Can start a session with at least two different agent CLIs, stream work, switch models, steer/queue, and rewind a Git task — all inside the MyGo app.
- No dependency on external cloud services for core operation.
- Architecture clean enough that mobile can be added later with the same core.

## 7. Decisions (2026-11-20)

- **Native UI.** The window is MyGo `ui`, including the transcript (`ui/markdown`). A web frontend is not the app. A later Browser tab may embed `Window.NewWebView` under the native view. That does not move the transcript, sidebar, or composer into a page.
- **Core is its own process.** The UI process holds presentation state and the protocol client. Projects, sessions, providers, Git, and storage live in the core process. Packages stay independent inside that split.

## 8. Open questions

- Decided for the second provider: Muse Code (`muse serve`, MSP). Claude Code and Codex stay later.
- Final graphite token values once Waku screenshots are captured and adjusted.
