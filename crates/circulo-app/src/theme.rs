use gpui::Rgba;

pub const BG_APP: Rgba = Rgba {
    r: 0.09,
    g: 0.09,
    b: 0.10,
    a: 1.0,
};
pub const BG_SIDEBAR: Rgba = Rgba {
    r: 0.11,
    g: 0.11,
    b: 0.12,
    a: 1.0,
};
pub const BG_MAIN: Rgba = Rgba {
    r: 0.13,
    g: 0.13,
    b: 0.14,
    a: 1.0,
};
pub const TEXT: Rgba = Rgba {
    r: 0.92,
    g: 0.92,
    b: 0.93,
    a: 1.0,
};
pub const TEXT_MUTED: Rgba = Rgba {
    r: 0.62,
    g: 0.62,
    b: 0.65,
    a: 1.0,
};
pub const BORDER: Rgba = Rgba {
    r: 0.22,
    g: 0.22,
    b: 0.24,
    a: 1.0,
};
pub const ACCENT: Rgba = Rgba {
    r: 0.45,
    g: 0.47,
    b: 0.95,
    a: 1.0,
};
/// shadcn `Input` surface (`bg-background` on dark).
pub const INPUT_BG: Rgba = BG_MAIN;
/// shadcn `Input` focus border (`border-ring`).
pub const INPUT_BORDER_FOCUS: Rgba = ACCENT;
/// shadcn `Input` height (`h-9`).
pub const INPUT_HEIGHT_PX: f32 = 36.0;
pub const SUCCESS: Rgba = Rgba {
    r: 0.45,
    g: 0.78,
    b: 0.55,
    a: 1.0,
};
pub const DANGER: Rgba = Rgba {
    r: 0.90,
    g: 0.42,
    b: 0.42,
    a: 1.0,
};
pub const DIFF_ADD: Rgba = Rgba {
    r: 0.22,
    g: 0.42,
    b: 0.28,
    a: 1.0,
};
pub const DIFF_DEL: Rgba = Rgba {
    r: 0.45,
    g: 0.22,
    b: 0.24,
    a: 1.0,
};
pub const CODE_BG: Rgba = Rgba {
    r: 0.08,
    g: 0.08,
    b: 0.09,
    a: 1.0,
};
/// Popover / context menu surface (shadcn `bg-popover`).
pub const POPOVER_BG: Rgba = Rgba {
    r: 0.12,
    g: 0.12,
    b: 0.13,
    a: 1.0,
};
/// Subtle popover ring (shadcn `ring-foreground/10`).
pub const POPOVER_RING: Rgba = Rgba {
    r: 0.92,
    g: 0.92,
    b: 0.93,
    a: 0.10,
};
/// Item focus/hover surface (shadcn `bg-accent`).
pub const ACCENT_SURFACE: Rgba = Rgba {
    r: 0.17,
    g: 0.17,
    b: 0.19,
    a: 1.0,
};
/// Destructive item focus/hover (shadcn `focus:bg-destructive/10`).
pub const DANGER_SURFACE: Rgba = Rgba {
    r: 0.28,
    g: 0.14,
    b: 0.16,
    a: 1.0,
};

/// Max width for chat transcript and floating composer card (Waku-style).
pub const CONTENT_MAX_WIDTH_PX: f32 = 768.0;
/// Horizontal inset shared by the transcript and floating composer.
pub const COMPOSER_GUTTER_PX: f32 = 20.0;
/// Bottom inset for the floating composer block.
pub const COMPOSER_BOTTOM_PADDING_PX: f32 = 8.0;
pub const MESSAGE_AVATAR_PX: f32 = 32.0;

pub const SIDEBAR_EXPANDED_PX: f32 = 260.0;
pub const SIDEBAR_MIN_PX: f32 = 200.0;
pub const SIDEBAR_MAX_PX: f32 = 480.0;
/// Interactive hit area for the sidebar resize handle.
pub const SIDEBAR_RESIZE_HANDLE_HIT_PX: f32 = 8.0;
/// Visible width of the gradient resize stripe (shown on hover/drag only).
pub const SIDEBAR_RESIZE_HANDLE_VISUAL_PX: f32 = 1.0;
/// Center color for the resize handle gradient (white peak).
pub const SIDEBAR_RESIZE_HANDLE_CENTER: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.85,
};
pub const SIDEBAR_RESIZE_HANDLE_CENTER_ACTIVE: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
/// Shared chrome row height (session header + sidebar titlebar spacer).
pub const APP_BAR_HEIGHT_PX: f32 = 40.0;
/// Left offset for the sidebar toggle (aligned with expanded sidebar layout).
pub const SIDEBAR_TOGGLE_LEFT_PX: f32 = 82.0;
pub const SIDEBAR_TOGGLE_SIZE_PX: f32 = 28.0;
/// Gap between the sidebar toggle and the session title when the sidebar is hidden.
pub const MAIN_HEADER_TITLE_GAP_PX: f32 = 12.0;
/// Top inset for the sidebar toggle in the app bar row.
pub const SIDEBAR_TOGGLE_TOP_PX: f32 =
    (APP_BAR_HEIGHT_PX - SIDEBAR_TOGGLE_SIZE_PX) / 2.0 + 1.0;
/// Session title inset from the window left when the sidebar is hidden.
pub const MAIN_HEADER_TITLE_LEFT_PX: f32 =
    SIDEBAR_TOGGLE_LEFT_PX + SIDEBAR_TOGGLE_SIZE_PX + MAIN_HEADER_TITLE_GAP_PX;
/// Session title inset inside the main column when the sidebar is visible.
pub const MAIN_HEADER_TITLE_INSET_PX: f32 = 12.0;
/// Session title in the main header row.
pub const MAIN_HEADER_TITLE_TEXT_PX: f32 = 11.0;
pub const TRAFFIC_LIGHT_X_PX: f32 = 12.0;
pub const TRAFFIC_LIGHT_Y_PX: f32 = 14.0;

pub fn sidebar_width_px(collapsed: bool, expanded_width: f32) -> f32 {
    if collapsed {
        0.0
    } else {
        expanded_width
    }
}
