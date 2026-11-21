//! The transcript: header, user message, the worked divider, assistant blocks
//! and, before a project is open, the empty state.
//!
//! Virtualization and live parts come with the protocol; the visual language is
//! already the final one.

use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::mock::{Block, ToolState, Workspace};
use crate::shell::View;
use crate::text::inline;
use crate::theme::Theme;

/// The reference keeps the transcript text off the panel edges by this much;
/// there is no narrower reading column.
const PADDING: f32 = 34.;

pub(crate) fn transcript(theme: &Theme, view: View, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .child(match view {
            View::Empty => header(theme, workspace.empty.header),
            View::Session => header(theme, workspace.title),
        })
        .child(match view {
            View::Empty => empty(theme, workspace).into_any_element(),
            View::Session => conversation(theme, workspace).into_any_element(),
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

fn conversation(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .id("transcript")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .px(px(PADDING))
        .pb(px(24.))
        .child(user_message(theme, workspace))
        .child(worked(theme, workspace))
        .child(
            div().flex().flex_col().gap(px(2.)).children(
                workspace
                    .blocks
                    .iter()
                    .map(|block| block_view(theme, block)),
            ),
        )
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

fn user_message(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div().flex().justify_end().pt(px(30.)).pb(px(26.)).child(
        div()
            .max_w(px(520.))
            .px(px(15.))
            .py(px(9.))
            .rounded(px(12.))
            .bg(theme.chip)
            .text_size(px(13.))
            .child(workspace.user_message),
    )
}

fn worked(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
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
                .gap(px(3.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(workspace.worked)
                .child(icon("icons/chevron-right.svg", 11., theme.text_faint)),
        )
        .child(div().flex_1().h(px(1.)).bg(theme.hairline))
}

fn block_view(theme: &Theme, block: &Block) -> AnyElement {
    match block {
        Block::Heading(text) => div()
            .py(px(7.))
            .text_size(px(13.))
            .line_height(px(20.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(*text)
            .into_any_element(),
        Block::Paragraph(text) => div()
            .py(px(7.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(inline(text, theme))
            .into_any_element(),
        Block::Bullet(text) => div()
            .flex()
            .gap(px(8.))
            .py(px(4.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(div().child("•"))
            .child(div().flex_1().child(inline(text, theme)))
            .into_any_element(),
        Block::Reasoning(summary) => {
            activity(theme, "icons/brain.svg", None, summary, true).into_any_element()
        }
        Block::Tool {
            icon: path,
            name,
            detail,
            state,
        } => {
            activity(theme, path, Some(name), detail, *state == ToolState::Done).into_any_element()
        }
    }
}

/// One collapsed activity row: icon, label, what it acted on, and a chevron or
/// a spinner. Expanded output arrives with streaming.
fn activity(
    theme: &Theme,
    path: &'static str,
    name: Option<&'static str>,
    detail: &'static str,
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
            row.child(div().text_color(theme.text_muted).child(name))
        })
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(theme.text_faint)
                .child(detail),
        )
        .child(if done {
            icon("icons/chevron-right.svg", 11., theme.text_faint)
        } else {
            icon("icons/loader-circle.svg", 11., theme.text_faint)
        })
}
