//! The sidebar: window controls, the two actions, the grouped task list and
//! settings. Width and paddings are the measured reference values.

use gpui::{IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::mock::{Group, Workspace};
use crate::theme::Theme;

const WIDTH: f32 = 252.;
/// Space reserved for the macOS traffic lights. Measured from the live app:
/// the sidebar toggle starts right after them.
const TRAFFIC_LIGHTS: f32 = 82.;

pub(crate) fn sidebar(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .bg(theme.sidebar)
        .child(window_controls(theme))
        .child(actions(theme))
        .child(task_list(theme, workspace))
        .child(settings(theme))
}

fn window_controls(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .h(px(46.))
        .pl(px(TRAFFIC_LIGHTS))
        .pr(px(8.))
        .child(tool(theme, "icons/panel-left.svg", theme.text_muted))
        .child(tool(theme, "icons/arrow-left.svg", theme.text_faint))
        .child(tool(theme, "icons/arrow-right.svg", theme.text_faint))
}

fn tool(theme: &Theme, path: &'static str, tint: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded(px(6.))
        .child(icon(path, 16., tint))
        .hover(move |style| style.bg(theme.hover))
}

fn actions(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .px(px(4.))
        .pb(px(6.))
        .child(action(theme, "icons/square-pen.svg", "New Task"))
        .child(action(theme, "icons/search.svg", "Search"))
}

fn action(theme: &Theme, path: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(32.))
        .px(px(18.))
        .rounded(px(8.))
        .text_size(px(13.))
        .text_color(theme.text_muted)
        .child(icon(path, 16., theme.text_muted))
        .child(label)
        .hover(move |style| style.bg(theme.hover))
}

fn task_list(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .overflow_hidden()
        .px(px(4.))
        .pt(px(2.))
        .children(workspace.groups.iter().map(|group| section(theme, group)))
}

fn section(theme: &Theme, group: &Group) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(1.))
        .pb(px(6.))
        .child(
            div()
                .flex()
                .items_center()
                .h(px(30.))
                .px(px(14.))
                .text_size(px(11.5))
                .text_color(theme.text_muted)
                .child(group.label)
                .child(div().flex_1())
                .when(group.has_selection(), |row| {
                    row.child(icon("icons/folder-plus.svg", 14., theme.text_faint))
                }),
        )
        .children(group.tasks.iter().map(|task| row(theme, task)))
}

fn row(theme: &Theme, task: &crate::mock::Task) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(3.))
        .px(px(14.))
        .py(px(8.))
        .rounded(px(8.))
        .when(task.selected, |row| row.bg(theme.selection))
        .child(
            div()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(13.))
                .line_height(px(17.))
                .child(task.title),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(11.5))
                .line_height(px(15.))
                .text_color(theme.text_faint)
                .child(icon("icons/folder.svg", 12., theme.text_faint))
                .child(task.project)
                .child(div().flex_1())
                .child(task.when),
        )
        .when(!task.selected, |row| {
            row.hover(move |style| style.bg(theme.hover))
        })
}

fn settings(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(40.))
        .px(px(20.))
        .pb(px(6.))
        .child(icon("icons/settings.svg", 16., theme.text_faint))
}
