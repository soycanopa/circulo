# Circulo

A fast, native **macOS** desktop client for talking to AI agents.

Circulo is built for **marketers, product people, and designers** — not for people who live in a terminal. The goal is a clean, structured chat that feels instant, plus a simple way to keep work in sessions and (when you want) group those sessions into projects.

> Enough tools to do great work.

This repository is in **pre-MVP planning**. Product and engineering docs exist; the app binaries are not implemented yet. We build through [OpenSpec](https://openspec.dev/) changes, not by dumping an entire MVP into one branch.

## Why Circulo

Most agent clients are made for developers. Streaming is a wall of plain text. Organizing work means a flat dump of chats. The apps themselves often feel heavy.

Circulo is the opposite bet:

- A **rich chat**: Markdown, tool-call cards, diffs, and basic task lists — rendered while the agent is still streaming.
- **Sessions first.** A new session has no project. It lives in Circulo’s special **Sessions** folder and shows **No project** until you pick a project folder in the composer.
- **Two sidebar views.** **Sessions** is a flat list (name, time, project or “No project”). **Groups** shows your project folders and the sessions inside them.
- **Native and fast.** GPUI + Rust. No Electron shell. Custom window chrome (traffic lights live in the sidebar).
- **Modular.** The UI never talks to an agent CLI. A small local daemon owns persistence and adapters. The first adapter is OpenCode.

## Status

| Item | State |
| --- | --- |
| Product definition | Current (`Circulo-Project-Definition.md` v0.6) |
| PRD / TRD / UX / Flows / Implementation | Current in `docs/` |
| Engineering contract | `AGENTS.md` |
| Application code | Workspace scaffold only (no product behavior) |
| Platform | macOS first (Windows / Linux later) |
| MVP agent | OpenCode only |

## How it fits together

Circulo is **two of our processes**. OpenCode is a third, external process.

```
circulo-app (GPUI)          process 1
        │
        │  HTTPS + SSE
        │  Circulo protocol (localhost)
        ▼
circulo-daemon              process 2
  SQLite · session store · adapters
        │
        │  HTTP + SSE
        │  OpenCode server API
        ▼
opencode serve              external
```

The app does **not** call OpenCode. The daemon does, through `circulo-adapter-opencode`. OpenCode’s own server (`opencode serve`) already speaks HTTP and SSE; we translate that into Circulo’s model.

If you only remember one sentence: **UI → Circulo daemon → OpenCode**.

## What the MVP includes

- Native macOS window, hidden title bar, traffic lights + sidebar hide control aligned in the sidebar (a min rail when collapsed).
- Dark theme. English UI, with every string in a locale catalog so more languages can land later.
- New session → unassigned → **Sessions** view with **No project**, unless the composer assigns a folder.
- **Sessions** view: name, session time, project or “No project”. Last view is remembered; if that fails, Sessions.
- **Groups** view: your projects, with their sessions nested. Empty state CTA: New project. Unassigned sessions stay out of this view.
- Project folder is chosen in the composer when the chat starts, then locked. No worktree switching in this phase.
- Delete a project and its sessions go with it. Archive a project and restore it from Settings.
- Rich streaming chat: Markdown, tool cards, diffs, basic tasks.
- Local **SQLite** store.
- One provider: OpenCode.

Explicitly out of the MVP: other agents, interactive question cards, a plugin marketplace, collaboration, cloud sync, deep theming, Windows/Linux.

## Repository layout

```
AGENTS.md                       How we work (read this before coding)
Circulo-Project-Definition.md   Product source
README.md
LICENSE                         MIT
Cargo.toml                      Workspace
rust-toolchain.toml             Pinned stable Rust
crates/
  circulo-app/                  UI process (no-op until app-shell)
  circulo-daemon/               Daemon process (no-op until local-daemon-api)
  circulo-core/
  circulo-protocol/
  circulo-adapter/
  circulo-adapter-fake/
  circulo-adapter-opencode/
  circulo-persist/
  circulo-i18n/
  circulo-markdown/
docs/
openspec/
scripts/check-crate-boundaries.py
```

## Building

macOS. Install [rustup](https://rustup.rs/) if you do not have Rust. The repo pins `stable` via `rust-toolchain.toml`.

```bash
cargo build --workspace
cargo test --workspace
python3 scripts/check-crate-boundaries.py
```

Binaries:

```bash
cargo run -p circulo-app
cargo run -p circulo-daemon
```

`circulo-app` still prints a scaffold line (no window yet).

`circulo-daemon` listens on `http://127.0.0.1:7432` (override with `CIRCULO_DAEMON_ADDR`, loopback only). It uses the fake adapter. TLS is not enabled yet. `GET /v1/health` is the smoke check.

OpenCode is not required until the OpenCode adapter change. UI work should use the fake adapter so the app is not hostage to a live provider. Bun is only for scripts/tooling, not the app runtime.

## How we build

Circulo is **spec-first**. Every feature is:

1. An OpenSpec change (`proposal` → `specs` → `design` → `tasks`)
2. One git branch: `feature/<change-name>`
3. Implementation only after investigation and explicit permission
4. Automated tests + manual checks
5. Granular conventional commits — **after** those checks, not before

Rules of the road live in [`AGENTS.md`](./AGENTS.md). Product requirements live in [`docs/PRD.md`](./docs/PRD.md).

If you use an AI coding agent, point it at `AGENTS.md` and the OpenSpec skills under `.agents/` / `.cursor/` / `.agent/`.

## Contributing

The project is public and early. Useful contributions right now:

- Review the docs and open issues on contradictions or missing decisions.
- Do **not** send a giant “here is the app” PR.
- One OpenSpec change per PR, mapped to one feature branch.

Please file issues in English or Spanish. User-facing product copy is English.

## License

[MIT](./LICENSE).
