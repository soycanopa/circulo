//! Design tokens, measured from the Waku reference.
//!
//! The values come from the two published Waku screenshots (`waku.sh`), read
//! pixel by pixel: window canvas, sidebar material, selected row, composer,
//! inline code and text steps. They are a starting point to match the
//! reference, not a final identity — `docs/ui.md` keeps the table and the
//! reason for every token.
//!
//! Waku is GPL-3.0. These are colours and measurements, reimplemented here; no
//! Waku code is copied.

use gpui::{Rgba, WindowAppearance, rgb, rgba};

#[derive(Clone, Copy)]
pub struct Theme {
    /// The transcript and composer background.
    pub canvas: Rgba,
    /// The sidebar material. Translucent so the window blur shows through, as
    /// in the reference.
    pub sidebar: Rgba,
    /// The selected task row.
    pub selection: Rgba,
    pub hover: Rgba,
    /// The composer surface, a step above the canvas.
    pub surface: Rgba,
    pub border: Rgba,
    /// Inline code and other small filled chips.
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
            canvas: rgb(0x141414),
            // The reference sidebar reads #1A1E20 over a blue wallpaper: it is
            // a vibrant material. A translucent graphite keeps that behaviour.
            sidebar: rgba(0x161a1cb3),
            selection: rgba(0xffffff12),
            hover: rgba(0xffffff0a),
            surface: rgb(0x191919),
            border: rgba(0xffffff14),
            chip: rgb(0x242424),
            text: rgb(0xdcdcdc),
            text_muted: rgb(0x9a9a9a),
            text_faint: rgb(0x6f6f6f),
            accent: rgb(0xe2795b),
            hairline: rgba(0xffffff1a),
        }
    }

    pub fn light() -> Self {
        Self {
            canvas: rgb(0xf5f4f5),
            sidebar: rgba(0xe7ebedcc),
            selection: rgba(0x0000000d),
            hover: rgba(0x00000008),
            surface: rgb(0xffffff),
            border: rgba(0x0000001a),
            chip: rgb(0xe6e5e6),
            text: rgb(0x1b1b1b),
            text_muted: rgb(0x6b6b6b),
            text_faint: rgb(0x8e8e8e),
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
