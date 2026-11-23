//! The composer: input surface, the shared controls and the footer.
//!
//! The text field is ours: GPUI has no text input element. `crate::input` owns
//! the buffer and the caret; this module paints it and the shell handles the
//! keys. Enter sends, Shift+Enter breaks the line.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window, div,
    prelude::*, px,
};

use crate::icons::icon;
use crate::input::{self, TextInput};
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::theme::Theme;
use circulo_protocol::ConfigOption;

/// How tall the field may grow before it scrolls: four lines of body text.
const FIELD_MAX_HEIGHT: f32 = 96.;

/// Keys the field does not handle itself, and what the send button does. The
/// shell owns both, because both change its state.
pub(crate) type KeyHandler = Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App)>;
pub(crate) type SendHandler = Rc<dyn Fn(&mut Window, &mut App)>;
/// Opening or closing an option menu, and picking a value. The shell owns
/// both, because both change its state.
pub(crate) type ToggleHandler = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
pub(crate) type ChooseHandler = Rc<dyn Fn(String, String, &mut Window, &mut App)>;

/// The daemon's options as the pickers draw them: which menu is open and what
/// choosing does. A missing option id keeps its mock label and does nothing.
pub(crate) struct Pickers<'a> {
    pub options: &'a [ConfigOption],
    pub open: Option<&'a str>,
    pub on_toggle: ToggleHandler,
    pub on_choose: ChooseHandler,
}

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
    pickers: Pickers<'_>,
    field: Field<'_>,
) -> Div {
    let running = transcript.status.running;
    let can_send = !field.input.is_blank() && !running;
    let menu = pickers
        .open
        .and_then(|open| pickers.options.iter().find(|option| option.id == open))
        .map(|option| menu_view(theme, option, &pickers.on_choose));

    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .children(menu)
        .child(input_box(
            theme, workspace, running, &pickers, field, can_send,
        ))
        .child(footer(theme, workspace, running))
}

fn input_box(
    theme: &Theme,
    workspace: &Workspace,
    running: bool,
    pickers: &Pickers<'_>,
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
                .child(input::field(theme, field.input, focused, "Do anything...")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(slot(theme, pickers, "model", None, workspace.model))
                .child(slot(theme, pickers, "effort", None, workspace.reasoning))
                .child(static_picker(
                    theme,
                    Some("icons/lock.svg"),
                    workspace.access,
                ))
                .child(slot(
                    theme,
                    pickers,
                    "mode",
                    Some("icons/hammer.svg"),
                    workspace.agent_mode,
                ))
                .child(div().flex_1())
                .child(send(theme, running, can_send, on_send)),
        )
}

/// One composer slot: the daemon's value when the option exists, the mock
/// label until then. A live option opens its menu on click.
fn slot(
    theme: &Theme,
    pickers: &Pickers<'_>,
    id: &str,
    path: Option<&'static str>,
    fallback: &'static str,
) -> impl IntoElement {
    match pickers.options.iter().find(|option| option.id == id) {
        Some(option) => {
            let label = current_label(option);
            let open = pickers.open == Some(option.id.as_str());
            let next = if open { None } else { Some(option.id.clone()) };
            let on_toggle = pickers.on_toggle.clone();
            picker(theme, path, label, open)
                .id(SharedString::from(format!("picker-{}", option.id)))
                .cursor_pointer()
                .on_click(move |_, window, cx| on_toggle(next.clone(), window, cx))
                .into_any_element()
        }
        None => static_picker(theme, path, fallback).into_any_element(),
    }
}

fn static_picker(
    theme: &Theme,
    path: Option<&'static str>,
    label: &'static str,
) -> impl IntoElement {
    picker(theme, path, label.to_owned(), false)
}

fn picker(theme: &Theme, path: Option<&'static str>, label: String, open: bool) -> Div {
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
        .child(
            div()
                .flex_shrink_0()
                .max_w(px(200.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(label),
        )
        .child(icon("icons/chevron-down.svg", 11., theme.text_faint))
        .when(open, |row| row.bg(theme.hover))
        .hover(move |style| style.bg(theme.hover))
}

/// The agent's wording for its current value, or the raw id when the choice
/// list does not name it.
fn current_label(option: &ConfigOption) -> String {
    option
        .options
        .iter()
        .find(|choice| choice.id == option.current)
        .map(|choice| choice.label.clone())
        .unwrap_or_else(|| option.current.clone())
}

/// The open option's choices, above the composer. The current value is marked;
/// anything else sends the choice to the daemon.
fn menu_view(theme: &Theme, option: &ConfigOption, on_choose: &ChooseHandler) -> AnyElement {
    let mut menu = div()
        .w_full()
        .flex()
        .flex_col()
        .mb(px(8.))
        .p(px(6.))
        .rounded(px(10.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(option.label.clone()),
        );

    for choice in &option.options {
        let current = choice.id == option.current;
        let id = option.id.clone();
        let value = choice.id.clone();
        let on_choose = on_choose.clone();
        menu = menu.child(
            div()
                .id(SharedString::from(format!(
                    "choice-{}-{}",
                    option.id, choice.id
                )))
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(8.))
                .py(px(6.))
                .rounded(px(6.))
                .text_size(px(12.5))
                .text_color(if current {
                    theme.text
                } else {
                    theme.text_muted
                })
                .child(
                    div()
                        .w(px(14.))
                        .flex_shrink_0()
                        .text_color(theme.accent)
                        .child(if current { "✓" } else { "" }),
                )
                .child(choice.label.clone())
                .cursor_pointer()
                .hover(move |style| style.bg(theme.hover))
                .on_click(move |_, window, cx| on_choose(id.clone(), value.clone(), window, cx)),
        );
    }

    menu.into_any_element()
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
