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
    pub const OPENCODE: &str = "icons/opencode.svg";
    /// Chevron used as the CommandCode mark in picker lists. CommandCode's
    /// CLI binary is `cmd`, so the prompt-style chevron fits the brand.
    pub const COMMANDCODE: &str = "icons/chevron-right.svg";
    pub const SHIELD: &str = "icons/shield.svg";
    pub const LIST: &str = "icons/list.svg";
    pub const WRENCH: &str = "icons/wrench.svg";
    pub const MESSAGE_CIRCLE: &str = "icons/message-circle.svg";
    pub const MESSAGE_CIRCLE_PLUS: &str = "icons/message-circle-plus.svg";
    pub const LAYERS: &str = "icons/layers.svg";
    pub const REFRESH_CW: &str = "icons/refresh-cw.svg";
    pub const LOADER_2: &str = "icons/loader-2.svg";
    pub const SETTINGS: &str = "icons/settings.svg";
}

pub const MODEL_PROVIDER_ICON_WIDTH_PX: f32 = 9.0;
pub const MODEL_PROVIDER_ICON_HEIGHT_PX: f32 = 11.0;

pub fn icon(asset_path: &'static str, size: Pixels, color: Rgba) -> impl IntoElement {
    icon_sized(asset_path, size, size, color)
}

pub fn icon_sized(
    asset_path: &'static str,
    width: Pixels,
    height: Pixels,
    color: Rgba,
) -> impl IntoElement {
    svg()
        .path(asset_path)
        .w(width)
        .h(height)
        .flex_none()
        .text_color(color)
}
