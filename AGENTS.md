# AGENTS.md — Circulo

Rules for any agent (or human) working in this repository.

**Current approach (2026-11-20):**  
Native desktop app built entirely with [MyGo](https://mygo.run/) (Go).  
The window is native UI (`ui`, including `ui/markdown`). The core is a separate process.  
v1.0 goal = feature parity with [egoist/waku](https://github.com/egoist/waku) (ideas and UX only — never copy code, GPL-3.0).  
Strong modular separation: UI, core, providers, protocol, persistence and Git are independent puzzle pieces.

## Working agreement

- Be direct and technical.
- Explain the *why* with documentation when you propose something. Primary sources: MyGo docs (https://mygo.run/docs), official provider docs (OpenCode, Claude Code, Codex, etc.), and the project docs in `docs/`.
- If a human decision contradicts best practice or the quality bar, raise your hand. Do not comply just to please. Build it solid.
- If you have a better idea, propose it and explain why.
- **Do not invent APIs.** If it is not in the official docs of MyGo or the provider, it does not exist. Read the docs before writing code.
- **Do not assume. Ask.** If product, protocol, or permission is unclear, ask before implementing.
- **No changes without a plan** (short is fine: issue or commit message). Large work → written plan under `docs/`.
- **Do not loop.** If an approach fails twice, stop, write what you tried, and ask or change strategy.
- **Reference projects: ideas yes, code never.** Waku is GPL-3.0-only. Learn its UX, layout (sidebar + transcript + right panel), graphite palette, streaming patterns and checkpoint ideas — but never copy a file, function or type. Circulo must stay free of that license.

## Architecture invariants

Everything is separated like puzzle pieces. Dependency direction is one-way.

- **UI (MyGo app)** knows only the neutral protocol and presentation state. It never imports providers, never talks to CLIs, and never owns persistent data.
- **Protocol package** is the only shared contract (events, parts, envelopes). Changing it requires updating both sides in the same change.
- **Core / Daemon** owns projects, sessions, provider lifecycle, Git/worktrees, checkpoints, SQLite and filesystem. It talks to providers only through the Driver interface.
- **Providers** are independent packages. Each implements the Driver interface and speaks only its own native protocol. They never import UI, MyGo or the core.
- **Persistence and Git** are owned by the core. No other piece touches the database or the worktree directly.
- New pieces (another provider, a mobile client, a remote tunnel, a new right-panel view) must be addable without modifying existing pieces.

## Git & delivery workflow

We work with small, reviewable, incremental changes.

### Branches & commits
- All work happens on branches (`feature/<short-topic>`, `fix/<short-topic>`, `docs/<topic>`). Never commit straight to `main`.
- **Granular commits**: one logical change per commit. Prefer many small commits over one big one.
- Commit messages: `type(scope): summary`  
  Examples: `feat(protocol): add part.delta event`, `fix(opencode): tolerate unknown events`, `docs: update TRD architecture`, `test(core): session registry`.
- Tests travel with the code. Protocol and adapter changes include fixtures or tests in the same commit (or the one immediately before).

### Pull Requests
- Open a PR for every branch. Keep PRs small and focused (ideally one concern).
- PR title follows the same `type(scope): summary` style.
- Description explains *what* and *why*, links the relevant issue or doc section, and lists how to test.
- At least one review (or self-review checklist if solo) before merge.
- CI / local checks must pass: `go test ./...` (and any MyGo-specific checks).

### Releases
- Small incremental releases are preferred over big-bang drops.
- Tag when a meaningful vertical slice works (e.g. “OpenCode streaming works”, “Git checkpoints land”, “v0.1-alpha”).
- Use semantic versioning for the app (`v0.x` while pre-1.0, then `v1.0` when Waku parity is reached).
- Release notes summarize user-visible changes and link the PRs.
- After v1.0, continue with small releases for new providers, mobile, remote, etc.

### Documentation
- Update the relevant file in `docs/` in the same PR as the behavior it describes.
- Keep `AGENTS.md` itself current when rules change.

## Code standards

- Clean code. Small functions, explicit errors (wrap with context in Go), no premature abstraction.
- Comments explain non-obvious *why* and invariants, not what the code already says.
- Unknown provider events are ignored gracefully (forward compatibility).
- Never block the UI thread. Heavy work (process spawn, Git, network, parsing) stays off the UI path.
- The window is MyGo native UI. Do not move the transcript, sidebar, or composer into a web frontend. A Browser tab may embed a WebView under the native view later.

## Project map (target)

- `docs/` — PRD, TRD, UX, UI, Flow, Implementation Plan, this file.
- `protocol/` — neutral wire contract (events, parts). Shared by UI and core.
- `core/` — registry, orchestrator, Git checkpoints, persistence.
- `providers/` — one package per agent (opencode, claude, codex, …).
- `ui/` — MyGo views (sidebar, transcript, composer, right panel).
- `cmd/` — composition roots: the desktop app and the core process.

## Quality bar

- Streaming must feel instant.
- Long transcripts must stay smooth.
- No orphaned agent processes after quit.
- Architecture must stay modular: a new provider or a new UI surface must not require rewriting existing pieces.
