## 1. Toolchain

- [x] 1.1 Add `rust-toolchain.toml` pinning `stable`
- [x] 1.2 Ensure rustup/cargo are available locally so the workspace can be verified

## 2. Workspace and crates

- [x] 2.1 Add root `Cargo.toml` workspace (`crates/*`, edition 2021, shared package metadata)
- [x] 2.2 Add library crates: `circulo-core`, `circulo-protocol`, `circulo-adapter`, `circulo-adapter-fake`, `circulo-adapter-opencode`, `circulo-persist`, `circulo-i18n`, `circulo-markdown` (empty `lib.rs`, documented purpose)
- [x] 2.3 Add binary crates `circulo-app` and `circulo-daemon` with no-op `main` that prints process identity
- [x] 2.4 Wire Cargo path dependencies exactly as `design.md` (app: protocol + i18n; daemon: protocol + persist + adapter trait; no app→opencode/persist; no adapter→app)

## 3. Verification

- [x] 3.1 Add a script or check that fails if `circulo-app` depends on `circulo-adapter-opencode` or `circulo-persist`
- [x] 3.2 Run `cargo build --workspace` and `cargo test --workspace`
- [x] 3.3 Confirm both binaries exist after build
- [x] 3.4 Update README with build prerequisites and commands
