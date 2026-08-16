## Context

See `proposal.md` for why. There is no Rust code yet; rustc/cargo are not installed on the current machine. Product stack (GPUI, SQLite, HTTPS/SSE, OpenCode) is decided in `docs/TRD.md` but must not be pulled into this change.

Crate list and dependency rules come from `docs/TRD.md` §4.

## Goals / Non-Goals

**Goals:**

- A `cargo build --workspace` that succeeds on macOS after rustup.
- Two binary packages and the library crates listed in the proposal, with Cargo dependencies that encode the TRD boundaries.
- A pinned `stable` toolchain file.
- README build instructions.

**Non-Goals:**

- Adding `gpui`, `rusqlite`, HTTP, or TLS crates.
- Implementing types, APIs, or locale files.
- Choosing a macOS deployment target.
- Installing rustup as a committed artifact (document it; install locally to verify).

## Decisions

### 1. Workspace members under `crates/`

**Choice:** one Cargo workspace, members = `crates/*`.

**Why:** matches `docs/IMPLEMENTATION.md`. Flat `crates/` keeps `cargo` simple.

**Alternative:** `apps/` + `crates/` split. Rejected for now — only two binaries, extra layout without benefit.

### 2. One package per crate, binaries have `main.rs` only

**Choice:** `circulo-app` and `circulo-daemon` are binary packages that print a one-line process identity to stdout and exit 0. Libraries export nothing (empty `lib.rs` or a crate-level doc comment).

**Why:** proves two processes without pretending they do work.

**Alternative:** app embeds daemon in a thread. Rejected — product decision is two processes.

### 3. Encode boundaries with Cargo deps, not comments only

**Choice:**

| Crate | May depend on |
| --- | --- |
| `circulo-core` | (none) |
| `circulo-protocol` | `circulo-core` |
| `circulo-adapter` | `circulo-core` |
| `circulo-adapter-fake` | `circulo-adapter` |
| `circulo-adapter-opencode` | `circulo-adapter` |
| `circulo-persist` | `circulo-core` |
| `circulo-i18n` | (none) |
| `circulo-markdown` | (none) |
| `circulo-daemon` | `circulo-protocol`, `circulo-persist`, `circulo-adapter` (not the OpenCode impl) |
| `circulo-app` | `circulo-protocol`, `circulo-i18n` |

Daemon depends on the **trait** crate, not `circulo-adapter-opencode`. Wiring the real adapter is a later change.

**Alternative:** daemon depends on OpenCode crate now. Rejected — would teach the wrong graph.

### 4. Pin `stable` in `rust-toolchain.toml`, edition 2021

**Choice:** channel `stable`. Edition 2021 (widely supported; 2024 can wait).

**Why:** we do not yet need a nightly GPUI pin. When `app-shell-window` lands, that change may tighten the pin if GPUI requires it.

**Alternative:** pin a specific date (`1.8x.0`). Deferred — unknown until rustup is installed; tasks will record the actual version after first `rustc -V`.

### 5. No rustfmt/clippy config beyond defaults

**Choice:** skip custom rustfmt.toml unless we already have a house style.

**Why:** do not invent style in the first change.

### 6. Cargo.lock committed

**Choice:** commit `Cargo.lock` for the workspace (binaries).

**Why:** reproducible developer builds.

## Risks / Trade-offs

- [Rust not installed] → Install rustup locally to verify; README states the prerequisite.
- [GPUI later needs nightly] → `app-shell-window` may change `rust-toolchain.toml`; do not pre-pin nightly.
- [Empty crates look like dead code] → Each `Cargo.toml` has a one-line description of future responsibility so they are not deleted as unused.
- [Daemon depending on persist/adapter with empty crates] → Fine: path deps on empty libs. Keeps the intended graph visible.

## Migration Plan

- New repo structure only. No data to migrate.
- Rollback: delete `crates/`, root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`.

## Open Questions

None that change this change. macOS deployment target and TLS stay in TRD §15 for later changes.
