//! Dedicated multiline composer text field (GPUI IME + actions).

use std::ops::Range;

use gpui::{
    actions, div, fill, point, prelude::*, px, relative, App, Bounds, Context, CursorStyle, Element,
    ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, InspectorElementId, IntoElement, KeyBinding, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ScrollHandle,
    SharedString, StatefulInteractiveElement, Style, UTF16Selection, Window,
};
use gpui::prelude::FluentBuilder;

use crate::composer::events::ComposerInputEvent;
use crate::composer::text_layout::{
    ComposerTextLayout, COMPOSER_MAX_LINES_COLLAPSED, visible_line_cap,
};
use crate::icons::{icon, path as icon_path};
use crate::theme::{ACCENT, BG_MAIN, TEXT, TEXT_MUTED};

actions!(
    composer_input,
    [Backspace, Delete, Enter, Newline, Paste]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("ComposerInput")),
        KeyBinding::new("delete", Delete, Some("ComposerInput")),
        KeyBinding::new("enter", Enter, Some("ComposerInput")),
        KeyBinding::new("shift-enter", Newline, Some("ComposerInput")),
        KeyBinding::new("cmd-v", Paste, Some("ComposerInput")),
    ]);
}

pub struct ComposerInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ComposerTextLayout>,
    last_bounds: Option<Bounds<Pixels>>,
    content_height: Pixels,
    visual_line_count: usize,
    line_height: Pixels,
    is_selecting: bool,
    read_only: bool,
    enabled: bool,
    expanded: bool,
    scroll: ScrollHandle,
}

impl ComposerInput {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            content_height: px(0.),
            visual_line_count: 1,
            line_height: px(0.),
            is_selecting: false,
            read_only: false,
            enabled: true,
            expanded: false,
            scroll: ScrollHandle::new(),
        }
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
    }

    pub fn set_read_only(&mut self, read_only: bool, cx: &mut Context<Self>) {
        self.read_only = read_only;
        cx.notify();
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, content: impl Into<SharedString>, cx: &mut Context<Self>) {
        let content = content.into();
        let end = content.len();
        self.content = content;
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
        cx.emit(ComposerInputEvent::Edited);
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.expanded = false;
        self.set_content("", cx);
    }

    pub fn focus(&self, window: &mut Window) {
        self.focus_handle.focus(window);
    }

    fn accepts_input(&self) -> bool {
        self.enabled && !self.read_only
    }

    fn show_expand_control(&self) -> bool {
        self.visual_line_count > COMPOSER_MAX_LINES_COLLAPSED
    }

    fn container_height(&self, line_height: Pixels) -> Pixels {
        line_height * self.visual_line_count.min(visible_line_cap(self.expanded)) as f32
    }

    fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        if self.expanded {
            self.scroll.scroll_to_bottom();
        }
        cx.notify();
    }

    fn ensure_cursor_visible(&mut self) {
        let Some(layout) = self.last_layout.as_ref() else {
            return;
        };
        let line_height = self.line_height;
        if line_height <= px(0.) {
            return;
        }
        let cursor = self.cursor_offset();
        let cursor_pos = layout.cursor_position(cursor);
        let visible_height = self.container_height(line_height);
        let scroll_y = self.scroll.offset().y;
        let visible_top = -scroll_y;
        let cursor_bottom = cursor_pos.y + line_height;

        if cursor_bottom > visible_top + visible_height {
            self.scroll
                .set_offset(point(px(0.), visible_height - cursor_bottom));
        } else if cursor_pos.y < visible_top {
            self.scroll.set_offset(point(px(0.), -cursor_pos.y));
        }
    }

    fn after_content_change(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.visual_line_count <= COMPOSER_MAX_LINES_COLLAPSED {
            self.expanded = false;
        }
        self.ensure_cursor_visible();
        cx.notify();
        cx.emit(ComposerInputEvent::Edited);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled || self.read_only {
            return;
        }
        let text = self.content.to_string();
        if !text.trim().is_empty() {
            cx.emit(ComposerInputEvent::Submit(text));
        }
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        self.replace_text_in_range(None, "\n", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.enabled {
            return;
        }
        self.focus(window);
        cx.emit(ComposerInputEvent::Focus);
        self.is_selecting = true;
        let index = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(layout)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return self.content.len();
        };

        let scroll_y = self.scroll.offset().y;
        let content_point = point(
            position.x - bounds.left(),
            position.y - bounds.top() - scroll_y,
        );
        layout.index_for_point(content_point, self.line_height)
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content[..offset]
            .char_indices()
            .last()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .char_indices()
            .find(|(index, _)| *index > offset)
            .map(|(index, _)| index)
            .unwrap_or(self.content.len())
    }
}

impl EntityInputHandler for ComposerInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        if !self.accepts_input() {
            return None;
        }
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if !self.accepts_input() {
            return None;
        }
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.accepts_input() {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range = None;
        self.after_content_change(window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.accepts_input() {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.after_content_change(window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let line_height = window.line_height();
        let scroll_y = self.scroll.offset().y;
        last_layout.bounds_for_range(range, line_height).map(|inner| {
            Bounds::from_corners(
                point(bounds.left() + inner.origin.x, bounds.top() + inner.origin.y + scroll_y),
                point(
                    bounds.left() + inner.bottom_right().x,
                    bounds.top() + inner.bottom_right().y + scroll_y,
                ),
            )
        })
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&position)?;
        let scroll_y = self.scroll.offset().y;
        let content_point = point(line_point.x, line_point.y - scroll_y);
        let layout = self.last_layout.as_ref()?;
        Some(self.offset_to_utf16(
            layout.index_for_point(content_point, self.line_height),
        ))
    }
}

struct ComposerInputElement {
    input: Entity<ComposerInput>,
}

struct PrepaintState {
    cursor: Option<PaintQuad>,
}

impl IntoElement for ComposerInputElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ComposerInputElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let height = input.content_height.max(window.line_height());
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = height.into();
        style.flex_grow = 1.;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let cursor = input.cursor_offset();
        let (display_text, color) = if input.content.is_empty() {
            (input.placeholder.clone(), TEXT_MUTED.into())
        } else {
            (input.content.clone(), TEXT.into())
        };

        let wrap_width = bounds.size.width.max(px(1.));
        let layout = ComposerTextLayout::shape(window, display_text, color, wrap_width);
        let line_height = window.line_height();
        let visual_line_count = layout.visual_line_count();
        let content_height = layout.content_height().max(line_height);
        let cursor_pos = layout.cursor_position(cursor.min(input.content.len()));
        let scroll_y = input.scroll.offset().y;

        let cursor_quad = if input.focus_handle.is_focused(window) && input.accepts_input() {
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_pos.x, bounds.top() + cursor_pos.y + scroll_y),
                    gpui::size(px(2.), line_height),
                ),
                ACCENT,
            ))
        } else {
            None
        };

        let needs_relayout = input.content_height != content_height
            || input.visual_line_count != visual_line_count
            || input
                .last_layout
                .as_ref()
                .is_some_and(|last| last.wrap_width() != wrap_width);

        self.input.update(cx, |input, cx| {
            input.last_layout = Some(layout);
            input.last_bounds = Some(bounds);
            input.content_height = content_height;
            input.visual_line_count = visual_line_count;
            input.line_height = line_height;
            if needs_relayout {
                cx.notify();
            }
        });

        PrepaintState { cursor: cursor_quad }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        let layout = self.input.read(cx).last_layout.clone();
        if let Some(layout) = layout {
            let scroll_y = self.input.read(cx).scroll.offset().y;
            let origin = point(bounds.origin.x, bounds.origin.y + scroll_y);
            layout.paint(origin, bounds, window, cx).ok();
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}

impl Render for ComposerInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let line_height = window.line_height();
        let container_height = self.container_height(line_height);
        let needs_scroll = self
            .last_layout
            .as_ref()
            .is_some_and(|layout| layout.needs_scroll(self.expanded));
        let show_expand = self.show_expand_control();
        let expanded = self.expanded;

        let mut root = div()
            .key_context("ComposerInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::paste))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .relative();

        if show_expand {
            let expand_icon = if expanded {
                icon_path::MINIMIZE_2
            } else {
                icon_path::MAXIMIZE_2
            };
            root = root.child(
                div()
                    .id("composer-expand")
                    .absolute()
                    .top(px(0.))
                    .right(px(0.))
                    .p(px(4.))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|style| style.bg(BG_MAIN))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_expanded(cx)))
                    .child(icon(expand_icon, px(14.), TEXT_MUTED)),
            );
        }

        let input_child = ComposerInputElement {
            input: cx.entity(),
        };
        let scroll_container = if needs_scroll {
            div()
                .id("composer-input-scroll")
                .w_full()
                .min_h(line_height)
                .max_h(container_height)
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .when(show_expand, |el| el.pr(px(22.)))
                .child(input_child)
        } else {
            div()
                .id("composer-input")
                .w_full()
                .min_h(line_height)
                .max_h(container_height)
                .when(show_expand, |el| el.pr(px(22.)))
                .child(input_child)
        };

        root.child(scroll_container)
    }
}

impl Focusable for ComposerInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<ComposerInputEvent> for ComposerInput {}
