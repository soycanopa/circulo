//! Git for the core: worktrees per session and conversation-aware checkpoints.
//!
//! Only the core uses this crate. A checkpoint pairs a commit (or a tree) with
//! the conversation position it belongs to, so rewinding restores files and
//! transcript together instead of leaving the two out of step.
//!
//! Checkpoints and rewind land in Phase 3 of `docs/implement.md`.

#![forbid(unsafe_code)]
