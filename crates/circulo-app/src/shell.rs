use std::collections::HashSet;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

use circulo_core::{Message, MessagePart, MessageRole, Project, Session, Uuid};
use circulo_i18n::Catalog;
use circulo_protocol::ProtocolEvent;
use gpui::{
    div, linear_color_stop, linear_gradient, prelude::FluentBuilder, px, AppContext, Context,
    CursorStyle, DragMoveEvent, Entity, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, Subscription, Window,
};

use crate::command_palette::{palette_catalog, PaletteItemKind, OpenPalette};
use time::OffsetDateTime;

use crate::client::{
    ensure_daemon, partition_sessions_by_day, session_project_label, DaemonClient,
};
use crate::composer::{can_send, project_picker_locked, Composer, ComposerEvent};
use crate::icons::{icon, path as icon_path};
use crate::parts::{render_text, task_list, tool_card, unsupported};
use crate::session_overlay::{session_overlay, SessionOverlay};
use crate::stream::{
    apply_protocol_event, resubscribe_delay, should_apply_post_transcript,
    should_apply_refresh_transcript, stream_attempts_after_event,
};
use crate::ui::{TextInput, TextInputEvent};
use crate::theme::{
    sidebar_width_px, ACCENT, ACCENT_SURFACE, BG_APP, BG_MAIN, BG_SIDEBAR, BORDER,
    COMPOSER_GUTTER_PX, CONTENT_MAX_WIDTH_PX, MESSAGE_AVATAR_PX, APP_BAR_HEIGHT_PX,
    MAIN_HEADER_TITLE_INSET_PX, MAIN_HEADER_TITLE_LEFT_PX, MAIN_HEADER_TITLE_TEXT_PX,
    SIDEBAR_EXPANDED_PX, SIDEBAR_MAX_PX, SIDEBAR_MIN_PX, SIDEBAR_RESIZE_HANDLE_CENTER,
    SIDEBAR_RESIZE_HANDLE_CENTER_ACTIVE, SIDEBAR_RESIZE_HANDLE_HIT_PX,
    SIDEBAR_RESIZE_HANDLE_VISUAL_PX, SIDEBAR_TOGGLE_LEFT_PX, SIDEBAR_TOGGLE_SIZE_PX,
    SIDEBAR_TOGGLE_TOP_PX, TEXT, TEXT_MUTED,
};
use crate::command_palette::PaletteItem;
use crate::timefmt::{format_relative, local_offset_or_utc};

/// How often the drain loop applies buffered stream events; doubles as render
/// batching for incoming deltas.
const DRAIN_INTERVAL: Duration = Duration::from_millis(50);
/// Distance from the bottom (px) within which the transcript keeps following
/// new content.
const ANCHOR_THRESHOLD: f32 = 80.0;
/// Meta line on session cards (folder + duration).
const SESSION_META_TEXT_PX: f32 = 10.5;
const PALETTE_BACKDROP: gpui::Rgba = gpui::Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.45,
};

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
    composer: Entity<Composer>,
    generating: bool,
    pub expanded_tools: HashSet<String>,
    error: Option<String>,
    loaded: bool,
    scroll: ScrollHandle,
    sidebar_scroll: ScrollHandle,
    palette_focus: gpui::FocusHandle,
    session_overlay: Option<SessionOverlay>,
    session_menu_focus: gpui::FocusHandle,
    pub(crate) session_menu_selected: usize,
    rename_input: Entity<TextInput>,
    stream_gen: u64,
    stream_session: Option<Uuid>,
    stream_attempts: u32,
    saw_stream_event: bool,
    _composer_subscription: Subscription,
    _rename_input_subscription: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| Composer::new(window, cx));
        let rename_input = cx.new(|cx| TextInput::new(window, cx));
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
            composer,
            generating: false,
            expanded_tools: HashSet::new(),
            error: None,
            loaded: false,
            scroll: ScrollHandle::new(),
            sidebar_scroll: ScrollHandle::new(),
            palette_focus: cx.focus_handle(),
            session_overlay: None,
            session_menu_focus: cx.focus_handle(),
            session_menu_selected: 0,
            rename_input,
            stream_gen: 0,
            stream_attempts: 0,
            stream_session: None,
            saw_stream_event: false,
            _composer_subscription: composer_subscription,
            _rename_input_subscription: rename_input_subscription,
        };
        shell.schedule_refresh(cx);
        shell.sync_composer(cx);
        shell
    }

    fn end_sidebar_resize(&mut self) {
        self.sidebar_resize_origin = None;
        self.sidebar_resize_hovered = false;
    }

    fn sync_composer(&mut self, cx: &mut Context<Self>) {
        let session = self.selected_session().cloned();
        let projects = self.projects.clone();
        let catalog = self.catalog.clone();
        self.composer.update(cx, |composer, cx| {
            composer.set_render_context(projects, session, catalog, cx);
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
            ComposerEvent::ProjectPicked(project_id) => {
                self.patch_session_project(Some(*project_id), cx);
            }
            ComposerEvent::ProjectCleared => {
                self.patch_session_project(None, cx);
            }
        }
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
}

impl AppShell {
    pub fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let daemon_down = self.catalog.get("sidebar.daemon_down").to_string();
        let selected = self.selected;
        let snapshot_gen = self.stream_gen;
        cx.spawn(async move |this, cx| {
            let snapshot = cx
                .background_executor()
                .spawn(async move {
                    let connect = ensure_daemon(&client);
                    match connect {
                        Ok(()) => {
                            let sessions = client.list_sessions().unwrap_or_default();
                            let projects = client.list_projects().unwrap_or_default();
                            let messages = selected
                                .and_then(|id| client.list_messages(id).ok())
                                .unwrap_or_default();
                            Ok((sessions, projects, messages))
                        }
                        Err(err) => Err(format!("{daemon_down} ({err})")),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.loaded = true;
                match snapshot {
                    Ok((sessions, projects, messages)) => {
                        this.sessions = sessions;
                        this.projects = projects;
                        if should_apply_refresh_transcript(
                            this.selected == selected,
                            snapshot_gen,
                            this.stream_gen,
                        ) {
                            this.messages = messages;
                        }
                        // Keep `messages.stream_dropped` if this session gave up
                        // on live updates; a late refetch must not hide it.
                        if this.stream_session.is_some() || this.selected.is_none() {
                            this.error = None;
                        }
                    }
                    Err(message) => this.error = Some(message),
                }
                if let Some(id) = this.selected {
                    if this.stream_session != Some(id) {
                        this.subscribe_stream(cx);
                    }
                }
                this.sync_composer(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn selected_session(&self) -> Option<&Session> {
        self.selected
            .and_then(|id| self.sessions.iter().find(|session| session.id == id))
    }

    /// True when the transcript is (roughly) at the bottom and should keep
    /// following new content. Offsets grow negative while scrolling down.
    fn anchored(&self) -> bool {
        let max = self.scroll.max_offset().height;
        max <= px(0.) || (max + self.scroll.offset().y) <= px(ANCHOR_THRESHOLD)
    }

    fn any_streaming(&self) -> bool {
        self.messages.iter().any(|message| message.is_streaming)
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
                match rx.try_recv() {
                    Ok(event) => {
                        let terminal = matches!(
                            event,
                            ProtocolEvent::SessionMessageCompleted { .. }
                                | ProtocolEvent::SessionMessageFailed { .. }
                        );
                        let _ = this.update(cx, |this, cx| {
                            if this.stream_gen != gen {
                                return;
                            }
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
                            let changed = apply_protocol_event(&mut this.messages, &event);
                            if terminal {
                                this.generating = false;
                                this.composer.update(cx, |composer, cx| {
                                    composer.set_generating(false, cx);
                                });
                            }
                            if changed && this.anchored() {
                                this.scroll.scroll_to_bottom();
                            }
                            cx.notify();
                        });
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        let _ = this.update(cx, |this, cx| {
                            if this.stream_gen != gen {
                                return;
                            }
                            this.recover_stream(session_id, gen, cx);
                        });
                        return;
                    }
                }
            }
            let stale = this
                .update(cx, |this, _| this.stream_gen != gen)
                .unwrap_or(true);
            if stale {
                return;
            }
            cx.background_executor().timer(DRAIN_INTERVAL).await;
        })
        .detach();
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
        self.selected = Some(id);
        self.close_palette(cx);
        self.close_session_overlay(cx);
        self.messages = self.client.list_messages(id).unwrap_or_default();
        self.stream_attempts = 0;
        self.error = None;
        self.sync_composer(cx);
        self.composer
            .update(cx, |composer, cx| composer.focus_after_session_select(window, cx));
        self.subscribe_stream(cx);
    }

    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_session_overlay(cx);
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

    pub(crate) fn try_send(&mut self, content: String, cx: &mut Context<Self>) {
        if !can_send(self.selected.is_some(), &content, self.generating) {
            return;
        }
        let Some(session_id) = self.selected else {
            return;
        };
        let locked = project_picker_locked(self.selected_session());
        let current_project = self
            .selected_session()
            .and_then(|session| session.project_id);
        let draft_project = self.composer.read(cx).draft_project();
        let should_patch = !locked && draft_project != current_project;
        let client = self.client.clone();
        let submitted = content.clone();

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
                    if should_patch {
                        client.set_session_project(session_id, draft_project)?;
                    }
                    client.post_message(session_id, content.trim())?;
                    let messages = client.list_messages(session_id)?;
                    let sessions = client.list_sessions()?;
                    Ok::<_, String>((messages, sessions))
                })
                .await;
            this.update(cx, |this, cx| {
                match result {
                    Ok((messages, sessions)) => {
                        this.sessions = sessions;
                        if this.selected == Some(session_id)
                            && should_apply_post_transcript(
                                this.saw_stream_event,
                                &this.messages,
                                &messages,
                            )
                        {
                            this.messages = messages;
                        }
                        this.generating = false;
                        this.saw_stream_event = false;
                        this.composer.update(cx, |composer, cx| {
                            composer.set_generating(false, cx);
                        });
                        this.sync_composer(cx);
                        this.error = None;
                    }
                    Err(message) => {
                        this.generating = false;
                        this.composer.update(cx, |composer, cx| {
                            composer.restore_content(submitted, cx);
                            composer.set_generating(false, cx);
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
        } else {
            self.messages.clear();
        }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(main_column(self, &catalog, cx)),
            )
            .child(sidebar_toggle(self.sidebar_collapsed, cx))
            .when(!collapsed, |el| {
                el.child(sidebar_resize_handle(
                    self.sidebar_width_expanded,
                    self.sidebar_resize_origin.is_some(),
                    self.sidebar_resize_hovered,
                    cx,
                ))
            })
            .when(self.palette_open, |el| {
                el.child(command_palette_overlay(self, &catalog, cx))
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
        .h_full()
        .bg(BG_SIDEBAR)
        .border_r_1()
        .border_color(BORDER)
        .child(div().flex_none().h(px(APP_BAR_HEIGHT_PX)))
        .child(sidebar_body(state, catalog, cx))
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
            this.sidebar_resize_hovered = *active;
            cx.notify();
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

fn sidebar_body(state: &AppShell, catalog: &Catalog, cx: &mut Context<AppShell>) -> gpui::Div {
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
                .text_color(TEXT_MUTED)
                .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_palette(window, cx);
                }))
                .child(
                    div()
                        .text_xs()
                        .text_color(TEXT_MUTED)
                        .child(icon(icon_path::SEARCH, px(14.), TEXT_MUTED)),
                )
                .child(catalog.get("sidebar.search").to_string()),
        );

    let mut scroll_content = div().flex().flex_col().gap_1().pb_2();

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
                .overflow_y_scroll()
                .track_scroll(&state.sidebar_scroll)
                .px_3()
                .child(scroll_content),
        )
        .child(
            div()
                .flex_none()
                .px_3()
                .pb_2()
                .child(label(catalog.get("sidebar.settings"), false)),
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
    let mut col = div().flex().flex_col().gap_1();

    if !today.is_empty() {
        col = col.child(
            collapsible_section_header(
                "section-today",
                catalog.get("sidebar.today"),
                state.today_expanded,
                cx.listener(|this, _, _, cx| {
                    this.today_expanded = !this.today_expanded;
                    cx.notify();
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
                col = col.child(session_row(
                    ("sess", session.id.as_u128() as usize),
                    &session.title,
                    &format_relative(now, activity),
                    &folder,
                    selected,
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
                col = col.child(session_row(
                    ("sess-earlier", session.id.as_u128() as usize),
                    &session.title,
                    &format_relative(now, activity),
                    &folder,
                    selected,
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
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let chevron_path = if expanded {
        icon_path::CHEVRON_DOWN
    } else {
        icon_path::CHEVRON_RIGHT
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .pt_2()
        .pb_1()
        .cursor_pointer()
        .on_click(on_click)
        .child(icon(chevron_path, px(12.), TEXT_MUTED))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_MUTED)
                .child(text.to_string()),
        )
}

fn action_row(
    id: &'static str,
    text: &str,
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
        .child(text.to_string())
}

fn session_row(
    id: (&'static str, usize),
    title: &str,
    time: &str,
    project: &str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_context_menu: impl Fn(&MouseDownEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_col()
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(selected, |el| el.bg(BG_MAIN))
        .on_click(on_click)
        .on_mouse_down(MouseButton::Right, on_context_menu)
        .child(div().text_sm().child(title.to_string()))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(SESSION_META_TEXT_PX))
                .text_color(TEXT_MUTED)
                .child(project.to_string())
                .child(time.to_string()),
        )
}

fn main_column(
    state: &mut AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    state.sync_composer(cx);
    let collapsed = state.sidebar_collapsed;
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
                .border_color(BORDER)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(MAIN_HEADER_TITLE_TEXT_PX))
                        .child(state.selected_title()),
                ),
        )
        .child(message_list(state, catalog, cx))
        .child(
            div()
                .flex_none()
                .px(px(COMPOSER_GUTTER_PX))
                .pb(px(16.))
                .child(state.composer.clone()),
        )
}

fn message_list(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let mut inner = div().flex().flex_col().w_full();
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
                cx,
            ));
        }
    }

    let list = div()
        .id("messages")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll()
        .track_scroll(&state.scroll)
        .py_2()
        .pb(px(8.))
        .child(transcript_column(inner));

    wrap_message_list(
        list,
        state.any_streaming() && !state.anchored(),
        catalog,
        cx,
    )
}

fn transcript_column(child: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .max_w(px(CONTENT_MAX_WIDTH_PX))
        .mx_auto()
        .px(px(COMPOSER_GUTTER_PX))
        .flex()
        .flex_col()
        .child(child)
}

/// Wraps the scrollable transcript with the floating jump-to-latest affordance
/// shown while content streams and the user is not anchored at the bottom.
fn wrap_message_list(
    list: impl IntoElement,
    show_jump: bool,
    catalog: &Catalog,
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
                    .id("jump-latest")
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(BG_SIDEBAR)
                    .border_1()
                    .border_color(BORDER)
                    .text_xs()
                    .text_color(TEXT)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.scroll.scroll_to_bottom();
                        cx.notify();
                    }))
                    .child(catalog.get("messages.jump_to_latest").to_string()),
            )
        })
}

fn message_column(
    message: &Message,
    index: usize,
    catalog: &Catalog,
    expanded: &HashSet<String>,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let is_user = message.role == MessageRole::User;
    let name = match message.role {
        MessageRole::User => catalog.get("message.user"),
        MessageRole::Assistant => catalog.get("message.assistant"),
        MessageRole::System => catalog.get("message.system"),
    };
    let initial = avatar_initial(name);

    let mut body = div()
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .when(is_user, |el| el.items_end())
        .when(!is_user, |el| el.items_start().flex_1())
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(TEXT)
                .when(is_user, |el| el.text_align(gpui::TextAlign::Right))
                .child(name.to_string()),
        );

    for (part_index, part) in message.parts.iter().enumerate() {
        let part_element = match part {
            MessagePart::Text { content } => {
                if is_user {
                    div()
                        .w_full()
                        .text_align(gpui::TextAlign::Right)
                        .child(render_text(content))
                        .into_any_element()
                } else {
                    render_text(content).into_any_element()
                }
            }
            MessagePart::TaskList { tasks } => task_list(tasks, catalog).into_any_element(),
            MessagePart::Question { .. } => unsupported(catalog, index, part_index),
            MessagePart::ToolCall { tool_call } => {
                let id = tool_call.id.clone();
                tool_card(
                    tool_call,
                    catalog,
                    expanded,
                    cx.listener(move |this, _, _, cx| {
                        if !this.expanded_tools.remove(&id) {
                            this.expanded_tools.insert(id.clone());
                        }
                        cx.notify();
                    }),
                )
                .into_any_element()
            }
        };
        body = body.child(part_element);
    }

    div()
        .id(("msg", index))
        .w_full()
        .flex()
        .py_3()
        .when(is_user, |el| el.justify_end())
        .when(!is_user, |el| el.justify_start())
        .child(
            div()
                .flex()
                .gap_3()
                .items_start()
                .when(is_user, |el| el.flex_row_reverse().max_w(px(480.)))
                .when(!is_user, |el| el.w_full().min_w_0())
                .child(message_avatar(&initial, is_user, index))
                .child(body),
        )
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

fn label(text: &str, active: bool) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .when(active, |el| el.bg(ACCENT).text_color(TEXT))
        .when(!active, |el| el.text_color(TEXT_MUTED))
        .child(text.to_string())
}

fn muted(text: &str) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .text_sm()
        .text_color(TEXT_MUTED)
        .child(text.to_string())
}
