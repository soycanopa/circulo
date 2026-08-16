use circulo_i18n::Catalog;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::theme::{
    ACCENT, BG_APP, BG_MAIN, BG_SIDEBAR, BORDER, TEXT, TEXT_MUTED, sidebar_width_px,
};

pub struct AppShell {
    pub sidebar_collapsed: bool,
    catalog: Catalog,
}

impl Default for AppShell {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            catalog: Catalog::english(),
        }
    }
}

impl AppShell {
    pub fn sidebar_width(&self) -> f32 {
        sidebar_width_px(self.sidebar_collapsed)
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        let width = self.sidebar_width();
        let catalog = self.catalog.clone();
        let toggle_label = catalog.get(if collapsed {
            "sidebar.show"
        } else {
            "sidebar.hide"
        });

        div()
            .flex()
            .size_full()
            .bg(BG_APP)
            .text_color(TEXT)
            .child(sidebar(collapsed, width, toggle_label, &catalog, cx))
            .child(main_column(&catalog))
    }
}

fn sidebar(
    collapsed: bool,
    width: f32,
    toggle_label: &str,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let top = div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(52.))
        .pl(px(80.))
        .pr_2()
        .child(
            div()
                .id("toggle-sidebar")
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .text_color(TEXT_MUTED)
                .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_collapsed = !this.sidebar_collapsed;
                    cx.notify();
                }))
                .child(toggle_label.to_string()),
        );

    let body = if collapsed {
        div().flex_1()
    } else {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .px_3()
            .gap_2()
            .child(label(catalog.get("sidebar.sessions"), true))
            .child(label(catalog.get("sidebar.groups"), false))
            .child(label(catalog.get("sidebar.new_session"), false))
            .child(label(catalog.get("sidebar.search"), false))
            .child(div().flex_1())
            .child(label(catalog.get("sidebar.settings"), false))
    };

    div()
        .flex()
        .flex_col()
        .w(px(width))
        .h_full()
        .bg(BG_SIDEBAR)
        .border_r_1()
        .border_color(BORDER)
        .child(top)
        .child(body)
}

fn main_column(catalog: &Catalog) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .h_full()
        .bg(BG_MAIN)
        .child(
            div()
                .h(px(52.))
                .px_4()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(BORDER)
                .child(catalog.get("session.none").to_string()),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(TEXT_MUTED)
                .child(catalog.get("session.empty").to_string()),
        )
        .child(
            div()
                .h(px(72.))
                .px_4()
                .flex()
                .items_center()
                .border_t_1()
                .border_color(BORDER)
                .text_color(TEXT_MUTED)
                .child(catalog.get("composer.placeholder").to_string()),
        )
}

fn label(text: &str, active: bool) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .when(active, |el| el.bg(ACCENT).text_color(TEXT))
        .when(!active, |el| el.text_color(TEXT_MUTED))
        .child(text.to_string())
}
