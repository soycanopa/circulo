//! Design tokens, measured from Waku 0.1.20 running on this machine.
//!
//! The live app is the ground truth: its sidebar is an opaque `#252525` over a
//! `#1A1A1A` canvas, not the vibrant material the marketing screenshots
//! suggest. Text, muted text and icons are `#E2E2E2`, `#A3A3A3` and `#747474`.
//! `docs/ui.md` records the table and where each value came from.
//!
//! Waku is GPL-3.0. These are colours and measurements, reimplemented here; no
//! Waku code is copied.

use gpui::{Rgba, WindowAppearance, rgb, rgba};

#[derive(Clone, Copy)]
pub struct Theme {
    /// The transcript background.
    pub canvas: Rgba,
    /// The sidebar surface. Opaque, a step lighter than the canvas.
    pub sidebar: Rgba,
    /// The selected task row.
    pub selection: Rgba,
    pub hover: Rgba,
    /// The composer surface.
    pub surface: Rgba,
    pub border: Rgba,
    /// Inline code and the user bubble.
    pub chip: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    /// Status, selection and the send affordance. Colour is meaning, never
    /// decoration. Nothing paints with it yet: it lands with the running turn
    /// and the active composer in Phase 1, and lives here so there is one place
    /// for it.
    #[allow(dead_code)]
    pub accent: Rgba,
    /// Divider lines inside the transcript.
    pub hairline: Rgba,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            canvas: rgb(0x1a1a1a),
            sidebar: rgb(0x252525),
            selection: rgba(0xffffff0a),
            hover: rgba(0xffffff08),
            surface: rgb(0x1f1f1f),
            border: rgba(0xffffff14),
            chip: rgb(0x2a2a2a),
            text: rgb(0xe2e2e2),
            text_muted: rgb(0xa3a3a3),
            text_faint: rgb(0x747474),
            accent: rgb(0xe2795b),
            hairline: rgba(0xffffff1a),
        }
    }

    /// Not captured from the live app yet: Waku follows the system appearance
    /// and this machine is in dark. These light values are a coherent pair to
    /// build against, and are corrected the first time we capture light mode.
    pub fn light() -> Self {
        Self {
            canvas: rgb(0xf5f5f5),
            sidebar: rgb(0xececec),
            selection: rgba(0x0000000a),
            hover: rgba(0x00000006),
            surface: rgb(0xffffff),
            border: rgba(0x00000014),
            chip: rgb(0xe9e9e9),
            text: rgb(0x1b1b1b),
            text_muted: rgb(0x6f6f6f),
            text_faint: rgb(0x9a9a9a),
            accent: rgb(0xd9663f),
            hairline: rgba(0x00000014),
        }
    }

    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }
}
