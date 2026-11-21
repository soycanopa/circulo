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
    AnyElement, FontWeight, IntoElement, ListAlignment, ListOffset, ListState, SharedString, div,
    list, prelude::*, px,
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
    /// Parsed once per text version and shared with the live transcript.
    Text(Rc<Vec<crate::markdown::Block>>),
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
                conversation(theme, workspace, transcript, list_state).into_any_element()
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
    list_state: &ListState,
) -> impl IntoElement {
    let rows = Rc::new(rows(workspace, transcript));
    // The list's builder is 'static; the theme is a handful of copied colours.
    let theme = *theme;

    // A streaming turn changes the height of its row without changing the row
    // count. Without a reset the list keeps the height it measured a frame ago
    // and the turn's blocks paint over each other. The list is bottom-anchored,
    // so a reset also puts the view back on the newest row: that is the
    // follow-the-stream behaviour, without a scroll call every frame.
    if list_state.item_count() != rows.len() || transcript.status.running {
        list_state.reset(rows.len());
    }

    let state = list_state.clone();
    // Reading the logical offset is enough: the list is anchored to the bottom,
    // so "at the newest row" is an offset at or past the last item.
    let at_bottom = list_state.logical_scroll_top().item_ix >= list_state.item_count();

    let list = list(state.clone(), move |index, _window, _cx| {
        row_view(&rows[index], index, &theme)
    })
    .size_full();

    div()
        .relative()
        .flex_1()
        .min_h(px(0.))
        .px(px(PADDING))
        .pb(px(24.))
        .child(list)
        .when(!at_bottom, |container| {
            container.child(
                div()
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .bottom(px(10.))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("jump-to-latest")
                            .size(px(26.))
                            .rounded_full()
                            .bg(theme.surface)
                            .border_1()
                            .border_color(theme.border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.chip))
                            .child(icon("icons/chevron-down.svg", 13., theme.text_muted))
                            .on_click(move |_, window, _| {
                                // The bottom of a bottom-anchored list is one
                                // item past the last one.
                                state.scroll_to(ListOffset {
                                    item_ix: state.item_count(),
                                    offset_in_item: px(0.),
                                });
                                window.refresh();
                            }),
                    ),
            )
        })
}

pub(crate) fn new_list_state() -> ListState {
    // Bottom alignment is the follow-the-stream behaviour: while a turn runs
    // the list is reset every frame, and a reset with this alignment puts the
    // view back on the newest row. Once the user scrolls up, the offset sticks
    // and the jump-to-latest button appears.
    ListState::new(0, ListAlignment::Bottom, px(OVERDRAW))
}

fn rows(workspace: &Workspace, transcript: &Transcript) -> Vec<Row> {
    let mut rows = Vec::new();

    for (index, turn) in transcript.turns.iter().enumerate() {
        rows.push(Row::Message(turn.prompt.clone()));

        // Only the last turn can still be running.
        let running = index + 1 == transcript.turns.len() && transcript.status.running;
        rows.push(Row::Divider {
            label: if running {
                "Working…".to_owned()
            } else {
                workspace.worked.to_owned()
            },
            running,
        });

        let segments = turn
            .parts
            .iter()
            .map(|segment| match segment {
                crate::live::Segment::Text { text, parsed } => Segment::Text(parsed.blocks(text)),
                crate::live::Segment::Reasoning(text) => Segment::Reasoning(text.clone()),
                crate::live::Segment::Tool {
                    name, detail, done, ..
                } => Segment::Tool {
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
        // Agent text is CommonMark while it streams: markers become weight,
        // mono runs, list indent and code surfaces.
        Segment::Text(blocks) => div()
            .id(id)
            .w_full()
            .min_w(px(0.))
            .flex_shrink_0()
            .py(px(7.))
            .child(crate::markdown::markdown(blocks, theme))
            .into_any_element(),
        Segment::Reasoning(text) => activity(&id, theme, "icons/brain.svg", None, text, true),
        Segment::Tool { name, detail, done } => {
            activity(&id, theme, tool_icon(name), Some(name), detail, *done)
        }
    }
}

/// A reasoning part and a shell command both can carry newlines. The collapsed
/// row shows one line, and GPUI paints every line of a `\n`-bearing string from
/// the same origin, so the text is joined with single spaces first.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
///
/// The row hugs its content (`min_h`, no fixed height) and the detail is
/// flattened to one line before it is painted. A provider may send reasoning
/// with paragraphs and a shell command with newlines, and GPUI breaks text on
/// `\n` whatever `white_space` says: a fixed-height row would paint those lines
/// outside its box, over the rows below it.
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
        .min_h(px(26.))
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
                .min_w(px(0.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(theme.text_faint)
                .child(flatten(detail)),
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

#[cfg(test)]
mod tests {
    use super::flatten;

    #[test]
    fn an_activity_detail_is_flattened_to_one_line() {
        // GPUI breaks a text element on every `\n` it finds, whatever the white
        // space says, so a multi-line detail must never reach the collapsed row.
        assert_eq!(flatten("a\nb\r\n\n c\t d "), "a b c d");
        assert_eq!(flatten("single line"), "single line");
        assert_eq!(flatten("   "), "");
    }
}
