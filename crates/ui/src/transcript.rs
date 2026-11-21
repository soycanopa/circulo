//! The transcript: header, user message, the worked divider, the parts the
//! daemon streams and, before a project is open, the empty state.
//!
//! Virtualization and the remaining part renderers come with the protocol
//! work; the visual language is already the final one.

use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::live::{Segment, Transcript};
use crate::mock::Workspace;
use crate::shell::View;
use crate::text::inline;
use crate::theme::Theme;

/// The reference keeps the transcript text off the panel edges by this much;
/// there is no narrower reading column.
const PADDING: f32 = 34.;

pub(crate) fn transcript(
    theme: &Theme,
    view: View,
    workspace: &Workspace,
    transcript: &Transcript,
    prompt: &str,
) -> impl IntoElement {
    let title = match view {
        View::Empty => workspace.empty.header,
        View::Session => workspace.title,
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .child(header(theme, title))
        .child(match view {
            View::Empty => empty(theme, workspace).into_any_element(),
            View::Session => conversation(theme, workspace, transcript, prompt).into_any_element(),
        })
}

fn header(theme: &Theme, title: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(46.))
        .px(px(24.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(div().flex_1())
        .child(icon("icons/panel-right.svg", 16., theme.text_faint))
}

fn conversation(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
    prompt: &str,
) -> impl IntoElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for turn in &transcript.turns {
        for segment in &turn.parts {
            rows.push(segment_view(theme, segment));
        }
    }

    div()
        .id("transcript")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .px(px(PADDING))
        .pb(px(24.))
        .child(user_message(theme, prompt))
        .when(!transcript.is_empty(), |column| {
            column.child(worked(theme, workspace, transcript))
        })
        .children(rows)
        .when_some(transcript.status.error.clone(), |column, error| {
            column.child(
                div()
                    .pt(px(10.))
                    .text_size(px(12.))
                    .text_color(theme.accent)
                    .child(error),
            )
        })
}

/// The reference empty state: a coral mark, the promise in two lines, one
/// primary action and one quiet way out. The folder picker arrives with the
/// core, so neither control is wired yet.
fn empty(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    let copy = &workspace.empty;

    div()
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .child(icon("icons/asterisk.svg", 24., theme.accent))
        .child(
            div()
                .pt(px(8.))
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(copy.title),
        )
        .child(
            div()
                .max_w(px(430.))
                .text_size(px(13.))
                .line_height(px(21.))
                .text_color(theme.text_muted)
                .text_center()
                .child(copy.subtitle),
        )
        .child(
            div()
                .mt(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .h(px(32.))
                .px(px(16.))
                .rounded_full()
                .bg(theme.text)
                .text_color(theme.canvas)
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(copy.primary)
                .id("open-project")
                .hover(|style| style.opacity(0.92)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(theme.text_faint)
                .child("×")
                .child(copy.secondary),
        )
}

fn user_message(theme: &Theme, prompt: &str) -> impl IntoElement {
    div().flex().justify_end().pt(px(30.)).pb(px(26.)).child(
        div()
            .max_w(px(520.))
            .px(px(15.))
            .py(px(9.))
            .rounded(px(12.))
            .bg(theme.chip)
            .text_size(px(13.))
            .child(prompt.to_owned()),
    )
}

/// While a turn streams the divider carries a spinner; when it ends, the
/// disclosure for the turn's activity.
fn worked(theme: &Theme, workspace: &Workspace, transcript: &Transcript) -> impl IntoElement {
    let (label, running) = if transcript.status.running {
        ("Working…", true)
    } else {
        (workspace.worked, false)
    };

    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .pb(px(18.))
        .child(div().flex_1().h(px(1.)).bg(theme.hairline))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .when(running, |row| {
                    row.child(icon("icons/loader-circle.svg", 11., theme.text_faint))
                })
                .child(label)
                .when(!running, |row| {
                    row.child(icon("icons/chevron-right.svg", 11., theme.text_faint))
                }),
        )
        .child(div().flex_1().h(px(1.)).bg(theme.hairline))
}

fn segment_view(theme: &Theme, segment: &Segment) -> AnyElement {
    match segment {
        // Paragraphs keep their shape while streaming; inline markdown is
        // parsed per paragraph. A real renderer replaces this with runs.
        Segment::Text(text) => {
            let mut column = div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .py(px(7.))
                .text_size(px(13.))
                .line_height(px(20.));

            for paragraph in text.split("\n\n").filter(|paragraph| !paragraph.is_empty()) {
                column = column.child(inline(paragraph, theme));
            }

            column.into_any_element()
        }
        Segment::Reasoning(text) => {
            activity(theme, "icons/brain.svg", None, text, true).into_any_element()
        }
        Segment::Tool { name, detail, done } => {
            activity(theme, tool_icon(name), Some(name), detail, *done).into_any_element()
        }
    }
}

fn tool_icon(name: &str) -> &'static str {
    match name {
        "Read" => "icons/file-text.svg",
        "Edit" => "icons/square-pen.svg",
        "Bash" => "icons/terminal.svg",
        _ => "icons/hammer.svg",
    }
}

/// One collapsed activity row: icon, label, what it acted on, and a chevron or
/// a spinner. Expanded output arrives with streaming.
fn activity(
    theme: &Theme,
    path: &'static str,
    name: Option<&str>,
    detail: &str,
    done: bool,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(26.))
        .text_size(px(12.))
        .child(icon(path, 13., theme.text_faint))
        .when_some(name, |row, name| {
            row.child(
                div()
                    .flex_shrink_0()
                    .text_color(theme.text_muted)
                    .child(name.to_owned()),
            )
        })
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(theme.text_faint)
                .child(detail.to_owned()),
        )
        .child(if done {
            icon("icons/chevron-right.svg", 11., theme.text_faint)
        } else {
            icon("icons/loader-circle.svg", 11., theme.text_faint)
        })
}
