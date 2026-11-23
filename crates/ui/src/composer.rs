//! The composer: input surface, the shared controls and the footer.
//!
//! The text field is ours: GPUI has no text input element. `crate::input` owns
//! the buffer and the caret; this module paints it and the shell handles the
//! keys. Enter sends, Shift+Enter breaks the line.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Corner, Div, FocusHandle, IntoElement, KeyDownEvent, Pixels, Point,
    SharedString, Window, anchored, canvas, deferred, div, point, prelude::*, px,
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
/// The effort editor's footprint, beside the model menu.
const EDITOR_WIDTH: f32 = 160.;

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
/// Starring a model, switching the model menu's tab, opening a row's effort
/// editor, and picking an effort for a model.
pub(crate) type StarHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub(crate) type TabHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub(crate) type EditHandler = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
pub(crate) type EffortHandler = Rc<dyn Fn(String, String, String, &mut Window, &mut App)>;

/// The daemon's options as the pickers draw them: which menu is open, which
/// model tab is showing, what choosing and starring do. A missing option id
/// keeps its mock label and does nothing.
pub(crate) struct Pickers<'a> {
    pub options: &'a [ConfigOption],
    pub open: Option<&'a str>,
    /// The model menu's tab, when the user picked one; `None` picks by itself.
    pub tab: Option<&'a str>,
    /// The model row whose effort editor is open, if any.
    pub editor: Option<&'a str>,
    /// The starred model ids, in the order they were starred.
    pub favorites: &'a [String],
    /// The agent behind the daemon, naming the models tab ("opencode").
    pub agent_label: &'a str,
    /// Where each chip sits, shared across frames: the canvas writes this
    /// frame's measurement, the menu reads last frame's.
    pub anchors: Rc<Cell<BTreeMap<String, Point<Pixels>>>>,
    pub on_toggle: ToggleHandler,
    pub on_choose: ChooseHandler,
    pub on_star: StarHandler,
    pub on_tab: TabHandler,
    pub on_edit: EditHandler,
    pub on_effort: EffortHandler,
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

    // The open menu and the effort editor both live at the top level, each
    // an anchored popup at the chip's measured point — the menu above the
    // chip, the editor beside the menu. Inside the menu's own subtree the
    // anchoring math does not compose; as siblings they do.
    let open_option = pickers
        .open
        .and_then(|open| pickers.options.iter().find(|option| option.id == open));
    let anchor = open_option.map(|option| {
        let map = pickers.anchors.take();
        let anchor = map.get(&option.id).copied().unwrap_or_default();
        pickers.anchors.set(map);
        anchor
    });
    let menu = open_option
        .zip(anchor)
        .map(|(option, anchor)| menu_view(theme, option, &pickers, anchor));
    let editor = open_option.zip(anchor).and_then(|(option, chip)| {
        let editor = pickers.editor?;
        let choice = option.options.iter().find(|choice| choice.id == editor)?;
        // The editor belongs to the row the user opened it from, so its
        // height comes from that row's Edit button — measured into the same
        // cell as the chips, one frame old at worst. Without a measurement
        // (the menu was closed a frame ago) it waits for the next one.
        let key = format!("edit-{}-{}", option.id, choice.id);
        let map = pickers.anchors.take();
        let row = map.get(&key).copied();
        pickers.anchors.set(map);
        let row = row?;
        // Beside the menu (the chip's column plus its width), level with
        // the clicked row.
        Some(effort_editor(
            theme,
            choice,
            &pickers,
            point(chip.x + px(MENU_WIDTH + 8.), row.y),
        ))
    });

    div()
        .flex()
        .flex_col()
        .px(px(24.))
        .pb(px(10.))
        .children(menu)
        .children(editor)
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
/// the chip itself. Where the chip sits is measured, not assumed: a
/// zero-size canvas beside it reports the wrapper's window origin every
/// frame into the shell's shared cell, and the menu anchors to the last
/// measurement — one frame old at worst, and the chips do not move.
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
            // The model chip carries its effort as a small tag, so the
            // composer answers "what am I running" in one glance.
            let tag = (id == "model")
                .then(|| current_effort_label(pickers))
                .flatten();

            // The anchor the menu reads is last frame's measurement, which is
            // why the cell lives on the shell: a fresh one per render would
            // only ever hold zero, the window's top-left corner.
            let track = Rc::clone(&pickers.anchors);
            let key = id.to_owned();
            let mut wrapper = div().flex().items_start().child(canvas(
                move |bounds, _, _| {
                    let mut anchors = track.take();
                    anchors.insert(key, bounds.origin);
                    track.set(anchors);
                },
                |_, _, _, _| {},
            ));
            wrapper = wrapper.child(
                picker(theme, path, label, tag, open)
                    .id(SharedString::from(format!("picker-{}", option.id)))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_toggle(next.clone(), window, cx)),
            );
            if open {
                // No menu without a measurement: an unmeasured anchor is
                // zero, and zero is the window's top-left corner. The chip's
                // canvas has run at least once before any click — and the
                // startup hook waits for it — so this never holds long.
                let map = pickers.anchors.take();
                let anchor = map.get(id).copied();
                pickers.anchors.set(map);
                if let Some(anchor) = anchor {
                    wrapper = wrapper.child(menu_view(theme, option, pickers, anchor));
                }
            }
            wrapper.into_any_element()
        }
        None => static_picker(theme, path, fallback).into_any_element(),
    }
}

fn static_picker(
    theme: &Theme,
    path: Option<&'static str>,
    label: &'static str,
) -> impl IntoElement {
    picker(theme, path, label.to_owned(), None, false)
}

fn picker(
    theme: &Theme,
    path: Option<&'static str>,
    label: String,
    tag: Option<String>,
    open: bool,
) -> Div {
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
        .when_some(tag, |row, tag| row.child(effort_tag(theme, &tag)))
        .child(icon("icons/chevron-down.svg", 11., theme.text_faint))
        .when(open, |row| row.bg(theme.hover))
        .hover(move |style| style.bg(theme.hover))
}

/// The effort tag that rides on the model chip: small, quiet, and readable
/// at a glance — `GLM-5.4 High`.
fn effort_tag(theme: &Theme, label: &str) -> Div {
    div()
        .flex_shrink_0()
        .px(px(5.))
        .py(px(1.))
        .rounded(px(4.))
        .bg(theme.chip)
        .text_size(px(10.5))
        .text_color(theme.text_muted)
        .child(label.to_owned())
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

/// The effort the model is running at, as the agent names it, for the chip's
/// tag. `None` while the agent lists no effort option at all.
fn current_effort_label(pickers: &Pickers<'_>) -> Option<String> {
    let effort = pickers
        .options
        .iter()
        .find(|option| option.id == "effort")?;
    Some(current_label(effort))
}

/// The open option's choices: a popup of fixed width and bounded height,
/// anchored above the picker chip that opened it — at the point the chip's
/// canvas measured last frame. The model menu gains tabs — the starred
/// models first, the agent's own list grouped by who serves each model —
/// and a star per row. The current value is marked; anything else sends
/// the choice to the daemon.
fn menu_view(
    theme: &Theme,
    option: &ConfigOption,
    pickers: &Pickers<'_>,
    anchor: Point<Pixels>,
) -> AnyElement {
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
        // Effort rides on the model menu when the agent lists it: every row
        // gains its editor. The agent does not advertise per-model support,
        // so a model without effort gets the daemon's refusal, not a hidden
        // button.
        let editable = pickers
            .options
            .iter()
            .find(|option| option.id == "effort")
            .map(|option| !option.options.is_empty())
            .unwrap_or(false);
        let tab = pickers
            .tab
            .unwrap_or_else(|| default_tab(pickers.favorites));
        if tab == FAVORITES_TAB {
            rows = favorite_rows(theme, option, pickers, editable);
        } else {
            rows = grouped_rows(theme, option, pickers, editable);
        }
    } else {
        for choice in &option.options {
            rows = rows.child(choice_row(theme, option, choice, pickers, false, false));
        }
    }

    // The popup's bottom-left corner sits just above the chip that opened it,
    // so the menu reads as coming out of that selector. `anchored` flips or
    // snaps the corner when the window edge would cut it. `deferred` moves
    // the paint to after every ancestor has drawn, so the surface paints
    // opaque over the composer instead of blending with it.
    deferred(
        anchored()
            .anchor(Corner::BottomLeft)
            .position(anchor)
            .offset(point(px(0.), px(-8.)))
            .child(
                menu.child(
                    rows.id(SharedString::from(format!("menu-rows-{}", option.id)))
                        .max_h(px(MENU_MAX_HEIGHT))
                        .overflow_y_scroll(),
                ),
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

    // Both tabs are icon-only: the star, and the agent's mark when one
    // exists (otherwise the agent's name with a generic glyph).
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
            None,
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
            if on { theme.text } else { theme.text_faint },
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
fn grouped_rows(
    theme: &Theme,
    option: &ConfigOption,
    pickers: &Pickers<'_>,
    editable: bool,
) -> Div {
    let groups = group_by_provider(&option.options);
    let mut rows = div().flex().flex_col();

    if groups.len() <= 1 {
        for choice in &option.options {
            rows = rows.child(choice_row(theme, option, choice, pickers, true, editable));
        }
        return rows;
    }

    for (provider, choices) in groups {
        rows = rows.child(
            div()
                .pl(px(28.))
                .pr(px(8.))
                .pt(px(8.))
                .pb(px(2.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(provider_label(provider).to_owned()),
        );
        for choice in choices {
            rows = rows.child(choice_row(theme, option, choice, pickers, true, editable));
        }
    }
    rows
}

/// The starred choices, in the order they were starred. A star that names a
/// model the agent no longer serves is skipped.
fn favorite_rows(
    theme: &Theme,
    option: &ConfigOption,
    pickers: &Pickers<'_>,
    editable: bool,
) -> Div {
    let mut rows = div().flex().flex_col();
    let mut shown = 0;

    for id in pickers.favorites {
        if let Some(choice) = option.options.iter().find(|choice| &choice.id == id) {
            shown += 1;
            rows = rows.child(choice_row(theme, option, choice, pickers, true, editable));
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

/// One choice line: the agent's mark, the cleaned name with its provider
/// beside it, and — for the model menu — the star and the Edit affordance.
/// Only the name's own area chooses the model: the star and Edit sit outside
/// that element, so their clicks never select anything.
fn choice_row(
    theme: &Theme,
    option: &ConfigOption,
    choice: &circulo_protocol::ConfigChoice,
    pickers: &Pickers<'_>,
    starred: bool,
    editable: bool,
) -> AnyElement {
    let current = choice.id == option.current;
    let id = option.id.clone();
    let value = choice.id.clone();
    let on_choose = pickers.on_choose.clone();
    let star = pickers.on_star.clone();
    let favorite = pickers.favorites.iter().any(|id| id == &choice.id);
    let editing = pickers.editor == Some(choice.id.as_str());

    let group = SharedString::from(format!("row-{}-{}", option.id, choice.id));
    let mut row = div()
        .id(SharedString::from(format!(
            "choice-{}-{}",
            option.id, choice.id
        )))
        .group(group.clone())
        .flex()
        .items_center()
        .gap(px(4.))
        .px(px(8.))
        .py(px(6.))
        .rounded(px(6.))
        .text_size(px(12.5))
        .text_color(if current {
            theme.text
        } else {
            theme.text_muted
        })
        .hover(move |style| style.bg(theme.hover))
        // Choosing is the name's own hit area — the buttons beside it are
        // siblings, so a click on them cannot reach this handler.
        .child(
            div()
                .id(SharedString::from(format!(
                    "pick-{}-{}",
                    option.id, choice.id
                )))
                .flex()
                .items_center()
                .gap(px(6.))
                .flex_1()
                .min_w(px(0.))
                .cursor_pointer()
                .child(
                    div()
                        .w(px(14.))
                        .flex_shrink_0()
                        .text_color(theme.accent)
                        .child(if current { "✓" } else { "" }),
                )
                .child(
                    div()
                        .min_w(px(0.))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(choice.label.clone()),
                )
                .on_click(move |_, window, cx| on_choose(id.clone(), value.clone(), window, cx)),
        );

    if editable {
        // Edit leads, the star follows, and Edit only shows while the row is
        // hovered — opacity, not removal, so rows keep their width.
        let on_edit = pickers.on_edit.clone();
        let model = choice.id.clone();
        // Where this button sits, every frame while the menu is open: the
        // effort editor reads it to line up with this row. Opacity does not
        // stop the canvas, so the measurement survives the hover leaving.
        let track = Rc::clone(&pickers.anchors);
        let key = format!("edit-{}", option.id);
        row = row.child(
            div()
                .id(SharedString::from(format!(
                    "edit-{}-{}",
                    option.id, choice.id
                )))
                .flex_shrink_0()
                .px(px(4.))
                .py(px(2.))
                .rounded(px(4.))
                .text_size(px(10.5))
                .text_color(if editing {
                    theme.text
                } else {
                    theme.text_faint
                })
                .when(editing, |edit| edit.bg(theme.hover))
                .child(canvas(
                    {
                        let key = format!("{}-{}", key, choice.id);
                        move |bounds, _, _| {
                            let mut anchors = track.take();
                            anchors.insert(key, bounds.origin);
                            track.set(anchors);
                        }
                    },
                    |_, _, _, _| {},
                ))
                .child("Edit")
                .cursor_pointer()
                .opacity(0.)
                .group_hover(group, move |style| style.opacity(1.))
                .hover(move |style| style.bg(theme.hover))
                .on_click(move |_, window, cx| {
                    on_edit(if editing { None } else { Some(model.clone()) }, window, cx);
                }),
        );
    }

    if starred {
        let value = choice.id.clone();
        row = row.child(
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
                        theme.text
                    } else {
                        theme.text_faint
                    },
                ))
                .on_click(move |_, window, cx| star(value.clone(), window, cx)),
        );
    }

    row.into_any_element()
}

/// The effort editor for one model: the agent's effort choices beside the
/// row. Picking one applies the model with it — one decision, two plain
/// settings on the wire.
fn effort_editor(
    theme: &Theme,
    choice: &circulo_protocol::ConfigChoice,
    pickers: &Pickers<'_>,
    anchor: Point<Pixels>,
) -> AnyElement {
    let Some(effort) = pickers.options.iter().find(|option| option.id == "effort") else {
        return div().into_any_element();
    };

    let mut panel = div()
        .w(px(EDITOR_WIDTH))
        .flex()
        .flex_col()
        .p(px(6.))
        .rounded(px(10.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .shadow_md()
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .text_size(px(11.5))
                .text_color(theme.text_faint)
                .child(effort.label.clone()),
        );

    for value in &effort.options {
        let current = value.id == effort.current;
        let model = choice.id.clone();
        let effort_id = effort.id.clone();
        let picked = value.id.clone();
        let on_effort = pickers.on_effort.clone();

        panel = panel.child(
            div()
                .id(SharedString::from(format!(
                    "effort-{}-{}",
                    choice.id, value.id
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
                .child(value.label.clone())
                .cursor_pointer()
                .hover(move |style| style.bg(theme.hover))
                .on_click(move |_, window, cx| {
                    on_effort(model.clone(), effort_id.clone(), picked.clone(), window, cx)
                }),
        );
    }

    // To the menu's right, with its top at the clicked row: the anchor the
    // composer passes already carries both (the menu's right edge, the row's
    // Edit button). `TopLeft`, not the menu's `BottomLeft` — this one grows
    // down from its row; when the row sits too low, `anchored` flips it to
    // grow up instead of spilling past the window. Plain `anchored`, not
    // `deferred`: the menu is already the deferred draw, and deferring again
    // from inside it aborts the app. The editor paints with the menu, which
    // is already the last pass.
    deferred(
        anchored()
            .anchor(Corner::TopLeft)
            .position(anchor)
            .child(panel),
    )
    .with_priority(1)
    .into_any_element()
}

/// Choices under their provider, in first-appearance order.
#[allow(dead_code)]
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
