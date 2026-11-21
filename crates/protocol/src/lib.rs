//! The neutral wire contract between the UI and the core.
//!
//! Every agent, whichever CLI it is, is seen by the UI only through the events
//! and parts defined in this crate. The UI never imports a provider and never
//! speaks a provider's native protocol; the core never sends UI types over the
//! wire. That is why this crate is the single contract both sides share, and
//! why a change here has to update both sides in the same change.
//!
//! It is deliberately empty for now. The first events (session lifecycle, part
//! deltas, permissions) land with the core loop in Phase 1 of
//! `docs/implement.md`, together with their tests.

#![forbid(unsafe_code)]
