//! The composer: input surface, the shared controls and the footer.
//!
//! The input is not editable yet: text entry arrives with the session input
//! work. The send affordance and the footer reflect the live turn.

use gpui::{IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::theme::Theme;

pub(crate) fn composer(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
) -> impl IntoElement {
    let running = transcript.status.running;

    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .child(input(theme, workspace, running))
        .child(footer(theme, workspace, running))
}

fn input(theme: &Theme, workspace: &Workspace, running: bool) -> impl IntoElement {
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
                .child(send(theme, running)),
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

fn send(theme: &Theme, running: bool) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded_full()
        .bg(theme.chip)
        .child(if running {
            icon("icons/loader-circle.svg", 14., theme.text_faint)
        } else {
            icon("icons/arrow-up.svg", 14., theme.text_faint)
        })
}

fn footer(theme: &Theme, workspace: &Workspace, running: bool) -> impl IntoElement {
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
        .when(running, |row| {
            row.child(icon("icons/loader-circle.svg", 12., theme.text_faint))
        })
}

fn meta(theme: &Theme, path: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(icon(path, 12., theme.text_faint))
        .child(label)
}
