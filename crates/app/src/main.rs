//! The composition root: it wires the pieces together and owns the window.
//!
//! The app assembles the UI and, later, the core. It holds no product logic —
//! if something here grows beyond wiring, it belongs in `crates/ui` or
//! `crates/core`.

#![forbid(unsafe_code)]

use circulo_ui::Shell;
use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1180.0), px(760.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shell::new()),
        )
        .expect("the Circulo window should open");
    });
}
