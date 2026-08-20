use circulo_core::{model_provider_tag, AgentType, ModelCatalogEntry};
use circulo_i18n::Catalog;
use gpui::{
    div, prelude::FluentBuilder, px, Context, FocusHandle, FontWeight, InteractiveElement,
    ParentElement, Styled, StatefulInteractiveElement, Window,
};

use crate::icons::{icon, icon_sized, path as icon_path};
use crate::shell::AppShell;
use crate::theme::{
    ACCENT, ACCENT_SURFACE, BG_MAIN, BG_SIDEBAR, BORDER, COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX,
    INPUT_BG, INPUT_HEIGHT_PX, SUCCESS, TEXT, TEXT_MUTED, TOGGLE_TRACK_OFF,
};

const MODEL_PROVIDER_BADGE_SIZE_PX: f32 = 12.0;

const MODEL_ROW_PY_PX: f32 = 10.0;
const MODELS_INITIAL_VISIBLE: usize = 12;

pub fn models_settings_panel(
    models: &[ModelCatalogEntry],
    enabled_ids: &[String],
    query: &str,
    expanded: bool,
    models_focus: &FocusHandle,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl gpui::IntoElement {
    let filtered = filter_models(models, query);
    let empty_catalog = models.is_empty();
    let searching = !query.trim().is_empty();
    let show_all = expanded || searching;
    let visible_count = if show_all {
        filtered.len()
    } else {
        filtered.len().min(MODELS_INITIAL_VISIBLE)
    };
    let has_more = !searching && !expanded && filtered.len() > MODELS_INITIAL_VISIBLE;
    let query_display = if query.is_empty() {
        catalog.get("settings.models.search_placeholder").to_string()
    } else {
        query.to_string()
    };
    let query_muted = query.is_empty();

    let mut list = div().flex().flex_col();
    if empty_catalog {
        list = list.child(
            div()
                .py_3()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(catalog.get("settings.models.empty_catalog").to_string()),
        );
    } else if filtered.is_empty() {
        list = list.child(
            div()
                .py_3()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(catalog.get("sidebar.search_empty").to_string()),
        );
    } else {
        // Sort: enabled first, then by (provider_name, name). Stable so
        // a refresh that doesn't change the enabled set keeps the same
        // order — important for the user's muscle memory.
        let mut filtered = filtered;
        filtered.sort_by(|left, right| {
            let left_enabled = enabled_ids.iter().any(|id| id == &left.id);
            let right_enabled = enabled_ids.iter().any(|id| id == &right.id);
            right_enabled
                .cmp(&left_enabled)
                .then_with(|| {
                    left
                        .provider_name
                        .cmp(&right.provider_name)
                        .then_with(|| left.name.cmp(&right.name))
                })
        });
        for (index, model) in filtered.iter().take(visible_count).enumerate() {
            let model_id = model.id.clone();
            let enabled = enabled_ids.iter().any(|id| id == &model_id);
            let tag = model_provider_tag(&model.provider_id, &model.provider_name);
            list = list.child(model_row(
                index,
                model.name.clone(),
                tag,
                model.agent,
                enabled,
                cx.listener(move |this, _, _, cx| {
                    this.toggle_model_enabled(&model_id, cx);
                }),
            ));
        }
    }

    div()
        .id("settings-models-panel")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .items_center()
        .px(px(COMPOSER_GUTTER_PX))
        .py(px(24.))
        .child(
            div()
                .id("settings-models-card")
                .track_focus(models_focus)
                .w_full()
                .max_w(px(CONTENT_MAX_WIDTH_PX))
                .mx_auto()
                .rounded_lg()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .p(px(16.))
                .flex()
                .flex_col()
                .on_key_down(cx.listener(|this, event, window, cx| {
                    if this.settings_models_focus.is_focused(window) {
                        this.handle_settings_models_key(event, cx);
                    }
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pb(px(12.))
                        .child(
                            div()
                                .id("settings-models-search")
                                .flex_1()
                                .min_w_0()
                                .h(px(INPUT_HEIGHT_PX))
                                .px_3()
                                .rounded_md()
                                .border_1()
                                .border_color(BORDER)
                                .bg(INPUT_BG)
                                .flex()
                                .items_center()
                                .text_sm()
                                .text_color(if query_muted { TEXT_MUTED } else { TEXT })
                                .cursor_text()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.settings_models_focus.focus(window);
                                    cx.notify();
                                }))
                                .child(query_display),
                        )
                        .child(
                            div()
                                .id("settings-models-refresh")
                                .flex_none()
                                .w(px(INPUT_HEIGHT_PX))
                                .h(px(INPUT_HEIGHT_PX))
                                .rounded_md()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(TEXT_MUTED)
                                .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.schedule_refresh(cx);
                                    cx.notify();
                                }))
                                .child(icon(icon_path::REFRESH_CW, px(16.), TEXT_MUTED)),
                        ),
                )
                .child(list)
                .when(has_more, |card| {
                    card.child(
                        div()
                            .pt(px(12.))
                            .child(
                                div()
                                    .id("settings-models-view-more")
                                    .text_sm()
                                    .text_color(ACCENT)
                                    .cursor_pointer()
                                    .hover(|style| style.opacity(0.85))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.settings_models_expanded = true;
                                        cx.notify();
                                    }))
                                    .child(catalog.get("settings.models.view_more").to_string()),
                            ),
                    )
                }),
        )
}

fn filter_models(models: &[ModelCatalogEntry], query: &str) -> Vec<ModelCatalogEntry> {
    let mut sorted: Vec<ModelCatalogEntry> = models.to_vec();
    sorted.sort_by(|left, right| {
        left.provider_name
            .cmp(&right.provider_name)
            .then_with(|| left.name.cmp(&right.name))
    });
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return sorted;
    }
    sorted
        .into_iter()
        .filter(|model| {
            model.name.to_ascii_lowercase().contains(&q)
                || model.provider_name.to_ascii_lowercase().contains(&q)
                || model_provider_tag(&model.provider_id, &model.provider_name)
                    .to_ascii_lowercase()
                    .contains(&q)
        })
        .collect()
}

fn model_row(
    index: usize,
    label: String,
    provider_tag: String,
    agent: AgentType,
    enabled: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    let provider_icon = match agent {
        AgentType::OpenCode => icon_path::OPENCODE,
        AgentType::CommandCode => icon_path::COMMANDCODE,
    };
    div()
        .id(("settings-model", index))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py(px(MODEL_ROW_PY_PX))
        .border_b_1()
        .border_color(BORDER)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(icon_sized(
                    provider_icon,
                    px(MODEL_PROVIDER_BADGE_SIZE_PX),
                    px(MODEL_PROVIDER_BADGE_SIZE_PX),
                    TEXT_MUTED,
                ))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .text_color(TEXT)
                        .child(label),
                )
                .child(provider_tag_chip(provider_tag)),
        )
        .child(toggle_switch(("model-toggle", index), enabled, on_toggle))
}

#[allow(dead_code)]
fn agent_badge_chip(_label: String) -> impl gpui::IntoElement {
    div()
}

fn provider_tag_chip(label: String) -> impl gpui::IntoElement {
    div()
        .flex_none()
        .px(px(6.))
        .py(px(2.))
        .rounded(px(4.))
        .bg(ACCENT_SURFACE)
        .border_1()
        .border_color(BORDER)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(TEXT_MUTED)
        .child(label)
}

pub(crate) fn toggle_switch(
    id: (&'static str, usize),
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    const TRACK_W_PX: f32 = 36.0;
    const TRACK_H_PX: f32 = 20.0;
    const THUMB_PX: f32 = 16.0;
    const PAD_PX: f32 = 2.0;

    div()
        .id(id)
        .flex_none()
        .w(px(TRACK_W_PX))
        .h(px(TRACK_H_PX))
        .px(px(PAD_PX))
        .rounded_full()
        .flex()
        .items_center()
        .when(enabled, |el| el.bg(SUCCESS).justify_end())
        .when(!enabled, |el| el.bg(TOGGLE_TRACK_OFF).justify_start())
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .w(px(THUMB_PX))
                .h(px(THUMB_PX))
                .rounded_full()
                .bg(TEXT),
        )
}
