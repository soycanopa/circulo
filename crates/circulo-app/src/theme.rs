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

pub const SIDEBAR_EXPANDED_PX: f32 = 260.0;
pub const SIDEBAR_RAIL_PX: f32 = 80.0;
pub const TRAFFIC_LIGHT_X_PX: f32 = 12.0;
pub const TRAFFIC_LIGHT_Y_PX: f32 = 14.0;

pub fn sidebar_width_px(collapsed: bool) -> f32 {
    if collapsed {
        SIDEBAR_RAIL_PX
    } else {
        SIDEBAR_EXPANDED_PX
    }
}
