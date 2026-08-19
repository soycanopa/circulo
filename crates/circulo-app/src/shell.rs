use std::collections::HashSet;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

use circulo_core::{
    ComposerInteractionMode, ComposerPermissionMode, Message, MessagePart, MessageRole,
    ModelCatalogEntry, Project, Session, Uuid,
};
use circulo_i18n::Catalog;
use circulo_protocol::{PreferencesBody, ProtocolEvent};
use gpui::{
    deferred, div, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, AppContext,
    Context, CursorStyle, DragMoveEvent, Entity, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, Subscription, Window,
};

use crate::command_palette::{palette_catalog, PaletteItemKind, OpenPalette};
use crate::project_picker::{project_picker_catalog, ProjectPickerItem, ProjectPickerItemKind};
use time::OffsetDateTime;

use crate::client::{
    ensure_daemon, partition_sessions_by_day, session_project_label, DaemonClient,
};
use crate::composer::{
    can_send, context_usage_fraction, placeholder_models, project_picker_locked, Composer,
    ComposerEvent, ComposerModel, InteractionMode, PermissionMode, WorkMode,
};
use crate::home::home_panel;
use crate::icons::{icon, path as icon_path};
use crate::parts::{render_text, unsupported};
use crate::ui::{
    activity_cluster, assistant_is_thinking, content_rail, message_segments, permission_banner,
    question_card, thinking_label, PendingPermission, PendingQuestion, MessageSegment,
};
use crate::session_overlay::{session_overlay, SessionOverlay};
use crate::settings::{
    active_projects_panel, archived_projects_panel, general_settings_panel, models_settings_panel,
    providers_panel,
    SettingsSection,
};
use crate::stream::{
    apply_protocol_event, resubscribe_delay, should_apply_refresh_transcript,
    should_unlock_composer, stream_attempts_after_event,
};
use crate::ui::{TextInput, TextInputEvent};
use crate::theme::{
    sidebar_width_px, ACCENT, ACCENT_SURFACE, BG_APP, BG_HOVER, BG_MAIN, BG_SIDEBAR, BORDER,
    BORDER_SUBTLE,
    COMPOSER_BOTTOM_PADDING_PX, MESSAGE_AVATAR_PX, APP_BAR_HEIGHT_PX,
    MAIN_HEADER_TITLE_INSET_PX, MAIN_HEADER_TITLE_LEFT_PX, MAIN_HEADER_TITLE_TEXT_PX,
    SIDEBAR_EXPANDED_PX, SIDEBAR_MAX_PX, SIDEBAR_MIN_PX, SIDEBAR_RESIZE_HANDLE_CENTER,
    SIDEBAR_RESIZE_HANDLE_CENTER_ACTIVE, SIDEBAR_RESIZE_HANDLE_HIT_PX,
    SIDEBAR_RESIZE_HANDLE_VISUAL_PX, SIDEBAR_TOGGLE_LEFT_PX, SIDEBAR_TOGGLE_SIZE_PX,
    SIDEBAR_TOGGLE_TOP_PX,
    TEXT, TEXT_MUTED, TEXT_TERTIARY,
};
use crate::command_palette::PaletteItem;
use crate::timefmt::{format_relative, local_offset_or_utc};

/// How often the drain loop applies buffered stream events; doubles as render
/// batching for incoming deltas.
const DRAIN_INTERVAL: Duration = Duration::from_millis(32);
/// Distance from the bottom (px) within which the transcript keeps following
/// new content.
const ANCHOR_THRESHOLD: f32 = 80.0;
/// Gap between the jump-to-latest control and the composer.
const JUMP_TO_LATEST_ABOVE_COMPOSER_PX: f32 = 12.0;
/// Session title (`--text-base` in Paper).
const SESSION_TITLE_TEXT_PX: f32 = 13.0;
const SESSION_TITLE_LINE_HEIGHT_PX: f32 = 18.0;
/// Project label on the meta row (`--text-sm`).
const SESSION_META_PROJECT_TEXT_PX: f32 = 12.0;
/// Relative time on the meta row (`--text-xs`).
const SESSION_META_TIME_TEXT_PX: f32 = 11.0;
const SESSION_META_TIME_LINE_HEIGHT_PX: f32 = 14.0;
const SESSION_META_ICON_PX: f32 = 12.0;
const SESSION_ACTIVITY_SPINNER: gpui::Rgba = gpui::Rgba {
    r: 0.847,
    g: 0.847,
    b: 0.847,
    a: 1.0,
};
/// Horizontal padding on the sidebar session scroll area (`px_3`).
const SIDEBAR_SESSION_SCROLL_PADDING_X_PX: f32 = 24.0;
/// Horizontal padding on each session row (`px_2`).
const SESSION_ROW_PADDING_X_PX: f32 = 16.0;
/// Approximate character width for session titles at 13px.
const SESSION_TITLE_CHAR_WIDTH_PX: f32 = 7.25;
/// Show `...` when the title exceeds this many characters.
const SESSION_TITLE_ELLIPSIS_AT_CHARS: usize = 40;
const SESSION_TITLE_ELLIPSIS: &str = "...";
const PALETTE_BACKDROP: gpui::Rgba = gpui::Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.45,
};

enum EitherProjectPick {
    Attach(Project),
    NewSession(Project, Session),
}

pub struct AppShell {
    pub sidebar_collapsed: bool,
    sidebar_width_expanded: f32,
    sidebar_resize_origin: Option<(gpui::Pixels, f32)>,
    sidebar_resize_hovered: bool,
    today_expanded: bool,
    earlier_expanded: bool,
    catalog: Catalog,
    client: DaemonClient,
    sessions: Vec<Session>,
    projects: Vec<Project>,
    messages: Vec<Message>,
    selected: Option<Uuid>,
    palette_open: bool,
    palette_query: String,
    palette_selected: usize,
    project_picker_open: bool,
    project_picker_query: String,
    project_picker_selected: usize,
    project_picker_pending_focus: bool,
    folder_picker_open: bool,
    composer_pending_focus: bool,
    composer: Entity<Composer>,
    generating: bool,
    work_mode: WorkMode,
    selected_model: String,
    selected_model_variant: String,
    permission_mode: PermissionMode,
    interaction_mode: InteractionMode,
    composer_models: Vec<ModelCatalogEntry>,
    enabled_model_ids: Vec<String>,
    preferences: circulo_core::UserPreferences,
    pending_provider_toggle: Option<(circulo_core::AgentType, bool)>,
    settings_open: bool,
    pub(crate) settings_section: SettingsSection,
    settings_health: Option<circulo_protocol::HealthResponse>,
    settings_health_error: Option<String>,
    archived_projects: Vec<Project>,
    pending_delete_project: Option<Uuid>,
    pending_rename_project: Option<Uuid>,
    available_agents: Vec<circulo_protocol::AgentDescriptor>,
    pub(crate) settings_models_query: String,
    pub(crate) settings_models_expanded: bool,
    pub(crate) settings_models_focus: gpui::FocusHandle,
    pub expanded_tools: HashSet<String>,
    pub expanded_reasoning: HashSet<String>,
    pub expanded_activity_clusters: HashSet<String>,
    pub collapsed_live_activity_clusters: HashSet<String>,
    error: Option<String>,
    loaded: bool,
    scroll: ScrollHandle,
    sidebar_scroll: ScrollHandle,
    jump_to_latest_visible: bool,
    palette_focus: gpui::FocusHandle,
    project_picker_focus: gpui::FocusHandle,
    session_overlay: Option<SessionOverlay>,
    session_menu_focus: gpui::FocusHandle,
    pub(crate) session_menu_selected: usize,
    rename_input: Entity<TextInput>,
    project_rename_input: Entity<TextInput>,
    stream_gen: u64,
    stream_session: Option<Uuid>,
    stream_attempts: u32,
    saw_stream_event: bool,
    pending_permission: Option<PendingPermission>,
    pending_question: Option<PendingQuestion>,
    question_answer_input: Entity<TextInput>,
    _composer_subscription: Subscription,
    _rename_input_subscription: Subscription,
    _project_rename_input_subscription: Subscription,
    _question_input_subscription: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| Composer::new(window, cx));
        let rename_input = cx.new(|cx| TextInput::new(window, cx));
        let question_answer_input = cx.new(|cx| TextInput::new(window, cx));
        let project_rename_input = cx.new(|cx| TextInput::new(window, cx));
        let composer_subscription = {
            let composer_entity = composer.clone();
            cx.subscribe(&composer_entity, |shell, _, event, cx| {
                shell.on_composer_event(event, cx);
            })
        };
        let rename_input_subscription = cx.subscribe(&rename_input, |shell, _, event, cx| {
            if matches!(event, TextInputEvent::Submit(_)) {
                shell.commit_rename_session(cx);
            }
        });
        let project_rename_input_subscription =
            cx.subscribe(&project_rename_input, |shell, _, event, cx| {
                if matches!(event, TextInputEvent::Submit(_)) {
                    if let Some(project_id) = shell.pending_rename_project {
                        let name = shell.project_rename_input.read(cx).content().to_string();
                        shell.commit_rename_project(project_id, name, cx);
                    }
                }
            });
        let question_input_subscription =
            cx.subscribe(&question_answer_input, |shell, input, event, cx| {
                if matches!(event, TextInputEvent::Submit(_)) {
                    let answer = input.read(cx).content().to_string();
                    shell.sync_question_custom_answer(&answer, cx);
                    shell.advance_question(cx);
                }
            });
        let mut shell = Self {
            sidebar_collapsed: false,
            sidebar_width_expanded: SIDEBAR_EXPANDED_PX,
            sidebar_resize_origin: None,
            sidebar_resize_hovered: false,
            today_expanded: true,
            earlier_expanded: true,
            catalog: Catalog::english(),
            client: DaemonClient::default(),
            sessions: Vec::new(),
            projects: Vec::new(),
            messages: Vec::new(),
            selected: None,
            palette_open: false,
            palette_query: String::new(),
            palette_selected: 0,
            project_picker_open: false,
            project_picker_query: String::new(),
            project_picker_selected: 0,
            project_picker_pending_focus: false,
            folder_picker_open: false,
            composer_pending_focus: false,
            composer,
            generating: false,
            work_mode: WorkMode::Local,
            selected_model: String::new(),
            selected_model_variant: String::new(),
            permission_mode: PermissionMode::default(),
            interaction_mode: InteractionMode::default(),
            composer_models: Vec::new(),
            enabled_model_ids: Vec::new(),
            preferences: circulo_core::UserPreferences::default(),
            pending_provider_toggle: None,
            settings_open: false,
            settings_section: SettingsSection::General,
            settings_health: None,
            settings_health_error: None,
            archived_projects: Vec::new(),
            pending_delete_project: None,
            pending_rename_project: None,
            available_agents: Vec::new(),
            settings_models_query: String::new(),
            settings_models_expanded: false,
            settings_models_focus: cx.focus_handle(),
            expanded_tools: HashSet::new(),
            expanded_reasoning: HashSet::new(),
            expanded_activity_clusters: HashSet::new(),
            collapsed_live_activity_clusters: HashSet::new(),
            error: None,
            loaded: false,
            scroll: ScrollHandle::new(),
            sidebar_scroll: ScrollHandle::new(),
            jump_to_latest_visible: false,
            palette_focus: cx.focus_handle(),
            project_picker_focus: cx.focus_handle(),
            session_overlay: None,
            session_menu_focus: cx.focus_handle(),
            session_menu_selected: 0,
            rename_input,
            project_rename_input,
            stream_gen: 0,
            stream_attempts: 0,
            stream_session: None,
            saw_stream_event: false,
            pending_permission: None,
            pending_question: None,
            question_answer_input,
            _composer_subscription: composer_subscription,
            _rename_input_subscription: rename_input_subscription,
            _project_rename_input_subscription: project_rename_input_subscription,
            _question_input_subscription: question_input_subscription,
        };
        shell.schedule_refresh(cx);
        shell.sync_composer(cx);
        shell
    }

    fn end_sidebar_resize(&mut self) {
        self.sidebar_resize_origin = None;
        self.sidebar_resize_hovered = false;
    }

    fn sync_transcript_scroll_state(&mut self, cx: &mut Context<Self>) {
        let visible = should_show_jump_to_latest(
            self.selected.is_some(),
            self.messages.len(),
            f32::from(self.scroll.max_offset().height),
            f32::from(self.scroll.offset().y),
            ANCHOR_THRESHOLD,
        );
        if visible != self.jump_to_latest_visible {
            self.jump_to_latest_visible = visible;
            cx.notify();
        }
    }

    fn scroll_transcript_to_bottom(&mut self, cx: &mut Context<Self>) {
        self.scroll.scroll_to_bottom();
        if self.jump_to_latest_visible {
            self.jump_to_latest_visible = false;
            cx.notify();
        }
    }

    fn apply_session_composer_state(&mut self) {
        let available_ids: Vec<String> = self
            .composer_models
            .iter()
            .filter(|model| self.enabled_model_ids.contains(&model.id))
            .map(|model| model.id.clone())
            .collect();
        let default_model_id = available_ids
            .first()
            .cloned()
            .unwrap_or_default();
        if let Some(session) = self.selected_session().cloned() {
            let model_id = session
                .composer_model_id
                .clone()
                .filter(|id| available_ids.contains(id))
                .unwrap_or(default_model_id);
            let permission_mode = session
                .composer_permission_mode
                .unwrap_or(ComposerPermissionMode::Supervised);
            let interaction_mode = session
                .composer_interaction_mode
                .unwrap_or(ComposerInteractionMode::Build);
            let session_variant = session.composer_model_variant.clone();
            self.selected_model = model_id.clone();
            self.selected_model_variant =
                self.resolve_model_variant(&model_id, session_variant);
            self.permission_mode = permission_mode;
            self.interaction_mode = interaction_mode;
        } else {
            self.selected_model = default_model_id;
            self.selected_model_variant =
                self.resolve_model_variant(&self.selected_model, None);
            self.permission_mode = ComposerPermissionMode::default();
            self.interaction_mode = ComposerInteractionMode::default();
        }
    }

    /// Keep the in-memory session row aligned with toolbar edits so re-renders
    /// do not snap the composer back to stale SQLite values before PATCH returns.
    fn sync_local_session_composer_fields(&mut self) {
        let Some(session_id) = self.selected else {
            return;
        };
        if let Some(index) = self.sessions.iter().position(|entry| entry.id == session_id) {
            let session = &mut self.sessions[index];
            session.composer_model_id = Some(self.selected_model.clone());
            session.composer_model_variant = if self.selected_model_variant.is_empty() {
                None
            } else {
                Some(self.selected_model_variant.clone())
            };
            session.composer_permission_mode = Some(self.permission_mode);
            session.composer_interaction_mode = Some(self.interaction_mode);
        }
    }

    fn resolve_model_variant(
        &self,
        model_id: &str,
        session_variant: Option<String>,
    ) -> String {
        let variants = self
            .composer_models
            .iter()
            .find(|model| model.id == model_id)
            .map(|model| model.reasoning_variants.clone())
            .unwrap_or_default();
        if variants.is_empty() {
            return String::new();
        }
        session_variant
            .filter(|variant| variants.iter().any(|entry| entry == variant))
            .or_else(|| variants.first().cloned())
            .unwrap_or_default()
    }

    fn filtered_composer_models(&self) -> Vec<ComposerModel> {
        let catalog_models: Vec<ComposerModel> = if self.composer_models.is_empty() {
            placeholder_models(&self.catalog)
        } else {
            self.composer_models
                .iter()
                .map(ComposerModel::from)
                .collect()
        };
        if self.enabled_model_ids.is_empty() {
            return Vec::new();
        }
        catalog_models
            .into_iter()
            .filter(|model| self.enabled_model_ids.contains(&model.id))
            .collect()
    }

    fn bootstrap_enabled_models_if_needed(&mut self, cx: &mut Context<Self>) {
        if !self.enabled_model_ids.is_empty() || self.composer_models.is_empty() {
            return;
        }
        self.enabled_model_ids = self
            .composer_models
            .iter()
            .map(|model| model.id.clone())
            .collect();
        self.save_preferences(cx);
    }

    pub(crate) fn handle_settings_models_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if key == "escape" {
            self.settings_models_query.clear();
            cx.stop_propagation();
            return;
        }
        if key == "backspace" {
            self.settings_models_query.pop();
        } else if let Some(ch) = typed_char(event) {
            self.settings_models_query.push_str(&ch);
        } else {
            return;
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(crate) fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.open_settings_section(SettingsSection::General, cx);
    }

    pub(crate) fn open_settings_section(
        &mut self,
        section: SettingsSection,
        cx: &mut Context<Self>,
    ) {
        self.settings_open = true;
        self.settings_section = section;
        self.settings_models_query.clear();
        self.settings_models_expanded = false;
        self.pending_delete_project = None;
        self.close_palette(cx);
        self.close_project_picker(cx);
        self.close_composer_popovers(cx);
        self.refresh_settings_health(cx);
        self.reload_archived_projects(cx);
        cx.notify();
    }

    pub(crate) fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = false;
        self.settings_models_expanded = false;
        self.settings_models_query.clear();
        self.pending_delete_project = None;
        cx.notify();
    }

    /// Stage a provider toggle. Disables always require confirmation;
    /// enables run immediately.
    pub(crate) fn request_provider_toggle(
        &mut self,
        agent: circulo_core::AgentType,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if enabled {
            self.confirm_provider_toggle(agent, true, cx);
        } else {
            self.pending_provider_toggle = Some((agent, false));
            cx.notify();
        }
    }

    pub(crate) fn cancel_provider_toggle(&mut self) {
        self.pending_provider_toggle = None;
    }

    pub(crate) fn confirm_provider_toggle(
        &mut self,
        agent: circulo_core::AgentType,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.set_provider_enabled(agent, enabled) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.preferences.disabled_agents.remove(&agent);
                    if !enabled {
                        this.preferences.disabled_agents.insert(agent);
                    }
                    this.pending_provider_toggle = None;
                    this.refresh();
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn refresh_settings_health(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { client.health() }).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(health) => {
                        this.settings_health = Some(health);
                        this.settings_health_error = None;
                    }
                    Err(err) => {
                        this.settings_health = None;
                        this.settings_health_error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reload_archived_projects(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.list_archived_projects() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(projects) = result {
                    this.archived_projects = projects;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn archive_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.archive_project(project_id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.pending_delete_project = None;
                    this.refresh();
                    this.reconcile_selection_after_refresh(cx);
                    this.reload_archived_projects(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn restore_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.restore_project(project_id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.refresh();
                    this.reconcile_selection_after_refresh(cx);
                    this.reload_archived_projects(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn request_delete_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) {
        self.pending_delete_project = Some(project_id);
        cx.notify();
    }

    pub(crate) fn cancel_delete_project(&mut self, cx: &mut Context<Self>) {
        self.pending_delete_project = None;
        cx.notify();
    }

    pub(crate) fn confirm_delete_project(&mut self, project_id: Uuid, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.delete_project(project_id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.pending_delete_project = None;
                    this.refresh();
                    this.reconcile_selection_after_refresh(cx);
                    this.reload_archived_projects(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn request_rename_project(
        &mut self,
        project_id: Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let initial = self
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.name.clone())
            .unwrap_or_default();
        let placeholder = self.catalog.get("settings.projects.rename_placeholder").to_string();
        self.project_rename_input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, cx);
            input.set_content(initial, cx);
        });
        self.pending_rename_project = Some(project_id);
        self.pending_delete_project = None;
        self.project_rename_input.read(cx).focus(window);
        cx.notify();
    }

    pub(crate) fn cancel_rename_project(&mut self, cx: &mut Context<Self>) {
        self.pending_rename_project = None;
        cx.notify();
    }

    pub(crate) fn commit_rename_project(
        &mut self,
        project_id: Uuid,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            self.error = Some("Project name cannot be empty.".to_string());
            cx.notify();
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.rename_project(project_id, trimmed) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.pending_rename_project = None;
                    this.refresh();
                    this.reload_archived_projects(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selected = None;
        self.messages.clear();
        self.jump_to_latest_visible = false;
        self.stream_session = None;
        self.stream_gen = self.stream_gen.wrapping_add(1);
        self.clear_generating(cx);
    }

    fn reconcile_selection_after_refresh(&mut self, cx: &mut Context<Self>) {
        let Some(selected) = self.selected else {
            return;
        };
        if self.sessions.iter().any(|session| session.id == selected) {
            return;
        }
        self.clear_selection(cx);
    }

    pub(crate) fn toggle_model_enabled(&mut self, model_id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self
            .enabled_model_ids
            .iter()
            .position(|id| id == model_id)
        {
            self.enabled_model_ids.remove(index);
        } else {
            self.enabled_model_ids.push(model_id.to_string());
        }
        self.save_preferences(cx);
        self.sync_composer(cx);
        cx.notify();
    }

    fn save_preferences(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let body = PreferencesBody {
            enabled_model_ids: self.enabled_model_ids.clone(),
            disabled_agents: self.preferences.disabled_agents.iter().copied().collect(),
        };
        cx.spawn(async move |this, cx| {
            let saved = cx
                .background_executor()
                .spawn(async move { client.put_preferences(&body) })
                .await;
            this.update(cx, |this, cx| {
                if let Ok(prefs) = saved {
                    this.enabled_model_ids = prefs.enabled_model_ids;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn sync_composer(&mut self, cx: &mut Context<Self>) {
        let session = self.selected_session().cloned();
        let projects = self.projects.clone();
        let catalog = self.catalog.clone();
        let models: Vec<ComposerModel> = self.filtered_composer_models();
        let context_window = models
            .iter()
            .find(|model| model.id == self.selected_model)
            .and_then(|model| model.context_window.as_deref());
        let usage_fraction = context_usage_fraction(&self.messages, context_window);
        let model_variant = self.selected_model_variant.clone();
        self.composer.update(cx, |composer, cx| {
            composer.set_render_context(
                projects,
                session,
                self.work_mode,
                models,
                self.selected_model.clone(),
                self.permission_mode,
                self.interaction_mode,
                catalog,
                usage_fraction,
                model_variant,
                cx,
            );
            composer.set_generating(self.generating, cx);
        });
    }

    fn on_composer_event(&mut self, event: &ComposerEvent, cx: &mut Context<Self>) {
        match event {
            ComposerEvent::Submit(content) => {
                let content = content.clone();
                let shell = cx.entity();
                cx.defer(move |cx| {
                    let _ = shell.update(cx, |shell, cx| shell.try_send(content, cx));
                });
            }
            ComposerEvent::Stop => {
                let shell = cx.entity();
                cx.defer(move |cx| {
                    let _ = shell.update(cx, |shell, cx| shell.try_abort(cx));
                });
            }
            ComposerEvent::ProjectPicked(project_id) => {
                self.patch_session_project(Some(*project_id), cx);
            }
            ComposerEvent::ProjectCleared => {
                self.patch_session_project(None, cx);
            }
            ComposerEvent::OpenProject => {
                self.open_project_picker(cx);
            }
            ComposerEvent::OpenModelSettings => {
                self.open_settings_section(SettingsSection::Models, cx);
            }
            ComposerEvent::WorkModeChanged(mode) => {
                self.work_mode = *mode;
                cx.notify();
            }
            ComposerEvent::ModelChanged(model_id) => {
                self.selected_model = model_id.clone();
                self.sync_local_session_composer_fields();
                self.sync_composer(cx);
                // If the new model is served by a different Circulo
                // provider, the daemon dispatch needs the session's
                // `agent` field updated too. set_model_and_agent
                // PATCHes both in one round trip; otherwise fall back
                // to patch_session_composer (variant-only / same
                // provider).
                let new_agent = self
                    .composer_models
                    .iter()
                    .find(|entry| entry.id == model_id.as_str())
                    .map(|entry| entry.agent)
                    .unwrap_or(circulo_core::AgentType::OpenCode);
                let same_provider = self
                    .selected_session()
                    .map(|session| session.agent == new_agent)
                    .unwrap_or(true);
                if same_provider {
                    self.patch_session_composer(cx);
                } else if let Some(session) = self.selected_session().cloned() {
                    self.set_model_and_agent(session.id, model_id.clone(), new_agent, cx);
                }
            }
            ComposerEvent::ModelVariantChanged(variant) => {
                self.selected_model_variant = variant.clone();
                self.sync_local_session_composer_fields();
                self.sync_composer(cx);
                self.patch_session_composer(cx);
            }
            ComposerEvent::PermissionModeChanged(mode) => {
                self.permission_mode = *mode;
                self.sync_local_session_composer_fields();
                self.sync_composer(cx);
                self.patch_session_composer(cx);
            }
            ComposerEvent::InteractionModeChanged(mode) => {
                self.interaction_mode = *mode;
                self.sync_local_session_composer_fields();
                self.sync_composer(cx);
                self.patch_session_composer(cx);
            }
        }
    }

    fn open_project_picker(&mut self, cx: &mut Context<Self>) {
        if project_picker_locked(self.selected_session()) {
            return;
        }
        self.close_palette(cx);
        self.close_composer_popovers(cx);
        self.project_picker_open = true;
        self.project_picker_query.clear();
        self.project_picker_selected = 0;
        self.project_picker_pending_focus = true;
        cx.notify();
    }

    /// Sidebar Today header: assign a folder to the current draft session, or open a new
    /// project when the composer project selector is locked after the first send.
    fn open_project_from_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if project_picker_locked(self.selected_session()) {
            self.open_project_from_home(window, cx);
        } else {
            self.open_project_picker(cx);
        }
    }

    fn close_project_picker(&mut self, cx: &mut Context<Self>) {
        self.project_picker_open = false;
        self.project_picker_query.clear();
        self.project_picker_selected = 0;
        cx.notify();
    }

    fn project_picker_item_count(&self) -> usize {
        project_picker_catalog(&self.projects, &self.project_picker_query, &self.catalog)
            .selectable_len()
    }

    fn clamp_project_picker_selection(&mut self) {
        let count = self.project_picker_item_count();
        if count == 0 {
            self.project_picker_selected = 0;
        } else if self.project_picker_selected >= count {
            self.project_picker_selected = count - 1;
        }
    }

    fn execute_project_picker_selection(&mut self, cx: &mut Context<Self>) {
        let catalog = project_picker_catalog(
            &self.projects,
            &self.project_picker_query,
            &self.catalog,
        );
        let Some(item) = catalog.selectable_item(self.project_picker_selected) else {
            return;
        };
        match item.kind {
            ProjectPickerItemKind::BrowseFinder => {
                self.close_project_picker(cx);
                self.prompt_open_project(cx);
            }
            ProjectPickerItemKind::Project(id) => {
                self.close_project_picker(cx);
                self.patch_session_project(Some(id), cx);
            }
        }
        cx.notify();
    }

    fn prompt_open_project(&mut self, cx: &mut Context<Self>) {
        if project_picker_locked(self.selected_session()) {
            return;
        }
        self.folder_picker_open = true;
        let dialog_title = self
            .catalog
            .get("composer.open_project_dialog_title")
            .to_string();
        let client = self.client.clone();
        let attach_to_current = self.selected.is_some();
        cx.spawn(async move |this, cx| {
            let picked = cx
                .background_executor()
                .spawn(async move {
                    std::thread::spawn(move || {
                        crate::platform::pick_project_folder(&dialog_title)
                    })
                    .join()
                    .ok()
                    .flatten()
                })
                .await;
            if picked.is_none() {
                let _ = this.update(cx, |this, _| {
                    this.folder_picker_open = false;
                });
                return;
            }
            let picked_path = picked.unwrap();
            let name = project_name_from_picked_path(&picked_path);
            let folder_path = picked_path.to_string_lossy().into_owned();
            let result = cx
                .background_executor()
                .spawn(async move {
                    ensure_daemon(&client)?;
                    let project = client.create_project(&name, Some(folder_path))?;
                    if attach_to_current {
                        Ok(EitherProjectPick::Attach(project))
                    } else {
                        let session = client.create_session_with_project(Some(project.id))?;
                        Ok(EitherProjectPick::NewSession(project, session))
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.folder_picker_open = false;
                match result {
                    Ok(EitherProjectPick::Attach(project)) => {
                        this.patch_session_project(Some(project.id), cx);
                    }
                    Ok(EitherProjectPick::NewSession(project, session)) => {
                        this.projects.push(project);
                        this.sessions.push(session.clone());
                        this.activate_session(session.id, cx);
                        this.schedule_refresh(cx);
                        this.error = None;
                    }
                    Err(err) => this.error = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn patch_session_project(&mut self, project_id: Option<Uuid>, cx: &mut Context<Self>) {
        let Some(session_id) = self.selected else {
            return;
        };
        if project_picker_locked(self.selected_session()) {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.set_session_project(session_id, project_id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if result.is_ok() {
                    this.refresh();
                    this.sync_composer(cx);
                } else if let Err(err) = result {
                    this.error = Some(err);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_model_and_agent(
        &mut self,
        session_id: Uuid,
        model_id: String,
        agent: circulo_core::AgentType,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.set_model_and_agent(session_id, model_id, agent) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(session) => {
                        if let Some(index) =
                            this.sessions.iter().position(|entry| entry.id == session_id)
                        {
                            this.sessions[index] = session;
                        }
                        this.sync_composer(cx);
                        this.error = None;
                    }
                    Err(err) => this.error = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn patch_session_composer(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.selected else {
            return;
        };
        if self.selected_model.is_empty() {
            return;
        }
        let client = self.client.clone();
        let model_id = self.selected_model.clone();
        let model_variant = if self.selected_model_variant.is_empty() {
            None
        } else {
            Some(self.selected_model_variant.clone())
        };
        let permission_mode = self.permission_mode;
        let interaction_mode = self.interaction_mode;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    client.patch_session_composer(
                        session_id,
                        model_id,
                        model_variant,
                        permission_mode,
                        interaction_mode,
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(session) => {
                        if let Some(index) =
                            this.sessions.iter().position(|entry| entry.id == session_id)
                        {
                            this.sessions[index] = session;
                        }
                        this.sync_composer(cx);
                        this.error = None;
                    }
                    Err(err) => this.error = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl AppShell {
    pub fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let client_catalog = self.client.clone();
        let daemon_down = self.catalog.get("sidebar.daemon_down").to_string();
        let selected = self.selected;
        let snapshot_gen = self.stream_gen;
        cx.spawn(async move |this, cx| {
            let phase1 = cx
                .background_executor()
                .spawn(async move {
                    let connect = ensure_daemon(&client);
                    match connect {
                        Ok(()) => {
                            let client_sessions = client.clone();
                            let client_projects = client.clone();
                            let (sessions, projects) = std::thread::scope(|scope| {
                                let sessions_handle = scope.spawn(|| {
                                    client_sessions.list_sessions().unwrap_or_default()
                                });
                                let projects_handle = scope.spawn(|| {
                                    client_projects.list_projects().unwrap_or_default()
                                });
                                (
                                    sessions_handle.join().unwrap_or_default(),
                                    projects_handle.join().unwrap_or_default(),
                                )
                            });
                            let messages = selected
                                .and_then(|id| client.list_messages(id).ok())
                                .unwrap_or_default();
                            Ok((sessions, projects, messages))
                        }
                        Err(err) => Err(format!("{daemon_down} ({err})")),
                    }
                })
                .await;

            let phase1_ok = this
                .update(cx, |this, cx| {
                    this.loaded = true;
                    match phase1 {
                        Ok((sessions, projects, messages)) => {
                            this.sessions = sessions;
                            this.projects = projects;
                            if should_apply_refresh_transcript(
                                this.selected == selected,
                                snapshot_gen,
                                this.stream_gen,
                            ) {
                                this.messages = messages;
                                this.jump_to_latest_visible = false;
                                this.maybe_unlock_composer(cx);
                            }
                            if this.stream_session.is_some() || this.selected.is_none() {
                                this.error = None;
                            }
                            true
                        }
                        Err(message) => {
                            this.error = Some(message);
                            false
                        }
                    }
                })
                .ok()
                .unwrap_or(false);

            if phase1_ok {
                let _ = this.update(cx, |this, cx| {
                    if let Some(id) = this.selected {
                        if this.stream_session != Some(id) {
                            this.subscribe_stream(cx);
                        }
                    }
                    this.sync_composer(cx);
                    cx.notify();
                });
            } else {
                let _ = this.update(cx, |_, cx| cx.notify());
                return;
            }

            let phase2 = cx
                .background_executor()
                .spawn(async move {
                    if ensure_daemon(&client_catalog).is_err() {
                        return None;
                    }
                    let models = client_catalog.list_models().unwrap_or_default();
                    let prefs = client_catalog.get_preferences().unwrap_or_default();
                    Some((models, prefs))
                })
                .await;

            if let Some((models, prefs)) = phase2 {
                let _ = this.update(cx, |this, cx| {
                    this.composer_models = models;
                    this.enabled_model_ids = prefs.enabled_model_ids;
                    this.bootstrap_enabled_models_if_needed(cx);
                    this.apply_session_composer_state();
                    this.sync_composer(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn selected_session(&self) -> Option<&Session> {
        self.selected
            .and_then(|id| self.sessions.iter().find(|session| session.id == id))
    }

    /// True when the transcript is (roughly) at the bottom and should keep
    /// following new content.
    fn anchored(&self) -> bool {
        is_transcript_anchored(
            f32::from(self.scroll.max_offset().height),
            f32::from(self.scroll.offset().y),
            ANCHOR_THRESHOLD,
        )
    }

    fn show_jump_to_latest(&self) -> bool {
        self.jump_to_latest_visible
    }

    fn composer_popovers_open(&self, cx: &Context<Self>) -> bool {
        self.composer.read(cx).any_popover_open()
    }

    fn close_composer_popovers(&mut self, cx: &mut Context<Self>) {
        self.composer.update(cx, |composer, cx| {
            composer.close_all_popovers(cx);
        });
        cx.notify();
    }

    fn any_streaming(&self) -> bool {
        self.messages.iter().any(|message| message.is_streaming)
    }

    fn any_thinking(&self) -> bool {
        self.messages.iter().any(assistant_is_thinking)
    }

    /// Opens the selected session's event stream. A dedicated thread does the
    /// blocking reads; a spawned task drains the channel on a timer, applies
    /// events through the reducer, and reconnects with backoff when the stream
    /// ends. The generation counter discards events from superseded streams.
    fn subscribe_stream(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.selected else {
            return;
        };
        self.stream_gen += 1;
        let gen = self.stream_gen;
        self.stream_session = Some(session_id);
        self.saw_stream_event = false;
        let client = self.client.clone();

        let (tx, rx) = mpsc::channel::<ProtocolEvent>();
        std::thread::spawn(move || {
            if let Ok(mut stream) = client.session_events(session_id) {
                while let Ok(Some(event)) = stream.next_event() {
                    if tx.send(event).is_err() {
                        break;
                    }
                }
            }
            // A failing open or a ended stream simply drops `tx`; the drain
            // loop notices the disconnect and drives the recovery path.
        });

        cx.spawn(async move |this, cx| loop {
            loop {
                let mut batch = Vec::new();
                loop {
                    match rx.try_recv() {
                        Ok(event) => batch.push(event),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            let _ = this.update(cx, |this, cx| {
                                if this.folder_picker_open || this.stream_gen != gen {
                                    return;
                                }
                                this.recover_stream(session_id, gen, cx);
                            });
                            return;
                        }
                    }
                }
                if batch.is_empty() {
                    break;
                }

                let _ = this.update(cx, |this, cx| {
                    if this.folder_picker_open || this.stream_gen != gen {
                        return;
                    }
                    let mut changed = false;
                    let mut terminal = false;
                    for event in batch {
                        terminal |= matches!(
                            event,
                            ProtocolEvent::SessionMessageCompleted { .. }
                                | ProtocolEvent::SessionMessageFailed { .. }
                        );
                        this.stream_attempts =
                            stream_attempts_after_event(&event, this.stream_attempts);
                        if matches!(event, ProtocolEvent::ServerConnected { .. }) {
                            this.error = None;
                        }
                        if matches!(
                            event,
                            ProtocolEvent::SessionMessageCreated { .. }
                                | ProtocolEvent::SessionMessageUpdated { .. }
                                | ProtocolEvent::SessionMessageCompleted { .. }
                                | ProtocolEvent::SessionMessageFailed { .. }
                        ) {
                            this.saw_stream_event = true;
                        }
                        if let ProtocolEvent::SessionQuestionRequested {
                            request_id,
                            questions,
                            ..
                        } = &event
                        {
                            let placeholder =
                                this.catalog.get("question.custom_placeholder").to_string();
                            this.pending_question =
                                Some(PendingQuestion::new(request_id.clone(), questions.clone()));
                            this.question_answer_input.update(cx, |input, cx| {
                                input.set_placeholder(placeholder, cx);
                                input.set_content("", cx);
                            });
                        }
                        if let ProtocolEvent::SessionPermissionRequested {
                            permission_id,
                            summary,
                            ..
                        } = &event
                        {
                            this.pending_permission = Some(PendingPermission {
                                permission_id: permission_id.clone(),
                                summary: summary.clone(),
                            });
                        }
                        if let ProtocolEvent::SessionTitleUpdated {
                            session_id,
                            title,
                            ..
                        } = &event
                        {
                            if let Some(entry) =
                                this.sessions.iter_mut().find(|session| session.id == *session_id)
                            {
                                entry.title = title.clone();
                            }
                        }
                        changed |= apply_protocol_event(&mut this.messages, &event);
                    }
                    if terminal {
                        this.clear_generating(cx);
                        this.pending_question = None;
                    } else {
                        this.maybe_unlock_composer(cx);
                    }
                    if changed {
                        this.sync_composer(cx);
                        if this.anchored() {
                            this.scroll.scroll_to_bottom();
                        }
                    }
                    cx.notify();
                });
            }
            let stale = this
                .update(cx, |this, _| this.stream_gen != gen || this.folder_picker_open)
                .unwrap_or(true);
            if stale {
                return;
            }
            let _ = this.update(cx, |this, cx| {
                if this.any_thinking() || this.any_streaming() {
                    cx.notify();
                }
                this.maybe_unlock_composer(cx);
            });
            cx.background_executor().timer(DRAIN_INTERVAL).await;
        })
        .detach();
    }

    fn sync_question_custom_answer(&mut self, answer: &str, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_question.as_mut() else {
            return;
        };
        let Some(question) = pending.current_question() else {
            return;
        };
        let question_id = question.id.clone();
        if answer.trim().is_empty() {
            pending.custom_answers.remove(&question_id);
        } else {
            pending.custom_answers.insert(question_id.clone(), answer.to_owned());
            pending.selections.remove(&question_id);
        }
        cx.notify();
    }

    fn sync_question_input_from_pending(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_question.as_ref() else {
            return;
        };
        let Some(question) = pending.current_question() else {
            return;
        };
        let restored = pending
            .custom_answers
            .get(&question.id)
            .cloned()
            .unwrap_or_default();
        self.question_answer_input.update(cx, |input, cx| {
            input.set_content(restored, cx);
        });
    }

    pub(crate) fn select_question_option(&mut self, label: String, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_question.as_mut() else {
            return;
        };
        let Some(question) = pending.current_question() else {
            return;
        };
        let question_id = question.id.clone();
        let multi_select = question.multi_select;
        let selected = pending.selections.entry(question_id.clone()).or_default();
        if multi_select {
            if let Some(index) = selected.iter().position(|answer| answer == &label) {
                selected.remove(index);
            } else {
                selected.push(label);
            }
        } else {
            selected.clear();
            selected.push(label);
        }
        if selected.is_empty() {
            pending.selections.remove(&question_id);
        }
        pending.custom_answers.remove(&question_id);
        self.question_answer_input.update(cx, |input, cx| input.set_content("", cx));
        cx.notify();
    }

    pub(crate) fn previous_question(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_question.as_mut() else {
            return;
        };
        if pending.question_index == 0 {
            return;
        }
        pending.question_index -= 1;
        self.sync_question_input_from_pending(cx);
        cx.notify();
    }

    pub(crate) fn advance_question(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.selected else {
            return;
        };
        let custom = self.question_answer_input.read(cx).content().to_string();
        self.sync_question_custom_answer(&custom, cx);
        let should_submit = {
            let Some(pending) = self.pending_question.as_mut() else {
                return;
            };
            let Some(question) = pending.current_question() else {
                return;
            };
            let answered = pending
                .custom_answers
                .get(&question.id)
                .is_some_and(|answer| !answer.trim().is_empty())
                || pending
                    .selections
                    .get(&question.id)
                    .is_some_and(|answers| !answers.is_empty());
            if !answered {
                return;
            }
            if pending.question_index + 1 < pending.questions.len() {
                pending.question_index += 1;
                false
            } else {
                true
            }
        };
        if should_submit {
            let Some(pending) = self.pending_question.take() else {
                return;
            };
            let request_id = pending.request_id.clone();
            let answers = pending.answers();
            let client = self.client.clone();
            cx.spawn(async move |this, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move { client.reply_question(session_id, &request_id, answers) })
                    .await;
                if let Err(message) = result {
                    let _ = this.update(cx, |this, cx| {
                        this.error = Some(message);
                        cx.notify();
                    });
                }
            })
            .detach();
            self.question_answer_input.update(cx, |input, cx| input.set_content("", cx));
        } else {
            self.sync_question_input_from_pending(cx);
        }
        cx.notify();
    }

    pub(crate) fn reply_permission(&mut self, allow: bool, cx: &mut Context<Self>) {
        let Some(session_id) = self.selected else {
            return;
        };
        let Some(pending) = self.pending_permission.take() else {
            return;
        };
        let client = self.client.clone();
        let permission_id = pending.permission_id;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.reply_permission(session_id, &permission_id, allow) })
                .await;
            if let Err(message) = result {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(message);
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// Refetches once and resubscribes with 1/2/4 s backoff, giving up after
    /// three attempts on this outage; a superseded generation stays dead.
    fn recover_stream(&mut self, session_id: Uuid, gen: u64, cx: &mut Context<Self>) {
        self.stream_attempts += 1;
        let Some(delay) = resubscribe_delay(self.stream_attempts) else {
            self.stream_session = None;
            self.error = Some(self.catalog.get("messages.stream_dropped").to_string());
            cx.notify();
            return;
        };
        self.schedule_refresh(cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |this, cx| {
                if this.stream_gen == gen && this.selected == Some(session_id) {
                    this.subscribe_stream(cx);
                }
            });
        })
        .detach();
    }

    fn select_session(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_session(id, cx);
        self.composer_pending_focus = false;
        self.composer
            .update(cx, |composer, cx| composer.focus_after_session_select(window, cx));
    }

    fn clear_generating(&mut self, cx: &mut Context<Self>) {
        self.generating = false;
        self.composer.update(cx, |composer, cx| {
            composer.set_generating(false, cx);
        });
    }

    fn maybe_unlock_composer(&mut self, cx: &mut Context<Self>) {
        if should_unlock_composer(self.generating, &self.messages) {
            self.clear_generating(cx);
        }
    }

    fn halt_streaming_messages(&mut self) {
        for message in &mut self.messages {
            if message.is_streaming {
                message.is_streaming = false;
            }
        }
    }

    fn activate_session(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.selected = Some(id);
        self.close_palette(cx);
        self.close_session_overlay(cx);
        self.pending_permission = None;
        self.pending_question = None;
        self.clear_generating(cx);
        self.messages = self.client.list_messages(id).unwrap_or_default();
        self.jump_to_latest_visible = false;
        if self.messages.iter().any(|message| message.is_streaming) {
            self.generating = true;
            self.composer.update(cx, |composer, cx| {
                composer.set_generating(true, cx);
            });
        }
        self.stream_attempts = 0;
        self.error = None;
        self.apply_session_composer_state();
        self.sync_composer(cx);
        self.subscribe_stream(cx);
        self.composer_pending_focus = true;
    }

    pub(crate) fn create_new_session(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.clear_generating(cx);
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    ensure_daemon(&client)?;
                    client.create_session()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(session) => {
                        this.sessions.push(session.clone());
                        this.activate_session(session.id, cx);
                        this.refresh();
                        this.error = None;
                    }
                    Err(message) => this.error = Some(message),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn open_project_from_home(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.folder_picker_open = true;
        let dialog_title = self
            .catalog
            .get("composer.open_project_dialog_title")
            .to_string();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let picked = cx
                .background_executor()
                .spawn(async move {
                    std::thread::spawn(move || {
                        crate::platform::pick_project_folder(&dialog_title)
                    })
                    .join()
                    .ok()
                    .flatten()
                })
                .await;
            if picked.is_none() {
                let _ = this.update(cx, |this, _| {
                    this.folder_picker_open = false;
                });
                return;
            }
            let picked_path = picked.unwrap();
            let name = project_name_from_picked_path(&picked_path);
            let folder_path = picked_path.to_string_lossy().into_owned();
            let result = cx
                .background_executor()
                .spawn(async move {
                    ensure_daemon(&client)?;
                    let project = client.create_project(&name, Some(folder_path))?;
                    let session = client.create_session_with_project(Some(project.id))?;
                    Ok((project, session))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.folder_picker_open = false;
                match result {
                    Ok((project, session)) => {
                        this.projects.push(project);
                        this.sessions.push(session.clone());
                        this.activate_session(session.id, cx);
                        this.schedule_refresh(cx);
                        this.error = None;
                    }
                    Err(err) => this.error = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_session_overlay(cx);
        self.close_project_picker(cx);
        self.close_composer_popovers(cx);
        self.palette_open = true;
        self.palette_query.clear();
        self.palette_selected = 0;
        self.palette_focus.focus(window);
        cx.notify();
    }

    fn close_palette(&mut self, cx: &mut Context<Self>) {
        self.palette_open = false;
        self.palette_query.clear();
        self.palette_selected = 0;
        cx.notify();
    }

    pub(crate) fn open_session_context_menu(
        &mut self,
        session_id: Uuid,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_palette(cx);
        self.close_project_picker(cx);
        self.close_composer_popovers(cx);
        self.session_overlay = Some(SessionOverlay::ContextMenu {
            session_id,
            position,
        });
        self.session_menu_selected = 0;
        self.session_menu_focus.focus(window);
        cx.notify();
    }

    pub(crate) fn close_session_overlay(&mut self, cx: &mut Context<Self>) {
        self.session_overlay = None;
        self.session_menu_selected = 0;
        cx.notify();
    }

    pub(crate) fn execute_session_menu_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(SessionOverlay::ContextMenu { session_id, .. }) = self.session_overlay else {
            return;
        };
        match self.session_menu_selected {
            0 => self.start_rename_session(session_id, window, cx),
            1 => self.delete_session(session_id, window, cx),
            _ => {}
        }
    }

    pub(crate) fn start_rename_session(
        &mut self,
        session_id: Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| session.title.clone())
            .unwrap_or_default();
        let placeholder = self.catalog.get("session.rename_placeholder").to_string();
        self.rename_input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, cx);
            input.set_content(title, cx);
        });
        self.session_overlay = Some(SessionOverlay::Rename { session_id });
        self.rename_input.read(cx).focus(window);
        cx.notify();
    }

    pub(crate) fn delete_session(
        &mut self,
        session_id: Uuid,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_session_overlay(cx);

        let clearing_selection = self.selected == Some(session_id);
        if clearing_selection {
            self.stream_gen += 1;
            self.stream_session = None;
            self.generating = false;
            self.composer.update(cx, |composer, cx| {
                composer.set_generating(false, cx);
            });
        }

        let client = self.client.clone();
        let delete_failed = self.catalog.get("session.delete_failed").to_string();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.delete_session(session_id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        if clearing_selection {
                            this.selected = None;
                            this.messages.clear();
                            this.sync_composer(cx);
                        }
                        this.sessions.retain(|session| session.id != session_id);
                        this.refresh();
                        this.error = None;
                    }
                    Err(err) => {
                        this.error = Some(format!("{delete_failed} ({err})"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn commit_rename_session(&mut self, cx: &mut Context<Self>) {
        let Some(SessionOverlay::Rename { session_id }) = self.session_overlay.clone() else {
            return;
        };
        let trimmed = self.rename_input.read(cx).content().trim();
        if trimmed.is_empty() {
            self.close_session_overlay(cx);
            return;
        }
        let client = self.client.clone();
        let new_title = trimmed.to_string();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.rename_session(session_id, new_title) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(session) => {
                        if let Some(entry) = this.sessions.iter_mut().find(|s| s.id == session_id) {
                            *entry = session;
                        }
                        this.close_session_overlay(cx);
                    }
                    Err(err) => {
                        this.error = Some(err);
                        this.close_session_overlay(cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn palette_item_count(&self) -> usize {
        palette_catalog(
            &self.sessions,
            &self.projects,
            &self.palette_query,
            self.sidebar_collapsed,
            &self.catalog,
        )
        .selectable_len()
    }

    fn clamp_palette_selection(&mut self) {
        let count = self.palette_item_count();
        if count == 0 {
            self.palette_selected = 0;
        } else if self.palette_selected >= count {
            self.palette_selected = count - 1;
        }
    }

    fn execute_palette_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let catalog = palette_catalog(
            &self.sessions,
            &self.projects,
            &self.palette_query,
            self.sidebar_collapsed,
            &self.catalog,
        );
        let Some(item) = catalog.selectable_item(self.palette_selected) else {
            return;
        };
        match item.kind {
            PaletteItemKind::NewSession => {
                if let Ok(session) = self.client.create_session() {
                    self.sessions.push(session.clone());
                    self.select_session(session.id, window, cx);
                    self.refresh();
                }
                self.close_palette(cx);
            }
            PaletteItemKind::ToggleSidebar => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.close_palette(cx);
            }
            PaletteItemKind::Session(id) => {
                self.select_session(id, window, cx);
            }
        }
        cx.notify();
    }

    pub(crate) fn try_abort(&mut self, cx: &mut Context<Self>) {
        if !self.generating {
            return;
        }
        let Some(session_id) = self.selected else {
            return;
        };
        self.clear_generating(cx);
        self.halt_streaming_messages();
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.abort_session(session_id) })
                .await;
            if let Err(message) = result {
                let _ = this.update(cx, |this, cx| {
                    this.generating = true;
                    this.composer.update(cx, |composer, cx| {
                        composer.set_generating(true, cx);
                    });
                    this.error = Some(message);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn try_send(&mut self, content: String, cx: &mut Context<Self>) {
        if !can_send(self.selected.is_some(), &content, self.generating) {
            return;
        }
        let Some(session_id) = self.selected else {
            return;
        };
        self.sync_local_session_composer_fields();
        if self.selected_model.is_empty() {
            self.error = Some(
                self.catalog
                    .get("composer.models.none")
                    .to_string(),
            );
            cx.notify();
            return;
        }
        let locked = project_picker_locked(self.selected_session());
        let current_project = self
            .selected_session()
            .and_then(|session| session.project_id);
        let draft_project = self.composer.read(cx).draft_project();
        let should_patch_project = !locked && draft_project != current_project;
        let client = self.client.clone();
        let submitted = content.clone();
        let model_id = self.selected_model.clone();
        let model_variant = if self.selected_model_variant.is_empty() {
            None
        } else {
            Some(self.selected_model_variant.clone())
        };
        let permission_mode = self.permission_mode;
        let interaction_mode = self.interaction_mode;

        self.composer.update(cx, |composer, cx| {
            composer.clear_after_send(cx);
            composer.set_generating(true, cx);
        });
        self.generating = true;
        if self.stream_session != Some(session_id) {
            self.stream_attempts = 0;
            self.subscribe_stream(cx);
        }

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    client.patch_session_composer(
                        session_id,
                        model_id,
                        model_variant,
                        permission_mode,
                        interaction_mode,
                    )?;
                    if should_patch_project {
                        client.set_session_project(session_id, draft_project)?;
                    }
                    let user = client.post_message(session_id, content.trim())?;
                    Ok::<_, String>(user)
                })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(user) => {
                        if this.selected == Some(session_id)
                            && !this
                                .messages
                                .iter()
                                .any(|message| message.id == user.id)
                        {
                            this.messages.push(user);
                        }
                        this.error = None;
                        let client = this.client.clone();
                        cx.spawn(async move |this, cx| {
                            if let Ok(sessions) = cx
                                .background_executor()
                                .spawn(async move { client.list_sessions() })
                                .await
                            {
                                let _ = this.update(cx, |this, cx| {
                                    if this.selected == Some(session_id) {
                                        this.sessions = sessions;
                                    }
                                    cx.notify();
                                });
                            }
                        })
                        .detach();
                    }
                    Err(message) => {
                        this.clear_generating(cx);
                        this.composer.update(cx, |composer, cx| {
                            composer.restore_content(submitted, cx);
                        });
                        this.error = Some(message);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl AppShell {
    pub fn sidebar_width(&self) -> f32 {
        sidebar_width_px(self.sidebar_collapsed, self.sidebar_width_expanded)
    }

    fn selected_title(&self) -> String {
        if self.selected.is_none() {
            return self.catalog.get("home.title").to_string();
        }
        self.selected_session()
            .map(|session| session.title.clone())
            .unwrap_or_else(|| self.catalog.get("session.none").to_string())
    }

    fn refresh(&mut self) {
        match ensure_daemon(&self.client) {
            Ok(()) => self.error = None,
            Err(err) => {
                self.error = Some(format!("{} ({err})", self.catalog.get("sidebar.daemon_down")));
                return;
            }
        }
        match self.client.list_sessions() {
            Ok(sessions) => self.sessions = sessions,
            Err(err) => self.error = Some(err),
        }
        match self.client.list_projects() {
            Ok(projects) => self.projects = projects,
            Err(err) => self.error = Some(err),
        }
        if let Some(id) = self.selected {
            match self.client.list_messages(id) {
                Ok(messages) => self.messages = messages,
                Err(err) => self.error = Some(err),
            }
            self.jump_to_latest_visible = false;
        } else {
            self.messages.clear();
            self.jump_to_latest_visible = false;
        }
        // Best-effort refresh of the available agents. Failure here is
        // non-fatal: the composer falls back to a single-entry list.
        let _ = self.client.list_agents().map(|agents| self.available_agents = agents);
    }

    /// Agents the user can pick in the AgentSelector: registered AND
    /// enabled. The AgentSelector chip renders when this list has more
    /// than one entry.
    #[allow(dead_code)]
    pub(crate) fn visible_agents(&self) -> Vec<circulo_protocol::AgentDescriptor> {
        self.available_agents
            .iter()
            .filter(|agent| {
                agent.enabled && !self.preferences.disabled_agents.contains(&agent.agent)
            })
            .cloned()
            .collect()
    }

        /// Change the agent of an unstarted session. Wired up by the AgentSelector
    /// in the composer; the selector is only rendered when
    /// `available_agents.len() > 1`, so this is currently dead code. The
    /// `commandcode-adapter` change wires it up.
    #[allow(dead_code)]
    pub(crate) fn set_session_agent(
        &mut self,
        session_id: Uuid,
        agent: circulo_core::AgentType,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.patch_session_agent(session_id, agent) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(err) = result {
                    this.error = Some(err);
                } else {
                    this.refresh();
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.project_picker_pending_focus {
            self.project_picker_pending_focus = false;
            let focus = self.project_picker_focus.clone();
            window.on_next_frame(move |window, _| {
                focus.focus(window);
            });
        }
        if self.composer_pending_focus {
            self.composer_pending_focus = false;
            let composer = self.composer.clone();
            window.on_next_frame(move |window, cx| {
                composer.update(cx, |composer, cx| {
                    composer.focus_after_session_select(window, cx);
                });
            });
        }
        let collapsed = self.sidebar_collapsed;
        let catalog = self.catalog.clone();

        div()
            .id("app-shell")
            .relative()
            .flex()
            .size_full()
            .bg(BG_APP)
            .text_color(TEXT)
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.sidebar_resize_origin.is_some() {
                        this.end_sidebar_resize();
                        cx.notify();
                    }
                }),
            )
            .on_action(cx.listener(|this, _: &OpenPalette, window, cx| {
                this.open_palette(window, cx);
            }))
            .when(!collapsed, |el| {
                el.child(sidebar_expanded(self, &catalog, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(main_column(self, &catalog, window, cx)),
            )
            .child(sidebar_toggle(collapsed, cx))
            .when(!collapsed, |el| {
                el.child(sidebar_resize_handle(
                    self.sidebar_width_expanded,
                    self.sidebar_resize_origin.is_some(),
                    self.sidebar_resize_hovered,
                    cx,
                ))
            })
            .when(self.composer_popovers_open(cx), |el| {
                el.child(composer_popover_dismiss_layer(cx))
            })
            .when(self.palette_open, |el| {
                el.child(command_palette_overlay(self, &catalog, cx))
            })
            .when(self.project_picker_open, |el| {
                el.child(project_picker_overlay(self, &catalog, cx))
            })
            .when(self.session_overlay.is_some(), |el| {
                let overlay = self.session_overlay.clone().expect("overlay present");
                el.child(session_overlay(
                    &overlay,
                    &self.session_menu_focus,
                    &self.rename_input,
                    self.session_menu_selected,
                    &catalog,
                    cx,
                ))
            })
    }
}

fn handle_palette_key(
    this: &mut AppShell,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    let key = event.keystroke.key.as_str();
    if key == "escape" {
        this.close_palette(cx);
        cx.stop_propagation();
        return;
    }
    if key == "up" {
        if this.palette_selected > 0 {
            this.palette_selected -= 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "down" {
        this.clamp_palette_selection();
        if this.palette_selected + 1 < this.palette_item_count() {
            this.palette_selected += 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "enter" {
        this.execute_palette_selection(window, cx);
        cx.stop_propagation();
        return;
    }
    if key == "backspace" {
        this.palette_query.pop();
        this.palette_selected = 0;
    } else if let Some(ch) = typed_char(event) {
        this.palette_query.push_str(&ch);
        this.palette_selected = 0;
    } else {
        return;
    }
    this.clamp_palette_selection();
    cx.stop_propagation();
    cx.notify();
}

fn handle_project_picker_key(
    this: &mut AppShell,
    event: &KeyDownEvent,
    cx: &mut Context<AppShell>,
) {
    let key = event.keystroke.key.as_str();
    if key == "escape" {
        this.close_project_picker(cx);
        cx.stop_propagation();
        return;
    }
    if key == "up" {
        if this.project_picker_selected > 0 {
            this.project_picker_selected -= 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "down" {
        this.clamp_project_picker_selection();
        if this.project_picker_selected + 1 < this.project_picker_item_count() {
            this.project_picker_selected += 1;
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if key == "enter" {
        this.execute_project_picker_selection(cx);
        cx.stop_propagation();
        return;
    }
    if key == "backspace" {
        this.project_picker_query.pop();
        this.project_picker_selected = 0;
    } else if let Some(ch) = typed_char(event) {
        this.project_picker_query.push_str(&ch);
        this.project_picker_selected = 0;
    } else {
        return;
    }
    this.clamp_project_picker_selection();
    cx.stop_propagation();
    cx.notify();
}

fn composer_popover_dismiss_layer(cx: &mut Context<AppShell>) -> impl IntoElement {
    deferred(
        div()
            .id("composer-popover-dismiss")
            .absolute()
            .size_full()
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.close_composer_popovers(cx);
            })),
    )
    .with_priority(5)
}

fn project_picker_overlay(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let catalog_items = project_picker_catalog(
        &state.projects,
        &state.project_picker_query,
        catalog,
    );
    let picker_focus = state.project_picker_focus.clone();
    let query_display = if state.project_picker_query.is_empty() {
        catalog.get("composer.project_picker.placeholder").to_string()
    } else {
        state.project_picker_query.clone()
    };

    let mut list = div().flex().flex_col().py_1();
    if catalog_items.selectable_len() == 0 {
        list = list.child(
            div()
                .px_3()
                .py_2()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(catalog.get("composer.project_picker.empty").to_string()),
        );
    } else {
        let mut selectable_index = 0usize;
        if !catalog_items.actions.is_empty() {
            list = list.child(palette_section_label(
                catalog
                    .get("composer.project_picker.section_actions")
                    .to_string(),
            ));
            for item in &catalog_items.actions {
                list = list.child(project_picker_row(
                    selectable_index,
                    state.project_picker_selected == selectable_index,
                    item,
                    cx,
                ));
                selectable_index += 1;
            }
        }
        if !catalog_items.actions.is_empty() && !catalog_items.projects.is_empty() {
            list = list.child(palette_separator());
        }
        if !catalog_items.projects.is_empty() {
            list = list.child(palette_section_label(
                catalog
                    .get("composer.project_picker.section_projects")
                    .to_string(),
            ));
            for item in &catalog_items.projects {
                list = list.child(project_picker_row(
                    selectable_index,
                    state.project_picker_selected == selectable_index,
                    item,
                    cx,
                ));
                selectable_index += 1;
            }
        }
    }

    div()
        .absolute()
        .size_full()
        .occlude()
        .bg(PALETTE_BACKDROP)
        .flex()
        .items_start()
        .justify_center()
        .pt(px(72.))
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.close_project_picker(cx);
        }))
        .child(
            div()
                .id("project-picker")
                .track_focus(&picker_focus)
                .w(px(520.))
                .max_h(px(420.))
                .flex()
                .flex_col()
                .rounded_lg()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .shadow_lg()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event, window, cx| {
                    if this.project_picker_focus.is_focused(window) {
                        handle_project_picker_key(this, event, cx);
                    }
                }))
                .child(
                    div()
                        .px_3()
                        .py_3()
                        .border_b_1()
                        .border_color(BORDER)
                        .text_sm()
                        .text_color(if state.project_picker_query.is_empty() {
                            TEXT_MUTED
                        } else {
                            TEXT
                        })
                        .child(query_display),
                )
                .child(
                    div()
                        .id("project-picker-results")
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .child(list),
                ),
        )
}

fn project_picker_row(
    index: usize,
    selected: bool,
    item: &ProjectPickerItem,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let mut row = div()
        .id(("project-picker-item", index))
        .flex()
        .flex_col()
        .mx_1()
        .px_2()
        .py_1()
        .rounded(px(4.))
        .cursor_pointer()
        .when(selected, |el| el.bg(ACCENT_SURFACE))
        .when(!selected, |el| el.hover(|style| style.bg(ACCENT_SURFACE)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.project_picker_selected = index;
                this.execute_project_picker_selection(cx);
            }),
        );
    row = row.child(div().text_sm().child(item.label.clone()));
    if let Some(detail) = &item.detail {
        row = row.child(
            div()
                .text_xs()
                .text_color(TEXT_MUTED)
                .child(detail.clone()),
        );
    }
    row
}

fn command_palette_overlay(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let catalog_items = palette_catalog(
        &state.sessions,
        &state.projects,
        &state.palette_query,
        state.sidebar_collapsed,
        catalog,
    );
    let palette_focus = state.palette_focus.clone();
    let query_display = if state.palette_query.is_empty() {
        catalog.get("command.palette_placeholder").to_string()
    } else {
        state.palette_query.clone()
    };

    let mut list = div().flex().flex_col().py_1();
    if catalog_items.selectable_len() == 0 {
        list = list.child(
            div()
                .px_3()
                .py_2()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(catalog.get("command.palette_empty").to_string()),
        );
    } else {
        let mut selectable_index = 0usize;
        if !catalog_items.commands.is_empty() {
            list = list.child(palette_section_label(
                catalog.get("command.section_commands").to_string(),
            ));
            for item in &catalog_items.commands {
                list = list.child(palette_row(
                    selectable_index,
                    state.palette_selected == selectable_index,
                    item,
                    cx,
                ));
                selectable_index += 1;
            }
        }
        if !catalog_items.commands.is_empty() && !catalog_items.sessions.is_empty() {
            list = list.child(palette_separator());
        }
        if !catalog_items.sessions.is_empty() {
            list = list.child(palette_section_label(
                catalog.get("command.section_sessions").to_string(),
            ));
            for item in &catalog_items.sessions {
                list = list.child(palette_row(
                    selectable_index,
                    state.palette_selected == selectable_index,
                    item,
                    cx,
                ));
                selectable_index += 1;
            }
        }
    }

    div()
        .absolute()
        .size_full()
        .occlude()
        .bg(PALETTE_BACKDROP)
        .flex()
        .items_start()
        .justify_center()
        .pt(px(72.))
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.close_palette(cx);
        }))
        .child(
            div()
                .id("command-palette")
                .track_focus(&palette_focus)
                .w(px(520.))
                .max_h(px(420.))
                .flex()
                .flex_col()
                .rounded_lg()
                .border_1()
                .border_color(BORDER)
                .bg(BG_SIDEBAR)
                .shadow_lg()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event, window, cx| {
                    if this.palette_focus.is_focused(window) {
                        handle_palette_key(this, event, window, cx);
                    }
                }))
                .child(
                    div()
                        .px_3()
                        .py_3()
                        .border_b_1()
                        .border_color(BORDER)
                        .text_sm()
                        .text_color(if state.palette_query.is_empty() {
                            TEXT_MUTED
                        } else {
                            TEXT
                        })
                        .child(query_display),
                )
                .child(
                    div()
                        .id("palette-results")
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .max_h(px(360.))
                        .child(list),
                ),
        )
}

fn palette_section_label(text: String) -> impl IntoElement {
    div()
        .px_2()
        .pt_1()
        .pb_1()
        .text_xs()
        .text_color(TEXT_MUTED)
        .child(text)
}

fn palette_separator() -> impl IntoElement {
    div().mx_2().my_1().h(px(1.)).bg(BORDER)
}

fn palette_row(
    index: usize,
    selected: bool,
    item: &PaletteItem,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let mut row = div()
        .id(("palette-item", index))
        .flex()
        .flex_col()
        .mx_1()
        .px_2()
        .py_1()
        .rounded(px(4.))
        .cursor_pointer()
        .when(selected, |el| el.bg(ACCENT_SURFACE))
        .when(!selected, |el| el.hover(|style| style.bg(ACCENT_SURFACE)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, window, cx| {
                this.palette_selected = index;
                this.execute_palette_selection(window, cx);
            }),
        );
    row = row.child(div().text_sm().child(item.label.clone()));
    if let Some(detail) = &item.detail {
        row = row.child(
            div()
                .text_xs()
                .text_color(TEXT_MUTED)
                .child(detail.clone()),
        );
    }
    row
}

fn typed_char(event: &KeyDownEvent) -> Option<String> {
    if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
        return None;
    }
    if event.keystroke.key == "enter" {
        return None;
    }
    if let Some(ch) = &event.keystroke.key_char {
        if !ch.is_empty() {
            return Some(ch.clone());
        }
    }
    let key = event.keystroke.key.as_str();
    if key == "space" {
        Some(" ".into())
    } else if key.len() == 1 {
        Some(key.to_string())
    } else {
        None
    }
}

fn sidebar_expanded(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let width = state.sidebar_width_expanded;
    div()
        .relative()
        .flex()
        .flex_col()
        .w(px(width))
        .min_w_0()
        .h_full()
        .bg(BG_SIDEBAR)
        .border_r_1()
        .border_color(BORDER)
        .child(div().flex_none().h(px(APP_BAR_HEIGHT_PX)))
        .child(sidebar_body(state, catalog, cx))
}

fn sidebar_toggle(collapsed: bool, cx: &mut Context<AppShell>) -> impl IntoElement {
    let icon_path = if collapsed {
        icon_path::PANEL_LEFT_OPEN
    } else {
        icon_path::PANEL_LEFT_CLOSE
    };
    div()
        .absolute()
        .left(px(SIDEBAR_TOGGLE_LEFT_PX))
        .top(px(SIDEBAR_TOGGLE_TOP_PX))
        .id("toggle-sidebar")
        .flex()
        .items_center()
        .justify_center()
        .w(px(SIDEBAR_TOGGLE_SIZE_PX))
        .h(px(SIDEBAR_TOGGLE_SIZE_PX))
        .rounded_md()
        .hover(|style| style.bg(BG_MAIN))
        .cursor_pointer()
        .on_click(cx.listener(|this, _, _, cx| {
            this.sidebar_collapsed = !this.sidebar_collapsed;
            cx.notify();
        }))
        .child(icon(icon_path, px(16.), TEXT_MUTED))
}

struct SidebarResizeDrag;

struct SidebarResizeDragPreview;

impl Render for SidebarResizeDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_0()
    }
}

fn sidebar_resize_handle(
    sidebar_width: f32,
    resizing: bool,
    hovered: bool,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let show_stripe = hovered || resizing;
    let visual_offset = (SIDEBAR_RESIZE_HANDLE_HIT_PX - SIDEBAR_RESIZE_HANDLE_VISUAL_PX) / 2.0;
    let center = if resizing {
        SIDEBAR_RESIZE_HANDLE_CENTER_ACTIVE
    } else {
        SIDEBAR_RESIZE_HANDLE_CENTER
    };
    let transparent = gpui::Rgba {
        r: center.r,
        g: center.g,
        b: center.b,
        a: 0.0,
    };
    let gradient_top = linear_gradient(
        180.,
        linear_color_stop(transparent, 0.),
        linear_color_stop(center, 1.),
    );
    let gradient_bottom = linear_gradient(
        180.,
        linear_color_stop(center, 0.),
        linear_color_stop(transparent, 1.),
    );

    div()
        .id("sidebar-resize-handle")
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(sidebar_width - SIDEBAR_RESIZE_HANDLE_HIT_PX / 2.))
        .w(px(SIDEBAR_RESIZE_HANDLE_HIT_PX))
        .cursor(CursorStyle::ResizeLeftRight)
        .on_hover(cx.listener(|this, active: &bool, _, cx| {
            if this.sidebar_resize_hovered != *active {
                this.sidebar_resize_hovered = *active;
                cx.notify();
            }
        }))
        .on_drag(SidebarResizeDrag, |_, _, _, cx| {
            cx.new(|_| SidebarResizeDragPreview)
        })
        .on_drag_move(cx.listener(|this, event: &DragMoveEvent<SidebarResizeDrag>, _, cx| {
            let pos = event.event.position.x;
            let (start_x, start_w) = match this.sidebar_resize_origin {
                Some(origin) => origin,
                None => {
                    this.sidebar_resize_origin = Some((pos, this.sidebar_width_expanded));
                    (pos, this.sidebar_width_expanded)
                }
            };
            let delta: f32 = (pos - start_x).into();
            this.sidebar_width_expanded =
                (start_w + delta).clamp(SIDEBAR_MIN_PX, SIDEBAR_MAX_PX);
            cx.notify();
        }))
        .when(show_stripe, |el| {
            el.child(
                div()
                    .absolute()
                    .left(px(visual_offset))
                    .top_0()
                    .bottom_0()
                    .w(px(SIDEBAR_RESIZE_HANDLE_VISUAL_PX))
                    .flex()
                    .flex_col()
                    .child(div().flex_1().bg(gradient_top))
                    .child(div().flex_1().bg(gradient_bottom)),
            )
        })
}

fn sidebar_body(state: &AppShell, catalog: &Catalog, cx: &mut Context<AppShell>) -> gpui::Div {
    if state.settings_open {
        return settings_sidebar_body(state, catalog, cx);
    }

    let fixed_header = div()
        .flex_none()
        .flex()
        .flex_col()
        .px_3()
        .pt_4()
        .pb_2()
        .gap_2()
        .child(action_row(
            "action-new-session",
            catalog.get("sidebar.new_session"),
            Some(icon_path::MESSAGE_CIRCLE_PLUS),
            cx.listener(|this, _, window, cx| {
                if let Ok(session) = this.client.create_session() {
                    this.sessions.push(session.clone());
                    this.select_session(session.id, window, cx);
                    this.refresh();
                }
                cx.notify();
            }),
        ))
        .child(
            div()
                .id("search-open")
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .text_color(TEXT)
                .hover(|style| style.bg(BG_MAIN))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_palette(window, cx);
                }))
                .child(
                    div()
                        .text_xs()
                        .child(icon(icon_path::SEARCH, px(14.), TEXT)),
                )
                .child(catalog.get("sidebar.search").to_string()),
        );

    let mut scroll_content = div()
        .flex()
        .flex_col()
        .gap_2()
        .pb_2()
        .min_w_0()
        .w_full();

    if let Some(error) = &state.error {
        scroll_content = scroll_content.child(
            div()
                .px_2()
                .py_2()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(error.clone()),
        );
    } else {
        scroll_content = scroll_content.child(sidebar_session_sections(state, catalog, cx));
    }

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(fixed_header)
        .child(
            div()
                .id("sidebar-sessions")
                .flex_1()
                .min_h_0()
                .min_w_0()
                .w_full()
                .overflow_hidden()
                .overflow_y_scroll()
                .track_scroll(&state.sidebar_scroll)
                .px_3()
                .child(scroll_content),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .h(px(APP_BAR_HEIGHT_PX))
                .px_3()
                .child(
                    div()
                        .id("open-settings")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(28.))
                        .h(px(28.))
                        .rounded_md()
                        .text_color(TEXT_MUTED)
                        .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_settings(cx);
                        }))
                        .child(icon(icon_path::SETTINGS, px(16.), TEXT_MUTED)),
                ),
        )
}

fn settings_sidebar_body(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    let fixed_header = div()
        .flex_none()
        .flex()
        .flex_col()
        .px_3()
        .pt_4()
        .pb_2()
        .gap_2()
        .child(action_row(
            "settings-back",
            catalog.get("settings.back"),
            None,
            cx.listener(|this, _, _, cx| {
                this.close_settings(cx);
            }),
        ));

    let mut nav = div().flex().flex_col().gap_1();
    for section in SettingsSection::ALL {
        let active = state.settings_section == section;
        let label_text = catalog.get(section.label_key()).to_string();
        nav = nav.child(
            div()
                .id(section.nav_id())
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .cursor_pointer()
                .when(active, |el| el.bg(BG_MAIN).text_color(TEXT))
                .when(!active, |el| {
                    el.text_color(TEXT_MUTED)
                        .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings_section = section;
                    if section == SettingsSection::General {
                        this.refresh_settings_health(cx);
                    }
                    if section == SettingsSection::Archived {
                        this.reload_archived_projects(cx);
                    }
                    cx.notify();
                }))
                .child(label_text),
        );
    }

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(fixed_header)
        .child(
            div()
                .id("settings-sidebar-nav")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px_3()
                .pb_2()
                .child(nav),
        )
}

fn sidebar_session_sections(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    let visible: Vec<&Session> = state.sessions.iter().collect();
    if visible.is_empty() {
        return div().child(muted(catalog.get("sidebar.empty_sessions")));
    }

    let now = OffsetDateTime::now_utc();
    let offset = local_offset_or_utc();
    let (today, earlier) = partition_sessions_by_day(&visible, now, offset);
    let without_folder = catalog.get("session.without_folder").to_string();
    let session_row_inner_width_px = (state.sidebar_width_expanded
        - SIDEBAR_SESSION_SCROLL_PADDING_X_PX
        - SESSION_ROW_PADDING_X_PX)
        .max(0.);
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .min_w_0()
        .w_full()
        .overflow_hidden();

    if !today.is_empty() {
        col = col.child(
            collapsible_section_header_with_open_project(
                "section-today",
                catalog.get("sidebar.today"),
                state.today_expanded,
                cx.listener(|this, _, _, cx| {
                    this.today_expanded = !this.today_expanded;
                    cx.notify();
                }),
                cx.listener(|this, _, window, cx| {
                    this.open_project_from_sidebar(window, cx);
                }),
            ),
        );
        if state.today_expanded {
            for session in today {
                let id = session.id;
                let selected = state.selected == Some(id);
                let activity = session.last_message_at.unwrap_or(session.created_at);
                let folder = session_project_label(
                    session.project_id,
                    &state.projects,
                    &without_folder,
                );
                let generating =
                    state.generating && state.selected == Some(id);
                col = col.child(session_row(
                    ("sess", session.id.as_u128() as usize),
                    &session.title,
                    &format_relative(now, activity),
                    &folder,
                    selected,
                    generating,
                    session_row_inner_width_px,
                    cx.listener(move |this, _, window, cx| {
                        this.select_session(id, window, cx);
                        cx.notify();
                    }),
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        this.open_session_context_menu(id, event.position, window, cx);
                        cx.stop_propagation();
                    }),
                ));
            }
        }
    }

    if !earlier.is_empty() {
        col = col.child(
            collapsible_section_header(
                "section-earlier",
                catalog.get("sidebar.earlier"),
                state.earlier_expanded,
                cx.listener(|this, _, _, cx| {
                    this.earlier_expanded = !this.earlier_expanded;
                    cx.notify();
                }),
            ),
        );
        if state.earlier_expanded {
            for session in earlier {
                let id = session.id;
                let selected = state.selected == Some(id);
                let activity = session.last_message_at.unwrap_or(session.created_at);
                let folder = session_project_label(
                    session.project_id,
                    &state.projects,
                    &without_folder,
                );
                let generating =
                    state.generating && state.selected == Some(id);
                col = col.child(session_row(
                    ("sess-earlier", session.id.as_u128() as usize),
                    &session.title,
                    &format_relative(now, activity),
                    &folder,
                    selected,
                    generating,
                    session_row_inner_width_px,
                    cx.listener(move |this, _, window, cx| {
                        this.select_session(id, window, cx);
                        cx.notify();
                    }),
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        this.open_session_context_menu(id, event.position, window, cx);
                        cx.stop_propagation();
                    }),
                ));
            }
        }
    }

    col
}

fn collapsible_section_header(
    id: &'static str,
    text: &str,
    expanded: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    section_header_layout(id, text, expanded, on_toggle, false, |_, _, _| {})
}

fn collapsible_section_header_with_open_project(
    id: &'static str,
    text: &str,
    expanded: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_open_project: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    section_header_layout(id, text, expanded, on_toggle, true, on_open_project)
}

fn section_open_project_button(
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id("section-today-open-project")
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .w(px(22.))
        .h(px(22.))
        .rounded(px(4.))
        .text_color(TEXT_MUTED)
        .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(icon(icon_path::FOLDER_PLUS, px(14.), TEXT_MUTED))
}

fn section_header_layout(
    id: &'static str,
    text: &str,
    expanded: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    show_open_project: bool,
    on_open_project: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let chevron_path = if expanded {
        icon_path::CHEVRON_DOWN
    } else {
        icon_path::CHEVRON_RIGHT
    };
    let mut row = div()
        .flex()
        .items_center()
        .w_full()
        .px_2()
        .pt_2()
        .pb_1()
        .child(
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .on_click(on_toggle)
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(TEXT_MUTED)
                        .child(text.to_string()),
                )
                .child(icon(chevron_path, px(12.), TEXT_MUTED)),
        );

    if show_open_project {
        row = row
            .child(div().flex_1())
            .child(section_open_project_button(on_open_project));
    }

    row
}

fn action_row(
    id: &'static str,
    text: &str,
    icon_path: Option<&'static str>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let mut row = div()
        .id(id)
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .text_color(TEXT)
        .hover(|style| style.bg(BG_MAIN))
        .cursor_pointer()
        .on_click(on_click);
    if let Some(path) = icon_path {
        row = row.child(icon(path, px(14.), TEXT));
    }
    row.child(div().flex_1().child(text.to_string()))
}

pub(crate) fn settings_text_button(
    id: (&'static str, usize),
    text: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .text_color(TEXT)
        .hover(|style| style.bg(BG_MAIN))
        .cursor_pointer()
        .on_click(on_click)
        .child(text)
}

pub(crate) fn settings_text_button_accent(
    id: (&'static str, usize),
    text: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .text_color(ACCENT)
        .hover(|style| style.bg(BG_MAIN))
        .cursor_pointer()
        .on_click(on_click)
        .child(text)
}

fn ellipsize_session_title(title: &str, available_width_px: f32) -> String {
    let width_chars = (available_width_px / SESSION_TITLE_CHAR_WIDTH_PX).floor() as usize;
    let max_chars = width_chars
        .min(SESSION_TITLE_ELLIPSIS_AT_CHARS)
        .max(1);
    let chars: Vec<char> = title.chars().collect();
    if chars.len() <= max_chars {
        return title.to_string();
    }
    let suffix_len = SESSION_TITLE_ELLIPSIS.chars().count();
    let keep = max_chars.saturating_sub(suffix_len);
    let mut out: String = chars.into_iter().take(keep).collect();
    out.push_str(SESSION_TITLE_ELLIPSIS);
    out
}

fn session_row(
    id: (&'static str, usize),
    title: &str,
    time: &str,
    project: &str,
    selected: bool,
    generating: bool,
    inner_content_width_px: f32,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_context_menu: impl Fn(&MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let title_width = inner_content_width_px
        - if generating {
            SESSION_META_ICON_PX + 8.
        } else {
            0.
        };
    let display_title = ellipsize_session_title(title, title_width);
    let mut title_row = div()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .overflow_hidden()
                .text_size(px(SESSION_TITLE_TEXT_PX))
                .line_height(px(SESSION_TITLE_LINE_HEIGHT_PX))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT)
                .child(display_title),
        );
    if generating {
        title_row = title_row.child(icon(
            icon_path::LOADER_2,
            px(SESSION_META_ICON_PX),
            SESSION_ACTIVITY_SPINNER,
        ));
    }

    div()
        .id(id)
        .flex()
        .flex_col()
        .gap_0p5()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(selected, |el| el.bg(BG_HOVER))
        .when(!selected, |el| el.hover(|style| style.bg(BG_HOVER)))
        .on_click(on_click)
        .on_mouse_down(MouseButton::Right, on_context_menu)
        .child(title_row)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .w_full()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .min_w_0()
                        .flex_1()
                        .child(icon(
                            icon_path::FOLDER,
                            px(SESSION_META_ICON_PX),
                            TEXT_TERTIARY,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .overflow_hidden()
                                .truncate()
                                .text_size(px(SESSION_META_PROJECT_TEXT_PX))
                                .line_height(px(SESSION_META_TIME_LINE_HEIGHT_PX))
                                .text_color(TEXT_TERTIARY)
                                .child(project.to_string()),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex_shrink_0()
                        .text_size(px(SESSION_META_TIME_TEXT_PX))
                        .line_height(px(SESSION_META_TIME_LINE_HEIGHT_PX))
                        .text_color(TEXT_TERTIARY)
                        .child(time.to_string()),
                ),
        )
}

    fn main_column(
    state: &mut AppShell,
    catalog: &Catalog,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    if state.settings_open {
        return settings_main_column(state, catalog, window, cx);
    }

    let no_session = state.selected.is_none();
    let collapsed = state.sidebar_collapsed;
    let mut column = div()
        .flex()
        .flex_col()
        .flex_1()
        .h_full()
        .bg(BG_MAIN)
        .child(
            div()
                .h(px(APP_BAR_HEIGHT_PX))
                .when(collapsed, |el| el.pl(px(MAIN_HEADER_TITLE_LEFT_PX)))
                .when(!collapsed, |el| {
                    el.pl(px(MAIN_HEADER_TITLE_INSET_PX)).pr(px(16.))
                })
                .flex()
                .items_center()
                .border_b_1()
                .border_color(BORDER_SUBTLE)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .overflow_hidden()
                        .child(
                            div()
                                .truncate()
                                .text_size(px(MAIN_HEADER_TITLE_TEXT_PX))
                                .child(state.selected_title()),
                        ),
                ),
        );

    if no_session {
        column = column.child(home_panel(catalog, cx));
    } else {
        column = column
            .child(message_list(state, catalog, cx))
            .when_some(state.pending_permission.as_ref(), |column, pending| {
                column.child(
                    content_rail(
                        div()
                            .flex_none()
                            .pb(px(8.))
                            .child(permission_banner(pending, catalog, cx)),
                    ),
                )
            })
            .when_some(state.pending_question.as_ref(), |column, pending| {
                column.child(
                    content_rail(
                        div()
                            .flex_none()
                            .pb(px(8.))
                            .child(question_card(
                                pending,
                                state.question_answer_input.clone(),
                                catalog,
                                cx,
                            )),
                    ),
                )
            })
            .child(
                content_rail(
                    div()
                        .flex_none()
                        .pb(px(COMPOSER_BOTTOM_PADDING_PX))
                        .child(state.composer.clone()),
                ),
            );
    }

    column
}

fn settings_main_column(
    state: &AppShell,
    catalog: &Catalog,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
    let collapsed = state.sidebar_collapsed;
    let panel: gpui::AnyElement = match state.settings_section {
        SettingsSection::General => general_settings_panel(
            state.settings_health.as_ref(),
            state.settings_health_error.as_deref(),
            &state.available_agents,
            catalog,
            cx,
        )
        .into_any_element(),
        SettingsSection::Projects => active_projects_panel(
            &state.projects,
            state.pending_delete_project,
            state.pending_rename_project,
            &state.project_rename_input,
            catalog,
            window,
            cx,
        )
        .into_any_element(),
        SettingsSection::Archived => archived_projects_panel(&state.archived_projects, catalog, cx)
            .into_any_element(),
        SettingsSection::Providers => providers_panel(
            &state.available_agents,
            state.pending_provider_toggle,
            catalog,
            cx,
        )
        .into_any_element(),
        SettingsSection::Models => models_settings_panel(
            &state.composer_models,
            &state.enabled_model_ids,
            &state.settings_models_query,
            state.settings_models_expanded,
            &state.settings_models_focus,
            catalog,
            cx,
        )
        .into_any_element(),
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .h_full()
        .bg(BG_MAIN)
        .child(
            div()
                .h(px(APP_BAR_HEIGHT_PX))
                .when(collapsed, |el| el.pl(px(MAIN_HEADER_TITLE_LEFT_PX)))
                .when(!collapsed, |el| {
                    el.pl(px(MAIN_HEADER_TITLE_INSET_PX)).pr(px(16.))
                })
                .flex()
                .items_center()
                .border_b_1()
                .border_color(BORDER_SUBTLE)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(MAIN_HEADER_TITLE_TEXT_PX))
                        .child(catalog.get("settings.title").to_string()),
                ),
        )
        .child(panel)
}

fn message_list(
    state: &mut AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let mut inner = div().flex().flex_col().w_full().min_w_0();
    if state.selected.is_none() {
        inner = inner
            .flex_1()
            .items_center()
            .justify_center()
            .text_color(TEXT_MUTED)
            .child(catalog.get("session.none").to_string());
    } else if state.messages.is_empty() {
        inner = inner
            .flex_1()
            .items_center()
            .justify_center()
            .text_color(TEXT_MUTED)
            .child(catalog.get("session.empty").to_string());
    } else {
        for (index, message) in state.messages.iter().enumerate() {
            inner = inner.child(message_column(
                message,
                index,
                catalog,
                &state.expanded_tools,
                &state.expanded_reasoning,
                &state.expanded_activity_clusters,
                &state.collapsed_live_activity_clusters,
                cx,
            ));
        }
    }

    let list = div()
        .id("messages")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&state.scroll)
        .on_scroll_wheel(cx.listener(|this, _, _, cx| {
            this.sync_transcript_scroll_state(cx);
        }))
        .py_2()
        .pb(px(8.))
        .child(content_rail(inner));

    wrap_message_list(list, state.show_jump_to_latest(), catalog, cx)
}

fn jump_to_latest_button(cx: &mut Context<AppShell>) -> impl IntoElement {
    div()
        .id("jump-latest")
        .flex()
        .items_center()
        .justify_center()
        .w(px(28.))
        .h(px(28.))
        .rounded_full()
        .bg(BG_SIDEBAR)
        .border_1()
        .border_color(BORDER)
        .shadow_lg()
        .cursor_pointer()
        .hover(|style| style.bg(BG_HOVER))
        .on_click(cx.listener(|this, _, _, cx| {
            this.scroll_transcript_to_bottom(cx);
        }))
        .child(icon(icon_path::CHEVRON_DOWN, px(14.), TEXT_MUTED))
}

fn wrap_message_list(
    list: impl IntoElement,
    show_jump: bool,
    _catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    div()
        .relative()
        .flex()
        .flex_1()
        .min_h_0()
        .child(list)
        .when(show_jump, |wrapper| {
            wrapper.child(
                div()
                    .absolute()
                    .bottom(px(JUMP_TO_LATEST_ABOVE_COMPOSER_PX))
                    .left(px(0.))
                    .right(px(0.))
                    .flex()
                    .justify_center()
                    .child(jump_to_latest_button(cx)),
            )
        })
}

fn message_column(
    message: &Message,
    index: usize,
    catalog: &Catalog,
    expanded_tools: &HashSet<String>,
    expanded_reasoning: &HashSet<String>,
    expanded_activity_clusters: &HashSet<String>,
    collapsed_live_activity_clusters: &HashSet<String>,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let is_user = message.role == MessageRole::User;
    let show_thinking = assistant_is_thinking(message);
    let name = match message.role {
        MessageRole::User => catalog.get("message.user"),
        MessageRole::Assistant => catalog.get("message.assistant"),
        MessageRole::System => catalog.get("message.system"),
    };
    let initial = avatar_initial(name);

    let mut content = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .when(is_user, |el| el.items_end());

    if is_user {
        for (part_index, part) in message.parts.iter().enumerate() {
            if let MessagePart::Text { content: text } = part {
                content = content.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .overflow_hidden()
                        .text_align(gpui::TextAlign::Right)
                        .child(render_text(text)),
                );
            } else {
                content = content.child(unsupported(catalog, index, part_index));
            }
        }
    } else {
        let segments = message_segments(&message.parts);
        for (segment_index, segment) in segments.iter().enumerate() {
            match segment {
                MessageSegment::Text { part_index } => {
                    if let MessagePart::Text { content: text } = &message.parts[*part_index] {
                        content = content.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .overflow_hidden()
                                .child(render_text(text)),
                        );
                    }
                }
                MessageSegment::Activity { part_indices } => {
                    let live = message.is_streaming && segment_index + 1 == segments.len();
                    content = content.child(activity_cluster(
                        message.id,
                        index,
                        segment_index,
                        part_indices,
                        &message.parts,
                        live,
                        message.is_streaming,
                        catalog,
                        expanded_activity_clusters,
                        collapsed_live_activity_clusters,
                        expanded_tools,
                        expanded_reasoning,
                        cx,
                    ));
                }
            }
        }
    }

    if show_thinking {
        content = content.child(thinking_label(catalog, message.id));
    }

    div()
        .id(("msg", message.id.as_u128() as u64))
        .w_full()
        .min_w_0()
        .py_3()
        .flex()
        .flex_col()
        .gap_2()
        .when(is_user, |el| el.items_end())
        .child(
            div()
                .flex()
                .gap_3()
                .items_center()
                .when(is_user, |el| el.flex_row_reverse())
                .child(message_avatar(&initial, is_user, index))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .child(name.to_string()),
                ),
        )
        .child(content)
}

fn message_avatar(initial: &str, is_user: bool, index: usize) -> impl IntoElement {
    div()
        .id(("avatar", index))
        .flex_none()
        .w(px(MESSAGE_AVATAR_PX))
        .h(px(MESSAGE_AVATAR_PX))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(TEXT)
        .bg(if is_user { ACCENT } else { BG_SIDEBAR })
        .border_1()
        .border_color(BORDER)
        .child(initial.to_string())
}

fn avatar_initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|ch| ch.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

fn project_name_from_picked_path(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Project")
        .to_string()
}

fn muted(text: &str) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .text_sm()
        .text_color(TEXT_MUTED)
        .child(text.to_string())
}

fn is_transcript_anchored(max_offset_height: f32, offset_y: f32, threshold: f32) -> bool {
    max_offset_height <= 0. || (max_offset_height + offset_y) <= threshold
}

fn should_show_jump_to_latest(
    has_session: bool,
    message_count: usize,
    max_offset_height: f32,
    offset_y: f32,
    threshold: f32,
) -> bool {
    has_session
        && message_count > 0
        && max_offset_height > 0.
        && !is_transcript_anchored(max_offset_height, offset_y, threshold)
}

#[cfg(test)]
mod session_title_tests {
    use super::{ellipsize_session_title, SESSION_TITLE_CHAR_WIDTH_PX, SESSION_TITLE_ELLIPSIS_AT_CHARS};

    #[test]
    fn short_titles_are_unchanged() {
        let title = "Short title";
        assert_eq!(
            ellipsize_session_title(title, 200.),
            title
        );
    }

    #[test]
    fn long_titles_end_with_ellipsis() {
        let title = "Q3 Campaign Draft from the publisher team and more copy here";
        let out = ellipsize_session_title(title, 400.);
        assert!(out.ends_with("..."));
        assert!(out.chars().count() <= SESSION_TITLE_ELLIPSIS_AT_CHARS);
        assert!(out.chars().count() > 20);
    }

    #[test]
    fn narrow_width_reduces_visible_characters() {
        let title = "abcdefghijklmnopqrs";
        let out = ellipsize_session_title(title, SESSION_TITLE_CHAR_WIDTH_PX * 8.);
        assert!(out.ends_with("..."));
        assert!(out.chars().count() < title.chars().count());
    }
}

#[cfg(test)]
mod transcript_scroll_tests {
    use super::{is_transcript_anchored, should_show_jump_to_latest, ANCHOR_THRESHOLD};

    #[test]
    fn anchored_when_at_bottom() {
        assert!(is_transcript_anchored(400., -400., ANCHOR_THRESHOLD));
        assert!(is_transcript_anchored(400., -320., ANCHOR_THRESHOLD));
    }

    #[test]
    fn not_anchored_when_scrolled_up() {
        assert!(!is_transcript_anchored(400., -100., ANCHOR_THRESHOLD));
    }

    #[test]
    fn jump_button_when_scrolled_up_with_messages() {
        assert!(should_show_jump_to_latest(true, 3, 400., -100., ANCHOR_THRESHOLD));
    }

    #[test]
    fn jump_button_hidden_at_bottom_or_empty() {
        assert!(!should_show_jump_to_latest(true, 3, 400., -400., ANCHOR_THRESHOLD));
        assert!(!should_show_jump_to_latest(true, 0, 400., -100., ANCHOR_THRESHOLD));
        assert!(!should_show_jump_to_latest(false, 3, 400., -100., ANCHOR_THRESHOLD));
    }
}
