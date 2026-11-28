//! The question card: the agent needs an answer, not an approval.
//!
//! ACP calls this elicitation in form mode; `docs/permissions.md` carries the
//! contract. The shape follows the reference's question card: the agent's
//! message, one block per field, and the answer options as full-width rows with
//! the chosen one tinted in the accent colour.
//!
//! Choice fields are option rows; a free-text field is the same input the
//! composer uses (`crate::input`), with its own focus. `Submit` waits until
//! every required field has an answer, whichever kind it is.

use std::collections::BTreeMap;
use std::rc::Rc;

use circulo_protocol::{QuestionField, QuestionKind};
use gpui::{
    AnyElement, App, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window, div, prelude::*,
    px,
};

use crate::input::{self, TextInput};
use crate::live::PendingQuestion;
use crate::theme::Theme;

/// What the reader did with the card.
pub(crate) enum QuestionEvent {
    Select { field: String, value: String },
    Submit,
    Decline,
    Cancel,
}

pub(crate) type QuestionHandler = Rc<dyn Fn(QuestionEvent, &mut Window, &mut App)>;
/// Editing keys inside a text field, tagged with the field they belong to.
pub(crate) type TextKeyHandler = Rc<dyn Fn(&str, &KeyDownEvent, &mut Window, &mut App)>;

pub(crate) struct Handlers {
    pub event: QuestionHandler,
    pub text_key: TextKeyHandler,
}

/// What the reader has answered so far: a value per choice field and a buffer
/// per text field.
pub(crate) struct Draft<'a> {
    pub choices: &'a BTreeMap<String, String>,
    pub texts: &'a BTreeMap<String, TextInput>,
    pub focuses: &'a BTreeMap<String, FocusHandle>,
}

impl Draft<'_> {
    fn value(&self, field: &QuestionField) -> Option<&str> {
        match &field.kind {
            QuestionKind::Choice { .. } => self.choices.get(&field.id).map(String::as_str),
            QuestionKind::Text => self
                .texts
                .get(&field.id)
                .map(TextInput::text)
                .filter(|text| !text.is_empty()),
        }
    }
}

pub(crate) fn question_card(
    theme: &Theme,
    request: &PendingQuestion,
    draft: &Draft<'_>,
    focus: &FocusHandle,
    window: &Window,
    handlers: &Handlers,
) -> AnyElement {
    let on = handlers.event.clone();
    let ready = complete(request, draft);

    let mut fields = div().flex().flex_col().gap(px(10.)).mt(px(10.));

    for field in &request.fields {
        fields = fields.child(field_block(theme, field, draft, window, handlers));
    }

    let submit_button = div()
        .id("question-submit")
        .h(px(28.))
        .px(px(13.))
        .rounded(px(7.))
        .flex()
        .items_center()
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .when(ready, |button| {
            let on = Rc::clone(&on);
            button
                .cursor_pointer()
                .bg(theme.text)
                .text_color(theme.canvas)
                .hover(|style| style.opacity(0.9))
                .active(|style| style.opacity(0.8))
                .child("Submit")
                .on_click(move |_, window, cx| on(QuestionEvent::Submit, window, cx))
        })
        .when(!ready, |button| {
            button
                .bg(theme.chip)
                .text_color(theme.text_faint)
                .child("Submit")
        });

    let decline = {
        let on = Rc::clone(&on);
        div()
            .id("question-decline")
            .h(px(28.))
            .px(px(13.))
            .rounded(px(7.))
            .flex()
            .items_center()
            .cursor_pointer()
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.5))
            .text_color(theme.text_muted)
            .hover(|style| style.bg(theme.chip).text_color(theme.text))
            .child("Decline")
            .on_click(move |_, window, cx| on(QuestionEvent::Decline, window, cx))
    };

    let cancel = {
        let on = Rc::clone(&on);
        div()
            .id("question-cancel")
            .h(px(28.))
            .px(px(10.))
            .rounded(px(7.))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(12.))
            .text_color(theme.text_faint)
            .hover(|style| style.text_color(theme.text))
            .child("Cancel")
            .on_click(move |_, window, cx| on(QuestionEvent::Cancel, window, cx))
    };

    let on_key = Rc::clone(&on);
    let is_ready = ready;

    div()
        .w_full()
        .px(px(24.))
        .pb(px(8.))
        .track_focus(focus)
        .on_key_down(
            move |event, window, cx| match event.keystroke.key.as_str() {
                "enter" => {
                    if is_ready {
                        on_key(QuestionEvent::Submit, window, cx);
                    }
                }
                "escape" => on_key(QuestionEvent::Cancel, window, cx),
                _ => {}
            },
        )
        .child(
            div()
                .w_full()
                .p(px(12.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .shadow_md()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text)
                        .child(SharedString::from(request.message.clone())),
                )
                .child(fields)
                .child(
                    div()
                        .mt(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(submit_button)
                        .child(decline)
                        .child(cancel),
                ),
        )
        .into_any_element()
}

fn field_block(
    theme: &Theme,
    field: &QuestionField,
    draft: &Draft<'_>,
    window: &Window,
    handlers: &Handlers,
) -> AnyElement {
    let mut block = div().flex().flex_col().gap(px(4.)).child(
        div()
            .text_size(px(11.5))
            .text_color(theme.text_muted)
            .child(SharedString::from(field.label.clone())),
    );

    let QuestionKind::Choice { options } = &field.kind else {
        return block
            .child(text_field(theme, field, draft, window, handlers))
            .into_any_element();
    };

    let on = handlers.event.clone();
    let selected = draft.choices.get(&field.id).cloned();

    for option in options {
        let on = Rc::clone(&on);
        let field_id = field.id.clone();
        let value = option.clone();
        let is_selected = selected.as_deref() == Some(option.as_str());

        block = block.child(
            div()
                .id(SharedString::from(format!(
                    "question-{}-{}",
                    field.id, option
                )))
                .min_h(px(30.))
                .px(px(10.))
                .py(px(4.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .cursor_pointer()
                .border_1()
                .border_color(if is_selected {
                    theme.accent
                } else {
                    theme.border
                })
                .bg(if is_selected {
                    theme.chip
                } else {
                    theme.surface
                })
                .text_size(px(12.5))
                .text_color(if is_selected {
                    theme.text
                } else {
                    theme.text_muted
                })
                .hover(|style| style.bg(theme.chip).text_color(theme.text))
                .child(SharedString::from(option.clone()))
                .on_click(move |_, window, cx| {
                    on(
                        QuestionEvent::Select {
                            field: field_id.clone(),
                            value: value.clone(),
                        },
                        window,
                        cx,
                    );
                }),
        );
    }

    block.into_any_element()
}

/// Every required field has an answer, whatever its kind.
fn complete(request: &PendingQuestion, draft: &Draft<'_>) -> bool {
    request
        .fields
        .iter()
        .filter(|field| field.required)
        .all(|field| {
            draft
                .value(field)
                .is_some_and(|value| !value.trim().is_empty())
        })
}

/// One free-text answer. The input, the caret and the keys are the composer's,
/// so both read the same.
fn text_field(
    theme: &Theme,
    field: &QuestionField,
    draft: &Draft<'_>,
    window: &Window,
    handlers: &Handlers,
) -> AnyElement {
    let input = draft.texts.get(&field.id);
    let handle = draft.focuses.get(&field.id);
    let (Some(input), Some(handle)) = (input, handle) else {
        return div().into_any_element();
    };

    let on_key = handlers.text_key.clone();
    let id = field.id.clone();

    div()
        .id(SharedString::from(format!("question-text-{}", field.id)))
        .min_h(px(30.))
        .px(px(10.))
        .py(px(4.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .text_size(px(12.5))
        .track_focus(handle)
        .on_key_down(move |event, window, cx| on_key(&id, event, window, cx))
        .on_click({
            let handle = handle.clone();
            move |_, window, _| handle.focus(window)
        })
        .child(input::field(
            theme,
            input,
            handle.is_focused(window),
            "Type an answer…",
        ))
        .into_any_element()
}
