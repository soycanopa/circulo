use gpui::{IntoElement, Pixels, Rgba, Styled, svg};

/// Lucide SVG paths (embedded under `assets/icons/`).
pub mod path {
    pub const SEARCH: &str = "icons/search.svg";
    pub const FOLDER: &str = "icons/folder.svg";
    pub const FOLDER_PLUS: &str = "icons/folder-plus.svg";
    pub const LAPTOP: &str = "icons/laptop.svg";
    pub const FORK: &str = "icons/fork.svg";
    pub const CHECK: &str = "icons/check.svg";
    pub const CHEVRON_DOWN: &str = "icons/chevron-down.svg";
    pub const CHEVRON_RIGHT: &str = "icons/chevron-right.svg";
    pub const ARROW_UP: &str = "icons/arrow-up.svg";
    pub const ELLIPSIS: &str = "icons/ellipsis.svg";
    pub const PANEL_LEFT_CLOSE: &str = "icons/panel-left-close.svg";
    pub const PANEL_LEFT_OPEN: &str = "icons/panel-left-open.svg";
    pub const PENCIL: &str = "icons/pencil.svg";
    pub const TRASH: &str = "icons/trash-2.svg";
    pub const MAXIMIZE_2: &str = "icons/maximize-2.svg";
    pub const MINIMIZE_2: &str = "icons/minimize-2.svg";
    pub const BOT: &str = "icons/bot.svg";
    pub const SHIELD: &str = "icons/shield.svg";
    pub const LIST: &str = "icons/list.svg";
    pub const WRENCH: &str = "icons/wrench.svg";
    pub const MESSAGE_CIRCLE: &str = "icons/message-circle.svg";
    pub const LAYERS: &str = "icons/layers.svg";
}

pub fn icon(asset_path: &'static str, size: Pixels, color: Rgba) -> impl IntoElement {
    svg()
        .path(asset_path)
        .size(size)
        .text_color(color)
}
