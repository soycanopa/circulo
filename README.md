# Circulo

A local-first native desktop app for working with coding-agent CLIs.

One window for projects, independent sessions, and live work: streaming transcripts, tool calls, reasoning, model and reasoning-effort controls, mid-turn steering, and conversation-aware Git checkpoints. Everything stays on the machine. No account. No cloud of its own.

Circulo is **inspired by** [Waku](https://github.com/egoist/waku). The product idea, the calm layout, and the way you watch an agent work are the reference. Circulo is its own application, built entirely in **Rust + GPUI** — the same GPU-accelerated UI framework behind Zed. Waku is GPL-3.0; none of its code is copied here.

**Status:** the interface is in place and the daemon runs end to end. A client connects, opens a session, sends a prompt and receives a streamed turn — reasoning, tool calls and text — through the protocol. The demo provider produces that turn; the desktop still renders sample data and gets wired to the daemon next. See [the implementation plan](docs/implement.md).

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
| Platform | Desktop first. macOS is the primary target. Linux and Windows through GPUI. Mobile comes after v1.0. |

Outside v1.0: mobile, remote access, and computer-use or browser automation.

## How it is built

Everything is a separate piece with a one-way dependency direction. A new provider, a new panel, or a future mobile client can be added without rewriting the rest.

```
┌─────────────────────────────────────┐
│  UI (GPUI app)                      │
│  presentation only · protocol client│
└─────────────────┬───────────────────┘
                  │ versioned protocol
                  ▼
┌─────────────────────────────────────┐
│  Core process                       │
│  projects, sessions, Git, storage   │
└─────────────────────────────────────┘
         │
         ├── Provider crates (one per agent, behind the Driver trait)
         ├── Protocol crate (the only shared contract)
         ├── Persistence crate
         └── Git crate
```

The UI never talks to a CLI. Providers never import the UI or the core. The protocol crate is the only shared contract. The core owns state, Git, and processes.

| Crate | Owns |
|---|---|
| `crates/protocol` | The neutral wire contract: messages, parts, framing. Shared by every client and the core. |
| `crates/core` | Sessions, turn orchestration, provider lifecycle. |
| `crates/daemon` | The daemon process: loopback listener, handshake, protocol loop. |
| `crates/client` | The protocol client used by the desktop, and later by mobile and remote. |
| `crates/providers/driver` | The `Driver` trait each provider implements. |
| `crates/providers/demo` | A provider that streams a canned turn, so the pipe is proven before a real CLI. |
| `crates/persistence` | SQLite, settings and blobs. Used only by the core. |
| `crates/git` | Worktrees and conversation-aware checkpoints. Used only by the core. |
| `crates/ui` | GPUI views: sidebar, transcript, composer, right panel. Presentation only. |
| `crates/app` | Composition root: wires the pieces and owns the window. |

## Requirements

- macOS with Xcode and its command line tools (GPUI renders with Metal).
- Rust stable, installed with [rustup](https://rustup.rs). The repository pins the `stable` channel in `rust-toolchain.toml`.

## Build and run

```sh
cargo run -p circulo          # the desktop window
cargo run -p circulo-daemon   # the daemon: prints its address and token, then serves
```

The daemon is the core process. It prints one line with its loopback address and token; a client presents both in the handshake. Remote and mobile are clients of the same daemon, so nothing needs to move when they arrive.

Checks to run before contributing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Documentation

| Doc | What it decides |
|---|---|
| [Product requirements](docs/prd.md) | Vision, scope, and success criteria for v1.0 |
| [Technical requirements](docs/trd.md) | Stack, crate boundaries, persistence, and risks |
| [UX](docs/ux.md) | Principles, layout, and interactions |
| [UI](docs/ui.md) | Visual language, surfaces, and tokens |
| [Flows](docs/flow.md) | Start, prompt, steering, permissions, checkpoints, quit |
| [Implementation plan](docs/implement.md) | Phases from foundation to the v1.0 tag |
| [AGENTS.md](AGENTS.md) | Rules for anyone changing this repository |

## Repository

`main` is protected, so every change lands through a pull request — the owner's included — on a `feature/…`, `fix/…` or `docs/…` branch. Commit messages use `type(scope): summary`; CI runs `cargo fmt --check`, clippy and the test suite, and it must be green before merging. Merges keep the granular commits.

Earlier attempts are kept for reference, with their full history and tags: [`legacy2`](https://github.com/soycanopa/circulo/tree/legacy2) holds the Go/MyGo application, and [`legacy`](https://github.com/soycanopa/circulo/tree/legacy) holds the Wails + React application before it.

## License

MIT. Circulo is inspired by [Waku](https://github.com/egoist/waku), which is GPL-3.0; no Waku code is used here.
