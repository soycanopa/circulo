use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use circulo_core::ModelCatalogEntry;
use circulo_i18n::Catalog;
use gpui::{
    div, prelude::FluentBuilder, px, FontWeight, InteractiveElement, ParentElement, Styled,
    StatefulInteractiveElement,
};

use crate::icons::{icon, path as icon_path};
use crate::theme::{
    ACCENT, ACCENT_SURFACE, BG_MAIN, BORDER, CONTENT_MAX_WIDTH_PX, TEXT, TEXT_MUTED,
};

pub fn models_settings_panel(
    models: &[ModelCatalogEntry],
    enabled_ids: &[String],
    catalog: &Catalog,
    on_toggle: impl FnMut(&str, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    let on_toggle = Rc::new(RefCell::new(on_toggle));
    let grouped = group_models_by_provider(models);
    let empty_catalog = models.is_empty();

    let mut body = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .child(catalog.get("settings.models.title").to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(TEXT_MUTED)
                        .child(catalog.get("settings.models.description").to_string()),
                ),
        );

    if empty_catalog {
        body = body.child(
            div()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(catalog.get("settings.models.empty_catalog").to_string()),
        );
    } else {
        for (provider_name, provider_models) in grouped {
            let mut section = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .pt_2()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(TEXT_MUTED)
                        .child(provider_name),
                );
            for (index, model) in provider_models.iter().enumerate() {
                let model_id = model.id.clone();
                let enabled = enabled_ids.iter().any(|id| id == &model_id);
                let label = model.name.clone();
                let detail = model
                    .context_window
                    .clone()
                    .unwrap_or_else(|| catalog.get("composer.model_context.none").to_string());
                let toggle = on_toggle.clone();
                section = section.child(
                    model_toggle_row(
                        ("settings-model", index),
                        label,
                        detail,
                        enabled,
                        move |_, _, cx| toggle.borrow_mut()(model_id.as_str(), cx),
                    ),
                );
            }
            body = body.child(section);
        }
    }

    div()
        .id("settings-models-panel")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(32.))
        .py(px(24.))
        .child(
            div()
                .w_full()
                .max_w(px(CONTENT_MAX_WIDTH_PX))
                .child(body),
        )
}

fn group_models_by_provider(models: &[ModelCatalogEntry]) -> BTreeMap<String, Vec<ModelCatalogEntry>> {
    let mut grouped: BTreeMap<String, Vec<ModelCatalogEntry>> = BTreeMap::new();
    for model in models {
        grouped
            .entry(model.provider_name.clone())
            .or_default()
            .push(model.clone());
    }
    for provider_models in grouped.values_mut() {
        provider_models.sort_by(|left, right| left.name.cmp(&right.name));
    }
    grouped
}

fn model_toggle_row(
    id: (&'static str, usize),
    label: String,
    detail: String,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(BORDER)
        .bg(BG_MAIN)
        .cursor_pointer()
        .hover(|style| style.bg(ACCENT_SURFACE))
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.))
                .h(px(20.))
                .rounded_md()
                .when(enabled, |el| el.bg(ACCENT))
                .when(!enabled, |el| el.border_1().border_color(BORDER))
                .child(
                    icon(
                        icon_path::CHECK,
                        px(12.),
                        if enabled { BG_MAIN } else { TEXT_MUTED },
                    ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(div().text_sm().text_color(TEXT).child(label))
                .child(div().text_xs().text_color(TEXT_MUTED).child(detail)),
        )
}
