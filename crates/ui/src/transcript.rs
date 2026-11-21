//! The transcript: header, user message, the worked divider and assistant
//! blocks. Virtualization and live parts come with the protocol; the visual
//! language is already the final one.

use gpui::{IntoElement, div, prelude::*, px};

use crate::icons::icon;
use crate::mock::{Block, Workspace};
use crate::text::inline;
use crate::theme::Theme;

/// The reference keeps the transcript text off the panel edges by this much;
/// there is no narrower reading column.
const PADDING: f32 = 34.;

pub(crate) fn transcript(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .child(header(theme, workspace))
        .child(
            div()
                .id("transcript")
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .px(px(PADDING))
                .child(user_message(theme, workspace))
                .child(worked(theme, workspace))
                .children(
                    workspace
                        .blocks
                        .iter()
                        .map(|block| block_view(theme, block)),
                ),
        )
}

fn header(theme: &Theme, workspace: &Workspace) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(46.))
        .px(px(24.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(workspace.title),
        )
        .child(div().flex_1())
        .child(icon("icons/panel-right.svg", 16., theme.text_faint))
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

fn block_view(theme: &Theme, block: &Block) -> impl IntoElement {
    match block {
        Block::Heading(text) => div()
            .py(px(7.))
            .text_size(px(13.))
            .line_height(px(20.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(*text),
        Block::Paragraph(text) => div()
            .py(px(7.))
            .text_size(px(13.))
            .child(inline(text, theme)),
        Block::Bullet(text) => div()
            .flex()
            .gap(px(8.))
            .py(px(4.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(div().child("•"))
            .child(div().flex_1().child(inline(text, theme))),
    }
}
