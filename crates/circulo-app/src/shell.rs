use std::collections::HashSet;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

use circulo_core::{Message, MessagePart, MessageRole, Project, Session, SidebarView, Uuid};
use circulo_i18n::Catalog;
use circulo_protocol::ProtocolEvent;
use gpui::{
    div, prelude::FluentBuilder, px, AppContext, Context, Entity, FontWeight, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, Subscription, Window,
};
use time::OffsetDateTime;

use crate::client::{
    ensure_daemon, filter_sessions, groups_need_new_project, resolve_view, session_project_label,
    DaemonClient,
};
use crate::composer::{can_send, project_picker_locked, Composer, ComposerEvent};
use crate::parts::{render_text, task_list, tool_card, unsupported};
use crate::stream::{
    apply_protocol_event, resubscribe_delay, should_apply_post_transcript,
    should_apply_refresh_transcript, stream_attempts_after_event,
};
use crate::theme::{
    sidebar_width_px, ACCENT, BG_APP, BG_MAIN, BG_SIDEBAR, BORDER, COMPOSER_GUTTER_PX,
    CONTENT_MAX_WIDTH_PX, MESSAGE_AVATAR_PX, TEXT, TEXT_MUTED,
};
use crate::timefmt::format_relative;

/// How often the drain loop applies buffered stream events; doubles as render
/// batching for incoming deltas.
const DRAIN_INTERVAL: Duration = Duration::from_millis(50);
/// Distance from the bottom (px) within which the transcript keeps following
/// new content.
const ANCHOR_THRESHOLD: f32 = 80.0;

pub struct AppShell {
    pub sidebar_collapsed: bool,
    catalog: Catalog,
    client: DaemonClient,
    view: SidebarView,
    sessions: Vec<Session>,
    projects: Vec<Project>,
    messages: Vec<Message>,
    selected: Option<Uuid>,
    search_query: String,
    search_focused: bool,
    composer: Entity<Composer>,
    generating: bool,
    pub expanded_tools: HashSet<String>,
    error: Option<String>,
    loaded: bool,
    scroll: ScrollHandle,
    search_focus: gpui::FocusHandle,
    stream_gen: u64,
    stream_session: Option<Uuid>,
    stream_attempts: u32,
    saw_stream_event: bool,
    _composer_subscription: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| Composer::new(window, cx));
        let composer_subscription = {
            let composer_entity = composer.clone();
            cx.subscribe(&composer_entity, |shell, _, event, cx| {
                shell.on_composer_event(event, cx);
            })
        };
        let mut shell = Self {
            sidebar_collapsed: false,
            catalog: Catalog::english(),
            client: DaemonClient::default(),
            view: SidebarView::Sessions,
            sessions: Vec::new(),
            projects: Vec::new(),
            messages: Vec::new(),
            selected: None,
            search_query: String::new(),
            search_focused: false,
            composer,
            generating: false,
            expanded_tools: HashSet::new(),
            error: None,
            loaded: false,
            scroll: ScrollHandle::new(),
            search_focus: cx.focus_handle(),
            stream_gen: 0,
            stream_attempts: 0,
            stream_session: None,
            saw_stream_event: false,
            _composer_subscription: composer_subscription,
        };
        shell.schedule_refresh(cx);
        shell.sync_composer(cx);
        shell
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
                            let view = client.preferences().ok().map(|prefs| prefs.sidebar_view);
                            let sessions = client.list_sessions().unwrap_or_default();
                            let projects = client.list_projects().unwrap_or_default();
                            let messages = selected
                                .and_then(|id| client.list_messages(id).ok())
                                .unwrap_or_default();
                            Ok((resolve_view(view), sessions, projects, messages))
                        }
                        Err(err) => Err(format!("{daemon_down} ({err})")),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.loaded = true;
                match snapshot {
                    Ok((view, sessions, projects, messages)) => {
                        this.view = view;
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
        self.search_focused = false;
        self.messages = self.client.list_messages(id).unwrap_or_default();
        self.stream_attempts = 0;
        self.error = None;
        self.sync_composer(cx);
        self.composer
            .update(cx, |composer, cx| composer.focus_after_session_select(window, cx));
        self.subscribe_stream(cx);
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
                    let sessions = client.list_sessions().unwrap_or_default();
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
        sidebar_width_px(self.sidebar_collapsed)
    }

    fn selected_title(&self) -> String {
        self.selected_session()
            .map(|session| session.title.clone())
            .unwrap_or_else(|| self.catalog.get("session.none").to_string())
    }

    fn refresh(&mut self) {
        match ensure_daemon(&self.client) {
            Ok(()) => self.error = None,
            Err(_) => {
                self.error = Some(self.catalog.get("sidebar.daemon_down").to_string());
                return;
            }
        }
        match self.client.preferences() {
            Ok(prefs) => self.view = resolve_view(Some(prefs.sidebar_view)),
            Err(_) => self.view = SidebarView::Sessions,
        }
        self.sessions = self.client.list_sessions().unwrap_or_default();
        self.projects = self.client.list_projects().unwrap_or_default();
        if let Some(id) = self.selected {
            self.messages = self.client.list_messages(id).unwrap_or_default();
        } else {
            self.messages.clear();
        }
    }

    fn persist_view(&mut self, view: SidebarView) {
        self.view = view;
        let _ = self.client.set_view(view);
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        let width = self.sidebar_width();
        let catalog = self.catalog.clone();
        let toggle_label = catalog.get(if collapsed {
            "sidebar.show"
        } else {
            "sidebar.hide"
        });

        div()
            .flex()
            .size_full()
            .bg(BG_APP)
            .text_color(TEXT)
            .child(sidebar(self, collapsed, width, toggle_label, &catalog, cx))
            .child(main_column(self, &catalog, cx))
    }
}

fn handle_search_key(this: &mut AppShell, event: &KeyDownEvent, cx: &mut Context<AppShell>) {
    let key = event.keystroke.key.as_str();
    if key == "backspace" {
        this.search_query.pop();
    } else if key == "escape" {
        this.search_query.clear();
        this.search_focused = false;
    } else if let Some(ch) = typed_char(event) {
        this.search_query.push_str(&ch);
    } else {
        return;
    }
    cx.stop_propagation();
    cx.notify();
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

fn sidebar(
    state: &AppShell,
    collapsed: bool,
    width: f32,
    toggle_label: &str,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let top = div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(52.))
        .pl(px(80.))
        .pr_2()
        .child(
            div()
                .id("toggle-sidebar")
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .text_color(TEXT_MUTED)
                .hover(|style| style.bg(BG_MAIN).text_color(TEXT))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_collapsed = !this.sidebar_collapsed;
                    cx.notify();
                }))
                .child(toggle_label.to_string()),
        );

    let body = if collapsed {
        div().flex_1().into_any_element()
    } else {
        sidebar_body(state, catalog, cx).into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .w(px(width))
        .h_full()
        .bg(BG_SIDEBAR)
        .border_r_1()
        .border_color(BORDER)
        .child(top)
        .child(body)
}

fn sidebar_body(state: &AppShell, catalog: &Catalog, cx: &mut Context<AppShell>) -> gpui::Div {
    let sessions_active = state.view == SidebarView::Sessions;
    let mut col = div()
        .flex()
        .flex_col()
        .flex_1()
        .px_3()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_1()
                .child(view_tab(
                    "tab-sessions",
                    catalog.get("sidebar.sessions"),
                    sessions_active,
                    cx.listener(|this, _, _, cx| {
                        this.persist_view(SidebarView::Sessions);
                        cx.notify();
                    }),
                ))
                .child(view_tab(
                    "tab-groups",
                    catalog.get("sidebar.groups"),
                    !sessions_active,
                    cx.listener(|this, _, _, cx| {
                        this.persist_view(SidebarView::Groups);
                        cx.notify();
                    }),
                )),
        )
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
        .child({
            let search_focus = state.search_focus.clone();
            div()
                .id("search")
                .track_focus(&search_focus)
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .when(state.search_focused, |el| {
                    el.border_1().border_color(ACCENT)
                })
                .text_color(if state.search_query.is_empty() {
                    TEXT_MUTED
                } else {
                    TEXT
                })
                .cursor_pointer()
                .on_key_down(cx.listener(|this, event, window, cx| {
                    if this.search_focus.is_focused(window) {
                        handle_search_key(this, event, cx);
                    }
                }))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.search_focused = true;
                    this.search_focus.focus(window);
                    cx.notify();
                }))
                .child(if state.search_query.is_empty() {
                    catalog.get("sidebar.search").to_string()
                } else {
                    state.search_query.clone()
                })
        });

    if let Some(error) = &state.error {
        col = col.child(
            div()
                .px_2()
                .py_2()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(error.clone()),
        );
        return col
            .child(div().flex_1())
            .child(label(catalog.get("sidebar.settings"), false));
    }

    if sessions_active {
        let visible = filter_sessions(&state.sessions, &state.search_query);
        if visible.is_empty() {
            col = col.child(muted(catalog.get("sidebar.empty_sessions")));
        }
        let now = OffsetDateTime::now_utc();
        let no_project = catalog.get("session.no_project").to_string();
        for session in visible {
            let id = session.id;
            let selected = state.selected == Some(id);
            let then = session.last_message_at.unwrap_or(session.created_at);
            let project = session_project_label(session.project_id, &state.projects, &no_project);
            col = col.child(session_row(
                ("sess", session.id.as_u128() as usize),
                &session.title,
                &format_relative(now, then),
                &project,
                selected,
                cx.listener(move |this, _, window, cx| {
                    this.select_session(id, window, cx);
                    cx.notify();
                }),
            ));
        }
    } else if groups_need_new_project(&state.projects) {
        col = col
            .child(muted(catalog.get("sidebar.empty_groups")))
            .child(action_row(
                "action-new-project",
                catalog.get("sidebar.new_project"),
                cx.listener(|this, _, _, cx| {
                    let _ = this.client.create_project("New project");
                    this.refresh();
                    cx.notify();
                }),
            ));
    } else {
        let now = OffsetDateTime::now_utc();
        for project in &state.projects {
            col = col.child(label(&project.name, false));
            for session in state
                .sessions
                .iter()
                .filter(|session| session.project_id == Some(project.id))
            {
                let id = session.id;
                let selected = state.selected == Some(id);
                let then = session.last_message_at.unwrap_or(session.created_at);
                col = col.child(session_row(
                    ("sess", session.id.as_u128() as usize),
                    &session.title,
                    &format_relative(now, then),
                    &project.name,
                    selected,
                    cx.listener(move |this, _, window, cx| {
                        this.select_session(id, window, cx);
                        cx.notify();
                    }),
                ));
            }
        }
    }

    col.child(div().flex_1())
        .child(label(catalog.get("sidebar.settings"), false))
}

fn view_tab(
    id: &'static str,
    text: &str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(active, |el| el.bg(ACCENT).text_color(TEXT))
        .when(!active, |el| el.text_color(TEXT_MUTED))
        .on_click(on_click)
        .child(text.to_string())
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
        .child(div().text_sm().child(title.to_string()))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
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
    div()
        .flex()
        .flex_col()
        .flex_1()
        .h_full()
        .bg(BG_MAIN)
        .child(
            div()
                .h(px(52.))
                .px_4()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(BORDER)
                .child(state.selected_title()),
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
