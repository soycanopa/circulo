//! Local storage, owned exclusively by the core.
//!
//! SQLite holds projects, sessions, messages and parts; JSON holds app
//! settings; attachments go to a content-addressed blob store. Nothing here
//! leaves the machine and nothing here is async-aware of the UI.
//!
//! The schema and its migrations land in Phase 1 of `docs/implement.md`.

#![forbid(unsafe_code)]
