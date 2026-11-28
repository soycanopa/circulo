//! The permission card: the agent is waiting before it runs a tool.
//!
//! Placement and anatomy follow the reference (see `docs/permissions.md`): the
//! card is a sibling of the transcript, between it and the composer, so a
//! pending question pushes the composer down instead of floating over it.
//!
//! The card never invents an option. The agent sends the options, their labels
//! and their kinds; this module only decides which one reads as the primary
//! action (the allow kinds) and which reads as the quiet way out (the reject
//! kinds). The detail block shows the command or path in full: the decision is
//! about what will actually run.

use std::rc::Rc;

use circulo_protocol::PermissionOptionKind;
use gpui::{AnyElement, App, FocusHandle, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::icons::icon;
use crate::live::PendingPermission;
use crate::theme::Theme;

/// Answering the card. `Some(option)` picks one of the agent's options;
/// `None` cancels the request. The window and app come from the click handler,
/// which is what lets the shell pop the card and reach the daemon.
pub(crate) type Answer = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;

pub(crate) fn permission_card(
    theme: &Theme,
    request: &PendingPermission,
    focus: &FocusHandle,
    answer: Answer,
) -> AnyElement {
    // Enter allows, Escape rejects: the two decisions a card of this shape
    // always has. Without a reject option, Escape cancels.
    let allow = first_of(request, true);
    let reject = first_of(request, false);

    let mut buttons = div().flex().items_center().gap(px(8.)).mt(px(10.));

    for option in &request.options {
        let answer = Rc::clone(&answer);
        let id = option.id.clone();
        let primary = is_allow(option.kind);

        buttons = buttons.child(
            div()
                .id(SharedString::from(format!(
                    "permission-{}-{}",
                    request.id, option.id
                )))
                .h(px(28.))
                .px(px(13.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(px(12.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .when(primary, |button| {
                    button
                        .bg(theme.text)
                        .text_color(theme.canvas)
                        .hover(|style| style.opacity(0.9))
                })
                .when(!primary, |button| {
                    button
                        .border_1()
                        .border_color(theme.border)
                        .text_color(theme.text_muted)
                        .hover(|style| style.bg(theme.chip).text_color(theme.text))
                })
                .active(|style| style.opacity(0.8))
                .child(SharedString::from(option.label.clone()))
                .on_click(move |_, window, cx| answer(Some(id.clone()), window, cx)),
        );
    }

    let on_key = Rc::clone(&answer);

    div()
        .w_full()
        .px(px(24.))
        .pb(px(8.))
        .track_focus(focus)
        .on_key_down(
            move |event, window, cx| match event.keystroke.key.as_str() {
                "enter" => {
                    if let Some(id) = &allow {
                        on_key(Some(id.clone()), window, cx);
                    }
                }
                "escape" => on_key(reject.clone(), window, cx),
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
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("icons/alert.svg", 13., theme.accent))
                        .child(
                            div()
                                .min_w(px(0.))
                                .flex_1()
                                .text_size(px(12.5))
                                .line_height(px(17.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text)
                                // The reference keeps a title to two lines;
                                // a longer one is clipped, not a wall.
                                .max_h(px(34.))
                                .overflow_hidden()
                                .child(SharedString::from(request.title.clone())),
                        ),
                )
                // A provider that has nothing but a path sends it as both the
                // title and the detail; showing it twice reads like a bug.
                .when(
                    !request.detail.is_empty() && request.detail != request.title,
                    |card| {
                        card.child(
                            div()
                                .id("permission-detail")
                                .mt(px(8.))
                                .max_h(px(92.))
                                .overflow_y_scroll()
                                .p(px(8.))
                                .rounded(px(7.))
                                .bg(theme.chip)
                                .font_family("Menlo")
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(theme.text_muted)
                                .whitespace_normal()
                                .child(SharedString::from(request.detail.clone())),
                        )
                    },
                )
                .child(buttons),
        )
        .into_any_element()
}

/// The first option of one side of the decision, by kind. `allow` picks the
/// allow kinds; the rest are rejections.
fn first_of(request: &PendingPermission, allow: bool) -> Option<String> {
    request
        .options
        .iter()
        .find(|option| is_allow(option.kind) == allow)
        .map(|option| option.id.clone())
}

fn is_allow(kind: PermissionOptionKind) -> bool {
    matches!(
        kind,
        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
    )
}
