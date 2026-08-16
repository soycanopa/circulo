use gpui::{
    Bounds, Pixels, Point, TitlebarOptions, WindowBounds, WindowOptions, point, px, size,
};

use crate::theme::{TRAFFIC_LIGHT_X_PX, TRAFFIC_LIGHT_Y_PX};

pub fn circulo_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Circulo".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(TRAFFIC_LIGHT_X_PX), px(TRAFFIC_LIGHT_Y_PX))),
        }),
        window_min_size: Some(size(px(800.), px(520.))),
        ..Default::default()
    }
}

pub fn titlebar_is_transparent(options: &WindowOptions) -> bool {
    options
        .titlebar
        .as_ref()
        .is_some_and(|bar| bar.appears_transparent)
}

pub fn traffic_light_position(options: &WindowOptions) -> Option<Point<Pixels>> {
    options
        .titlebar
        .as_ref()
        .and_then(|bar| bar.traffic_light_position)
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::*;
    use crate::theme::{SIDEBAR_EXPANDED_PX, SIDEBAR_RAIL_PX, sidebar_width_px};

    #[test]
    fn window_uses_transparent_titlebar_and_traffic_lights() {
        let bounds = Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1100.), px(720.)),
        };
        let options = circulo_window_options(bounds);
        assert!(titlebar_is_transparent(&options));
        let pos = traffic_light_position(&options).expect("traffic lights");
        assert!(pos.x > px(0.));
        assert!(pos.y > px(0.));
    }

    #[test]
    fn collapsed_sidebar_is_a_smaller_rail() {
        let expanded = sidebar_width_px(false);
        let rail = sidebar_width_px(true);
        assert_eq!(expanded, SIDEBAR_EXPANDED_PX);
        assert_eq!(rail, SIDEBAR_RAIL_PX);
        assert!(rail > 0.0);
        assert!(rail < expanded);
    }
}
