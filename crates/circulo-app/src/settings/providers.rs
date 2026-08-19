use circulo_core::AgentType;
use circulo_i18n::Catalog;
use circulo_protocol::AgentDescriptor;
use gpui::{
    div, px, Context, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled,
};

use crate::shell::{settings_text_button, settings_text_button_accent, AppShell};
use crate::theme::{BG_SIDEBAR, BORDER, COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX, SUCCESS, TEXT_MUTED};

pub fn providers_panel(
    descriptors: &[AgentDescriptor],
    pending: Option<(AgentType, bool)>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl gpui::IntoElement {
    let mut list = div().flex().flex_col().gap_2();
    if descriptors.is_empty() {
        list = list.child(empty_row(catalog.get("settings.providers.empty").to_string()));
    } else {
        for (index, descriptor) in descriptors.iter().enumerate() {
            list = list.child(provider_row(descriptor, index, catalog, cx));
        }
    }
    if let Some((agent, enabled)) = pending {
        list = list.child(confirm_strip(agent, enabled, catalog, cx));
    }
    panel_shell(
        "settings-providers-panel",
        catalog.get("settings.providers.subtitle").to_string(),
        list,
    )
}

fn provider_row(
    descriptor: &AgentDescriptor,
    index: usize,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    let agent = descriptor.agent;
    let currently_enabled = descriptor.enabled;
    let (status_text, ok) = if !currently_enabled {
        (
            catalog.get("settings.providers.disabled").to_string(),
            false,
        )
    } else if descriptor.available {
        (
            catalog.get("settings.providers.active").to_string(),
            true,
        )
    } else {
        (
            catalog.get("settings.providers.not_installed").to_string(),
            false,
        )
    };
    let label = match agent {
        AgentType::OpenCode => "OpenCode".to_string(),
        AgentType::CommandCode => catalog.get("settings.commandcode.title").to_string(),
    };
    let action_text = if currently_enabled {
        catalog.get("settings.providers.disable").to_string()
    } else {
        catalog.get("settings.providers.enable").to_string()
    };
    let shell = cx.entity();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(BORDER)
        .bg(BG_SIDEBAR)
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if ok { SUCCESS } else { TEXT_MUTED })
                                .child(status_text),
                        ),
                )
                .child(settings_text_button(
                    ("settings-providers-toggle", index),
                    action_text,
                    move |_, _, cx| {
                        shell.update(cx, |this, cx| {
                            this.request_provider_toggle(agent, !currently_enabled, cx);
                        });
                    },
                )),
        )
}

fn confirm_strip(
    agent: AgentType,
    enabled: bool,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    let provider_label = match agent {
        AgentType::OpenCode => "OpenCode".to_string(),
        AgentType::CommandCode => catalog.get("settings.commandcode.title").to_string(),
    };
    let copy = catalog
        .get("settings.providers.confirm_disable")
        .replace("{provider}", &provider_label)
        .replace("{default_provider}", "OpenCode");
    let confirm_text = catalog.get("settings.providers.confirm_action").to_string();
    let cancel_text = catalog.get("settings.providers.cancel").to_string();
    let confirm_shell = cx.entity();
    let cancel_shell = cx.entity();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .pt_2()
        .border_t_1()
        .border_color(BORDER)
        .child(
            div()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(copy),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(settings_text_button_accent(
                    ("settings-providers-confirm", 0),
                    confirm_text,
                    move |_, _, cx| {
                        confirm_shell.update(cx, |this, cx| {
                            this.confirm_provider_toggle(agent, enabled, cx);
                        });
                    },
                ))
                .child(settings_text_button(
                    ("settings-providers-cancel", 0),
                    cancel_text,
                    move |_, _, cx| {
                        cancel_shell.update(cx, |this, _cx| {
                            this.cancel_provider_toggle();
                        });
                    },
                )),
        )
}

fn panel_shell(
    id: &'static str,
    description: String,
    list: gpui::Div,
) -> impl gpui::IntoElement {
    div()
        .id(id)
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
                        .child(description),
                )
                .child(list),
        )
}

fn empty_row(message: String) -> gpui::Div {
    div()
        .py_3()
        .text_sm()
        .text_color(TEXT_MUTED)
        .child(message)
}
