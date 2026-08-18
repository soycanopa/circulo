//! Composer container: input card + footer controls (project select, work mode).

use std::collections::HashMap;

use circulo_core::{Project, Session, Uuid};
use circulo_i18n::Catalog;
use gpui::{
    div, prelude::*, px, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};

use crate::client::session_project_label;
use crate::composer::events::{
    ComposerEvent, ComposerInputEvent, InteractionMode, PermissionMode, WorkMode,
};
use crate::composer::labels::{
    interaction_accent, interaction_icon, interaction_label_key, permission_description_key,
    permission_label_key, reasoning_display_label,
};
use circulo_core::{ComposerInteractionMode, ComposerPermissionMode};
use crate::composer::models::ComposerModel;
use crate::composer::helpers::{can_send, project_picker_locked};
use crate::composer::input::ComposerInput;
use crate::icons::{icon, path as icon_path};
use crate::context_menu::{
    menu_chip_model_selector_popover, menu_chip_popover, menu_item, menu_item_model_selector,
    menu_item_with_description, menu_item_with_edit_model_selector, menu_separator,
    MODEL_SELECTOR_MENU_WIDTH_PX,
};
use crate::ui::menu_chip::{
    menu_chip, menu_chip_dropdown_above, menu_chip_subpopover_right_offset,
    model_context_indicator, model_menu_chip,
};
use crate::theme::{
    ACCENT, BG_MAIN, BG_SIDEBAR, BORDER, CONTENT_MAX_WIDTH_PX, TEXT, TEXT_MUTED,
};

const SEND_BUTTON_PX: f32 = 26.0;
/// Vertical gap between the composer card and footer chips (project + work mode).
const FOOTER_ROW_GAP_PX: f32 = 4.;
/// Gap between footer chips (project folder + work mode).
const FOOTER_CHIP_GAP_PX: f32 = 0.;
const FOOTER_ROW_HEIGHT_PX: f32 = 24.;

pub struct Composer {
    input: Entity<ComposerInput>,
    draft_project: Option<Uuid>,
    project_menu_open: bool,
    work_mode_menu_open: bool,
    model_menu_open: bool,
    permission_menu_open: bool,
    interaction_menu_open: bool,
    model_reasoning_edit: Option<String>,
    selected_model: String,
    selected_model_variant: String,
    models: Vec<ComposerModel>,
    permission_mode: PermissionMode,
    interaction_mode: InteractionMode,
    session_drafts: HashMap<Uuid, String>,
    active_session: Option<Uuid>,
    generating: bool,
    projects: Vec<Project>,
    selected_session: Option<Session>,
    work_mode: WorkMode,
    catalog: Catalog,
    context_usage_fraction: f32,
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
            project_menu_open: false,
            work_mode_menu_open: false,
            model_menu_open: false,
            permission_menu_open: false,
            interaction_menu_open: false,
            model_reasoning_edit: None,
            selected_model: String::new(),
            selected_model_variant: String::new(),
            models: Vec::new(),
            permission_mode: PermissionMode::default(),
            interaction_mode: InteractionMode::default(),
            session_drafts: HashMap::new(),
            active_session: None,
            generating: false,
            projects: Vec::new(),
            selected_session: None,
            work_mode: WorkMode::Local,
            catalog: Catalog::english(),
            context_usage_fraction: 0.0,
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
        work_mode: WorkMode,
        models: Vec<ComposerModel>,
        selected_model: String,
        permission_mode: PermissionMode,
        interaction_mode: InteractionMode,
        catalog: Catalog,
        context_usage_fraction: f32,
        selected_model_variant: String,
        cx: &mut Context<Self>,
    ) {
        let session_changed = self
            .selected_session
            .as_ref()
            .map(|s| s.id)
            != session.as_ref().map(|s| s.id);
        self.projects = projects;
        self.selected_session = session.clone();
        self.work_mode = work_mode;
        self.models = models;
        self.selected_model = selected_model;
        self.permission_mode = permission_mode;
        self.interaction_mode = interaction_mode;
        self.catalog = catalog;
        self.context_usage_fraction = context_usage_fraction.clamp(0.0, 1.0);
        self.selected_model_variant = selected_model_variant;

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
            self.project_menu_open = false;
            self.work_mode_menu_open = false;
            self.model_menu_open = false;
            self.permission_menu_open = false;
            self.interaction_menu_open = false;
            self.model_reasoning_edit = None;
        }
        cx.notify();
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
        self.project_menu_open = false;
        if self.active_session.is_some() {
            if let Some(id) = project_id {
                cx.emit(ComposerEvent::ProjectPicked(id));
            } else {
                cx.emit(ComposerEvent::ProjectCleared);
            }
        }
        cx.notify();
    }

    fn open_project(&mut self, cx: &mut Context<Self>) {
        self.project_menu_open = false;
        cx.emit(ComposerEvent::OpenProject);
        cx.notify();
    }

    fn close_footer_menus(&mut self) {
        self.project_menu_open = false;
        self.work_mode_menu_open = false;
    }

    fn close_toolbar_menus(&mut self) {
        self.model_menu_open = false;
        self.permission_menu_open = false;
        self.interaction_menu_open = false;
        self.model_reasoning_edit = None;
    }

    fn close_all_menus(&mut self) {
        self.close_footer_menus();
        self.close_toolbar_menus();
    }

    fn toggle_project_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_session.is_some() && !self.locked() {
            self.close_toolbar_menus();
            self.work_mode_menu_open = false;
            self.project_menu_open = !self.project_menu_open;
            cx.notify();
        }
    }

    fn toggle_work_mode_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_session.is_some() && !self.locked() {
            self.close_toolbar_menus();
            self.project_menu_open = false;
            self.work_mode_menu_open = !self.work_mode_menu_open;
            cx.notify();
        }
    }

    fn toggle_model_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_session.is_some() && !self.generating {
            self.close_footer_menus();
            self.permission_menu_open = false;
            self.interaction_menu_open = false;
            let opening = !self.model_menu_open;
            self.model_menu_open = opening;
            if !opening {
                self.model_reasoning_edit = None;
            }
            cx.notify();
        }
    }

    fn toggle_model_reasoning_edit(&mut self, model_id: String, cx: &mut Context<Self>) {
        if self.model_reasoning_edit.as_ref() == Some(&model_id) {
            self.model_reasoning_edit = None;
        } else {
            self.model_reasoning_edit = Some(model_id);
        }
        cx.notify();
    }

    fn pick_model_variant(
        &mut self,
        model_id: String,
        variant: String,
        cx: &mut Context<Self>,
    ) {
        self.model_reasoning_edit = None;
        self.model_menu_open = false;
        self.selected_model = model_id;
        self.selected_model_variant = variant;
        cx.emit(ComposerEvent::ModelChanged(self.selected_model.clone()));
        cx.emit(ComposerEvent::ModelVariantChanged(self.selected_model_variant.clone()));
        cx.notify();
    }

    fn toggle_permission_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_session.is_some() && !self.generating {
            self.close_footer_menus();
            self.model_menu_open = false;
            self.interaction_menu_open = false;
            self.permission_menu_open = !self.permission_menu_open;
            cx.notify();
        }
    }

    fn toggle_interaction_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_session.is_some() && !self.generating {
            self.close_footer_menus();
            self.model_menu_open = false;
            self.permission_menu_open = false;
            self.interaction_menu_open = !self.interaction_menu_open;
            cx.notify();
        }
    }

    fn pick_model(&mut self, model_id: String, cx: &mut Context<Self>) {
        self.model_menu_open = false;
        self.model_reasoning_edit = None;
        self.selected_model = model_id;
        if let Some(model) = self.models.iter().find(|entry| entry.id == self.selected_model) {
            self.selected_model_variant = model
                .resolve_variant(Some(&self.selected_model_variant))
                .unwrap_or_default();
        } else {
            self.selected_model_variant.clear();
        }
        cx.emit(ComposerEvent::ModelChanged(self.selected_model.clone()));
        if !self.selected_model_variant.is_empty() {
            cx.emit(ComposerEvent::ModelVariantChanged(
                self.selected_model_variant.clone(),
            ));
        }
        cx.notify();
    }

    fn pick_permission_mode(&mut self, mode: PermissionMode, cx: &mut Context<Self>) {
        self.permission_menu_open = false;
        self.permission_mode = mode;
        cx.emit(ComposerEvent::PermissionModeChanged(mode));
        cx.notify();
    }

    fn pick_interaction_mode(&mut self, mode: InteractionMode, cx: &mut Context<Self>) {
        self.interaction_menu_open = false;
        self.interaction_mode = mode;
        cx.emit(ComposerEvent::InteractionModeChanged(mode));
        cx.notify();
    }

    fn pick_work_mode(&mut self, mode: WorkMode, cx: &mut Context<Self>) {
        self.work_mode_menu_open = false;
        self.work_mode = mode;
        cx.emit(ComposerEvent::WorkModeChanged(mode));
        cx.notify();
    }

    fn locked(&self) -> bool {
        project_picker_locked(self.selected_session.as_ref())
    }

    fn selected_model_entry(&self) -> Option<&ComposerModel> {
        self.models.iter().find(|model| model.id == self.selected_model)
    }
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_session = self.active_session.is_some();
        let locked = self.locked();
        let sendable = can_send(has_session, &self.content(cx), self.generating);
        let project_label = if has_session {
            session_project_label(
                self.draft_project,
                &self.projects,
                self.catalog.get("session.without_folder"),
            )
        } else {
            self.catalog.get("session.none").to_string()
        };

        let toolbar_enabled = has_session && !self.generating;

        let model_label = if self.models.is_empty() {
            self.catalog.get("composer.models.none").to_string()
        } else {
            self
                .selected_model_entry()
                .map(|model| model.name.clone())
                .unwrap_or_else(|| self.catalog.get("composer.model.default").to_string())
        };

        let reasoning_tag = self.selected_model_entry().and_then(|model| {
            if !model.supports_reasoning() {
                return None;
            }
            let variant = model
                .resolve_variant(Some(&self.selected_model_variant))
                .unwrap_or_default();
            if variant.is_empty() {
                None
            } else {
                Some(reasoning_display_label(&self.catalog, &variant))
            }
        });

        let model_context = model_context_indicator(self.context_usage_fraction);

        let model_chip = model_menu_chip(
            "composer-model",
            icon_path::OPENCODE,
            model_label,
            reasoning_tag,
            self.model_menu_open,
            !toolbar_enabled,
            cx.listener(|this, _, _, cx| this.toggle_model_menu(cx)),
        );
        let mut model_control = div().relative().child(model_chip);
        if toolbar_enabled && self.model_menu_open {
            let edit_label = self.catalog.get("composer.model.edit").to_string();
            let reasoning_title = self.catalog.get("composer.reasoning.title").to_string();
            let mut menu = menu_chip_model_selector_popover();
            for (index, model) in self.models.iter().enumerate() {
                let id = model.id.clone();
                let selected = self.selected_model == id;
                let edit_open = self.model_reasoning_edit.as_ref() == Some(&id);
                let mut row_wrap = div().relative();
                if model.supports_reasoning() {
                    let pick_id = id.clone();
                    let edit_toggle_id = id.clone();
                    row_wrap = row_wrap.child(menu_item_with_edit_model_selector(
                        ("composer-model", index),
                        ("composer-model-select", index),
                        ("composer-model-edit", index),
                        SharedString::from(format!("composer-model-row-{index}")),
                        model.name.clone(),
                        selected,
                        edit_open,
                        Some(icon_path::OPENCODE),
                        edit_label.clone(),
                        cx.listener(move |this, _, _, cx| this.pick_model(pick_id.clone(), cx)),
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_model_reasoning_edit(edit_toggle_id.clone(), cx);
                        }),
                    ));
                } else {
                    row_wrap = row_wrap.child(menu_item_model_selector(
                        ("composer-model", index),
                        model.name.clone(),
                        selected,
                        false,
                        Some(icon_path::OPENCODE),
                        cx.listener(move |this, _, _, cx| this.pick_model(id.clone(), cx)),
                    ));
                }
                if edit_open {
                    let mut sub = menu_chip_model_selector_popover().min_w(px(176.));
                    sub = sub.child(
                        div()
                            .px(px(10.))
                            .pt(px(4.))
                            .pb(px(6.))
                            .text_xs()
                            .text_color(TEXT_MUTED)
                            .child(reasoning_title.clone()),
                    );
                    for (variant_index, variant) in model.reasoning_variants.iter().enumerate() {
                        let variant_id = variant.clone();
                        let pick_model_id = model.id.clone();
                        let variant_selected = self.selected_model == model.id
                            && self.selected_model_variant == variant_id;
                        let variant_label =
                            reasoning_display_label(&self.catalog, &variant_id);
                        sub = sub.child(
                            menu_item_model_selector(
                                SharedString::from(format!(
                                    "composer-reasoning-{index}-{variant_index}"
                                )),
                                variant_label,
                                variant_selected,
                                false,
                                None,
                                cx.listener(move |this, _, _, cx| {
                                    this.pick_model_variant(
                                        pick_model_id.clone(),
                                        variant_id.clone(),
                                        cx,
                                    );
                                }),
                            ),
                        );
                    }
                    row_wrap = row_wrap.child(
                        menu_chip_subpopover_right_offset(
                            sub,
                            MODEL_SELECTOR_MENU_WIDTH_PX + 6.,
                        ),
                    );
                }
                menu = menu.child(row_wrap);
            }
            model_control = model_control.child(menu_chip_dropdown_above(menu));
        }

        let permission_chip = menu_chip(
            "composer-permission",
            icon_path::SHIELD,
            self.catalog.get(permission_label_key(self.permission_mode)).to_string(),
            self.permission_menu_open,
            !toolbar_enabled,
            false,
            false,
            cx.listener(|this, _, _, cx| this.toggle_permission_menu(cx)),
        );
        let mut permission_control = div().relative().child(permission_chip);
        if toolbar_enabled && self.permission_menu_open {
            let mut menu = menu_chip_popover().min_w(px(288.));
            for (index, mode) in ComposerPermissionMode::ALL.into_iter().enumerate() {
                let picked = mode;
                let selected = self.permission_mode == mode;
                menu = menu.child(
                    menu_item_with_description(
                        ("composer-permission", index),
                        self.catalog.get(permission_label_key(mode)).to_string(),
                        self.catalog.get(permission_description_key(mode)).to_string(),
                        selected,
                        Some(icon_path::SHIELD),
                        cx.listener(move |this, _, _, cx| this.pick_permission_mode(picked, cx)),
                    ),
                );
            }
            permission_control = permission_control.child(menu_chip_dropdown_above(menu));
        }

        let interaction_icon_path = interaction_icon(self.interaction_mode);
        let interaction_accent_flag = interaction_accent(self.interaction_mode);
        let interaction_chip = menu_chip(
            "composer-mode",
            interaction_icon_path,
            self.catalog
                .get(interaction_label_key(self.interaction_mode))
                .to_string(),
            self.interaction_menu_open,
            !toolbar_enabled,
            interaction_accent_flag,
            false,
            cx.listener(|this, _, _, cx| this.toggle_interaction_menu(cx)),
        );
        let mut interaction_control = div().relative().child(interaction_chip);
        if toolbar_enabled && self.interaction_menu_open {
            let mut menu = menu_chip_popover().min_w(px(160.));
            for (index, mode) in ComposerInteractionMode::ALL.into_iter().enumerate() {
                let picked = mode;
                let selected = self.interaction_mode == mode;
                menu = menu.child(
                    menu_item(
                        ("composer-mode", index),
                        self.catalog.get(interaction_label_key(mode)).to_string(),
                        selected,
                        false,
                        Some(interaction_icon(mode)),
                        cx.listener(move |this, _, _, cx| this.pick_interaction_mode(picked, cx)),
                    ),
                );
            }
            interaction_control = interaction_control.child(menu_chip_dropdown_above(menu));
        }

        let send_control = if self.generating {
            div()
                .id("generating")
                .flex_none()
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
                .flex_none()
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
        };

        let input_row = div()
            .px(px(10.))
            .pt(px(2.))
            .pb(px(4.))
            .child(self.input.clone());

        let toolbar_row = div()
            .mt(px(8.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(11.5))
            .line_height(px(14.))
            .child(model_control)
            .child(permission_control)
            .child(interaction_control)
            .child(div().flex_1())
            .child(send_control);

        let can_configure = has_session && !locked;

        let project_chip = menu_chip(
            "workspace-project",
            icon_path::FOLDER,
            project_label,
            self.project_menu_open,
            !can_configure,
            true,
            false,
            cx.listener(|this, _, _, cx| this.toggle_project_menu(cx)),
        );

        let mut project_control = div().relative().child(project_chip);
        if can_configure && self.project_menu_open {
            let mut menu = menu_chip_popover().min_w(px(200.));
            for (index, project) in self.projects.iter().enumerate() {
                let id = project.id;
                let selected = self.draft_project == Some(id);
                menu = menu.child(
                    menu_item(
                        ("select-proj", index),
                        project.name.clone(),
                        selected,
                        false,
                        Some(icon_path::FOLDER),
                        cx.listener(move |this, _, _, cx| this.pick_project(Some(id), cx)),
                    ),
                );
            }
            menu = menu.child(
                menu_item(
                    "select-none",
                    self.catalog.get("session.without_folder").to_string(),
                    self.draft_project.is_none(),
                    false,
                    None,
                    cx.listener(|this, _, _, cx| this.pick_project(None, cx)),
                ),
            );
            menu = menu.child(menu_separator());
            menu = menu.child(
                menu_item(
                    "select-open-project",
                    self.catalog.get("composer.open_project").to_string(),
                    false,
                    false,
                    Some(icon_path::FOLDER_PLUS),
                    cx.listener(|this, _, _, cx| this.open_project(cx)),
                ),
            );
            project_control = project_control.child(menu_chip_dropdown_above(menu));
        }

        let (work_mode_label, work_mode_icon) = match self.work_mode {
            WorkMode::Local => (
                self.catalog.get("composer.work_mode.local").to_string(),
                icon_path::LAPTOP,
            ),
            WorkMode::Remote => (
                self.catalog.get("composer.work_mode.remote").to_string(),
                icon_path::FORK,
            ),
        };

        let work_mode_chip = menu_chip(
            "workspace-worktree",
            work_mode_icon,
            work_mode_label,
            self.work_mode_menu_open,
            !can_configure,
            true,
            false,
            cx.listener(|this, _, _, cx| this.toggle_work_mode_menu(cx)),
        );

        let mut work_mode_control = div().relative().child(work_mode_chip);
        if can_configure && self.work_mode_menu_open {
            let local_selected = self.work_mode == WorkMode::Local;
            let remote_selected = self.work_mode == WorkMode::Remote;
            let menu = menu_chip_popover()
                .min_w(px(180.))
                .child(
                    menu_item(
                        "work-mode-local",
                        self.catalog.get("composer.work_mode.local").to_string(),
                        local_selected,
                        false,
                        Some(icon_path::LAPTOP),
                        cx.listener(|this, _, _, cx| this.pick_work_mode(WorkMode::Local, cx)),
                    ),
                )
                .child(
                    menu_item(
                        "work-mode-remote",
                        self.catalog.get("composer.work_mode.remote").to_string(),
                        remote_selected,
                        false,
                        Some(icon_path::FORK),
                        cx.listener(|this, _, _, cx| this.pick_work_mode(WorkMode::Remote, cx)),
                    ),
                );
            work_mode_control = work_mode_control.child(menu_chip_dropdown_above(menu));
        }

        div()
            .w_full()
            .max_w(px(CONTENT_MAX_WIDTH_PX))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(FOOTER_ROW_GAP_PX))
            .child(
                div()
                    .w_full()
                    .relative()
                    .rounded(px(13.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(BG_SIDEBAR)
                    .py(px(10.))
                    .child(input_row)
                    .child(toolbar_row),
            )
            .child(
                div()
                    .h(px(FOOTER_ROW_HEIGHT_PX))
                    .pl(px(10.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(FOOTER_CHIP_GAP_PX))
                    .child(project_control)
                    .child(work_mode_control)
                    .child(div().flex_1())
                    .child(model_context),
            )
    }
}
