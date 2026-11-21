//! The composition root: it wires the pieces together and owns the window.
//!
//! The app assembles the UI and, later, the core. It holds no product logic —
//! if something here grows beyond wiring, it belongs in `crates/ui` or
//! `crates/core`.

#![forbid(unsafe_code)]

use circulo_ui::{Assets, Shell};
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, point, px, size,
};

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.), px(740.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Circulo".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(20.), px(18.))),
                }),
                // The sidebar is a translucent material over the desktop,
                // as in the reference. The main column paints over it.
                window_background: WindowBackgroundAppearance::Blurred,
                window_min_size: Some(size(px(980.), px(680.))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shell::new()),
        )
        .expect("the Circulo window should open");

        cx.activate(true);
    });
}
