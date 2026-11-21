# Circulo

A local-first native desktop app for working with coding-agent CLIs.

One window for projects, independent sessions, and live work: streaming transcripts, tool calls, reasoning, model and reasoning-effort controls, mid-turn steering, and conversation-aware Git checkpoints. Everything stays on the machine. No account. No cloud of its own.

Circulo is **inspired by** [Waku](https://github.com/egoist/waku). The product idea, the calm layout, and the way you watch an agent work are the reference. Circulo is its own application, written in Go with [MyGo](https://mygo.run/). Waku is GPL-3.0; none of its code is copied here.

**Status:** v1.0 is specified and not yet implemented. This repository currently holds the product and technical documents.

## Who it is for

A developer who already uses one or more agent CLIs (Claude Code, Codex, OpenCode, Cursor CLI, and others) and wants a single native control surface instead of living in terminals or separate web UIs.

## What v1.0 covers

| Area | Commitment |
|---|---|
| Agents | Native protocols and session continuity. Day-one target: OpenCode, Claude Code, and Codex CLI, with room for more. |
| Projects and sessions | Several projects, independent sessions, and Git worktrees where isolation helps. |
| Chat | Live streaming of text, reasoning, and tool calls, with markdown. |
| Control | Shared model, reasoning-effort, and access-mode pickers. |
| Steering | Queue a follow-up or steer an agent mid-turn. |
| Git | Rewind Git-backed work with conversation-aware checkpoints. |
| Local-first | Projects, sessions, transcripts, attachments, and settings on disk. |
| Platform | Desktop first. macOS is the primary target. Linux and Windows through MyGo. Mobile comes after v1.0. |

Outside v1.0: mobile, remote access, and computer-use or browser automation.

## How it is built

MyGo (Go) for the shell and the window. The UI is native (`ui`, including `ui/markdown` for streaming replies). The core runs in its own process. A WebView is reserved for a later Browser tab, under the native view.

The system is split into independent pieces. Dependency direction is one way.

```
┌─────────────────────────────────────┐
│  UI (MyGo app)                      │
│  presentation only · protocol client│
└─────────────────┬───────────────────┘
                  │ versioned protocol
                  ▼
┌─────────────────────────────────────┐
│  Core process                       │
│  projects, sessions, Git, storage   │
└─────────────────────────────────────┘
         │
         ├── Provider adapters (one package each)
         ├── Protocol (the only shared contract)
         └── Persistence
```

The UI never talks to a CLI. Providers never import the UI. The protocol package is the only shared contract. Core owns state, Git, and processes.

## Run the prototype

The window is a native MyGo app, with a hidden title bar, a project sidebar, the transcript, and a composer. On launch it starts `cmd/core` as another process, bound to loopback. The core starts `opencode serve` when `opencode` is on `PATH`, stores projects and sessions in SQLite, and streams the reply into the transcript. Type a folder path, choose the agent, choose Add, then send a prompt. Enter sends; Shift+Enter adds a line; Escape stops a turn that is still running. The model and reasoning pickers switch the session. While a reply is streaming, Steer, Queue, and Stop are available. The sidebar chooses OpenCode or Muse Code before adding a folder. In a Git repository, Rewind restores the files and the local transcript to the checkpoint taken before a prompt. When Muse asks for approval, the card uses the choices Muse sent.

`cmd/core -demo` still plays a canned transcript and does not start OpenCode.

```sh
go tool mygo dev
```

To run the core yourself, start it and point the window at the address it prints:

```sh
go run ./cmd/core
CIRCULO_CORE_ADDR=127.0.0.1:PORT go tool mygo dev
```

## Documentation

| Doc | What it decides |
|---|---|
| [Product requirements](docs/prd.md) | Vision, scope, and success criteria for v1.0 |
| [Technical requirements](docs/trd.md) | Stack, module boundaries, persistence, and risks |
| [UX](docs/ux.md) | Principles, layout, and interactions |
| [UI](docs/ui.md) | Visual language, surfaces, and tokens |
| [Flows](docs/flow.md) | Start, prompt, steering, permissions, checkpoints, quit |
| [Implementation plan](docs/implement.md) | Phases from foundation to the v1.0 tag |
| [AGENTS.md](AGENTS.md) | Rules for anyone changing this repository |

## Repository

Work happens on branches (`feature/…`, `fix/…`, `docs/…`) and lands through small pull requests. Commit messages use `type(scope): summary`.

The previous desktop app (Go, Wails, and React) is preserved on the [`legacy`](https://github.com/soycanopa/circulo/tree/legacy) branch, including its tags.
