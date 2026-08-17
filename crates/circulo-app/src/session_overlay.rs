use circulo_core::Uuid;
use circulo_i18n::Catalog;
use gpui::{
    div, px, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Point, Pixels, StatefulInteractiveElement, Styled,
    Window,
};

use crate::context_menu::{menu_content, menu_group, menu_item, menu_separator};
use crate::icons::path as icon_path;
use crate::shell::AppShell;
use crate::theme::{ACCENT, ACCENT_SURFACE, BORDER, BG_SIDEBAR, TEXT, TEXT_MUTED};
use crate::ui::{field_label, TextInput};

#[derive(Clone, Debug)]
pub enum SessionOverlay {
    ContextMenu {
        session_id: Uuid,
        position: Point<Pixels>,
    },
    Rename {
        session_id: Uuid,
    },
}

pub fn session_overlay(
    overlay: &SessionOverlay,
    menu_focus: &FocusHandle,
    rename_input: &Entity<TextInput>,
    menu_selected: usize,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::AnyElement {
    match overlay.clone() {
        SessionOverlay::ContextMenu {
            session_id,
            position,
        } => context_menu_overlay(session_id, position, menu_selected, menu_focus, catalog, cx)
            .into_any_element(),
        SessionOverlay::Rename { session_id } => {
            rename_overlay(session_id, rename_input, catalog, cx).into_any_element()
        }
    }
}

fn context_menu_overlay(
    session_id: Uuid,
    position: Point<Pixels>,
    menu_selected: usize,
    menu_focus: &FocusHandle,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let focus = menu_focus.clone();
    let rename_label = catalog.get("session.rename").to_string();
    let delete_label = catalog.get("session.delete").to_string();
    div()
        .absolute()
        .size_full()
        .occlude()
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.close_session_overlay(cx);
        }))
        .child(
            div()
                .absolute()
                .left(position.x)
                .top(position.y)
                .child(
                    menu_content()
                        .track_focus(&focus)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_key_down({
                            let focus = focus.clone();
                            cx.listener(move |this, event, window, cx| {
                                if focus.is_focused(window) {
                                    handle_menu_key(this, event, window, cx);
                                }
                            })
                        })
                        .child(
                            menu_group().child(
                                menu_item(
                                    "session-menu-rename",
                                    rename_label,
                                    menu_selected == 0,
                                    false,
                                    Some(icon_path::PENCIL),
                                    cx.listener(move |this, _, window, cx| {
                                        this.start_rename_session(session_id, window, cx);
                                    }),
                                ),
                            ),
                        )
                        .child(menu_separator())
                        .child(
                            menu_group().child(
                                menu_item(
                                    "session-menu-delete",
                                    delete_label,
                                    menu_selected == 1,
                                    true,
                                    Some(icon_path::TRASH),
                                    cx.listener(move |this, _, window, cx| {
                                        this.delete_session(session_id, window, cx);
                                    }),
                                ),
                            ),
                        ),
                ),
        )
}

fn rename_overlay(
    _session_id: Uuid,
    rename_input: &Entity<TextInput>,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    div()
        .absolute()
        .size_full()
        .occlude()
        .bg(gpui::Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.45,
        })
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.close_session_overlay(cx);
        }))
        .child(
            div()
                .id("session-rename")
                .w(px(420.))
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .shadow_lg()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        this.close_session_overlay(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(TEXT)
                        .child(catalog.get("session.rename").to_string()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(field_label(catalog.get("session.rename_placeholder").to_string()))
                        .child(rename_input.clone()),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            div()
                                .id("session-rename-cancel")
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .text_sm()
                                .text_color(TEXT_MUTED)
                                .cursor_pointer()
                                .hover(|style| style.bg(ACCENT_SURFACE))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_session_overlay(cx);
                                }))
                                .child(catalog.get("action.cancel").to_string()),
                        )
                        .child(
                            div()
                                .id("session-rename-save")
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .text_sm()
                                .text_color(TEXT)
                                .bg(ACCENT)
                                .cursor_pointer()
                                .hover(|style| style.opacity(0.9))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.commit_rename_session(cx);
                                }))
                                .child(catalog.get("action.save").to_string()),
                        ),
                ),
        )
}

const MENU_ITEM_COUNT: usize = 2;

fn handle_menu_key(
    this: &mut AppShell,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    let key = event.keystroke.key.as_str();
    if key == "escape" {
        this.close_session_overlay(cx);
        cx.stop_propagation();
        return;
    }
    if key == "down" {
        if this.session_menu_selected + 1 < MENU_ITEM_COUNT {
            this.session_menu_selected += 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "up" {
        if this.session_menu_selected > 0 {
            this.session_menu_selected -= 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "enter" {
        this.execute_session_menu_selection(window, cx);
        cx.stop_propagation();
    }
}
