use circulo_core::Project;
use circulo_i18n::Catalog;
use gpui::{
    div, prelude::FluentBuilder, px, Context, FontWeight, InteractiveElement, ParentElement,
    StatefulInteractiveElement, Styled,
};
use uuid::Uuid;

use crate::shell::{settings_text_button, settings_text_button_accent, AppShell};
use crate::theme::{
    BG_SIDEBAR, BORDER, COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX, TEXT, TEXT_MUTED,
};

pub fn active_projects_panel(
    projects: &[Project],
    pending_delete: Option<Uuid>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl gpui::IntoElement {
    let mut list = div().flex().flex_col().gap_2();
    if projects.is_empty() {
        list = list.child(empty_row(catalog.get("settings.projects.empty").to_string()));
    } else {
        for (index, project) in projects.iter().enumerate() {
            let project_id = project.id;
            let confirming = pending_delete == Some(project_id);
            let name = project.name.clone();
            let folder = project
                .folder_path
                .clone()
                .unwrap_or_else(|| catalog.get("session.without_folder").to_string());

            let mut row = div()
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
                                        .truncate()
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(TEXT_MUTED)
                                        .truncate()
                                        .child(folder),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .gap_1()
                                .child(settings_text_button(
                                    ("settings-archive", index),
                                    catalog.get("settings.projects.archive").to_string(),
                                    {
                                        let shell = cx.entity();
                                        move |_, _, cx| {
                                            shell
                                                .update(cx, |this, cx| {
                                                    this.archive_project(project_id, cx);
                                                });
                                        }
                                    },
                                ))
                                .child(settings_text_button(
                                    ("settings-delete", index),
                                    catalog.get("settings.projects.delete").to_string(),
                                    {
                                        let shell = cx.entity();
                                        move |_, _, cx| {
                                            shell
                                                .update(cx, |this, cx| {
                                                    this.request_delete_project(project_id, cx);
                                                });
                                        }
                                    },
                                )),
                        ),
                );

            if confirming {
                row = row.child(
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
                                .child(catalog.get("settings.projects.delete_confirm").to_string()),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(settings_text_button_accent(
                                    ("settings-delete-confirm", index),
                                    catalog
                                        .get("settings.projects.delete_confirm_action")
                                        .to_string(),
                                    {
                                        let shell = cx.entity();
                                        move |_, _, cx| {
                                            shell
                                                .update(cx, |this, cx| {
                                                    this.confirm_delete_project(project_id, cx);
                                                });
                                        }
                                    },
                                ))
                                .child(settings_text_button(
                                    ("settings-delete-cancel", index),
                                    catalog.get("settings.projects.delete_cancel").to_string(),
                                    {
                                        let shell = cx.entity();
                                        move |_, _, cx| {
                                            shell
                                                .update(cx, |this, cx| {
                                                    this.cancel_delete_project(cx);
                                                });
                                        }
                                    },
                                )),
                        ),
                );
            }

            list = list.child(row);
        }
    }

    panel_shell(
        "settings-projects-panel",
        catalog.get("settings.projects.description").to_string(),
        list,
    )
}

pub fn archived_projects_panel(
    projects: &[Project],
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl gpui::IntoElement {
    let mut list = div().flex().flex_col().gap_2();
    if projects.is_empty() {
        list = list.child(empty_row(catalog.get("settings.archived.empty").to_string()));
    } else {
        for (index, project) in projects.iter().enumerate() {
            let project_id = project.id;
            let name = project.name.clone();
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(BORDER)
                    .bg(BG_SIDEBAR)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(name),
                    )
                    .child(settings_text_button(
                        ("settings-restore", index),
                        catalog.get("settings.archived.restore").to_string(),
                        {
                            let shell = cx.entity();
                            move |_, _, cx| {
                                shell
                                    .update(cx, |this, cx| {
                                        this.restore_project(project_id, cx);
                                    });
                            }
                        },
                    )),
            );
        }
    }

    panel_shell(
        "settings-archived-panel",
        catalog.get("settings.archived.description").to_string(),
        list,
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
