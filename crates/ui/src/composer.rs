//! The composer: input surface, the shared controls and the footer.
//!
//! Static for now. The pickers become real menus when the drivers can report
//! models and reasoning effort; the send button becomes active with the core.

use gpui::{IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::mock::Workspace;
use crate::theme::Theme;

pub(crate) fn composer(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .child(input(theme, workspace))
        .child(footer(theme, workspace))
}

fn input(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(13.))
        .rounded(px(14.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .text_size(px(13.))
        .child(div().text_color(theme.text_faint).child("Do anything..."))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(picker(theme, None, workspace.model))
                .child(picker(theme, None, workspace.reasoning))
                .child(picker(theme, Some("icons/lock.svg"), workspace.access))
                .child(picker(
                    theme,
                    Some("icons/hammer.svg"),
                    workspace.agent_mode,
                ))
                .child(div().flex_1())
                .child(send(theme)),
        )
}

fn picker(theme: &Theme, path: Option<&'static str>, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .h(px(26.))
        .px(px(6.))
        .rounded(px(6.))
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .when_some(path, |row, path| {
            row.child(icon(path, 13., theme.text_muted))
        })
        .child(label)
        .child(icon("icons/chevron-down.svg", 11., theme.text_faint))
        .hover(move |style| style.bg(theme.hover))
}

fn send(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded_full()
        .bg(theme.chip)
        .child(icon("icons/arrow-up.svg", 14., theme.text_faint))
}

fn footer(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(14.))
        .pt(px(10.))
        .px(px(4.))
        .text_size(px(11.5))
        .text_color(theme.text_faint)
        .child(meta(theme, "icons/folder.svg", workspace.project))
        .child(meta(theme, "icons/monitor.svg", "Local"))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(icon("icons/git-branch.svg", 12., theme.text_faint))
                .child(workspace.branch)
                .child(icon("icons/chevron-down.svg", 10., theme.text_faint)),
        )
        .child(div().flex_1())
        .child(icon("icons/loader-circle.svg", 12., theme.text_faint))
}

fn meta(theme: &Theme, path: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(icon(path, 12., theme.text_faint))
        .child(label)
}
