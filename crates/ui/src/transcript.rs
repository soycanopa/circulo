//! The transcript: header, and the rows the daemon streams.
//!
//! Rows live in a GPUI `List`, the same primitive the reference uses: it
//! virtualizes, owns the scroll offset, and gives each row a stable id. A plain
//! flex column that is also the scroll container measures its children against
//! the viewport and they end up compressed on top of each other.
//!
//! The empty state stays outside the list.

use std::rc::Rc;

use gpui::{
    AnyElement, FontWeight, IntoElement, ListAlignment, ListState, SharedString, div, list,
    prelude::*, px,
};

use crate::icons::icon;
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::shell::View;
use crate::theme::Theme;

/// The reference keeps the transcript text off the panel edges by this much;
/// there is no narrower reading column.
const PADDING: f32 = 34.;
/// Rendered above and below the viewport so streaming rows do not pop in.
const OVERDRAW: f32 = 400.;

/// One row of the transcript, owned so the list can render it lazily.
pub(crate) enum Row {
    Message(String),
    Divider {
        label: String,
        running: bool,
    },
    /// One assistant turn. Its parts are stacked inside the row, the way the
    /// reference treats a turn as one list item: the list measures a bounded
    /// block instead of every chunk of text.
    Turn(Vec<Segment>),
    Error(String),
}

/// Owned copy of a streamed part, so the list can render rows lazily.
pub(crate) enum Segment {
    Text(String),
    Reasoning(String),
    Tool {
        name: String,
        detail: String,
        done: bool,
    },
}

pub(crate) fn transcript(
    theme: &Theme,
    view: View,
    workspace: &Workspace,
    transcript: &Transcript,
    prompt: &str,
    list_state: &ListState,
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
            View::Session => {
                conversation(theme, workspace, transcript, prompt, list_state).into_any_element()
            }
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
    list_state: &ListState,
) -> impl IntoElement {
    let rows = Rc::new(rows(workspace, transcript, prompt));
    // The list's builder is 'static; the theme is a handful of copied colours.
    let theme = *theme;

    // A streaming turn changes the height of its row without changing the row
    // count. Without a reset the list keeps the height it measured a frame ago
    // and the turn's blocks paint over each other.
    if list_state.item_count() != rows.len() || transcript.status.running {
        list_state.reset(rows.len());
    }

    let state = list_state.clone();

    div()
        .flex_1()
        .min_h(px(0.))
        .px(px(PADDING))
        .pb(px(24.))
        .child(
            list(state, move |index, _window, _cx| {
                row_view(&rows[index], index, &theme)
            })
            .size_full(),
        )
}

pub(crate) fn new_list_state() -> ListState {
    // Top alignment keeps the offset at zero, which is the state the list lays
    // out reliably; pinning to the newest row is an explicit scroll, as in the
    // reference's scroll-to-bottom layer.
    ListState::new(0, ListAlignment::Top, px(OVERDRAW))
}

fn rows(workspace: &Workspace, transcript: &Transcript, prompt: &str) -> Vec<Row> {
    let mut rows = vec![Row::Message(prompt.to_owned())];

    if !transcript.is_empty() {
        rows.push(Row::Divider {
            label: if transcript.status.running {
                "Working…".to_owned()
            } else {
                workspace.worked.to_owned()
            },
            running: transcript.status.running,
        });
    }

    for turn in &transcript.turns {
        let segments = turn
            .parts
            .iter()
            .map(|segment| match segment {
                crate::live::Segment::Text(text) => Segment::Text(text.clone()),
                crate::live::Segment::Reasoning(text) => Segment::Reasoning(text.clone()),
                crate::live::Segment::Tool { name, detail, done } => Segment::Tool {
                    name: name.clone(),
                    detail: detail.clone(),
                    done: *done,
                },
            })
            .collect();

        rows.push(Row::Turn(segments));
    }

    if let Some(error) = &transcript.status.error {
        rows.push(Row::Error(error.clone()));
    }

    rows
}

fn row_view(row: &Row, index: usize, theme: &Theme) -> AnyElement {
    // Every row is a full-width block with a stable id, as in the reference.
    // An unbounded row is measured at its minimum width, so wrapped text is
    // reported shorter than it paints and the next row lands on top of it.
    let id = SharedString::from(format!("row-{index}"));

    match row {
        Row::Message(text) => div()
            .id(id)
            .w_full()
            .min_w(px(0.))
            .flex()
            .justify_end()
            .pt(px(30.))
            .pb(px(26.))
            .child(
                div()
                    .max_w(px(520.))
                    .px(px(15.))
                    .py(px(9.))
                    .rounded(px(12.))
                    .bg(theme.chip)
                    .text_size(px(13.))
                    .child(text.clone()),
            )
            .into_any_element(),
        Row::Divider { label, running } => div()
            .id(id)
            .w_full()
            .min_w(px(0.))
            .flex()
            .items_center()
            .gap(px(10.))
            .pb(px(18.))
            .child(div().flex_1().h(px(1.)).bg(theme.hairline))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(11.5))
                    .text_color(theme.text_faint)
                    .when(*running, |row| {
                        row.child(icon("icons/loader-circle.svg", 11., theme.text_faint))
                    })
                    .child(label.clone())
                    .when(!*running, |row| {
                        row.child(icon("icons/chevron-right.svg", 11., theme.text_faint))
                    }),
            )
            .child(div().flex_1().h(px(1.)).bg(theme.hairline))
            .into_any_element(),
        Row::Turn(segments) => {
            let mut turn = div().id(id).w_full().min_w(px(0.)).flex().flex_col();

            for (segment_index, segment) in segments.iter().enumerate() {
                let segment_id = SharedString::from(format!("row-{index}-segment-{segment_index}"));
                turn = turn.child(segment_view(segment, segment_id, theme));
            }

            turn.into_any_element()
        }
        Row::Error(text) => div()
            .id(id)
            .w_full()
            .min_w(px(0.))
            .pt(px(10.))
            .text_size(px(12.))
            .text_color(theme.accent)
            .child(text.clone())
            .into_any_element(),
    }
}

fn segment_view(segment: &Segment, id: SharedString, theme: &Theme) -> AnyElement {
    match segment {
        // One text block per part: the deltas already carry their own
        // paragraphs. Markdown styling lands with the renderer.
        Segment::Text(text) => div()
            .id(id)
            .w_full()
            .min_w(px(0.))
            .flex_shrink_0()
            .py(px(7.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(text.clone())
            .into_any_element(),
        Segment::Reasoning(text) => activity(&id, theme, "icons/brain.svg", None, text, true),
        Segment::Tool { name, detail, done } => {
            activity(&id, theme, tool_icon(name), Some(name), detail, *done)
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
    id: &SharedString,
    theme: &Theme,
    path: &'static str,
    name: Option<&str>,
    detail: &str,
    done: bool,
) -> AnyElement {
    div()
        .id(id.clone())
        .w_full()
        .min_w(px(0.))
        .flex_shrink_0()
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
        .into_any_element()
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
