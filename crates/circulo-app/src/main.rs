use circulo_app::shell::AppShell;
use circulo_app::window::circulo_window_options;
use gpui::{Application, Bounds, WindowBounds, prelude::*, px, size};

fn main() {
    Application::new().run(|cx| {
        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        let mut options = circulo_window_options(bounds);
        if options.window_bounds.is_none() {
            options.window_bounds = Some(WindowBounds::Windowed(bounds));
        }
        let _ = cx.open_window(options, |_, cx| cx.new(|_| AppShell::default()));
        cx.activate(true);
    });
}
