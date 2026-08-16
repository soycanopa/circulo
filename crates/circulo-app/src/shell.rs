use circulo_core::{Project, Session, SidebarView};
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
    selected: Option<circulo_core::Uuid>,
    search_query: String,
    search_focused: bool,
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
            selected: None,
            search_query: String::new(),
            search_focused: false,
            error: None,
            loaded: false,
        }
    }
}

impl AppShell {
    pub fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let daemon_down = self.catalog.get("sidebar.daemon_down").to_string();
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
                            Ok((resolve_view(view), sessions, projects))
                        }
                        Err(err) => Err(format!("{daemon_down} ({err})")),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.loaded = true;
                match snapshot {
                    Ok((view, sessions, projects)) => {
                        this.view = view;
                        this.sessions = sessions;
                        this.projects = projects;
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
        self.selected
            .and_then(|id| {
                self.sessions
                    .iter()
                    .find(|session| session.id == id)
                    .map(|session| session.title.clone())
            })
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
            .child(main_column(&self.selected_title(), &catalog))
    }
}

fn on_key(this: &mut AppShell, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<AppShell>) {
    if !this.search_focused {
        return;
    }
    let key = event.keystroke.key.as_str();
    if key == "backspace" {
        this.search_query.pop();
    } else if key == "escape" {
        this.search_query.clear();
        this.search_focused = false;
    } else if key.len() == 1 && !event.keystroke.modifiers.platform {
        this.search_query.push_str(key);
    } else {
        return;
    }
    cx.notify();
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
                    this.selected = Some(session.id);
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
                    this.selected = Some(id);
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
                        this.selected = Some(id);
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

fn main_column(title: &str, catalog: &Catalog) -> impl IntoElement {
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
                .child(title.to_string()),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(TEXT_MUTED)
                .child(catalog.get("session.empty").to_string()),
        )
        .child(
            div()
                .h(px(72.))
                .px_4()
                .flex()
                .items_center()
                .border_t_1()
                .border_color(BORDER)
                .text_color(TEXT_MUTED)
                .child(catalog.get("composer.placeholder").to_string()),
        )
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
