//! The composer: input surface, the shared controls and the footer.
//!
//! The text field is ours: GPUI has no text input element. `crate::input` owns
//! the buffer and the caret; this module paints it and the shell handles the
//! keys. Enter sends, Shift+Enter breaks the line.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Corner, Div, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window,
    anchored, div, point, prelude::*, px,
};

use crate::icons::icon;
use crate::input::{self, TextInput};
use crate::live::Transcript;
use crate::mock::Workspace;
use crate::theme::Theme;
use circulo_protocol::ConfigOption;

/// How tall the field may grow before it scrolls: four lines of body text.
const FIELD_MAX_HEIGHT: f32 = 96.;

/// The menu's footprint: a popup, not a panel. Wide enough for a model name
/// and its provider, short enough to never swallow the transcript.
const MENU_WIDTH: f32 = 360.;
const MENU_MAX_HEIGHT: f32 = 320.;

/// The model menu's tabs. The favourites lead when there are any; the agent's
/// own list is always one click away.
const FAVORITES_TAB: &str = "favorites";
const MODELS_TAB: &str = "models";

/// Keys the field does not handle itself, and what the send button does. The
/// shell owns both, because both change its state.
pub(crate) type KeyHandler = Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App)>;
pub(crate) type SendHandler = Rc<dyn Fn(&mut Window, &mut App)>;
/// Opening or closing an option menu, and picking a value. The shell owns
/// both, because both change its state.
pub(crate) type ToggleHandler = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
pub(crate) type ChooseHandler = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
/// Starring a model, and switching the model menu's tab.
pub(crate) type StarHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub(crate) type TabHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// The daemon's options as the pickers draw them: which menu is open, which
/// model tab is showing, what choosing and starring do. A missing option id
/// keeps its mock label and does nothing.
pub(crate) struct Pickers<'a> {
    pub options: &'a [ConfigOption],
    pub open: Option<&'a str>,
    /// The model menu's tab, when the user picked one; `None` picks by itself.
    pub tab: Option<&'a str>,
    /// The starred model ids, in the order they were starred.
    pub favorites: &'a [String],
    /// The agent behind the daemon, naming the models tab ("opencode").
    pub agent_label: &'a str,
    pub on_toggle: ToggleHandler,
    pub on_choose: ChooseHandler,
    pub on_star: StarHandler,
    pub on_tab: TabHandler,
}

/// Everything the field needs that changes between frames.
pub(crate) struct Field<'a> {
    pub input: &'a TextInput,
    pub focus: &'a FocusHandle,
    pub focused: bool,
    pub on_key: KeyHandler,
    pub on_send: SendHandler,
}

pub(crate) fn composer(
    theme: &Theme,
    workspace: &Workspace,
    transcript: &Transcript,
    pickers: Pickers<'_>,
    field: Field<'_>,
) -> Div {
    let running = transcript.status.running;
    let can_send = !field.input.is_blank() && !running;

    // The open menu is not laid out here: it lives inside the picker that
    // opened it, anchored above the chip, so it never pushes the composer
    // around and always reads as coming from its own selector.
    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .child(input_box(
            theme, workspace, running, &pickers, field, can_send,
        ))
        .child(footer(theme, workspace, running))
}

fn input_box(
    theme: &Theme,
    workspace: &Workspace,
    running: bool,
    pickers: &Pickers<'_>,
    field: Field<'_>,
    can_send: bool,
) -> impl IntoElement {
    let focus = field.focus;
    let focused = field.focused;
    let on_key = field.on_key;
    let on_send = field.on_send;
    let click_focus = focus.clone();

    div()
        .id("composer")
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(13.))
        .rounded(px(14.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .text_size(px(13.))
        .track_focus(focus)
        .on_key_down(move |event, window, cx| on_key(event, window, cx))
        .on_click(move |_, window, _| click_focus.focus(window))
        .child(
            div()
                .id("composer-field")
                .max_h(px(FIELD_MAX_HEIGHT))
                .overflow_y_scroll()
                .whitespace_normal()
                .child(input::field(theme, field.input, focused, "Do anything...")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(slot(theme, pickers, "model", None, workspace.model))
                .child(slot(theme, pickers, "effort", None, workspace.reasoning))
                .child(static_picker(
                    theme,
                    Some("icons/lock.svg"),
                    workspace.access,
                ))
                .child(slot(
                    theme,
                    pickers,
                    "mode",
                    Some("icons/hammer.svg"),
                    workspace.agent_mode,
                ))
                .child(div().flex_1())
                .child(send(theme, running, can_send, on_send)),
        )
}

/// One composer slot: the daemon's value when the option exists, the mock
/// label until then. A live option opens its menu on click, anchored above
/// the chip itself.
fn slot(
    theme: &Theme,
    pickers: &Pickers<'_>,
    id: &str,
    path: Option<&'static str>,
    fallback: &'static str,
) -> impl IntoElement {
    match pickers.options.iter().find(|option| option.id == id) {
        Some(option) => {
            let label = current_label(option);
            let open = pickers.open == Some(option.id.as_str());
            let next = if open { None } else { Some(option.id.clone()) };
            let on_toggle = pickers.on_toggle.clone();
            let mut anchor = picker(theme, path, label, open)
                .id(SharedString::from(format!("picker-{}", option.id)))
                .cursor_pointer()
                .on_click(move |_, window, cx| on_toggle(next.clone(), window, cx));
            if open {
                anchor = anchor.child(menu_view(theme, option, pickers));
            }
            anchor.into_any_element()
        }
        None => static_picker(theme, path, fallback).into_any_element(),
    }
}

fn static_picker(
    theme: &Theme,
    path: Option<&'static str>,
    label: &'static str,
) -> impl IntoElement {
    picker(theme, path, label.to_owned(), false)
}

fn picker(theme: &Theme, path: Option<&'static str>, label: String, open: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .h(px(26.))
        .px(px(6.))
        .rounded(px(6.))
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .when_some(path, |row, path| {
            row.child(icon(path, 13., theme.text_muted))
        })
        .child(
            div()
                .flex_shrink_0()
                .max_w(px(200.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(label),
        )
        .child(icon("icons/chevron-down.svg", 11., theme.text_faint))
        .when(open, |row| row.bg(theme.hover))
        .hover(move |style| style.bg(theme.hover))
}

/// The agent's wording for its current value, or the raw id when the choice
/// list does not name it.
fn current_label(option: &ConfigOption) -> String {
    option
        .options
        .iter()
        .find(|choice| choice.id == option.current)
        .map(|choice| choice.label.clone())
        .unwrap_or_else(|| option.current.clone())
}

/// The open option's choices: a popup of fixed width and bounded height,
/// anchored above the picker chip that opened it, whatever the agent lists.
/// The model menu gains tabs — the starred models first, the agent's own
/// list grouped by who serves each model — and a star per row. The current
/// value is marked; anything else sends the choice to the daemon.
fn menu_view(theme: &Theme, option: &ConfigOption, pickers: &Pickers<'_>) -> AnyElement {
    let tabbed = option.id == "model";

    let mut menu = div()
        .w(px(MENU_WIDTH))
        .flex()
        .flex_col()
        .p(px(6.))
        .rounded(px(10.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .shadow_md();

    if tabbed {
        menu = menu.child(tab_bar(theme, option, pickers));
    } else {
        menu = menu.child(
            div()
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(option.label.clone()),
        );
    }

    let mut rows = div().flex().flex_col();
    if tabbed {
        let tab = pickers
            .tab
            .unwrap_or_else(|| default_tab(pickers.favorites));
        if tab == FAVORITES_TAB {
            rows = favorite_rows(theme, option, pickers);
        } else {
            rows = grouped_rows(theme, option, pickers);
        }
    } else {
        for choice in &option.options {
            rows = rows.child(choice_row(theme, option, choice, pickers, false));
        }
    }

    // The popup's bottom-left corner sits just above the chip that opened it,
    // so the menu reads as coming out of that selector. `anchored` flips or
    // snaps the corner when the window edge would cut it.
    anchored()
        .anchor(Corner::BottomLeft)
        .offset(point(px(0.), px(-8.)))
        .child(
            menu.child(
                rows.id(SharedString::from(format!("menu-rows-{}", option.id)))
                    .max_h(px(MENU_MAX_HEIGHT))
                    .overflow_y_scroll(),
            ),
        )
        .into_any_element()
}

/// Without a choice from the user, favourites lead when they exist: that tab
/// is the reason they were starred.
fn default_tab(favorites: &[String]) -> &'static str {
    if favorites.is_empty() {
        MODELS_TAB
    } else {
        FAVORITES_TAB
    }
}

/// The model menu's two tabs: what was starred, and what the agent serves.
fn tab_bar(theme: &Theme, option: &ConfigOption, pickers: &Pickers<'_>) -> Div {
    let active = pickers
        .tab
        .unwrap_or_else(|| default_tab(pickers.favorites));

    // The models tab carries the agent's own mark when one exists — icon
    // only, the way a logo wants to be read — and falls back to the name
    // with a generic glyph for agents without one.
    let agent = agent_icon(pickers.agent_label);
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .px(px(4.))
        .pb(px(6.))
        .border_b_1()
        .border_color(theme.border)
        .child(tab_button(
            theme,
            option,
            FAVORITES_TAB,
            "icons/star.svg",
            Some("Favorites"),
            active == FAVORITES_TAB,
            pickers,
        ))
        .child(tab_button(
            theme,
            option,
            MODELS_TAB,
            agent.unwrap_or("icons/layers.svg"),
            agent.map_or(Some(pickers.agent_label), |_| None),
            active == MODELS_TAB,
            pickers,
        ))
}

fn tab_button<'a>(
    theme: &Theme,
    option: &ConfigOption,
    id: &'static str,
    path: &'static str,
    label: Option<&'a str>,
    on: bool,
    pickers: &Pickers<'a>,
) -> impl IntoElement + use<'a> {
    let on_tab = pickers.on_tab.clone();
    div()
        .id(SharedString::from(format!("tab-{}-{}", option.id, id)))
        .flex()
        .items_center()
        .gap(px(5.))
        .px(px(8.))
        .py(px(5.))
        .rounded(px(6.))
        .text_size(px(12.))
        .text_color(if on { theme.text } else { theme.text_faint })
        .when(on, |tab| tab.bg(theme.hover))
        .child(icon(
            path,
            13.,
            if on { theme.accent } else { theme.text_faint },
        ))
        .when_some(label, |tab, label| tab.child(label.to_owned()))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.hover))
        .on_click(move |_, window, cx| {
            if !on {
                on_tab(id.to_owned(), window, cx);
            }
        })
}

/// The agent's own mark, when the interface has one. Any agent without a
/// known logo keeps its name on the tab.
fn agent_icon(agent: &str) -> Option<&'static str> {
    match agent {
        "opencode" => Some("icons/opencode.svg"),
        _ => None,
    }
}

/// The agent's choices under one heading per provider, in the order the
/// agent listed them. A list with no providers at all stays flat.
fn grouped_rows(theme: &Theme, option: &ConfigOption, pickers: &Pickers<'_>) -> Div {
    let groups = group_by_provider(&option.options);
    let mut rows = div().flex().flex_col();

    if groups.len() <= 1 {
        for choice in &option.options {
            rows = rows.child(choice_row(theme, option, choice, pickers, true));
        }
        return rows;
    }

    for (provider, choices) in groups {
        rows = rows.child(
            div()
                .px(px(8.))
                .pt(px(8.))
                .pb(px(2.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(provider_label(provider).to_owned()),
        );
        for choice in choices {
            rows = rows.child(choice_row(theme, option, choice, pickers, true));
        }
    }
    rows
}

/// The starred choices, in the order they were starred. A star that names a
/// model the agent no longer serves is skipped.
fn favorite_rows(theme: &Theme, option: &ConfigOption, pickers: &Pickers<'_>) -> Div {
    let mut rows = div().flex().flex_col();
    let mut shown = 0;

    for id in pickers.favorites {
        if let Some(choice) = option.options.iter().find(|choice| &choice.id == id) {
            shown += 1;
            rows = rows.child(choice_row(theme, option, choice, pickers, true));
        }
    }

    if shown == 0 {
        rows = rows.child(
            div()
                .px(px(8.))
                .py(px(10.))
                .text_size(px(12.))
                .text_color(theme.text_faint)
                .child("Star a model to pin it here."),
        );
    }
    rows
}

/// One choice line: the agent's mark on the left, the cleaned name, and — for
/// the model menu — the star. The star stops the click so choosing a model
/// and starring it never fire together.
fn choice_row(
    theme: &Theme,
    option: &ConfigOption,
    choice: &circulo_protocol::ConfigChoice,
    pickers: &Pickers<'_>,
    starred: bool,
) -> AnyElement {
    let current = choice.id == option.current;
    let id = option.id.clone();
    let value = choice.id.clone();
    let on_choose = pickers.on_choose.clone();
    let star = pickers.on_star.clone();
    let favorite = pickers.favorites.iter().any(|id| id == &choice.id);

    let row = div()
        .id(SharedString::from(format!(
            "choice-{}-{}",
            option.id, choice.id
        )))
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(8.))
        .py(px(6.))
        .rounded(px(6.))
        .text_size(px(12.5))
        .text_color(if current {
            theme.text
        } else {
            theme.text_muted
        })
        .child(
            div()
                .w(px(14.))
                .flex_shrink_0()
                .text_color(theme.accent)
                .child(if current { "✓" } else { "" }),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(choice.label.clone()),
        );

    let row = if starred {
        let value = choice.id.clone();
        row.child(
            div()
                .id(SharedString::from(format!(
                    "star-{}-{}",
                    option.id, choice.id
                )))
                .flex_shrink_0()
                .p(px(2.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.hover))
                .child(icon(
                    "icons/star.svg",
                    14.,
                    if favorite {
                        theme.accent
                    } else {
                        theme.text_faint
                    },
                ))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    star(value.clone(), window, cx);
                }),
        )
    } else {
        row
    };

    row.cursor_pointer()
        .hover(move |style| style.bg(theme.hover))
        .on_click(move |_, window, cx| on_choose(id.clone(), value.clone(), window, cx))
        .into_any_element()
}

/// Choices under their provider, in first-appearance order.
fn group_by_provider(
    choices: &[circulo_protocol::ConfigChoice],
) -> Vec<(&str, Vec<&circulo_protocol::ConfigChoice>)> {
    let mut groups: Vec<(&str, Vec<&circulo_protocol::ConfigChoice>)> = Vec::new();
    for choice in choices {
        let provider = choice.provider.as_str();
        match groups.iter_mut().find(|(name, _)| name == &provider) {
            Some((_, group)) => group.push(choice),
            None => groups.push((provider, vec![choice])),
        }
    }
    groups
}

/// The provider's name as a person would read it. The map covers the agents
/// seen so far; anything else shows its slug, which is at least honest.
fn provider_label(provider: &str) -> &str {
    match provider {
        "deepseek" => "DeepSeek",
        "minimax-coding-plan" => "MiniMax",
        "opencode" => "OpenCode",
        "zai-coding-plan" => "Z.ai",
        other => other,
    }
}

fn send(theme: &Theme, running: bool, can_send: bool, on_send: SendHandler) -> impl IntoElement {
    let button = div()
        .id("composer-send")
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded_full()
        .bg(if can_send { theme.text } else { theme.chip })
        .child(if running {
            icon("icons/loader-circle.svg", 14., theme.text_faint)
        } else {
            icon(
                "icons/arrow-up.svg",
                14.,
                if can_send {
                    theme.canvas
                } else {
                    theme.text_faint
                },
            )
        });

    if can_send {
        button
            .cursor_pointer()
            .hover(|style| style.opacity(0.9))
            .on_click(move |_, window, cx| on_send(window, cx))
    } else {
        button
    }
}

fn footer(theme: &Theme, workspace: &Workspace, running: bool) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(14.))
        .pt(px(10.))
        .px(px(4.))
        .text_size(px(11.5))
        .text_color(theme.text_faint)
        .child(meta(theme, "icons/folder.svg", workspace.project))
        .child(meta(theme, "icons/monitor.svg", "Local"))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(icon("icons/git-branch.svg", 12., theme.text_faint))
                .child(workspace.branch)
                .child(icon("icons/chevron-down.svg", 10., theme.text_faint)),
        )
        .child(div().flex_1())
        .when(running, |row| {
            row.child(icon("icons/loader-circle.svg", 12., theme.text_faint))
        })
}

fn meta(theme: &Theme, path: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(icon(path, 12., theme.text_faint))
        .child(label)
}
