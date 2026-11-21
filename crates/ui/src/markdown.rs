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
//! What is deliberately not supported yet: tables, footnotes, math, raw HTML
//! (dropped), and links are coloured but not clickable.

use gpui::{
    AnyElement, Font, FontFeatures, FontStyle, FontWeight, Hsla, IntoElement, SharedString,
    StrikethroughStyle, StyledText, TextRun, UnderlineStyle, div, prelude::*, px,
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
}

/// A run of text with one inline style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub text: String,
    pub style: Inline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    Bullet,
    Ordered(u64),
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
    Rule,
}

/// Parse a message into blocks and paint them.
pub(crate) fn markdown(text: &str, theme: &Theme) -> AnyElement {
    let blocks = blocks(text);

    let mut column = div().w_full().min_w(px(0.)).flex().flex_col();
    let mut previous = None;
    let mut first = true;

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
        column = column.child(div().mt(px(top)).child(block_view(block, theme)));
    }

    column.into_any_element()
}

fn block_view(block: Block, theme: &Theme) -> AnyElement {
    match block {
        Block::Paragraph(spans) => text_block(
            spans,
            theme,
            theme.text,
            FontWeight::NORMAL,
            BODY_SIZE,
            BODY_LINE,
        ),
        Block::Heading { level, spans } => {
            let (size, line) = match level {
                1 => (17., 24.),
                2 => (15.5, 22.),
                _ => (14., 21.),
            };
            text_block(spans, theme, theme.text, FontWeight::SEMIBOLD, size, line)
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
                        .child(marker_text(marker, depth)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(BODY_SIZE))
                        .line_height(px(BODY_LINE))
                        .child(styled(spans, theme, theme.text, FontWeight::NORMAL)),
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
                    .child(styled(spans, theme, theme.text_muted, FontWeight::NORMAL)),
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
        Block::Rule => div()
            .w_full()
            .h(px(1.))
            .my(px(2.))
            .bg(theme.hairline)
            .into_any_element(),
    }
}

fn text_block(
    spans: Vec<Span>,
    theme: &Theme,
    color: gpui::Rgba,
    weight: FontWeight,
    size: f32,
    line: f32,
) -> AnyElement {
    div()
        .w_full()
        .min_w(px(0.))
        .text_size(px(size))
        .line_height(px(line))
        .child(styled(spans, theme, color, weight))
        .into_any_element()
}

fn marker_text(marker: Option<Marker>, depth: usize) -> SharedString {
    match marker {
        Some(Marker::Ordered(number)) => SharedString::from(format!("{number}.")),
        Some(Marker::Bullet) => SharedString::from(if depth > 1 { "◦" } else { "•" }),
        None => SharedString::new_static(""),
    }
}

/// One text element with a run per span. The runs must cover the text exactly;
/// `StyledText::with_runs` asserts it.
fn styled(spans: Vec<Span>, theme: &Theme, color: gpui::Rgba, weight: FontWeight) -> StyledText {
    let mut plain = String::new();
    let mut runs = Vec::with_capacity(spans.len());

    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        plain.push_str(&span.text);
        runs.push(text_run(&span, theme, color, weight));
    }

    StyledText::new(plain).with_runs(runs)
}

fn text_run(span: &Span, theme: &Theme, color: gpui::Rgba, weight: FontWeight) -> TextRun {
    let style = span.style;
    let color: Hsla = if style.link {
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

/// Parse CommonMark into blocks. The only options that change meaning for an
/// agent's output: strikethrough and task lists are GitHub-flavoured and agents
/// write them; everything else stays CommonMark.
pub(crate) fn blocks(text: &str) -> Vec<Block> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
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
    /// One entry per open list, innermost last.
    lists: Vec<ListState>,
    /// One entry per open list item, innermost last.
    items: Vec<ItemState>,
    quote: usize,
    code: Option<String>,
    heading: Option<u8>,
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
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                let style = Inline {
                    code: true,
                    ..self.style
                };
                self.push_span(&text, style);
            }
            // Raw HTML and footnote references are dropped rather than shown
            // as markup: an agent's `<br>` is noise, not content.
            Event::Html(_) | Event::InlineHtml(_) | Event::FootnoteReference(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => self.heading = Some(level as u8),
            Tag::BlockQuote(_) => self.quote += 1,
            Tag::CodeBlock(_) => self.code = Some(String::new()),
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
            Tag::Link { .. } => self.style.link = true,
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
            TagEnd::Item => {
                // A tight list has no paragraph wrapper, so text still sitting
                // in the buffer belongs to the item that is closing.
                self.flush();
                self.items.pop();
            }
            TagEnd::Emphasis => self.style.italic = false,
            TagEnd::Strong => self.style.bold = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link => self.style.link = false,
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
        {
            last.text.push_str(text);
            return;
        }
        self.spans.push(Span {
            text: text.to_owned(),
            style,
        });
    }

    /// Turn the buffered inline content into the block it belongs to.
    fn flush(&mut self) {
        if self.spans.is_empty() {
            return;
        }

        let spans = std::mem::take(&mut self.spans);

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

    fn finish(&mut self) {
        self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inline(text: &str, style: Inline) -> Span {
        Span {
            text: text.to_owned(),
            style,
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
    fn links_are_marked_and_carry_only_their_text() {
        assert_eq!(
            blocks("see [the docs](https://example.com)"),
            vec![Block::Paragraph(vec![
                plain("see "),
                inline(
                    "the docs",
                    Inline {
                        link: true,
                        ..Inline::default()
                    }
                ),
            ])]
        );
    }
}
