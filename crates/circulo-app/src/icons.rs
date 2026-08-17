use gpui::{IntoElement, Pixels, Rgba, Styled, svg};

/// Lucide SVG paths (embedded under `assets/icons/`).
pub mod path {
    pub const SEARCH: &str = "icons/search.svg";
    pub const CHEVRON_DOWN: &str = "icons/chevron-down.svg";
    pub const CHEVRON_RIGHT: &str = "icons/chevron-right.svg";
    pub const ARROW_UP: &str = "icons/arrow-up.svg";
    pub const ELLIPSIS: &str = "icons/ellipsis.svg";
    pub const PANEL_LEFT_CLOSE: &str = "icons/panel-left-close.svg";
    pub const PANEL_LEFT_OPEN: &str = "icons/panel-left-open.svg";
    pub const PENCIL: &str = "icons/pencil.svg";
    pub const TRASH: &str = "icons/trash-2.svg";
}

pub fn icon(asset_path: &'static str, size: Pixels, color: Rgba) -> impl IntoElement {
    svg()
        .path(asset_path)
        .size(size)
        .text_color(color)
}
