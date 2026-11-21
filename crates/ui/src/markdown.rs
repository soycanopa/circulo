//! Markdown rendering for assistant text.
//!
//! Agent text arrives as CommonMark. Without this, markers like `**bold**` and
//! `` `code` `` paint literally. The parser below turns a turn's text into
//! blocks (paragraph, heading, bullet list, code block, rule) with inline bold
//! and code runs, and each block renders through GPUI `StyledText` so ranges
//! highlight against the plain text's byte offsets.

use std::ops::Range;

use gpui::{
    AnyElement, FontStyle, FontWeight, HighlightStyle, Hsla, IntoElement, ParentElement, Styled,
    StyledText, TextStyle, div, px,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::theme::Theme;

/// One renderable block of a text part.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Block {
    Paragraph(Vec<Inline>),
    Heading { level: u8, inlines: Vec<Inline> },
    List(Vec<Vec<Inline>>),
    Code(String),
    Rule,
}

/// One inline run inside a block.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Inline {
    Text(String),
    Bold(String),
    Code(String),
    Italic(String),
}

/// Parse CommonMark into blocks. Unknown constructs keep their inner text so a
/// newer agent cannot break the transcript.
pub(crate) fn parse(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Option<Vec<Inline>> = None;
    let mut heading: Option<(u8, Vec<Inline>)> = None;
    let mut list: Option<Vec<Vec<Inline>>> = None;
    let mut item: Option<Vec<Inline>> = None;
    let mut code: Option<String> = None;
    // Strong and emphasis nest; only the innermost style wins.
    let mut styles: Vec<Style> = Vec::new();

    for event in Parser::new_ext(text, Options::empty()) {
        match event {
            Event::Start(Tag::Paragraph) => paragraph = Some(Vec::new()),
            Event::End(TagEnd::Paragraph) => {
                if let Some(inlines) = paragraph.take() {
                    blocks.push(Block::Paragraph(inlines));
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((heading_level(level), Vec::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, inlines)) = heading.take() {
                    blocks.push(Block::Heading { level, inlines });
                }
            }
            Event::Start(Tag::List(_)) => list = Some(Vec::new()),
            Event::Start(Tag::Item) => item = Some(Vec::new()),
            Event::End(TagEnd::Item) => {
                if let (Some(list), Some(item)) = (list.as_mut(), item.take()) {
                    list.push(item);
                }
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(list) = list.take() {
                    blocks.push(Block::List(list));
                }
            }
            Event::Start(Tag::CodeBlock(_)) => code = Some(String::new()),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code.take() {
                    blocks.push(Block::Code(code));
                }
            }
            Event::Start(Tag::Strong) => styles.push(Style::Bold),
            Event::End(TagEnd::Strong) => {
                styles.pop();
            }
            Event::Start(Tag::Emphasis) => styles.push(Style::Italic),
            Event::End(TagEnd::Emphasis) => {
                styles.pop();
            }
            Event::Start(_) | Event::End(_) => {}
            Event::Text(text) => {
                emit_text(
                    &styles,
                    &mut paragraph,
                    &mut heading,
                    &mut item,
                    &mut code,
                    &text,
                );
            }
            Event::Code(code) => {
                let inline = Inline::Code(code.into_string());
                if let Some(item) = item.as_mut() {
                    push_inline(item, inline);
                } else if let Some((_, inlines)) = heading.as_mut() {
                    push_inline(inlines, inline);
                } else if let Some(inlines) = paragraph.as_mut() {
                    push_inline(inlines, inline);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                emit_text(
                    &styles,
                    &mut paragraph,
                    &mut heading,
                    &mut item,
                    &mut code,
                    "\n",
                );
            }
            Event::Rule => blocks.push(Block::Rule),
            Event::Html(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_) => {}
        }
    }

    blocks
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Bold,
    Italic,
}

/// Append text to whichever inline context is open: a fenced code block, a
/// list item, a heading, or a paragraph, in that priority.
fn emit_text(
    styles: &[Style],
    paragraph: &mut Option<Vec<Inline>>,
    heading: &mut Option<(u8, Vec<Inline>)>,
    item: &mut Option<Vec<Inline>>,
    code: &mut Option<String>,
    text: &str,
) {
    if let Some(code) = code.as_mut() {
        code.push_str(text);
        return;
    }
    let inline = match styles.last() {
        Some(Style::Bold) => Inline::Bold(text.to_owned()),
        Some(Style::Italic) => Inline::Italic(text.to_owned()),
        None => Inline::Text(text.to_owned()),
    };
    if let Some(item) = item.as_mut() {
        push_inline(item, inline);
    } else if let Some((_, inlines)) = heading.as_mut() {
        push_inline(inlines, inline);
    } else if let Some(inlines) = paragraph.as_mut() {
        push_inline(inlines, inline);
    }
}

fn push_inline(inlines: &mut Vec<Inline>, inline: Inline) {
    match (inlines.last_mut(), inline) {
        (Some(Inline::Text(last)), Inline::Text(next)) => last.push_str(&next),
        (Some(Inline::Bold(last)), Inline::Bold(next)) => last.push_str(&next),
        (Some(Inline::Code(last)), Inline::Code(next)) => last.push_str(&next),
        (Some(Inline::Italic(last)), Inline::Italic(next)) => last.push_str(&next),
        (_, inline) => inlines.push(inline),
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Flatten inlines into plain text plus highlight ranges over its byte offsets.
/// The ranges borrow nothing and stay valid while the plain text is untouched.
pub(crate) fn plain_and_highlights(
    inlines: &[Inline],
    chip: Hsla,
) -> (String, Vec<(Range<usize>, HighlightStyle)>) {
    let mut plain = String::new();
    let mut highlights = Vec::new();

    for inline in inlines {
        let (text, style) = match inline {
            Inline::Text(text) => (text.as_str(), None),
            Inline::Bold(text) => (
                text.as_str(),
                Some(HighlightStyle {
                    font_weight: Some(FontWeight::BOLD),
                    ..HighlightStyle::default()
                }),
            ),
            Inline::Code(text) => (
                text.as_str(),
                Some(HighlightStyle {
                    background_color: Some(chip),
                    ..HighlightStyle::default()
                }),
            ),
            Inline::Italic(text) => (
                text.as_str(),
                Some(HighlightStyle {
                    font_style: Some(FontStyle::Italic),
                    ..HighlightStyle::default()
                }),
            ),
        };

        let start = plain.len();
        plain.push_str(text);
        if let Some(style) = style {
            highlights.push((start..plain.len(), style));
        }
    }

    (plain, highlights)
}

fn base_style(theme: &Theme, size: f32) -> TextStyle {
    TextStyle {
        color: Hsla::from(theme.text),
        font_size: px(size).into(),
        line_height: px(size + 7.).into(),
        ..TextStyle::default()
    }
}

/// Render one assistant text part as a stack of markdown blocks.
pub(crate) fn markdown_view(text: &str, theme: &Theme) -> AnyElement {
    let blocks = parse(text);
    if blocks.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.))
            .flex_shrink_0()
            .py(px(7.))
            .text_size(px(13.))
            .line_height(px(20.))
            .child(text.to_owned())
            .into_any_element();
    }

    let mut column = div().w_full().min_w(px(0.)).flex().flex_col();
    for block in &blocks {
        column = column.child(block_view(block, theme));
    }
    column.into_any_element()
}

fn block_view(block: &Block, theme: &Theme) -> AnyElement {
    match block {
        Block::Paragraph(inlines) => {
            let (plain, highlights) = plain_and_highlights(inlines, Hsla::from(theme.chip));
            let style = base_style(theme, 13.);
            div()
                .w_full()
                .min_w(px(0.))
                .flex_shrink_0()
                .py(px(7.))
                .child(StyledText::new(plain).with_default_highlights(&style, highlights))
                .into_any_element()
        }
        Block::Heading { level, inlines } => {
            let (plain, highlights) = plain_and_highlights(inlines, Hsla::from(theme.chip));
            let size = match level {
                1 => 17.,
                2 => 15.,
                _ => 13.,
            };
            let mut style = base_style(theme, size);
            style.font_weight = FontWeight::SEMIBOLD;
            div()
                .w_full()
                .min_w(px(0.))
                .flex_shrink_0()
                .pt(px(10.))
                .pb(px(4.))
                .child(StyledText::new(plain).with_default_highlights(&style, highlights))
                .into_any_element()
        }
        Block::List(items) => {
            let mut list = div().w_full().min_w(px(0.)).flex().flex_col();
            for item in items {
                let (plain, highlights) = plain_and_highlights(item, Hsla::from(theme.chip));
                let style = base_style(theme, 13.);
                list = list.child(
                    div()
                        .w_full()
                        .min_w(px(0.))
                        .flex_shrink_0()
                        .flex()
                        .py(px(2.))
                        .child(div().flex_shrink_0().pr(px(8.)).child("•"))
                        .child(StyledText::new(plain).with_default_highlights(&style, highlights)),
                );
            }
            list.into_any_element()
        }
        Block::Code(code) => div()
            .w_full()
            .min_w(px(0.))
            .flex_shrink_0()
            .my(px(6.))
            .p(px(10.))
            .rounded(px(8.))
            .bg(theme.chip)
            .text_size(px(12.))
            .line_height(px(18.))
            .child(code.trim_end().to_owned())
            .into_any_element(),
        Block::Rule => div()
            .w_full()
            .min_w(px(0.))
            .flex_shrink_0()
            .py(px(10.))
            .child(div().h(px(1.)).w_full().bg(theme.hairline))
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chip() -> Hsla {
        Hsla::default()
    }

    #[test]
    fn bold_parses_to_a_highlight_range() {
        let blocks = parse("Hello **world**");
        assert_eq!(
            blocks,
            vec![Block::Paragraph(vec![
                Inline::Text("Hello ".into()),
                Inline::Bold("world".into()),
            ])]
        );

        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a paragraph");
        };
        let (plain, highlights) = plain_and_highlights(inlines, chip());
        assert_eq!(plain, "Hello world");
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].0, 6..11);
        assert_eq!(highlights[0].1.font_weight, Some(FontWeight::BOLD));
    }

    #[test]
    fn inline_code_parses_to_a_chip_range() {
        let blocks = parse("Run `cargo test` now");
        assert_eq!(
            blocks,
            vec![Block::Paragraph(vec![
                Inline::Text("Run ".into()),
                Inline::Code("cargo test".into()),
                Inline::Text(" now".into()),
            ])]
        );
    }

    #[test]
    fn a_heading_keeps_its_level_and_inner_styles() {
        let blocks = parse("# Ship **it**");
        assert_eq!(
            blocks,
            vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Ship ".into()), Inline::Bold("it".into()),],
            }]
        );
    }

    #[test]
    fn bullets_become_one_list() {
        let blocks = parse("- first\n- second\n");
        assert_eq!(
            blocks,
            vec![Block::List(vec![
                vec![Inline::Text("first".into())],
                vec![Inline::Text("second".into())],
            ])]
        );
    }

    #[test]
    fn a_fenced_block_stays_verbatim() {
        let blocks = parse("```rust\nlet x = **not bold**;\n```\n");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], Block::Code(_)));
        if let Block::Code(code) = &blocks[0] {
            assert!(code.contains("**not bold**"));
        }
    }

    #[test]
    fn highlight_ranges_cover_multibyte_text() {
        let blocks = parse("héllo **wörld**");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("expected a paragraph");
        };
        let (plain, highlights) = plain_and_highlights(inlines, chip());
        assert_eq!(plain, "héllo wörld");
        let range = highlights[0].0.clone();
        assert_eq!(&plain[range], "wörld");
    }
}
