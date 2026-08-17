use circulo_app::assets::Assets;
use circulo_app::command_palette::init_command_palette;
use circulo_app::composer::init_composer_input;
use circulo_app::shell::AppShell;
use circulo_app::ui::init_text_input;
use circulo_app::window::circulo_window_options;
use gpui::{prelude::*, px, size, Application, Bounds, WindowBounds};

fn main() {
    Application::new().with_assets(Assets).run(|cx| {
        init_composer_input(cx);
        init_command_palette(cx);
        init_text_input(cx);
        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        let mut options = circulo_window_options(bounds);
        if options.window_bounds.is_none() {
            options.window_bounds = Some(WindowBounds::Windowed(bounds));
        }
        let _ = cx.open_window(options, |window, cx| cx.new(|cx| AppShell::new(window, cx)));
        cx.activate(true);
    });
}
