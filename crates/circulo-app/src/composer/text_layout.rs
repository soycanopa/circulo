//! Wrapped multiline layout for the composer input.

use std::ops::Range;

use gpui::{
    point, px, App, Bounds, Pixels, Point, Result, SharedString, TextRun, Window, WrapBoundary,
    WrappedLine,
};

pub const COMPOSER_MIN_LINES: usize = 1;
pub const COMPOSER_MAX_LINES_COLLAPSED: usize = 5;
pub const COMPOSER_MAX_LINES_EXPANDED: usize = 10;

#[derive(Clone, Debug)]
struct VisualLineSegment {
    range: Range<usize>,
    y: Pixels,
}

#[derive(Clone, Debug)]
pub struct ComposerTextLayout {
    lines: Vec<WrappedLine>,
    visual_lines: Vec<VisualLineSegment>,
    content_height: Pixels,
    wrap_width: Pixels,
}

impl ComposerTextLayout {
    pub fn shape(
        window: &mut Window,
        text: SharedString,
        color: gpui::Hsla,
        wrap_width: Pixels,
    ) -> Self {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let run = TextRun {
            len: text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let lines: Vec<WrappedLine> = window
            .text_system()
            .shape_text(text, font_size, &[run], Some(wrap_width), None)
            .map(|lines| lines.into_vec())
            .unwrap_or_default();

        let line_height = window.line_height();
        let mut visual_lines = Vec::new();
        let mut content_height = px(0.);
        let mut line_start = 0;
        let mut y = px(0.);

        for wrapped_line in &lines {
            let line_end = line_start + wrapped_line.len();
            let boundaries = wrapped_line.wrap_boundaries();
            let mut segment_start = 0usize;

            for (boundary_ix, boundary) in boundaries.iter().enumerate() {
                let segment_end = byte_index_at_boundary(wrapped_line, boundary);
                visual_lines.push(VisualLineSegment {
                    range: (line_start + segment_start)..(line_start + segment_end),
                    y,
                });
                y += line_height;
                segment_start = segment_end;
                let _ = boundary_ix;
            }

            visual_lines.push(VisualLineSegment {
                range: (line_start + segment_start)..line_end,
                y,
            });
            y += line_height;

            content_height += wrapped_line.size(line_height).height;
            line_start = line_end + 1;
        }

        if visual_lines.is_empty() {
            visual_lines.push(VisualLineSegment {
                range: 0..0,
                y: px(0.),
            });
            content_height = line_height;
        }

        Self {
            lines,
            visual_lines,
            content_height,
            wrap_width,
        }
    }

    pub fn wrap_width(&self) -> Pixels {
        self.wrap_width
    }

    pub fn content_height(&self) -> Pixels {
        self.content_height
    }

    pub fn visual_line_count(&self) -> usize {
        self.visual_lines.len().max(COMPOSER_MIN_LINES)
    }

    pub fn needs_scroll(&self, expanded: bool) -> bool {
        self.visual_line_count() > visible_line_cap(expanded)
    }

    pub fn paint(
        &self,
        origin: Point<Pixels>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        let line_height = window.line_height();
        let mut line_origin = origin;
        let text_style = window.text_style();

        for wrapped_line in &self.lines {
            wrapped_line
                .paint_background(
                    line_origin,
                    line_height,
                    text_style.text_align,
                    Some(bounds),
                    window,
                    cx,
                )
                .ok();
            wrapped_line
                .paint(
                    line_origin,
                    line_height,
                    text_style.text_align,
                    Some(bounds),
                    window,
                    cx,
                )
                .ok();
            line_origin.y += wrapped_line.size(line_height).height;
        }

        Ok(())
    }

    pub fn index_for_point(&self, point: Point<Pixels>, line_height: Pixels) -> usize {
        let y = point.y;
        for segment in &self.visual_lines {
            if y >= segment.y && y < segment.y + line_height {
                let line_ix = segment_to_wrapped_line_index(self, segment);
                let line_start = line_byte_start(self, line_ix);
                let wrapped = &self.lines[line_ix];
                let local_index = wrapped
                    .unwrapped_layout
                    .closest_index_for_x(point.x);
                let byte_index = line_start + local_index;
                return byte_index.clamp(segment.range.start, segment.range.end);
            }
        }

        self.visual_lines
            .last()
            .map(|segment| segment.range.end)
            .unwrap_or(0)
    }

    pub fn cursor_position(&self, offset: usize) -> Point<Pixels> {
        let clamped = offset.min(total_content_len(&self.lines));
        for segment in &self.visual_lines {
            if clamped >= segment.range.start && clamped <= segment.range.end {
                let line_ix = segment_to_wrapped_line_index(self, segment);
                let line_start = line_byte_start(self, line_ix);
                let x = self.lines[line_ix]
                    .unwrapped_layout
                    .x_for_index(clamped.saturating_sub(line_start));
                return point(x, segment.y);
            }
        }

        let y = self
            .visual_lines
            .last()
            .map(|segment| segment.y)
            .unwrap_or(px(0.));
        point(px(0.), y)
    }

    pub fn bounds_for_range(
        &self,
        range: Range<usize>,
        line_height: Pixels,
    ) -> Option<Bounds<Pixels>> {
        let start = self.cursor_position(range.start);
        let end = self.cursor_position(range.end);
        Some(Bounds::from_corners(
            point(start.x, start.y),
            point(end.x.max(start.x + px(1.)), start.y + line_height),
        ))
    }
}

pub fn visible_line_cap(expanded: bool) -> usize {
    if expanded {
        COMPOSER_MAX_LINES_EXPANDED
    } else {
        COMPOSER_MAX_LINES_COLLAPSED
    }
}

fn byte_index_at_boundary(wrapped: &WrappedLine, boundary: &WrapBoundary) -> usize {
    wrapped
        .unwrapped_layout
        .runs
        .get(boundary.run_ix)
        .and_then(|run| run.glyphs.get(boundary.glyph_ix))
        .map(|glyph| glyph.index)
        .unwrap_or(wrapped.len())
}

fn line_byte_start(layout: &ComposerTextLayout, line_ix: usize) -> usize {
    let mut offset = 0;
    for (ix, wrapped) in layout.lines.iter().enumerate() {
        if ix == line_ix {
            return offset;
        }
        offset += wrapped.len() + 1;
    }
    offset
}

fn total_content_len(lines: &[WrappedLine]) -> usize {
    if lines.is_empty() {
        return 0;
    }
    let mut total = 0;
    for (index, line) in lines.iter().enumerate() {
        total += line.len();
        if index + 1 < lines.len() {
            total += 1;
        }
    }
    total
}

fn segment_to_wrapped_line_index(layout: &ComposerTextLayout, segment: &VisualLineSegment) -> usize {
    let mut line_start = 0;
    for (ix, wrapped) in layout.lines.iter().enumerate() {
        let line_end = line_start + wrapped.len();
        if segment.range.start >= line_start && segment.range.start <= line_end {
            return ix;
        }
        line_start = line_end + 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::{visible_line_cap, COMPOSER_MAX_LINES_COLLAPSED, COMPOSER_MAX_LINES_EXPANDED};

    #[test]
    fn visible_line_cap_matches_expanded_state() {
        assert_eq!(visible_line_cap(false), COMPOSER_MAX_LINES_COLLAPSED);
        assert_eq!(visible_line_cap(true), COMPOSER_MAX_LINES_EXPANDED);
    }
}
