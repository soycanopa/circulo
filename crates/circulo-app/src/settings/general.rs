use circulo_i18n::Catalog;
use circulo_protocol::HealthResponse;
use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, ParentElement,
    StatefulInteractiveElement, Styled,
};

use crate::shell::AppShell;
use crate::theme::{
    BG_SIDEBAR, BORDER, COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX, SUCCESS, TEXT, TEXT_MUTED,
};

pub fn general_settings_panel(
    health: Option<&HealthResponse>,
    health_error: Option<&str>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl gpui::IntoElement {
    let daemon_ok = health.is_some() && health_error.is_none();
    let daemon_label = if daemon_ok {
        catalog.get("settings.health.daemon_ok").to_string()
    } else {
        health_error
            .map(str::to_string)
            .unwrap_or_else(|| catalog.get("settings.health.daemon_unknown").to_string())
    };

    let (opencode_label, opencode_detail) = match health.and_then(|h| h.opencode.as_ref()) {
        Some(opencode) if opencode.available => (
            catalog.get("settings.health.opencode_ok").to_string(),
            opencode
                .version
                .clone()
                .unwrap_or_else(|| catalog.get("settings.health.version_unknown").to_string()),
        ),
        Some(_) => (
            catalog.get("settings.health.opencode_unavailable").to_string(),
            health
                .and_then(|h| h.adapter_message.clone())
                .unwrap_or_else(|| catalog.get("settings.health.opencode_hint").to_string()),
        ),
        None => (
            catalog.get("settings.health.opencode_unknown").to_string(),
            catalog.get("settings.health.opencode_hint").to_string(),
        ),
    };

    div()
        .id("settings-general-panel")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(COMPOSER_GUTTER_PX))
        .pb(px(24.))
        .child(
            div()
                .w_full()
                .max_w(px(CONTENT_MAX_WIDTH_PX))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(TEXT_MUTED)
                        .child(catalog.get("settings.general.description").to_string()),
                )
                .child(health_card(
                    catalog.get("settings.health.daemon_title").to_string(),
                    daemon_label,
                    None,
                    daemon_ok,
                ))
                .child(health_card(
                    catalog.get("settings.health.opencode_title").to_string(),
                    opencode_label,
                    Some(opencode_detail),
                    health
                        .and_then(|h| h.opencode.as_ref())
                        .is_some_and(|o| o.available),
                ))
                .child(
                    div()
                        .id("settings-health-retry")
                        .flex()
                        .items_center()
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .text_sm()
                        .text_color(TEXT)
                        .border_1()
                        .border_color(BORDER)
                        .hover(|style| style.bg(BG_SIDEBAR))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.refresh_settings_health(cx);
                        }))
                        .child(catalog.get("settings.health.retry").to_string()),
                ),
        )
}

fn health_card(
    title: String,
    status: String,
    detail: Option<String>,
    ok: bool,
) -> impl gpui::IntoElement {
    let status_color = if ok { SUCCESS } else { TEXT_MUTED };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(BORDER)
        .bg(BG_SIDEBAR)
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(
            div()
                .text_sm()
                .text_color(status_color)
                .child(status),
        )
        .when_some(detail, |el, detail| {
            el.child(
                div()
                    .text_xs()
                    .text_color(TEXT_MUTED)
                    .child(detail),
            )
        })
}
