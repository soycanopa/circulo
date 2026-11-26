//! Markdown in the transcript: the subset agents actually write, rendered with
//! GPUI text runs.
//!
//! The agent streams CommonMark, so `**bold**`, `` `code` ``, headings, lists
//! and fenced code must read as formatting, not as punctuation. Parsing is
//! `pulldown-cmark`; drawing is `StyledText` with one `TextRun` per styled span.
//! Runs, rather than `with_default_highlights`, because inline code needs a
//! different font family and highlights can only change weight, colour and
//! decorations.
//!
//! The input is partial while a turn streams. The builder never fails on
//! unbalanced input: an unclosed fence is still a code block, an unclosed
//! `**` stays literal text until its closing marker arrives.
//!
//! Tables are laid out as rows of weighted cells, so a two-column table with one
//! long column does not waste half the width.
//!
//! Footnotes parse into superscript references and a numbered list at the end.
//!
//! What is deliberately not supported yet: inline math images and raw HTML
//! (dropped). Display math (`$$…$$`) is laid out by RaTeX and painted as an
//! image; inline math (`$…$`) stays a mono chip with its source until there is
//! an inline layout. A link keeps its destination, and `InteractiveText`
//! hit-tests the byte ranges GPUI lays out, so clicking a link opens it in the
//! system browser.

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    AnyElement, Font, FontFeatures, FontStyle, FontWeight, Hsla, InteractiveText, IntoElement,
    SharedString, StrikethroughStyle, StyledText, TextRun, UnderlineStyle, div, prelude::*, px,
    relative,
};
use pulldown_cmark::{Event, Options, Tag, TagEnd};

use crate::theme::Theme;

/// The system UI face, as everywhere else in the app.
const UI_FAMILY: &str = ".SystemUIFont";
/// Inline code and code blocks. Menlo ships with macOS; GPUI falls back to the
/// system face if the family is missing.
const MONO_FAMILY: &str = "Menlo";

/// Body size and leading, shared with the plain text the transcript used to
/// paint, so a turn reads the same before and after formatting arrives.
const BODY_SIZE: f32 = 13.;
const BODY_LINE: f32 = 20.;
/// Space between two blocks of one message.
const BLOCK_GAP: f32 = 9.;
/// Space between two items of the same list: a list reads as one unit, not as
/// a run of paragraphs.
const ITEM_GAP: f32 = 4.;
/// Width of the marker column in a list, and the indent of one nested level.
const LIST_INDENT: f32 = 18.;

/// Inline formatting carried by one span of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Inline {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    pub link: bool,
    /// A footnote reference, painted in the accent colour. The number is kept
    /// here rather than parsed back out of the glyphs.
    pub note: Option<usize>,
}

/// A run of text with one inline style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub text: String,
    pub style: Inline,
    /// The URL a link span points at. `None` for ordinary text.
    pub link: Option<SharedString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    Bullet,
    Ordered(u64),
}

/// Column alignment from the table's delimiter row. CommonMark's "none" reads
/// as left, which is what a cell does by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
}

/// One block-level element. Everything a message contains lands in one list of
/// these, in document order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    Paragraph(Vec<Span>),
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    /// A list item. `marker` is `None` for the continuation paragraphs of an
    /// item, which are indented but carry no bullet or number.
    Item {
        depth: usize,
        marker: Option<Marker>,
        spans: Vec<Span>,
    },
    Quote(Vec<Span>),
    Code(String),
    Table {
        aligns: Vec<Align>,
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
    /// A footnote definition, painted like an ordered list item with its number.
    Footnote {
        number: usize,
        spans: Vec<Span>,
    },
    /// Display math (`$$…$$`), rendered as an image.
    Math(String),
    Rule,
}

/// The parsed form of one streamed text segment, kept between frames.
///
/// A finished message only changes when the window re-renders (scroll, hover,
/// resize), and re-parsing it every time is waste. The length is the version:
/// streamed text only ever grows, and the cache dies with the segment that owns
/// it, so a new session cannot read a stale parse.
#[derive(Default)]
pub(crate) struct Cached {
    seen: std::cell::Cell<usize>,
    blocks: std::cell::RefCell<Option<Rc<Vec<Block>>>>,
}

impl Cached {
    pub(crate) fn blocks(&self, text: &str) -> Rc<Vec<Block>> {
        if let Some(blocks) = self.blocks.borrow().as_ref()
            && self.seen.get() == text.len()
        {
            return blocks.clone();
        }

        let blocks = Rc::new(blocks(text));
        *self.blocks.borrow_mut() = Some(blocks.clone());
        self.seen.set(text.len());
        blocks
    }
}

/// Paint parsed blocks.
pub(crate) fn markdown(blocks: &[Block], theme: &Theme) -> AnyElement {
    let mut column = div().w_full().min_w(px(0.)).flex().flex_col();
    let mut previous = None;
    let mut first = true;
    // One counter per rendered segment: two identical links in different
    // paragraphs still get distinct element ids, so their hit-test state
    // cannot collide.
    let mut link_ids = 0u64;

    for block in blocks {
        let item = matches!(block, Block::Item { .. });
        let top = if first {
            0.
        } else if item && previous == Some(true) {
            ITEM_GAP
        } else {
            BLOCK_GAP
        };
        first = false;
        previous = Some(item);

        // The margin lives on a wrapper so every block kind can use it without
        // repeating the call.
        column = column.child(div().flex_shrink_0().mt(px(top)).child(block_view(
            block,
            theme,
            &mut link_ids,
        )));
    }

    column.into_any_element()
}

fn block_view(block: &Block, theme: &Theme, links: &mut u64) -> AnyElement {
    match block {
        Block::Paragraph(spans) => text_block(
            spans,
            theme,
            theme.text,
            FontWeight::NORMAL,
            BODY_SIZE,
            BODY_LINE,
            links,
        ),
        Block::Heading { level, spans } => {
            let (size, line) = match level {
                1 => (17., 24.),
                2 => (15.5, 22.),
                _ => (14., 21.),
            };
            text_block(
                spans,
                theme,
                theme.text,
                FontWeight::SEMIBOLD,
                size,
                line,
                links,
            )
        }
        Block::Item {
            depth,
            marker,
            spans,
        } => {
            let indent = LIST_INDENT * depth.saturating_sub(1) as f32;

            div()
                .w_full()
                .min_w(px(0.))
                .flex()
                .pl(px(indent))
                .child(
                    // A fixed-width slot keeps the text of every item aligned,
                    // wrapped lines included.
                    div()
                        .w(px(LIST_INDENT))
                        .flex_shrink_0()
                        .text_color(theme.text_muted)
                        .text_size(px(BODY_SIZE))
                        .line_height(px(BODY_LINE))
                        .child(marker_text(*marker, *depth)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(BODY_SIZE))
                        .line_height(px(BODY_LINE))
                        .child(styled_element(
                            spans,
                            theme,
                            theme.text,
                            FontWeight::NORMAL,
                            links,
                        )),
                )
                .into_any_element()
        }
        Block::Quote(spans) => div()
            .w_full()
            .min_w(px(0.))
            .border_l(px(2.))
            .border_color(theme.hairline)
            .pl(px(12.))
            .child(
                div()
                    .text_size(px(BODY_SIZE))
                    .line_height(px(BODY_LINE))
                    .child(styled_element(
                        spans,
                        theme,
                        theme.text_muted,
                        FontWeight::NORMAL,
                        links,
                    )),
            )
            .into_any_element(),
        Block::Code(text) => div()
            .w_full()
            .min_w(px(0.))
            .rounded(px(8.))
            .bg(theme.chip)
            .px(px(12.))
            .py(px(9.))
            .text_size(px(12.))
            .line_height(px(18.))
            .font_family(MONO_FAMILY)
            .text_color(theme.text)
            // Newlines break lines in GPUI text, so the block needs no runs.
            .child(StyledText::new(text))
            .into_any_element(),
        Block::Table { aligns, head, rows } => table(aligns, head, rows, theme, links),
        Block::Footnote { number, spans } => div()
            .w_full()
            .min_w(px(0.))
            .flex()
            .child(
                div()
                    .w(px(LIST_INDENT))
                    .flex_shrink_0()
                    .text_size(px(12.))
                    .line_height(px(BODY_LINE))
                    .text_color(theme.text_faint)
                    .child(SharedString::from(format!("{number}."))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(12.))
                    .line_height(px(BODY_LINE))
                    .text_color(theme.text_muted)
                    .child(styled_element(
                        spans,
                        theme,
                        theme.text_muted,
                        FontWeight::NORMAL,
                        links,
                    )),
            )
            .into_any_element(),
        Block::Math(source) => crate::math::view(source, theme),
        Block::Rule => div()
            .w_full()
            .h(px(1.))
            .my(px(2.))
            .bg(theme.hairline)
            .into_any_element(),
    }
}

/// A table, drawn as rows of cells. Every cell of a column gets the same
/// flex basis, weighted by the longest text the column holds, so a two-column
/// table with one long column does not waste half the width on the short one.
fn table(
    aligns: &[Align],
    head: &[Vec<Span>],
    rows: &[Vec<Vec<Span>>],
    theme: &Theme,
    links: &mut u64,
) -> AnyElement {
    let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return div().into_any_element();
    }

    // Weight per column: the longest cell text, floored so a narrow "N" column
    // still gets a readable share.
    let mut weights = vec![6_usize; columns];
    for row in std::iter::once(head).chain(rows.iter().map(Vec::as_slice)) {
        for (index, cell) in row.iter().enumerate() {
            let length: usize = cell.iter().map(|span| span.text.chars().count()).sum();
            weights[index] = weights[index].max(length);
        }
    }
    let total: usize = weights.iter().sum();
    let ratios: Vec<f32> = weights
        .iter()
        .map(|weight| *weight as f32 / total as f32)
        .collect();

    let mut grid = div()
        .w_full()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .border_t(px(1.))
        .border_color(theme.hairline);

    if !head.is_empty() {
        grid = grid.child(table_row(
            head,
            &ratios,
            aligns,
            theme,
            FontWeight::SEMIBOLD,
            links,
        ));
    }
    for row in rows {
        grid = grid.child(table_row(
            row,
            &ratios,
            aligns,
            theme,
            FontWeight::NORMAL,
            links,
        ));
    }

    grid.into_any_element()
}

fn table_row(
    cells: &[Vec<Span>],
    ratios: &[f32],
    aligns: &[Align],
    theme: &Theme,
    weight: FontWeight,
    links: &mut u64,
) -> AnyElement {
    let mut row = div()
        .w_full()
        .min_w(px(0.))
        .flex()
        .border_b(px(1.))
        .border_color(theme.hairline);

    for (index, ratio) in ratios.iter().enumerate() {
        let align = aligns.get(index).copied().unwrap_or(Align::Left);
        let text_align = match align {
            Align::Left => gpui::TextAlign::Left,
            Align::Center => gpui::TextAlign::Center,
            Align::Right => gpui::TextAlign::Right,
        };

        row = row.child(
            div()
                .flex_basis(relative(*ratio))
                .flex_grow()
                .min_w(px(0.))
                .px(px(8.))
                .py(px(5.))
                .text_size(px(12.5))
                .line_height(px(18.))
                .text_align(text_align)
                .child(styled_element(
                    cells.get(index).map(Vec::as_slice).unwrap_or(&[]),
                    theme,
                    theme.text,
                    weight,
                    links,
                )),
        );
    }

    row.into_any_element()
}

fn text_block(
    spans: &[Span],
    theme: &Theme,
    color: gpui::Rgba,
    weight: FontWeight,
    size: f32,
    line: f32,
    links: &mut u64,
) -> AnyElement {
    div()
        .w_full()
        .min_w(px(0.))
        .text_size(px(size))
        .line_height(px(line))
        .child(styled_element(spans, theme, color, weight, links))
        .into_any_element()
}

/// Superscript digits for a footnote reference. A bare `[1]` breaks across a
/// line: GPUI treats a closing bracket as a break opportunity.
fn superscript(number: usize) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let digits: String = number
        .to_string()
        .chars()
        .filter_map(|digit| digit.to_digit(10).map(|digit| DIGITS[digit as usize]))
        .collect();
    digits
}

/// Every span vector a block owns, for the passes that rewrite text after the
/// parse (today: dropping notes that were never defined).
fn block_spans_mut(block: &mut Block) -> Vec<&mut Vec<Span>> {
    match block {
        Block::Paragraph(spans) | Block::Quote(spans) | Block::Footnote { spans, .. } => {
            vec![spans]
        }
        Block::Heading { spans, .. } => vec![spans],
        Block::Item { spans, marker, .. } => {
            let _ = marker;
            vec![spans]
        }
        Block::Table { head, rows, .. } => {
            head.iter_mut().chain(rows.iter_mut().flatten()).collect()
        }
        Block::Code(_) | Block::Math(_) | Block::Rule => Vec::new(),
    }
}

fn align_of(align: pulldown_cmark::Alignment) -> Align {
    match align {
        pulldown_cmark::Alignment::Center => Align::Center,
        pulldown_cmark::Alignment::Right => Align::Right,
        pulldown_cmark::Alignment::None | pulldown_cmark::Alignment::Left => Align::Left,
    }
}

fn marker_text(marker: Option<Marker>, depth: usize) -> SharedString {
    match marker {
        Some(Marker::Ordered(number)) => SharedString::from(format!("{number}.")),
        Some(Marker::Bullet) => SharedString::from(if depth > 1 { "◦" } else { "•" }),
        None => SharedString::new_static(""),
    }
}

/// Build the run-per-span text and the byte ranges that belong to links.
/// Adjacent runs of the same link merge, so one phrase is one range.
fn styled_parts(
    spans: &[Span],
    theme: &Theme,
    color: gpui::Rgba,
    weight: FontWeight,
) -> (StyledText, Vec<(Range<usize>, SharedString)>) {
    let mut plain = String::new();
    let mut runs = Vec::with_capacity(spans.len());
    let mut links: Vec<(Range<usize>, SharedString)> = Vec::new();

    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        let start = plain.len();
        plain.push_str(&span.text);
        if let Some(target) = &span.link {
            if let Some((range, last)) = links.last_mut()
                && range.end == start
                && last == target
            {
                range.end = plain.len();
            } else {
                links.push((start..plain.len(), target.clone()));
            }
        }
        runs.push(text_run(span, theme, color, weight));
    }

    (StyledText::new(plain).with_runs(runs), links)
}

/// The same text, clickable when it contains links. `InteractiveText` hit-tests
/// the byte ranges GPUI already lays out, so a link opens where it is painted;
/// plain text keeps the smaller `StyledText` element.
fn styled_element(
    spans: &[Span],
    theme: &Theme,
    color: gpui::Rgba,
    weight: FontWeight,
    links: &mut u64,
) -> AnyElement {
    let (text, link_ranges) = styled_parts(spans, theme, color, weight);
    if link_ranges.is_empty() {
        return text.into_any_element();
    }

    let id = SharedString::from(format!("md-links-{}", *links));
    *links += 1;

    let ranges: Vec<Range<usize>> = link_ranges.iter().map(|(range, _)| range.clone()).collect();
    let targets: Vec<SharedString> = link_ranges.into_iter().map(|(_, target)| target).collect();

    InteractiveText::new(id, text)
        .on_click(ranges, move |index, _window, cx| {
            if let Some(url) = targets.get(index) {
                cx.open_url(url.as_ref());
            }
        })
        .into_any_element()
}

fn text_run(span: &Span, theme: &Theme, color: gpui::Rgba, weight: FontWeight) -> TextRun {
    let style = span.style;
    let color: Hsla = if style.link || style.note.is_some() {
        theme.accent.into()
    } else {
        color.into()
    };

    TextRun {
        len: span.text.len(),
        font: Font {
            family: if style.code { MONO_FAMILY } else { UI_FAMILY }.into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: if style.bold { FontWeight::BOLD } else { weight },
            style: if style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
        },
        color,
        // Only inline code gets a chip; a code block paints its own surface.
        background_color: style.code.then(|| theme.chip.into()),
        underline: style.link.then(|| UnderlineStyle {
            thickness: px(1.),
            color: Some(color),
            wavy: false,
        }),
        strikethrough: style.strike.then(|| StrikethroughStyle {
            thickness: px(1.),
            color: Some(color),
        }),
    }
}

/// Parse CommonMark into blocks, with the GitHub extensions agents write:
/// tables, strikethrough, task lists and callout quotes.
pub(crate) fn blocks(text: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
        | Options::ENABLE_MATH;
    let mut builder = Builder::default();

    for event in pulldown_cmark::Parser::new_ext(text, options) {
        builder.event(event);
    }
    builder.finish();

    builder.blocks
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    /// Inline content of the block being built.
    spans: Vec<Span>,
    /// Formatting that applies to the next span.
    style: Inline,
    /// The link target that applies to the next span, when inside a link.
    link: Option<SharedString>,
    /// One entry per open list, innermost last.
    lists: Vec<ListState>,
    /// One entry per open list item, innermost last.
    items: Vec<ItemState>,
    quote: usize,
    code: Option<String>,
    heading: Option<u8>,
    /// The table being built, if the parser is inside one.
    table: Option<TableState>,
    /// Footnote labels in the order they were first referenced. The position
    /// plus one is the number both the reference and the definition paint.
    notes: Vec<NoteEntry>,
    /// The footnote definition being collected.
    footnote: Option<FootnoteState>,
}

struct FootnoteState {
    number: usize,
    spans: Vec<Span>,
}

struct NoteEntry {
    label: String,
    defined: bool,
}

#[derive(Default)]
struct TableState {
    aligns: Vec<Align>,
    head: Vec<Vec<Span>>,
    rows: Vec<Vec<Vec<Span>>>,
    row: Vec<Vec<Span>>,
}

struct ListState {
    ordered: bool,
    next: u64,
}

struct ItemState {
    depth: usize,
    marker: Marker,
    /// Whether this item already painted its marker.
    marked: bool,
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => {
                if let Some(code) = &mut self.code {
                    code.push_str(&text);
                } else {
                    self.push_span(&text, self.style);
                }
            }
            Event::Code(code) => {
                let style = Inline {
                    code: true,
                    ..self.style
                };
                self.push_span(&code, style);
            }
            Event::SoftBreak => self.break_line(" "),
            Event::HardBreak => self.break_line("\n"),
            Event::Rule => {
                self.flush();
                self.blocks.push(Block::Rule);
            }
            Event::TaskListMarker(checked) => {
                self.push_span(if checked { "☑ " } else { "☐ " }, self.style);
            }
            Event::InlineMath(text) => {
                // GPUI cannot put an image inside a text run; until there is an
                // inline layout, the TeX source is an honest mono chip.
                let style = Inline {
                    code: true,
                    ..self.style
                };
                self.push_span(&format!("${text}$"), style);
            }
            Event::DisplayMath(text) => {
                // A display equation is its own block: flush the paragraph in
                // progress and let the renderer draw an image.
                self.flush();
                self.blocks.push(Block::Math(text.trim().to_owned()));
            }
            // Raw HTML is dropped rather than shown as markup: an agent's
            // `<br>` is noise, not content.
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::FootnoteReference(label) => {
                let number = self.note_number(&label);
                let style = Inline {
                    note: Some(number),
                    ..self.style
                };
                self.push_span(&superscript(number), style);
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => self.heading = Some(level as u8),
            Tag::BlockQuote(_) => self.quote += 1,
            Tag::CodeBlock(_) => self.code = Some(String::new()),
            Tag::Table(alignment) => {
                self.table = Some(TableState {
                    aligns: alignment.iter().map(|align| align_of(*align)).collect(),
                    ..TableState::default()
                })
            }
            Tag::FootnoteDefinition(label) => {
                let number = self.note_number(&label);
                self.notes[number - 1].defined = true;
                self.footnote = Some(FootnoteState {
                    number,
                    spans: Vec::new(),
                });
            }
            Tag::TableHead => {}
            Tag::TableRow => {
                if let Some(table) = &mut self.table {
                    table.row.clear();
                }
            }
            Tag::TableCell => {}
            Tag::List(start) => self.lists.push(ListState {
                ordered: start.is_some(),
                next: start.unwrap_or(1),
            }),
            Tag::Item => {
                // A tight list gives no paragraph wrapper, so text of the
                // enclosing item can still be buffered when a nested item
                // starts. It belongs to the enclosing item, not to this one.
                self.flush();

                let depth = self.lists.len();
                let marker = match self.lists.last_mut() {
                    Some(list) if list.ordered => {
                        let number = list.next;
                        list.next += 1;
                        Marker::Ordered(number)
                    }
                    _ => Marker::Bullet,
                };
                self.items.push(ItemState {
                    depth,
                    marker,
                    marked: false,
                });
            }
            Tag::Emphasis => self.style.italic = true,
            Tag::Strong => self.style.bold = true,
            Tag::Strikethrough => self.style.strike = true,
            Tag::Link { dest_url, .. } => {
                self.style.link = true;
                self.link = Some(SharedString::from(dest_url.to_string()));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.flush(),
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.blocks.push(Block::Code(code));
                }
            }
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::TableCell => {
                if let Some(table) = &mut self.table {
                    table.row.push(std::mem::take(&mut self.spans));
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = &mut self.table {
                    // A header carries its cells without a row tag of its own.
                    table.head = std::mem::take(&mut table.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    let row = std::mem::take(&mut table.row);
                    table.rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.blocks.push(Block::Table {
                        aligns: table.aligns,
                        head: table.head,
                        rows: table.rows,
                    });
                }
            }
            TagEnd::FootnoteDefinition => {
                if let Some(footnote) = self.footnote.take() {
                    self.blocks.push(Block::Footnote {
                        number: footnote.number,
                        spans: footnote.spans,
                    });
                }
            }
            TagEnd::Item => {
                // A tight list has no paragraph wrapper, so text still sitting
                // in the buffer belongs to the item that is closing.
                self.flush();
                self.items.pop();
            }
            TagEnd::Emphasis => self.style.italic = false,
            TagEnd::Strong => self.style.bold = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link => {
                self.style.link = false;
                self.link = None;
            }
            _ => {}
        }
    }

    /// A line break inside the current block. In a code block it stays a
    /// newline; in prose a hard break becomes `\n`, which GPUI paints as a line
    /// break, and a soft break collapses to a space as CommonMark asks.
    fn break_line(&mut self, text: &str) {
        if let Some(code) = &mut self.code {
            code.push_str(text);
            return;
        }
        self.push_span(text, self.style);
    }

    fn push_span(&mut self, text: &str, style: Inline) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.style == style
            && last.link == self.link
        {
            last.text.push_str(text);
            return;
        }
        self.spans.push(Span {
            text: text.to_owned(),
            style,
            link: self.link.clone(),
        });
    }

    /// Turn the buffered inline content into the block it belongs to.
    fn flush(&mut self) {
        if self.spans.is_empty() {
            return;
        }

        let spans = std::mem::take(&mut self.spans);

        if let Some(footnote) = &mut self.footnote {
            // A definition may hold several paragraphs; they read as one note.
            if !footnote.spans.is_empty() {
                footnote.spans.push(Span {
                    text: " ".into(),
                    style: Inline::default(),
                    link: None,
                });
            }
            footnote.spans.extend(spans);
            return;
        }

        if let Some(level) = self.heading.take() {
            self.blocks.push(Block::Heading { level, spans });
            return;
        }

        if let Some(item) = self.items.last_mut() {
            let marker = if item.marked {
                None
            } else {
                item.marked = true;
                Some(item.marker)
            };
            let depth = item.depth;
            self.blocks.push(Block::Item {
                depth,
                marker,
                spans,
            });
            return;
        }

        if self.quote > 0 {
            self.blocks.push(Block::Quote(spans));
            return;
        }

        self.blocks.push(Block::Paragraph(spans));
    }

    /// The number a footnote label paints as: its place in the reference order,
    /// one-based. A definition that was never referenced takes the next number,
    /// so a note still renders instead of vanishing.
    fn note_number(&mut self, label: &str) -> usize {
        match self.notes.iter().position(|note| note.label == label) {
            Some(index) => index + 1,
            None => {
                self.notes.push(NoteEntry {
                    label: label.to_owned(),
                    defined: false,
                });
                self.notes.len()
            }
        }
    }

    /// A reference with no definition is not a note: GitHub leaves the literal
    /// `[^label]` in place, so a dangling number never points at nothing.
    fn drop_dangling_notes(&mut self) {
        if self.notes.iter().all(|note| note.defined) {
            return;
        }

        for block in &mut self.blocks {
            for spans in block_spans_mut(block) {
                for span in spans {
                    let Some(number) = span.style.note else {
                        continue;
                    };
                    if let Some(note) = self.notes.get(number.wrapping_sub(1))
                        && !note.defined
                    {
                        span.text = format!("[^{}]", note.label);
                        span.style.note = None;
                    }
                }
            }
        }
    }

    fn finish(&mut self) {
        self.flush();
        self.drop_dangling_notes();
        self.notes_to_the_end();
    }

    /// A definition may sit anywhere in the message; notes read as a block at
    /// the end, in reference order, the way CommonMark renderers show them.
    fn notes_to_the_end(&mut self) {
        let mut body = Vec::with_capacity(self.blocks.len());
        let mut notes = Vec::new();

        for block in std::mem::take(&mut self.blocks) {
            match block {
                Block::Footnote { .. } => notes.push(block),
                other => body.push(other),
            }
        }

        notes.sort_by_key(|block| match block {
            Block::Footnote { number, .. } => *number,
            _ => usize::MAX,
        });

        body.extend(notes);
        self.blocks = body;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inline(text: &str, style: Inline) -> Span {
        Span {
            text: text.to_owned(),
            style,
            link: None,
        }
    }

    fn plain(text: &str) -> Span {
        inline(text, Inline::default())
    }

    #[test]
    fn strong_and_inline_code_become_spans() {
        assert_eq!(
            blocks("a **bold** and `code` end"),
            vec![Block::Paragraph(vec![
                plain("a "),
                inline(
                    "bold",
                    Inline {
                        bold: true,
                        ..Inline::default()
                    }
                ),
                plain(" and "),
                inline(
                    "code",
                    Inline {
                        code: true,
                        ..Inline::default()
                    }
                ),
                plain(" end"),
            ])]
        );
    }

    #[test]
    fn emphasis_is_italic() {
        assert_eq!(
            blocks("*soft* text"),
            vec![Block::Paragraph(vec![
                inline(
                    "soft",
                    Inline {
                        italic: true,
                        ..Inline::default()
                    }
                ),
                plain(" text"),
            ])]
        );
    }

    #[test]
    fn nested_formatting_keeps_both_styles() {
        assert_eq!(
            blocks("**a `b` c**"),
            vec![Block::Paragraph(vec![
                inline(
                    "a ",
                    Inline {
                        bold: true,
                        ..Inline::default()
                    }
                ),
                inline(
                    "b",
                    Inline {
                        bold: true,
                        code: true,
                        ..Inline::default()
                    }
                ),
                inline(
                    " c",
                    Inline {
                        bold: true,
                        ..Inline::default()
                    }
                ),
            ])]
        );
    }

    #[test]
    fn headings_carry_their_level() {
        assert_eq!(
            blocks("# Title\n\ntext"),
            vec![
                Block::Heading {
                    level: 1,
                    spans: vec![plain("Title")],
                },
                Block::Paragraph(vec![plain("text")]),
            ]
        );
    }

    #[test]
    fn bullet_lists_record_their_depth() {
        assert_eq!(
            blocks("- one\n- two\n  - deep"),
            vec![
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("one")],
                },
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("two")],
                },
                Block::Item {
                    depth: 2,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("deep")],
                },
            ]
        );
    }

    #[test]
    fn ordered_lists_number_from_their_start() {
        assert_eq!(
            blocks("3. three\n4. four"),
            vec![
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Ordered(3)),
                    spans: vec![plain("three")],
                },
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Ordered(4)),
                    spans: vec![plain("four")],
                },
            ]
        );
    }

    #[test]
    fn a_fenced_block_keeps_its_lines_and_ignores_the_language() {
        assert_eq!(
            blocks("```rust\nlet a = 1;\nlet b = 2;\n```"),
            vec![Block::Code("let a = 1;\nlet b = 2;\n".to_owned())]
        );
    }

    #[test]
    fn an_unclosed_fence_is_still_a_code_block() {
        // What the transcript sees mid-stream: the closing marker has not
        // arrived yet.
        assert_eq!(
            blocks("```\nlet a = 1;"),
            vec![Block::Code("let a = 1;".to_owned())]
        );
    }

    #[test]
    fn an_unclosed_marker_stays_literal() {
        assert_eq!(
            blocks("still **open"),
            vec![Block::Paragraph(vec![plain("still **open")])]
        );
    }

    #[test]
    fn a_rule_is_its_own_block() {
        assert_eq!(
            blocks("above\n\n---\n\nbelow"),
            vec![
                Block::Paragraph(vec![plain("above")]),
                Block::Rule,
                Block::Paragraph(vec![plain("below")]),
            ]
        );
    }

    #[test]
    fn soft_breaks_join_and_hard_breaks_split() {
        assert_eq!(
            blocks("one\ntwo  \nthree"),
            vec![Block::Paragraph(vec![plain("one two\nthree")])]
        );
    }

    #[test]
    fn a_quote_keeps_its_spans() {
        assert_eq!(
            blocks("> **note**"),
            vec![Block::Quote(vec![inline(
                "note",
                Inline {
                    bold: true,
                    ..Inline::default()
                }
            )])]
        );
    }

    #[test]
    fn a_continuation_paragraph_has_no_second_marker() {
        assert_eq!(
            blocks("- first\n\n  more"),
            vec![
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("first")],
                },
                Block::Item {
                    depth: 1,
                    marker: None,
                    spans: vec![plain("more")],
                },
            ]
        );
    }

    #[test]
    fn task_list_markers_are_text() {
        assert_eq!(
            blocks("- [x] done\n- [ ] todo"),
            vec![
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("☑ done")],
                },
                Block::Item {
                    depth: 1,
                    marker: Some(Marker::Bullet),
                    spans: vec![plain("☐ todo")],
                },
            ]
        );
    }

    #[test]
    fn a_table_becomes_a_head_and_rows() {
        assert_eq!(
            blocks("| Crate | Owns |\n|---|---|\n| `core` | sessions |"),
            vec![Block::Table {
                aligns: vec![Align::Left, Align::Left],
                head: vec![vec![plain("Crate")], vec![plain("Owns")]],
                rows: vec![vec![
                    vec![inline(
                        "core",
                        Inline {
                            code: true,
                            ..Inline::default()
                        }
                    )],
                    vec![plain("sessions")],
                ]],
            }]
        );
    }

    #[test]
    fn a_table_keeps_its_column_alignment() {
        assert_eq!(
            blocks("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |"),
            vec![Block::Table {
                aligns: vec![Align::Left, Align::Center, Align::Right],
                head: vec![vec![plain("a")], vec![plain("b")], vec![plain("c")]],
                rows: vec![vec![vec![plain("1")], vec![plain("2")], vec![plain("3")]]],
            }]
        );
    }

    #[test]
    fn a_table_cell_carries_inline_formatting() {
        assert_eq!(
            blocks("| **bold** |\n|---|\n| `code` |"),
            vec![Block::Table {
                aligns: vec![Align::Left],
                head: vec![vec![inline(
                    "bold",
                    Inline {
                        bold: true,
                        ..Inline::default()
                    }
                )]],
                rows: vec![vec![vec![inline(
                    "code",
                    Inline {
                        code: true,
                        ..Inline::default()
                    }
                )]]],
            }]
        );
    }

    #[test]
    fn a_pipe_line_without_a_delimiter_stays_a_paragraph() {
        assert_eq!(
            blocks("a | b"),
            vec![Block::Paragraph(vec![plain("a | b")])]
        );
    }

    #[test]
    fn a_footnote_reference_and_its_definition_share_a_number() {
        assert_eq!(
            blocks("see [^why]\n\n[^why]: because."),
            vec![
                Block::Paragraph(vec![
                    plain("see "),
                    inline(
                        "¹",
                        Inline {
                            note: Some(1),
                            ..Inline::default()
                        }
                    ),
                ]),
                Block::Footnote {
                    number: 1,
                    spans: vec![plain("because.")],
                },
            ]
        );
    }

    #[test]
    fn footnotes_are_numbered_in_reference_order() {
        assert_eq!(
            blocks("a[^b] c[^a]\n\n[^a]: A\n[^b]: B"),
            vec![
                Block::Paragraph(vec![
                    plain("a"),
                    inline(
                        "¹",
                        Inline {
                            note: Some(1),
                            ..Inline::default()
                        }
                    ),
                    plain(" c"),
                    inline(
                        "²",
                        Inline {
                            note: Some(2),
                            ..Inline::default()
                        }
                    ),
                ]),
                Block::Footnote {
                    number: 1,
                    spans: vec![plain("B")],
                },
                Block::Footnote {
                    number: 2,
                    spans: vec![plain("A")],
                },
            ]
        );
    }

    #[test]
    fn a_reference_without_a_definition_stays_literal() {
        assert_eq!(
            blocks("see [^missing] and [^there]\n\n[^there]: here"),
            vec![
                Block::Paragraph(vec![
                    plain("see [^missing] and "),
                    inline(
                        "¹",
                        Inline {
                            note: Some(1),
                            ..Inline::default()
                        }
                    ),
                ]),
                Block::Footnote {
                    number: 1,
                    spans: vec![plain("here")],
                },
            ]
        );
    }

    #[test]
    fn a_parsed_message_is_reused_until_the_text_grows() {
        let cache = Cached::default();
        let first = cache.blocks("hello");
        let again = cache.blocks("hello");
        assert!(Rc::ptr_eq(&first, &again), "the parse is reused");

        let grown = cache.blocks("hello world");
        assert!(!Rc::ptr_eq(&first, &grown));
        assert_eq!(grown.len(), 1);
    }

    #[test]
    fn inline_math_is_a_chip_and_display_math_is_a_block() {
        assert_eq!(
            blocks("value $x^2$"),
            vec![Block::Paragraph(vec![
                plain("value "),
                Span {
                    text: "$x^2$".into(),
                    style: Inline {
                        code: true,
                        ..Inline::default()
                    },
                    link: None,
                },
            ])]
        );

        assert_eq!(
            blocks("$$\n\\frac{1}{2}\n$$"),
            vec![Block::Math("\\frac{1}{2}".into())]
        );
    }

    #[test]
    fn links_keep_their_target() {
        assert_eq!(
            blocks("see [the docs](https://example.com)"),
            vec![Block::Paragraph(vec![
                plain("see "),
                Span {
                    text: "the docs".into(),
                    style: Inline {
                        link: true,
                        ..Inline::default()
                    },
                    link: Some("https://example.com".into()),
                },
            ])]
        );
    }

    #[test]
    fn links_become_clickable_ranges_with_their_targets() {
        let spans = vec![
            plain("see "),
            Span {
                text: "the docs".into(),
                style: Inline {
                    link: true,
                    ..Inline::default()
                },
                link: Some("https://example.com".into()),
            },
            plain(" now"),
        ];

        let (_, links) = styled_parts(
            &spans,
            &Theme::dark(),
            gpui::rgb(0xe2e2e2),
            FontWeight::NORMAL,
        );

        assert_eq!(links.len(), 1, "one range for one link");
        assert_eq!(links[0].0, 4..12, "the range covers only the link text");
        assert_eq!(links[0].1.as_ref(), "https://example.com");
    }
}
