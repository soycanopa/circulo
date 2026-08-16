## Why

Circulo has product and engineering docs but no compilable Rust workspace. Later changes (`core-and-protocol`, `local-daemon-api`, `app-shell-window`) cannot land as modules if crates and the two binaries do not exist first.

## What Changes

- Add a Cargo workspace at the repo root with the crate layout from `docs/TRD.md`.
- Add two no-op binaries: `circulo-app` and `circulo-daemon` (two Circulo processes).
- Add library crates: `circulo-core`, `circulo-protocol`, `circulo-adapter`, `circulo-adapter-fake`, `circulo-adapter-opencode`, `circulo-persist`, `circulo-i18n`, `circulo-markdown`.
- Pin the Rust toolchain (`rust-toolchain.toml`) so builds are reproducible on macOS.
- Document how to build the workspace in `README.md`.
- Enforce crate dependency rules at the workspace level: `circulo-app` does not depend on OpenCode or persist; adapters do not depend on GPUI.

This change does **not** implement chat, HTTP, SQLite, GPUI UI, or OpenCode.

## Capabilities

### New Capabilities

- `workspace-scaffold`: The repository is a Cargo workspace that builds two Circulo binaries and the modular library crates with the documented dependency boundaries.

### Modified Capabilities

- (none)

## Non-goals

- No GPUI window, no daemon HTTP/SSE, no SQLite schema, no OpenCode adapter logic.
- No worktree switching (product: out of scope now).
- No Windows/Linux targets.
- No plugin loader.
- Does not close remaining TRD open questions (TLS, OpenCode spawn, macOS minimum deployment target).

## Impact

- New files: root `Cargo.toml`, `rust-toolchain.toml`, `crates/**`.
- Dev dependency: Rust stable via rustup (not currently installed on this machine).
- No runtime APIs, no user-facing product strings.
- Unblocks OpenSpec changes 2–6 in `docs/IMPLEMENTATION.md`.

## Open questions (not resolved here)

- Minimum macOS version / deployment target (TRD §15).
- Exact rustc channel pin beyond “current stable at implement time”.
