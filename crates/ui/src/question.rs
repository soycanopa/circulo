//! The question card: the agent needs an answer, not an approval.
//!
//! ACP calls this elicitation in form mode; `docs/permissions.md` carries the
//! contract. The shape follows the reference's question card: the agent's
//! message, one block per field, and the answer options as full-width rows with
//! the chosen one tinted in the accent colour.
//!
//! Only choice fields can be answered here. A field that needs free text shows
//! what it wants and the card falls back to Decline, because typing lives in
//! the composer that does not exist yet.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use circulo_protocol::{QuestionField, QuestionKind};
use gpui::{AnyElement, App, FocusHandle, IntoElement, SharedString, Window, div, prelude::*, px};

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

pub(crate) fn question_card(
    theme: &Theme,
    request: &PendingQuestion,
    answers: &BTreeMap<String, String>,
    focus: &FocusHandle,
    on: QuestionHandler,
) -> AnyElement {
    let needs_typing = request
        .fields
        .iter()
        .any(|field| !matches!(field.kind, QuestionKind::Choice { .. }));
    let ready = !needs_typing && complete(request, answers);

    let mut fields = div().flex().flex_col().gap(px(10.)).mt(px(10.));

    for field in &request.fields {
        fields = fields.child(field_block(theme, field, answers, on.clone()));
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
        .on_key_down(move |event, window, cx| match event.keystroke.key.as_str() {
            "enter" => {
                if is_ready {
                    on_key(QuestionEvent::Submit, window, cx);
                }
            }
            "escape" => on_key(QuestionEvent::Cancel, window, cx),
            _ => {}
        })
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
                .when(needs_typing, |card| {
                    card.child(
                        div()
                            .mt(px(8.))
                            .text_size(px(12.))
                            .text_color(theme.text_faint)
                            .child("This question needs a typed answer. Decline it and ask in the chat."),
                    )
                })
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
    answers: &BTreeMap<String, String>,
    on: QuestionHandler,
) -> AnyElement {
    let mut block = div().flex().flex_col().gap(px(4.)).child(
        div()
            .text_size(px(11.5))
            .text_color(theme.text_muted)
            .child(SharedString::from(field.label.clone())),
    );

    let QuestionKind::Choice { options } = &field.kind else {
        return block
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_faint)
                    .child("A text answer is not available in this card."),
            )
            .into_any_element();
    };

    let selected = answers.get(&field.id).cloned();

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

/// Every required field has an answer, and no field is unanswered by choice.
fn complete(request: &PendingQuestion, answers: &BTreeMap<String, String>) -> bool {
    let answered: BTreeSet<&str> = answers
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(field, _)| field.as_str())
        .collect();

    request
        .fields
        .iter()
        .filter(|field| field.required)
        .all(|field| answered.contains(field.id.as_str()))
}
