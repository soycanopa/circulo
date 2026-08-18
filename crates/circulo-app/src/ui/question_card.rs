use std::collections::HashMap;

use circulo_i18n::Catalog;
use circulo_protocol::UserQuestionBody;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled,
};

use crate::icons::{icon, path as icon_path};
use crate::shell::AppShell;
use crate::theme::{
    ACCENT, ACCENT_SURFACE, ACTIVITY_HOVER, BORDER, BG_SIDEBAR, INPUT_HEIGHT_PX, TEXT,
    TEXT_MUTED, TEXT_TERTIARY,
};
use crate::ui::TextInput;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingQuestion {
    pub request_id: String,
    pub questions: Vec<UserQuestionBody>,
    pub question_index: usize,
    pub selections: HashMap<String, Vec<String>>,
    pub custom_answers: HashMap<String, String>,
}

impl PendingQuestion {
    pub fn new(request_id: String, questions: Vec<UserQuestionBody>) -> Self {
        Self {
            request_id,
            questions,
            question_index: 0,
            selections: HashMap::new(),
            custom_answers: HashMap::new(),
        }
    }

    pub fn current_question(&self) -> Option<&UserQuestionBody> {
        self.questions.get(self.question_index)
    }

    pub fn answers(&self) -> Vec<circulo_protocol::QuestionAnswerBody> {
        self.questions
            .iter()
            .map(|question| {
                let custom = self
                    .custom_answers
                    .get(&question.id)
                    .map(|answer| answer.trim())
                    .filter(|answer| !answer.is_empty());
                circulo_protocol::QuestionAnswerBody {
                    question_id: question.id.clone(),
                    answers: custom.map_or_else(
                        || {
                            self.selections
                                .get(&question.id)
                                .cloned()
                                .unwrap_or_default()
                        },
                        |answer| vec![answer.to_owned()],
                    ),
                }
            })
            .collect()
    }
}

pub fn question_card(
    pending: &PendingQuestion,
    custom_input: Entity<TextInput>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let Some(question) = pending.current_question() else {
        return div().into_any_element();
    };
    let selected = pending
        .selections
        .get(&question.id)
        .cloned()
        .unwrap_or_default();
    let has_custom = pending
        .custom_answers
        .get(&question.id)
        .is_some_and(|answer| !answer.trim().is_empty());
    let can_continue = has_custom || !selected.is_empty();
    let is_last = pending.question_index + 1 == pending.questions.len();
    let request_id = pending.request_id.clone();
    let question_index = pending.question_index;
    let back_label = catalog.get("question.back").to_string();
    let continue_label = if is_last {
        catalog.get("question.submit").to_string()
    } else {
        catalog.get("question.next").to_string()
    };

    let mut options = div().mt(px(9.)).flex().flex_col().gap(px(4.));
    for (index, option) in question.options.iter().enumerate() {
        let is_selected = selected.iter().any(|answer| answer == &option.label);
        let click_label = option.label.clone();
        options = options.child(
            div()
                .id(SharedString::from(format!(
                    "user-input-{request_id}-{question_index}-option-{index}"
                )))
                .min_h(px(36.))
                .px(px(10.))
                .py(px(5.))
                .rounded(px(8.))
                .border_1()
                .when(is_selected, |row| row.border_color(ACCENT).bg(ACCENT_SURFACE))
                .when(!is_selected, |row| {
                    row.border_color(BG_SIDEBAR).bg(ACTIVITY_HOVER).hover(|style| {
                        style.border_color(BORDER).bg(ACCENT_SURFACE)
                    })
                })
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_question_option(click_label.clone(), cx);
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(TEXT)
                                .child(option.label.clone()),
                        )
                        .when_some(option.description.as_ref(), |el, description| {
                            el.child(
                                div()
                                    .mt(px(1.))
                                    .text_size(px(10.))
                                    .line_height(px(13.))
                                    .text_color(TEXT_MUTED)
                                    .child(description.clone()),
                            )
                        }),
                )
                .when(is_selected, |row| {
                    row.child(icon(icon_path::CHECK, px(12.), ACCENT))
                }),
        );
    }

    div()
        .flex_none()
        .w_full()
        .px(px(14.))
        .pt(px(12.))
        .pb(px(10.))
        .rounded(px(13.))
        .border_1()
        .border_color(BORDER)
        .bg(BG_SIDEBAR)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT_TERTIARY)
                        .child(question.header.clone()),
                )
                .when(pending.questions.len() > 1, |row| {
                    row.child(
                        div()
                            .h(px(18.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(ACTIVITY_HOVER)
                            .flex()
                            .items_center()
                            .text_size(px(9.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT_TERTIARY)
                            .child(format!(
                                "{}/{}",
                                question_index + 1,
                                pending.questions.len()
                            )),
                    )
                }),
        )
        .child(
            div()
                .mt(px(5.))
                .text_size(px(13.))
                .line_height(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT)
                .child(question.question.clone()),
        )
        .when(!question.options.is_empty(), |card| card.child(options))
        .child(
            div()
                .mt(px(if question.options.is_empty() { 9. } else { 4. }))
                .h(px(INPUT_HEIGHT_PX))
                .px(px(10.))
                .rounded(px(8.))
                .border_1()
                .when(has_custom, |field| field.border_color(ACCENT).bg(ACCENT_SURFACE))
                .when(!has_custom, |field| {
                    field.border_color(BG_SIDEBAR).bg(ACTIVITY_HOVER)
                })
                .flex()
                .items_center()
                .gap(px(7.))
                .child(icon(
                    icon_path::PENCIL,
                    px(11.),
                    if has_custom {
                        ACCENT
                    } else {
                        TEXT_TERTIARY
                    },
                ))
                .child(custom_input.clone()),
        )
        .child(
            div()
                .mt(px(8.))
                .flex()
                .items_center()
                .when(question_index > 0, |row| {
                    row.child(
                        div()
                            .id(SharedString::from(format!(
                                "user-input-{request_id}-{question_index}-back"
                            )))
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT_TERTIARY)
                            .cursor_pointer()
                            .hover(|style| style.bg(ACTIVITY_HOVER).text_color(TEXT_MUTED))
                            .on_click(cx.listener(|this, _, _, cx| this.previous_question(cx)))
                            .child(back_label),
                    )
                })
                .child(div().flex_1())
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "user-input-{request_id}-{question_index}-continue"
                        )))
                        .h(px(26.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .when(can_continue, |button| {
                            button
                                .bg(ACCENT)
                                .text_color(TEXT)
                                .cursor_pointer()
                                .hover(|style| style.opacity(0.9))
                                .on_click(cx.listener(|this, _, _, cx| this.advance_question(cx)))
                        })
                        .when(!can_continue, |button| {
                            button.bg(ACTIVITY_HOVER).text_color(TEXT_TERTIARY)
                        })
                        .child(continue_label),
                ),
        )
        .into_any_element()
}
