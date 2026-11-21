//! The core: projects, sessions, the provider lifecycle and orchestration.
//!
//! The core owns all state. It owns the database and the worktree as well, but
//! only through the `circulo-persistence` and `circulo-git` crates; no other
//! piece touches them directly. Providers are reached only through the Driver
//! trait, so the core never learns a provider's native protocol and providers
//! never learn about the core.
//!
//! The registry, the session loop and process supervision land in Phase 1 of
//! `docs/implement.md`.

#![forbid(unsafe_code)]
