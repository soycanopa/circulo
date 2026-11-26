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
    AnyElement, App, FontWeight, IntoElement, ListAlignment, ListOffset, ListState, SharedString,
    Window, div, list, prelude::*, px,
};

use crate::icons::icon;
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::theme::Theme;
use circulo_protocol::FinishReason;

/// Opening the rewind confirmation for a turn. The shell owns the request.
pub(crate) type RewindTurnHandler = Rc<dyn Fn(u64, &mut Window, &mut App)>;
/// Opening or closing a tool row. The shell owns the presentation state.
pub(crate) type ToggleToolHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// The transcript's actions, grouped so its signature stays readable.
pub(crate) struct TranscriptControls {
    pub can_rewind: bool,
    pub on_rewind: RewindTurnHandler,
    pub on_toggle_tool: ToggleToolHandler,
}

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
        /// The turn this divider closes; the rewind action names it.
        turn: u64,
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
        /// The provider's call id; the row toggles open by it.
        id: String,
        name: String,
        detail: String,
        done: bool,
        /// What the tool printed, shown when the row is open.
        output: Option<String>,
        expanded: bool,
    },
}

pub(crate) fn transcript(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
    list_state: &ListState,
    title: &str,
    controls: TranscriptControls,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .child(header(theme, title))
        .child(conversation(
            theme, workspace, transcript, list_state, controls,
        ))
}

fn header(theme: &Theme, title: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(46.))
        .px(px(24.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(div().flex_1())
        .child(icon("icons/panel-right.svg", 16., theme.text_faint))
}

fn conversation(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
    list_state: &ListState,
    controls: TranscriptControls,
) -> impl IntoElement {
    let rows = Rc::new(rows(workspace, transcript));
    let TranscriptControls {
        can_rewind,
        on_rewind,
        on_toggle_tool,
    } = controls;
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

    let on_rewind = on_rewind.clone();
    let on_toggle_tool = on_toggle_tool.clone();
    let list = list(state.clone(), move |index, _window, _cx| {
        row_view(
            &rows[index],
            index,
            &theme,
            can_rewind,
            &on_rewind,
            &on_toggle_tool,
        )
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
        let stopped = turn.finished == Some(FinishReason::Aborted);
        rows.push(Row::Divider {
            label: if running {
                "Working…".to_owned()
            } else if stopped {
                "Stopped".to_owned()
            } else {
                workspace.worked.to_owned()
            },
            running,
            turn: turn.turn,
        });

        let segments = turn
            .parts
            .iter()
            .map(|segment| match segment {
                crate::live::Segment::Text { text, parsed } => Segment::Text(parsed.blocks(text)),
                crate::live::Segment::Reasoning(text) => Segment::Reasoning(text.clone()),
                crate::live::Segment::Tool {
                    id,
                    name,
                    detail,
                    done,
                    output,
                    expanded,
                } => Segment::Tool {
                    id: id.clone(),
                    name: name.clone(),
                    detail: detail.clone(),
                    done: *done,
                    output: output.clone(),
                    expanded: *expanded,
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

fn row_view(
    row: &Row,
    index: usize,
    theme: &Theme,
    can_rewind: bool,
    on_rewind: &RewindTurnHandler,
    on_toggle_tool: &ToggleToolHandler,
) -> AnyElement {
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
        Row::Divider {
            label,
            running,
            turn,
        } => {
            let group = SharedString::from(format!("turn-{turn}"));
            div()
                .id(id)
                .group(group.clone())
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
                        })
                        .when(can_rewind && !*running, |row| {
                            let on = Rc::clone(on_rewind);
                            let turn = *turn;
                            row.child(
                                div()
                                    .id(SharedString::from(format!("rewind-turn-{turn}")))
                                    .flex_shrink_0()
                                    .px(px(4.))
                                    .cursor_pointer()
                                    .opacity(0.)
                                    .group_hover(group.clone(), |style| style.opacity(1.))
                                    .hover(|style| style.text_color(theme.text))
                                    .child("Rewind")
                                    .on_click(move |_, window, cx| on(turn, window, cx)),
                            )
                        }),
                )
                .child(div().flex_1().h(px(1.)).bg(theme.hairline))
                .into_any_element()
        }
        Row::Turn(segments) => {
            let mut turn = div().id(id).w_full().min_w(px(0.)).flex().flex_col();

            for (segment_index, segment) in segments.iter().enumerate() {
                let segment_id = SharedString::from(format!("row-{index}-segment-{segment_index}"));
                turn = turn.child(segment_view(segment, segment_id, theme, on_toggle_tool));
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

fn segment_view(
    segment: &Segment,
    id: SharedString,
    theme: &Theme,
    on_toggle_tool: &ToggleToolHandler,
) -> AnyElement {
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
        Segment::Tool {
            id: tool_id,
            name,
            detail,
            done,
            output,
            expanded,
        } => tool_row(
            ToolRow {
                row_id: &id,
                tool_id,
                name,
                detail,
                done: *done,
                output: output.as_deref(),
                expanded: *expanded,
            },
            theme,
            on_toggle_tool,
        ),
    }
}

/// The pieces of one tool row: the header text and the output it may reveal.
struct ToolRow<'a> {
    row_id: &'a SharedString,
    tool_id: &'a str,
    name: &'a str,
    detail: &'a str,
    done: bool,
    output: Option<&'a str>,
    expanded: bool,
}

/// A tool row plus, when opened, the output it printed. The header is the
/// collapsed row the transcript always had; a finished tool with output grows
/// a chevron and becomes clickable.
fn tool_row(row: ToolRow<'_>, theme: &Theme, on_toggle_tool: &ToggleToolHandler) -> AnyElement {
    let ToolRow {
        row_id,
        tool_id,
        name,
        detail,
        done,
        output,
        expanded,
    } = row;
    let has_output = output.is_some_and(|text| !text.is_empty());
    let mut column = div()
        .id(row_id.clone())
        .w_full()
        .min_w(px(0.))
        .flex_shrink_0()
        .flex()
        .flex_col();

    let mut header = div()
        .id(SharedString::from(format!("{row_id}-header")))
        .w_full()
        .min_w(px(0.))
        .flex()
        .items_center()
        .gap(px(6.))
        .min_h(px(26.))
        .text_size(px(12.))
        .child(icon(tool_icon(name), 13., theme.text_faint))
        .child(
            div()
                .flex_shrink_0()
                .text_color(theme.text_muted)
                .child(name.to_owned()),
        )
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
        .child(if !done {
            icon("icons/loader-circle.svg", 11., theme.text_faint)
        } else if has_output && expanded {
            icon("icons/chevron-down.svg", 11., theme.text_faint)
        } else {
            icon("icons/chevron-right.svg", 11., theme.text_faint)
        });

    if has_output {
        let on = Rc::clone(on_toggle_tool);
        let tool_id = tool_id.to_owned();
        header = header
            .cursor_pointer()
            .on_click(move |_, window, cx| on(&tool_id, window, cx));
    }
    column = column.child(header);

    if expanded && let Some(output) = output {
        column = column.child(output_block(output, theme));
    }

    column.into_any_element()
}

/// What the tool printed, in the same chip and mono face as a fenced code
/// block. GPUI breaks the text on `\n`, so the block needs no line layout.
fn output_block(text: &str, theme: &Theme) -> AnyElement {
    div()
        .w_full()
        .min_w(px(0.))
        .flex_shrink_0()
        .mt(px(2.))
        .mb(px(6.))
        .rounded(px(8.))
        .bg(theme.chip)
        .px(px(10.))
        .py(px(8.))
        .text_size(px(11.5))
        .line_height(px(17.))
        .font_family("Menlo")
        .text_color(theme.text_muted)
        .child(text.to_owned())
        .into_any_element()
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
