## Purpose

Provides a compilable Rust workspace on macOS with two Circulo process binaries and modular library crates, so later features can land without inventing structure ad hoc.

## ADDED Requirements

### Requirement: Two Circulo process binaries

The repository SHALL produce two independent binaries named `circulo-app` and `circulo-daemon` that build on macOS with the pinned toolchain.

#### Scenario: Workspace builds both binaries

- **GIVEN** a clean checkout with the pinned Rust toolchain installed
- **WHEN** a developer runs `cargo build --workspace`
- **THEN** the build succeeds
- **AND** the artifacts include `circulo-app` and `circulo-daemon`

#### Scenario: Binaries are distinct processes

- **GIVEN** a successful workspace build
- **WHEN** a developer inspects the built binary names
- **THEN** `circulo-app` and `circulo-daemon` are separate executables
- **AND** neither binary is required to implement product chat, HTTP, or UI behavior yet

### Requirement: Modular crate boundaries

The workspace SHALL keep the documented module boundaries so the frontend cannot depend on OpenCode and adapters cannot depend on the UI.

#### Scenario: App does not depend on OpenCode

- **GIVEN** the workspace dependency graph
- **WHEN** a developer inspects dependencies of `circulo-app`
- **THEN** `circulo-app` MUST NOT depend on `circulo-adapter-opencode`
- **AND** `circulo-app` MUST NOT depend on `circulo-persist`

#### Scenario: Adapter crates do not depend on the app

- **GIVEN** the workspace dependency graph
- **WHEN** a developer inspects dependencies of `circulo-adapter-opencode` and `circulo-adapter-fake`
- **THEN** those crates MUST NOT depend on `circulo-app`

#### Scenario: Protocol depends on core only

- **GIVEN** the workspace dependency graph
- **WHEN** a developer inspects dependencies of `circulo-protocol`
- **THEN** `circulo-protocol` MAY depend on `circulo-core`
- **AND** `circulo-protocol` MUST NOT depend on `circulo-app`, `circulo-daemon`, or adapter crates

### Requirement: Pinned toolchain

The repository SHALL pin a Rust stable toolchain so two machines building the same commit use the same rustc channel.

#### Scenario: Toolchain file is present

- **GIVEN** a clean checkout
- **WHEN** a developer inspects the repository root
- **THEN** a `rust-toolchain.toml` exists
- **AND** it pins the `stable` channel

### Requirement: Empty crates stay no-op

Library crates introduced by this change SHALL compile with no product behavior (no HTTP server, no SQLite schema, no GPUI window, no OpenCode client).

#### Scenario: Default library surface is empty

- **GIVEN** a successful `cargo test --workspace`
- **WHEN** tests finish
- **THEN** all member crates pass
- **AND** no test requires a running OpenCode server or a display
