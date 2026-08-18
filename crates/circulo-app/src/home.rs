//! Home empty state when no chat session is selected.

use circulo_i18n::Catalog;
use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, ParentElement,
    StatefulInteractiveElement, Styled, TextAlign, Window,
};

use crate::icons::{icon, path as icon_path};
use crate::shell::AppShell;
use crate::theme::{BG_MAIN, BG_SIDEBAR, BORDER, CONTENT_MAX_WIDTH_PX, TEXT, TEXT_MUTED};

const HOME_CARD_WIDTH_PX: f32 = 184.0;
const HOME_CARD_HEIGHT_PX: f32 = 104.0;
const HOME_CARD_ICON_PX: f32 = 16.0;
const HOME_CARD_GAP_PX: f32 = 16.0;
const HOME_HEADER_GAP_PX: f32 = 28.0;
const HOME_GRID_WIDTH_PX: f32 = HOME_CARD_WIDTH_PX * 2.0 + HOME_CARD_GAP_PX;

pub fn home_panel(catalog: &Catalog, cx: &mut Context<AppShell>) -> impl gpui::IntoElement {
    let new_session = home_card(
        "home-new-session",
        icon_path::MESSAGE_CIRCLE,
        catalog.get("home.card.new_session").to_string(),
        catalog.get("home.card.new_session_detail").to_string(),
        false,
        cx.listener(|this, _, window, cx| {
            this.create_new_session(window, cx);
        }),
    );
    let open_project = home_card(
        "home-open-project",
        icon_path::FOLDER_PLUS,
        catalog.get("home.card.open_project").to_string(),
        catalog.get("home.card.open_project_detail").to_string(),
        false,
        cx.listener(|this, _, window, cx| {
            this.open_project_from_home(window, cx);
        }),
    );
    let search = home_card(
        "home-search",
        icon_path::SEARCH,
        catalog.get("home.card.search").to_string(),
        catalog.get("home.card.search_detail").to_string(),
        false,
        cx.listener(|this, _, window, cx| {
            this.open_palette(window, cx);
        }),
    );
    let mobile = home_card(
        "home-mobile",
        icon_path::LAPTOP,
        catalog.get("home.card.mobile").to_string(),
        catalog.get("home.card.mobile_detail").to_string(),
        true,
        |_, _, _| {},
    );

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(24.))
        .child(
            div()
                .w_full()
                .max_w(px(CONTENT_MAX_WIDTH_PX))
                .mx_auto()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(HOME_HEADER_GAP_PX))
                .child(
                    div()
                        .w(px(HOME_GRID_WIDTH_PX))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .w_full()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(TEXT)
                                .text_align(TextAlign::Center)
                                .child(catalog.get("home.title").to_string()),
                        )
                        .child(
                            div()
                                .w_full()
                                .text_sm()
                                .text_color(TEXT_MUTED)
                                .text_align(TextAlign::Center)
                                .child(catalog.get("home.subtitle").to_string()),
                        ),
                )
                .child(
                    div()
                        .w(px(HOME_GRID_WIDTH_PX))
                        .flex()
                        .flex_col()
                        .gap(px(HOME_CARD_GAP_PX))
                        .child(
                            div()
                                .flex()
                                .gap(px(HOME_CARD_GAP_PX))
                                .child(new_session)
                                .child(open_project),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(HOME_CARD_GAP_PX))
                                .child(search)
                                .child(mobile),
                        ),
                ),
        )
}

fn home_card(
    id: &'static str,
    icon_path: &'static str,
    title: String,
    detail: String,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    div()
        .id(id)
        .w(px(HOME_CARD_WIDTH_PX))
        .h(px(HOME_CARD_HEIGHT_PX))
        .px(px(14.))
        .py(px(12.))
        .rounded_lg()
        .border_1()
        .border_color(BORDER)
        .bg(BG_SIDEBAR)
        .flex()
        .flex_col()
        .gap(px(8.))
        .when(disabled, |el| el.opacity(0.55).cursor_default())
        .when(!disabled, |el| {
            el.cursor_pointer()
                .hover(|style| style.bg(BG_MAIN))
                .on_click(on_click)
        })
        .child(icon(icon_path, px(HOME_CARD_ICON_PX), TEXT_MUTED))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT)
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .text_color(TEXT_MUTED)
                .line_height(px(14.))
                .child(detail),
        )
}
