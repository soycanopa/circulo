//! Composer container: floating card + toolbar (Waku-style layout).

use std::collections::HashMap;

use circulo_core::{Project, Session, Uuid};
use circulo_i18n::Catalog;
use gpui::{
    div, prelude::*, px, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};

use crate::client::session_project_label;
use crate::composer::events::{ComposerEvent, ComposerInputEvent};
use crate::composer::helpers::{can_send, project_picker_locked};
use crate::composer::input::ComposerInput;
use crate::icons::{icon, path as icon_path};
use crate::theme::{ACCENT, BG_MAIN, BG_SIDEBAR, BORDER, CONTENT_MAX_WIDTH_PX, TEXT, TEXT_MUTED};

const SEND_BUTTON_PX: f32 = 26.0;

pub struct Composer {
    input: Entity<ComposerInput>,
    draft_project: Option<Uuid>,
    picker_open: bool,
    session_drafts: HashMap<Uuid, String>,
    active_session: Option<Uuid>,
    generating: bool,
    projects: Vec<Project>,
    selected_session: Option<Session>,
    catalog: Catalog,
    _input_subscription: Subscription,
}

impl Composer {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| ComposerInput::new(window, cx));
        let input_subscription = cx.subscribe(&input, move |this, _, event, cx| {
            this.on_input_event(event, cx);
        });
        Self {
            input,
            draft_project: None,
            picker_open: false,
            session_drafts: HashMap::new(),
            active_session: None,
            generating: false,
            projects: Vec::new(),
            selected_session: None,
            catalog: Catalog::english(),
            _input_subscription: input_subscription,
        }
    }

    pub fn input(&self) -> &Entity<ComposerInput> {
        &self.input
    }

    pub fn draft_project(&self) -> Option<Uuid> {
        self.draft_project
    }

    pub fn content(&self, cx: &gpui::App) -> String {
        self.input.read(cx).content().to_string()
    }

    pub fn set_render_context(
        &mut self,
        projects: Vec<Project>,
        session: Option<Session>,
        catalog: Catalog,
        cx: &mut Context<Self>,
    ) {
        let session_changed = self
            .selected_session
            .as_ref()
            .map(|s| s.id)
            != session.as_ref().map(|s| s.id);
        self.projects = projects;
        self.selected_session = session.clone();
        self.catalog = catalog;

        if session_changed {
            if let Some(old) = self.active_session {
                self.session_drafts.insert(old, self.content(cx));
            }
            let new_id = session.as_ref().map(|s| s.id);
            let restored = new_id
                .and_then(|id| self.session_drafts.get(&id).cloned())
                .unwrap_or_default();
            let placeholder = if new_id.is_some() {
                self.catalog.get("composer.placeholder").to_string()
            } else {
                self.catalog.get("composer.no_session").to_string()
            };
            self.input.update(cx, |input, cx| {
                input.set_content(restored, cx);
                input.set_enabled(new_id.is_some(), cx);
                input.set_placeholder(placeholder, cx);
            });
            self.active_session = new_id;
            self.draft_project = session.as_ref().and_then(|s| s.project_id);
            self.picker_open = false;
            cx.notify();
        }
    }

    pub fn focus_after_session_select(&self, window: &mut Window, cx: &gpui::App) {
        if self.active_session.is_some() {
            self.input.read(cx).focus(window);
        }
    }

    pub fn set_generating(&mut self, generating: bool, cx: &mut Context<Self>) {
        if self.generating == generating {
            return;
        }
        self.generating = generating;
        self.input.update(cx, |input, cx| {
            input.set_read_only(generating, cx);
        });
        cx.notify();
    }

    pub fn restore_content(&mut self, content: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.set_content(content, cx);
        });
        if let Some(id) = self.active_session {
            self.session_drafts.insert(id, self.content(cx));
        }
    }

    pub fn clear_after_send(&mut self, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.clear(cx));
        if let Some(id) = self.active_session {
            self.session_drafts.remove(&id);
        }
    }

    fn on_input_event(&mut self, event: &ComposerInputEvent, cx: &mut Context<Self>) {
        match event {
            ComposerInputEvent::Edited => {
                if let Some(id) = self.active_session {
                    self.session_drafts.insert(id, self.content(cx));
                }
                cx.notify();
            }
            ComposerInputEvent::Submit(_) => self.submit(cx),
            ComposerInputEvent::Focus => {}
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let text = self.content(cx);
        if can_send(self.active_session.is_some(), &text, self.generating) {
            cx.emit(ComposerEvent::Submit(text));
        }
    }

    fn pick_project(&mut self, project_id: Option<Uuid>, cx: &mut Context<Self>) {
        self.draft_project = project_id;
        self.picker_open = false;
        if self.active_session.is_some() {
            if let Some(id) = project_id {
                cx.emit(ComposerEvent::ProjectPicked(id));
            } else {
                cx.emit(ComposerEvent::ProjectCleared);
            }
        }
        cx.notify();
    }

    fn locked(&self) -> bool {
        project_picker_locked(self.selected_session.as_ref())
    }
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_session = self.active_session.is_some();
        let locked = self.locked();
        let sendable = can_send(has_session, &self.content(cx), self.generating);
        let project_label = session_project_label(
            self.draft_project,
            &self.projects,
            self.catalog.get("session.without_folder"),
        );

        let mut card = div()
            .w_full()
            .max_w(px(CONTENT_MAX_WIDTH_PX))
            .mx_auto()
            .rounded(px(13.))
            .border_1()
            .border_color(BORDER)
            .bg(BG_SIDEBAR)
            .py(px(10.))
            .relative();

        if has_session && self.picker_open && !locked {
            let mut menu = div()
                .absolute()
                .bottom_full()
                .left(px(10.))
                .right(px(10.))
                .mb(px(6.))
                .flex()
                .flex_col()
                .rounded_md()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .shadow_md();
            let none_selected = self.draft_project.is_none();
            menu = menu.child(picker_item(
                "picker-none",
                self.catalog.get("session.without_folder"),
                none_selected,
                cx.listener(|this, _, _, cx| this.pick_project(None, cx)),
            ));
            for (index, project) in self.projects.iter().enumerate() {
                let id = project.id;
                let selected = self.draft_project == Some(id);
                menu = menu.child(picker_item(
                    ("picker-proj", index),
                    &project.name,
                    selected,
                    cx.listener(move |this, _, _, cx| this.pick_project(Some(id), cx)),
                ));
            }
            card = card.child(menu);
        }

        card.child(div().px(px(10.)).pt(px(2.)).child(self.input.clone()))
            .child(
                div()
                    .mt(px(8.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_xs()
                    .line_height(px(14.))
                    .child(
                        div()
                            .id("project-picker")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_color(TEXT_MUTED)
                            .when(has_session && !locked, |el| {
                                el.cursor_pointer()
                                    .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.active_session.is_some() && !this.locked() {
                                    this.picker_open = !this.picker_open;
                                }
                                cx.notify();
                            }))
                            .child(if has_session {
                                project_label
                            } else {
                                self.catalog.get("session.none").to_string()
                            }),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .text_color(TEXT_MUTED)
                            .child(self.catalog.get("composer.agent_opencode").to_string()),
                    )
                    .child(div().flex_1())
                    .child(if self.generating {
                        div()
                            .id("generating")
                            .w(px(SEND_BUTTON_PX))
                            .h(px(SEND_BUTTON_PX))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(BG_MAIN)
                            .text_color(TEXT_MUTED)
                            .child(icon(icon_path::ELLIPSIS, px(14.), TEXT_MUTED))
                    } else {
                        div()
                            .id("send")
                            .w(px(SEND_BUTTON_PX))
                            .h(px(SEND_BUTTON_PX))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .when(sendable, |el| {
                                el.bg(ACCENT)
                                    .text_color(TEXT)
                                    .cursor_pointer()
                                    .hover(|style| style.opacity(0.85))
                            })
                            .when(!sendable, |el| {
                                el.bg(BG_MAIN).text_color(TEXT_MUTED).cursor_default()
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.submit(cx)))
                            .child(icon(
                                icon_path::ARROW_UP,
                                px(14.),
                                if sendable { TEXT } else { TEXT_MUTED },
                            ))
                    }),
            )
    }
}

fn picker_item(
    id: impl Into<gpui::ElementId>,
    text: &str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .text_sm()
        .cursor_pointer()
        .when(selected, |el| el.bg(BG_MAIN))
        .hover(|style| style.bg(BG_MAIN))
        .on_click(on_click)
        .child(text.to_string())
}
