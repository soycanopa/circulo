//! The composer: input surface, the shared controls and the footer.
//!
//! The text field is ours: GPUI has no text input element. `crate::input` owns
//! the buffer and the caret; this module paints it and the shell handles the
//! keys. Enter sends, Shift+Enter breaks the line.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, FocusHandle, Font, FontFeatures, FontStyle, FontWeight, IntoElement,
    KeyDownEvent, StyledText, TextRun, Window, div, prelude::*, px,
};

use crate::icons::icon;
use crate::input::TextInput;
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::theme::Theme;

/// How tall the field may grow before it scrolls: four lines of body text.
const FIELD_MAX_HEIGHT: f32 = 96.;

/// Keys the field does not handle itself, and what the send button does. The
/// shell owns both, because both change its state.
pub(crate) type KeyHandler = Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App)>;
pub(crate) type SendHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Everything the field needs that changes between frames.
pub(crate) struct Field<'a> {
    pub input: &'a TextInput,
    pub focus: &'a FocusHandle,
    pub focused: bool,
    pub on_key: KeyHandler,
    pub on_send: SendHandler,
}

pub(crate) fn composer(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
    field: Field<'_>,
) -> Div {
    let running = transcript.status.running;
    let can_send = !field.input.is_blank() && !running;

    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .child(input_box(theme, workspace, running, field, can_send))
        .child(footer(theme, workspace, running))
}

fn input_box(
    theme: &Theme,
    workspace: &Workspace,
    running: bool,
    field: Field<'_>,
    can_send: bool,
) -> impl IntoElement {
    let focus = field.focus;
    let focused = field.focused;
    let on_key = field.on_key;
    let on_send = field.on_send;
    let click_focus = focus.clone();

    div()
        .id("composer")
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(13.))
        .rounded(px(14.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .text_size(px(13.))
        .track_focus(focus)
        .on_key_down(move |event, window, cx| on_key(event, window, cx))
        .on_click(move |_, window, _| click_focus.focus(window))
        .child(
            div()
                .id("composer-field")
                .max_h(px(FIELD_MAX_HEIGHT))
                .overflow_y_scroll()
                .whitespace_normal()
                .child(text_field(theme, field.input, focused)),
        )
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
                .child(send(theme, running, can_send, on_send)),
        )
}

/// The text, with the caret as a block behind the next character. Runs are the
/// only way to colour part of a `StyledText`, and a caret is exactly that.
fn text_field(theme: &Theme, input: &TextInput, focused: bool) -> AnyElement {
    if input.is_blank() && input.text().is_empty() {
        if focused {
            return caret(theme).into_any_element();
        }
        return div()
            .text_color(theme.text_faint)
            .child("Do anything...")
            .into_any_element();
    }

    let text = input.text();
    let caret = input.caret().min(text.len());
    let mut styled = Vec::new();
    let mut runs = Vec::new();

    if caret > 0 {
        styled.push(text[..caret].to_owned());
        runs.push(run(&text[..caret], theme.text, None));
    }

    if focused {
        // Highlight the character the caret sits on; at the end, a space.
        let next = text[caret..].chars().next();
        let marked: String = match next {
            Some(character) => character.to_string(),
            None => " ".to_owned(),
        };
        styled.push(marked.clone());
        runs.push(run(&marked, theme.canvas, Some(theme.accent)));

        if next.is_some() {
            let rest = &text[caret + marked.len()..];
            if !rest.is_empty() {
                styled.push(rest.to_owned());
                runs.push(run(rest, theme.text, None));
            }
        }
    } else if caret < text.len() {
        styled.push(text[caret..].to_owned());
        runs.push(run(&text[caret..], theme.text, None));
    }

    StyledText::new(styled.concat())
        .with_runs(runs)
        .into_any_element()
}

fn run(text: &str, color: gpui::Rgba, background: Option<gpui::Rgba>) -> TextRun {
    TextRun {
        len: text.len(),
        font: Font {
            family: ".SystemUIFont".into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        },
        color: color.into(),
        background_color: background.map(Into::into),
        underline: None,
        strikethrough: None,
    }
}

fn caret(theme: &Theme) -> impl IntoElement {
    div().h(px(17.)).w(px(1.5)).bg(theme.text)
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

fn send(theme: &Theme, running: bool, can_send: bool, on_send: SendHandler) -> impl IntoElement {
    let button = div()
        .id("composer-send")
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded_full()
        .bg(if can_send { theme.text } else { theme.chip })
        .child(if running {
            icon("icons/loader-circle.svg", 14., theme.text_faint)
        } else {
            icon(
                "icons/arrow-up.svg",
                14.,
                if can_send {
                    theme.canvas
                } else {
                    theme.text_faint
                },
            )
        });

    if can_send {
        button
            .cursor_pointer()
            .hover(|style| style.opacity(0.9))
            .on_click(move |_, window, cx| on_send(window, cx))
    } else {
        button
    }
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
