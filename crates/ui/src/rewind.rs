//! The rewind confirmation card.
//!
//! Rewinding is destructive: it restores the project worktree to a checkpoint,
//! removes the turns after it and drops the agent's provider session (ACP has no
//! rewind of its own). The card says all three things before anything runs, and
//! Cancel is the default: Enter and Escape both close it.

use std::rc::Rc;

use gpui::{AnyElement, App, FocusHandle, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::theme::Theme;

/// What the user chose in the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewindEvent {
    /// Restore the snapshot taken before this turn's prompt; the turn goes too.
    Before,
    /// Restore the snapshot taken after this turn; later turns go.
    After,
    Cancel,
}

pub(crate) type RewindHandler = Rc<dyn Fn(RewindEvent, &mut Window, &mut App)>;

pub(crate) fn rewind_card(
    theme: &Theme,
    turn: u64,
    focus: &FocusHandle,
    on: RewindHandler,
) -> AnyElement {
    let before = {
        let on = Rc::clone(&on);
        move |window: &mut Window, cx: &mut App| on(RewindEvent::Before, window, cx)
    };
    let after = {
        let on = Rc::clone(&on);
        move |window: &mut Window, cx: &mut App| on(RewindEvent::After, window, cx)
    };
    let cancel = {
        let on = Rc::clone(&on);
        move |window: &mut Window, cx: &mut App| on(RewindEvent::Cancel, window, cx)
    };
    let on_key = Rc::clone(&on);

    div()
        .w_full()
        .px(px(24.))
        .pb(px(8.))
        .track_focus(focus)
        .on_key_down(
            move |event, window, cx| match event.keystroke.key.as_str() {
                // The safe way out is the default, as the plan says.
                "enter" | "escape" => on_key(RewindEvent::Cancel, window, cx),
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
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(SharedString::from(format!("Rewind to turn {turn}?"))),
                )
                .child(
                    div()
                        .pt(px(5.))
                        .text_size(px(11.5))
                        .text_color(theme.text_muted)
                        .child(
                            "Files go back to the checkpoint. Later turns are removed. \
                             Untracked files created after it are deleted. The agent starts \
                             a fresh conversation on the next prompt.",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .mt(px(10.))
                        .child(button(theme, "Before this turn", true, before))
                        .child(button(theme, "After this turn", false, after))
                        .child(div().flex_1())
                        .child(button(theme, "Cancel", false, cancel)),
                ),
        )
        .into_any_element()
}

fn button(
    theme: &Theme,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("rewind-{label}")))
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
        .child(SharedString::from(label))
        .on_click(move |_, window, cx| on_click(window, cx))
}
