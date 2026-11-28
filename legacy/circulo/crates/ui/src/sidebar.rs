//! The sidebar: window controls, the two actions, the session list and
//! settings. Width and paddings are the measured reference values.
//!
//! Sessions come from the database through the daemon, grouped by the day they
//! were last touched. A row shows its title, the project below it, and the time
//! on the right; the project is chosen in the composer, as in the reference.

use chrono::{DateTime, Datelike, Local, TimeZone};
use circulo_protocol::SessionSummary;
use gpui::{Context, Div, IntoElement, SharedString, div, prelude::*, px};

use crate::icons::icon;
use crate::shell::Shell;
use crate::theme::Theme;

const WIDTH: f32 = 252.;
/// Space reserved for the macOS traffic lights. Measured from the live app:
/// the sidebar toggle starts right after them.
const TRAFFIC_LIGHTS: f32 = 82.;

pub(crate) fn sidebar(
    theme: &Theme,
    sessions: &[SessionSummary],
    open: &str,
    working: Option<std::time::Duration>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .bg(theme.sidebar)
        .child(window_controls(theme))
        .child(actions(theme, cx))
        .child(task_list(theme, sessions, open, working, cx))
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

fn actions(theme: &Theme, cx: &mut Context<Shell>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .px(px(4.))
        .pb(px(6.))
        .child(
            action(theme, "icons/square-pen.svg", "New Task")
                .id("new-task")
                .on_click(cx.listener(|shell, _event, _window, cx| shell.new_session(cx))),
        )
        .child(action(theme, "icons/search.svg", "Search"))
}

fn action(theme: &Theme, path: &'static str, label: &'static str) -> Div {
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

fn task_list(
    theme: &Theme,
    sessions: &[SessionSummary],
    open: &str,
    working: Option<std::time::Duration>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    let now = Local::now();
    let mut groups: Vec<(String, Vec<&SessionSummary>)> = Vec::new();

    for session in sessions {
        let label = group_label(session.updated_at, now);
        match groups.last_mut() {
            Some((current, rows)) if *current == label => rows.push(session),
            _ => groups.push((label, vec![session])),
        }
    }

    let sections = groups
        .iter()
        .map(|(label, rows)| section(theme, label, rows, open, working, now, cx).into_any_element())
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .overflow_hidden()
        .px(px(4.))
        .pt(px(2.))
        .children(sections)
}

fn section(
    theme: &Theme,
    label: &str,
    sessions: &[&SessionSummary],
    open: &str,
    working: Option<std::time::Duration>,
    now: DateTime<Local>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    let rows = sessions
        .iter()
        .map(|session| row(theme, session, open, working, now, cx).into_any_element())
        .collect::<Vec<_>>();

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
                .child(label.to_owned()),
        )
        .children(rows)
}

fn row(
    theme: &Theme,
    session: &SessionSummary,
    open: &str,
    working: Option<std::time::Duration>,
    now: DateTime<Local>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    let selected = session.id == open;
    let title = if session.title.is_empty() {
        "New session".to_owned()
    } else {
        session.title.clone()
    };
    let project = project_name(session.project.as_deref());
    let time = if selected {
        working
            .map(working_label)
            .unwrap_or_else(|| ago(session.updated_at, now))
    } else {
        ago(session.updated_at, now)
    };
    let id = session.id.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(3.))
        .px(px(14.))
        .py(px(8.))
        .rounded(px(8.))
        .when(selected, |row| row.bg(theme.selection))
        .child(
            div()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(13.))
                .line_height(px(17.))
                .child(title),
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
                .child(project)
                .child(div().flex_1())
                .child(time),
        )
        .when(!selected, |row| {
            row.hover(move |style| style.bg(theme.hover))
        })
        .id(SharedString::from(session.id.clone()))
        .on_click(cx.listener(move |shell, _event, _window, cx| shell.open_session(id.clone(), cx)))
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

/// The project a session belongs to, as a folder name. Sessions created without
/// a project say so, as the reference does.
pub(crate) fn project_name(project: Option<&str>) -> String {
    match project {
        Some(path) => std::path::Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_owned()),
        None => "No project".to_owned(),
    }
}

/// Today, Yesterday, This Month, or the month a session was last touched. The
/// list is already newest first, so equal labels stay contiguous.
fn group_label(updated_at: u64, now: DateTime<Local>) -> String {
    let Some(date) = Local.timestamp_opt(updated_at as i64, 0).single() else {
        return "Older".to_owned();
    };
    let date = date.date_naive();
    let today = now.date_naive();
    let yesterday = today.pred_opt().unwrap_or(today);

    if date == today {
        "Today".to_owned()
    } else if date == yesterday {
        "Yesterday".to_owned()
    } else if date.year() == today.year() && date.month() == today.month() {
        "This Month".to_owned()
    } else {
        format!("{} {}", date.format("%B"), date.year())
    }
}

/// The open turn's elapsed time, as the reference writes it in the row.
fn working_label(elapsed: std::time::Duration) -> String {
    let seconds = elapsed.as_secs();
    match seconds {
        0..=59 => format!("Working {seconds}s"),
        60..=3_599 => format!("Working {}m", seconds / 60),
        _ => format!("Working {}h", seconds / 3_600),
    }
}

/// The reference's relative time: minutes, hours, days, then the date.
fn ago(updated_at: u64, now: DateTime<Local>) -> String {
    let seconds = (now.timestamp() - updated_at as i64).max(0);
    match seconds {
        0..=59 => "now".to_owned(),
        60..=3_599 => format!("{}m", seconds / 60),
        3_600..=86_399 => format!("{}h", seconds / 3_600),
        86_400..=604_799 => format!("{}d", seconds / 86_400),
        _ => Local
            .timestamp_opt(updated_at as i64, 0)
            .single()
            .map(|date| date.format("%b %-d").to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Local> {
        chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M")
            .expect("date")
            .and_local_timezone(Local)
            .single()
            .expect("local time")
    }

    #[test]
    fn relative_time_follows_the_reference_scale() {
        let now = at("2026-11-21 15:00");
        assert_eq!(ago(now.timestamp() as u64 - 10, now), "now");
        assert_eq!(ago(now.timestamp() as u64 - 16 * 60, now), "16m");
        assert_eq!(ago(now.timestamp() as u64 - 3 * 3_600, now), "3h");
        assert_eq!(ago(now.timestamp() as u64 - 2 * 86_400, now), "2d");
        assert_eq!(ago(now.timestamp() as u64 - 40 * 86_400, now), "Oct 12");
    }

    #[test]
    fn groups_by_day_then_month() {
        let now = at("2026-11-21 15:00");
        let day = 86_400;
        let now_stamp = now.timestamp() as u64;

        assert_eq!(group_label(now_stamp - 60, now), "Today");
        assert_eq!(group_label(now_stamp - day, now), "Yesterday");
        assert_eq!(group_label(now_stamp - 5 * day, now), "This Month");
        assert_eq!(group_label(now_stamp - 60 * day, now), "September 2026");
    }

    #[test]
    fn a_project_becomes_a_folder_name() {
        assert_eq!(project_name(Some("/Users/me/circulo")), "circulo");
        assert_eq!(project_name(Some("/")), "/");
        assert_eq!(project_name(None), "No project");
    }
}
