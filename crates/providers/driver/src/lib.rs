//! The provider contract: one Driver trait, no provider knowledge.
//!
//! A provider crate (OpenCode, Claude Code, Codex, …) depends on this crate and
//! on nothing else of Circulo. It implements the Driver trait and speaks only
//! its own native protocol. The core depends on this crate too, which is how it
//! drives every agent without importing any of them. Because the trait is the
//! only thing both sides know, adding a provider never changes the core or the
//! UI.
//!
//! The trait itself lands in Phase 1 of `docs/implement.md`, together with the
//! protocol types it streams, so that it is shaped by a real adapter instead of
//! a guess.

#![forbid(unsafe_code)]
