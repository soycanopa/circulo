//! GPUI views for Circulo: sidebar, transcript, composer and right panel.
//!
//! Presentation only. This crate knows the protocol and its own view state, and
//! nothing else: no provider, no CLI, no database, no file system.
//!
//! The layout, the tokens and the components reproduce the Waku reference from
//! screenshots — measured, then reimplemented here. Waku is GPL-3.0 and no Waku
//! code is copied; `docs/ui.md` records what was measured and why.

#![forbid(unsafe_code)]

mod assets;
mod composer;
mod icons;
mod mock;
mod shell;
mod sidebar;
mod text;
mod theme;
mod transcript;

pub use assets::Assets;
pub use shell::Shell;
