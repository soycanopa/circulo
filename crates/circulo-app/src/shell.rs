use circulo_core::{Message, MessageRole, Project, Session, SidebarView, Uuid};
use circulo_i18n::Catalog;
use gpui::{
    Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use time::OffsetDateTime;

use crate::client::{
    DaemonClient, ensure_daemon, filter_sessions, groups_need_new_project, resolve_view,
    session_project_label,
};
use crate::composer::{can_send, project_picker_locked, summarize_message};
use crate::theme::{
    ACCENT, BG_APP, BG_MAIN, BG_SIDEBAR, BORDER, TEXT, TEXT_MUTED, sidebar_width_px,
};
use crate::timefmt::format_relative;

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
    composer_focused: bool,
    draft: String,
    draft_project: Option<Uuid>,
    picker_open: bool,
    generating: bool,
    error: Option<String>,
    loaded: bool,
}

impl Default for AppShell {
    fn default() -> Self {
        Self {
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
            composer_focused: false,
            draft: String::new(),
            draft_project: None,
            picker_open: false,
            generating: false,
            error: None,
            loaded: false,
        }
    }
}

impl AppShell {
    pub fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let daemon_down = self.catalog.get("sidebar.daemon_down").to_string();
        let selected = self.selected;
        cx.spawn(async move |this, cx| {
            let snapshot = cx
                .background_executor()
                .spawn(async move {
                    let connect = ensure_daemon(&client);
                    match connect {
                        Ok(()) => {
                            let view = client
                                .preferences()
                                .ok()
                                .map(|prefs| prefs.sidebar_view);
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
                        if this.selected == selected {
                            this.messages = messages;
                        }
                        this.error = None;
                    }
                    Err(message) => this.error = Some(message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn selected_session(&self) -> Option<&Session> {
        self.selected.and_then(|id| {
            self.sessions.iter().find(|session| session.id == id)
        })
    }

    fn select_session(&mut self, id: Uuid) {
        self.selected = Some(id);
        self.search_focused = false;
        self.composer_focused = true;
        self.picker_open = false;
        if let Some(session) = self.sessions.iter().find(|session| session.id == id) {
            self.draft_project = session.project_id;
        }
        self.messages = self.client.list_messages(id).unwrap_or_default();
    }

    fn try_send(&mut self, cx: &mut Context<Self>) {
        if !can_send(self.selected.is_some(), &self.draft, self.generating) {
            return;
        }
        let Some(session_id) = self.selected else {
            return;
        };
        let content = self.draft.clone();
        let locked = project_picker_locked(self.selected_session());
        let current_project = self.selected_session().and_then(|session| session.project_id);
        let draft_project = self.draft_project;
        let should_patch = !locked && draft_project != current_project;
        let client = self.client.clone();

        self.generating = true;
        self.picker_open = false;

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
                this.generating = false;
                match result {
                    Ok((messages, sessions)) => {
                        this.sessions = sessions;
                        if this.selected == Some(session_id) {
                            this.messages = messages;
                            this.draft.clear();
                            if let Some(session) =
                                this.sessions.iter().find(|session| session.id == session_id)
                            {
                                this.draft_project = session.project_id;
                            }
                        }
                        this.error = None;
                    }
                    Err(message) => this.error = Some(message),
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
            .on_key_down(cx.listener(on_key))
            .child(sidebar(self, collapsed, width, toggle_label, &catalog, cx))
            .child(main_column(self, &catalog, cx))
    }
}

fn on_key(this: &mut AppShell, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<AppShell>) {
    if this.composer_focused {
        handle_composer_key(this, event, cx);
        return;
    }
    if !this.search_focused {
        return;
    }
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
    cx.notify();
}

fn handle_composer_key(
    this: &mut AppShell,
    event: &KeyDownEvent,
    cx: &mut Context<AppShell>,
) {
    let key = event.keystroke.key.as_str();
    if key == "enter" {
        if event.keystroke.modifiers.shift {
            this.draft.push('\n');
        } else {
            this.try_send(cx);
        }
    } else if key == "backspace" {
        this.draft.pop();
    } else if key == "escape" {
        this.composer_focused = false;
        this.picker_open = false;
    } else if let Some(ch) = typed_char(event) {
        this.draft.push_str(&ch);
    } else {
        return;
    }
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

fn sidebar_body(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> gpui::Div {
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
            cx.listener(|this, _, _, cx| {
                if let Ok(session) = this.client.create_session() {
                    this.sessions.push(session.clone());
                    this.select_session(session.id);
                    this.refresh();
                }
                cx.notify();
            }),
        ))
        .child(
            div()
                .id("search")
                .px_2()
                .py_1()
                .rounded_md()
                .text_sm()
                .when(state.search_focused, |el| el.border_1().border_color(ACCENT))
                .text_color(if state.search_query.is_empty() {
                    TEXT_MUTED
                } else {
                    TEXT
                })
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.search_focused = true;
                    this.composer_focused = false;
                    this.picker_open = false;
                    cx.notify();
                }))
                .child(if state.search_query.is_empty() {
                    catalog.get("sidebar.search").to_string()
                } else {
                    state.search_query.clone()
                }),
        );

    if let Some(error) = &state.error {
        col = col.child(
            div()
                .px_2()
                .py_2()
                .text_sm()
                .text_color(TEXT_MUTED)
                .child(error.clone()),
        );
        return col.child(div().flex_1()).child(label(catalog.get("sidebar.settings"), false));
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
                cx.listener(move |this, _, _, cx| {
                    this.select_session(id);
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
                    cx.listener(move |this, _, _, cx| {
                        this.select_session(id);
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
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
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
        .child(message_list(state, catalog))
        .child(composer(state, catalog, cx))
}

fn message_list(state: &AppShell, catalog: &Catalog) -> impl IntoElement {
    let mut list = div()
        .id("messages")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll()
        .py_2();
    if state.selected.is_none() {
        return list
            .items_center()
            .justify_center()
            .text_color(TEXT_MUTED)
            .child(catalog.get("session.none").to_string());
    }
    if state.messages.is_empty() {
        return list
            .items_center()
            .justify_center()
            .text_color(TEXT_MUTED)
            .child(catalog.get("session.empty").to_string());
    }
    for (index, message) in state.messages.iter().enumerate() {
        let role = match message.role {
            MessageRole::User => catalog.get("message.user"),
            MessageRole::Assistant => catalog.get("message.assistant"),
            MessageRole::System => catalog.get("message.system"),
        };
        let body = summarize_message(message);
        list = list.child(message_block(("msg", index), role, &body));
    }
    list
}

fn message_block(id: (&'static str, usize), role: &str, body: &str) -> impl IntoElement {
    let mut col = div()
        .id(id)
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .py_2()
        .child(div().text_xs().text_color(TEXT_MUTED).child(role.to_string()));
    if body.is_empty() {
        col
    } else {
        for line in body.lines() {
            col = col.child(div().text_sm().child(line.to_string()));
        }
        col
    }
}

fn composer(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let has_session = state.selected.is_some();
    let locked = project_picker_locked(state.selected_session());
    let sendable = can_send(has_session, &state.draft, state.generating);
    let project_label = session_project_label(
        state.draft_project,
        &state.projects,
        catalog.get("session.no_project"),
    );
    let draft_empty = state.draft.is_empty();
    let field_text = if !has_session {
        catalog.get("composer.no_session").to_string()
    } else if draft_empty {
        catalog.get("composer.placeholder").to_string()
    } else {
        state.draft.clone()
    };

    let mut col = div()
        .flex()
        .flex_col()
        .px_4()
        .py_2()
        .gap_2()
        .border_t_1()
        .border_color(BORDER);

    if has_session && state.picker_open && !locked {
        col = col.child(picker_menu(state, catalog, cx));
    }

    col.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(picker_button(
                has_session,
                locked,
                &project_label,
                catalog,
                cx,
            ))
            .child(send_status(state.generating, sendable, catalog, cx)),
    )
    .child(
        div()
            .id("composer")
            .min_h(px(56.))
            .px_2()
            .py_2()
            .rounded_md()
            .text_sm()
            .when(state.composer_focused && has_session, |el| {
                el.border_1().border_color(ACCENT)
            })
            .text_color(if !has_session || draft_empty {
                TEXT_MUTED
            } else {
                TEXT
            })
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                if this.selected.is_some() {
                    this.composer_focused = true;
                    this.search_focused = false;
                }
                cx.notify();
            }))
            .child(draft_view(&field_text)),
    )
}

fn draft_view(text: &str) -> impl IntoElement {
    let mut col = div().flex().flex_col();
    if text.is_empty() {
        return col;
    }
    for line in text.lines() {
        col = col.child(div().child(line.to_string()));
    }
    if text.ends_with('\n') {
        col = col.child(div().child(" "));
    }
    col
}

fn picker_button(
    has_session: bool,
    locked: bool,
    label: &str,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    div()
        .id("project-picker")
        .px_2()
        .py_1()
        .rounded_md()
        .text_xs()
        .text_color(TEXT_MUTED)
        .when(has_session && !locked, |el| {
            el.cursor_pointer()
                .hover(|style| style.bg(BG_SIDEBAR).text_color(TEXT))
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            if this.selected.is_some() && !project_picker_locked(this.selected_session()) {
                this.picker_open = !this.picker_open;
                this.composer_focused = true;
                this.search_focused = false;
            }
            cx.notify();
        }))
        .child(if has_session {
            label.to_string()
        } else {
            catalog.get("session.none").to_string()
        })
}

fn picker_menu(
    state: &AppShell,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    let mut menu = div()
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(BORDER)
        .bg(BG_SIDEBAR);
    let none_selected = state.draft_project.is_none();
    menu = menu.child(picker_item(
        "picker-none",
        catalog.get("session.no_project"),
        none_selected,
        cx.listener(|this, _, _, cx| {
            this.draft_project = None;
            this.picker_open = false;
            cx.notify();
        }),
    ));
    for (index, project) in state.projects.iter().enumerate() {
        let id = project.id;
        let selected = state.draft_project == Some(id);
        menu = menu.child(picker_item(
            ("picker-proj", index),
            &project.name,
            selected,
            cx.listener(move |this, _, _, cx| {
                this.draft_project = Some(id);
                this.picker_open = false;
                cx.notify();
            }),
        ));
    }
    menu
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

fn send_status(
    generating: bool,
    sendable: bool,
    catalog: &Catalog,
    cx: &mut Context<AppShell>,
) -> impl IntoElement {
    if generating {
        return div()
            .id("generating")
            .px_2()
            .py_1()
            .text_xs()
            .text_color(TEXT_MUTED)
            .child(catalog.get("composer.generating").to_string())
            .into_any_element();
    }
    div()
        .id("send")
        .px_2()
        .py_1()
        .rounded_md()
        .text_xs()
        .when(sendable, |el| el.text_color(TEXT).cursor_pointer().bg(ACCENT))
        .when(!sendable, |el| el.text_color(TEXT_MUTED))
        .on_click(cx.listener(|this, _, _, cx| {
            this.try_send(cx);
            cx.notify();
        }))
        .child(catalog.get("composer.send").to_string())
        .into_any_element()
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
    div().px_2().py_1().text_sm().text_color(TEXT_MUTED).child(text.to_string())
}
