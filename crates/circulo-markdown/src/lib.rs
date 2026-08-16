//! Markdown and unified-diff parse for Circulo chat parts.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph { inlines: Vec<Inline> },
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
    },
    Quote { blocks: Vec<Block> },
    Code {
        language: Option<String>,
        text: String,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { dest: String, children: Vec<Inline> },
    SoftBreak,
    HardBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Context,
    Add,
    Delete,
    Header,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

pub fn parse(markdown: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);
    let mut builder = Builder::new();
    for event in parser {
        builder.feed(event);
    }
    builder.finish()
}

pub fn parse_unified_diff(input: &str) -> Vec<DiffLine> {
    input
        .lines()
        .map(|line| {
            if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
                DiffLine {
                    kind: DiffKind::Header,
                    text: line.to_string(),
                }
            } else if let Some(rest) = line.strip_prefix('+') {
                DiffLine {
                    kind: DiffKind::Add,
                    text: rest.to_string(),
                }
            } else if let Some(rest) = line.strip_prefix('-') {
                DiffLine {
                    kind: DiffKind::Delete,
                    text: rest.to_string(),
                }
            } else {
                DiffLine {
                    kind: DiffKind::Context,
                    text: line.strip_prefix(' ').unwrap_or(line).to_string(),
                }
            }
        })
        .collect()
}

pub fn diff_lines(diff: Option<&str>, new_content: &str) -> Vec<DiffLine> {
    match diff {
        Some(raw) if !raw.trim().is_empty() => parse_unified_diff(raw),
        _ => new_content
            .lines()
            .map(|line| DiffLine {
                kind: DiffKind::Add,
                text: line.to_string(),
            })
            .collect(),
    }
}

struct Builder {
    stack: Vec<Frame>,
}

enum Frame {
    Root(Vec<Block>),
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
        current: Vec<Block>,
    },
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Code {
        language: Option<String>,
        text: String,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        in_head: bool,
        row: Vec<Vec<Inline>>,
        cell: Vec<Inline>,
    },
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        dest: String,
        children: Vec<Inline>,
    },
}

impl Builder {
    fn new() -> Self {
        Self {
            stack: vec![Frame::Root(Vec::new())],
        }
    }

    fn feed(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(end) => self.end(end),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.push_inline(Inline::Code(code.into_string())),
            Event::SoftBreak => self.push_inline(Inline::SoftBreak),
            Event::HardBreak => self.push_inline(Inline::HardBreak),
            Event::Rule => self.push_block(Block::ThematicBreak),
            Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::TaskListMarker(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.stack.push(Frame::Paragraph(Vec::new())),
            Tag::Heading { level, .. } => self.stack.push(Frame::Heading {
                level: heading_level(level),
                inlines: Vec::new(),
            }),
            Tag::BlockQuote(_) => self.stack.push(Frame::Quote(Vec::new())),
            Tag::List(start) => self.stack.push(Frame::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
                current: Vec::new(),
            }),
            Tag::Item => {}
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.into_string()),
                    _ => None,
                };
                self.stack.push(Frame::Code {
                    language,
                    text: String::new(),
                });
            }
            Tag::Table(_) => self.stack.push(Frame::Table {
                headers: Vec::new(),
                rows: Vec::new(),
                in_head: false,
                row: Vec::new(),
                cell: Vec::new(),
            }),
            Tag::TableHead => {
                if let Some(Frame::Table { in_head, .. }) = self.stack.last_mut() {
                    *in_head = true;
                }
            }
            Tag::TableRow => {}
            Tag::TableCell => {
                if let Some(Frame::Table { cell, .. }) = self.stack.last_mut() {
                    cell.clear();
                }
            }
            Tag::Emphasis => self.stack.push(Frame::Emphasis(Vec::new())),
            Tag::Strong => self.stack.push(Frame::Strong(Vec::new())),
            Tag::Strikethrough => self.stack.push(Frame::Strikethrough(Vec::new())),
            Tag::Link { dest_url, .. } => self.stack.push(Frame::Link {
                dest: dest_url.into_string(),
                children: Vec::new(),
            }),
            Tag::Image { dest_url, .. } => self.push_inline(Inline::Text(dest_url.into_string())),
            _ => {}
        }
    }

    fn end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph => {
                if let Some(Frame::Paragraph(inlines)) = self.stack.pop() {
                    self.push_block(Block::Paragraph { inlines });
                }
            }
            TagEnd::Heading(_) => {
                if let Some(Frame::Heading { level, inlines }) = self.stack.pop() {
                    self.push_block(Block::Heading { level, inlines });
                }
            }
            TagEnd::BlockQuote(_) => {
                if let Some(Frame::Quote(blocks)) = self.stack.pop() {
                    self.push_block(Block::Quote { blocks });
                }
            }
            TagEnd::Item => {
                if let Some(Frame::List { current, items, .. }) = self.stack.last_mut() {
                    items.push(std::mem::take(current));
                }
            }
            TagEnd::List(_) => {
                if let Some(Frame::List {
                    ordered,
                    start,
                    items,
                    ..
                }) = self.stack.pop()
                {
                    self.push_block(Block::List {
                        ordered,
                        start,
                        items,
                    });
                }
            }
            TagEnd::CodeBlock => {
                if let Some(Frame::Code { language, text }) = self.stack.pop() {
                    self.push_block(Block::Code { language, text });
                }
            }
            TagEnd::TableCell => {
                if let Some(Frame::Table { cell, row, .. }) = self.stack.last_mut() {
                    row.push(std::mem::take(cell));
                }
            }
            TagEnd::TableHead => {
                if let Some(Frame::Table {
                    in_head,
                    row,
                    headers,
                    ..
                }) = self.stack.last_mut()
                {
                    *headers = std::mem::take(row);
                    *in_head = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(Frame::Table { row, rows, in_head, .. }) = self.stack.last_mut() {
                    if !*in_head {
                        rows.push(std::mem::take(row));
                    }
                }
            }
            TagEnd::Table => {
                if let Some(Frame::Table { headers, rows, .. }) = self.stack.pop() {
                    self.push_block(Block::Table { headers, rows });
                }
            }
            TagEnd::Emphasis => {
                if let Some(Frame::Emphasis(children)) = self.stack.pop() {
                    self.push_inline(Inline::Emphasis(children));
                }
            }
            TagEnd::Strong => {
                if let Some(Frame::Strong(children)) = self.stack.pop() {
                    self.push_inline(Inline::Strong(children));
                }
            }
            TagEnd::Strikethrough => {
                if let Some(Frame::Strikethrough(children)) = self.stack.pop() {
                    self.push_inline(Inline::Strikethrough(children));
                }
            }
            TagEnd::Link => {
                if let Some(Frame::Link { dest, children }) = self.stack.pop() {
                    self.push_inline(Inline::Link { dest, children });
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(Frame::Code { text: body, .. }) = self.stack.last_mut() {
            body.push_str(text);
            return;
        }
        self.push_inline(Inline::Text(text.to_string()));
    }

    fn push_inline(&mut self, inline: Inline) {
        for frame in self.stack.iter_mut().rev() {
            match frame {
                Frame::Paragraph(inlines)
                | Frame::Heading { inlines, .. }
                | Frame::Emphasis(inlines)
                | Frame::Strong(inlines)
                | Frame::Strikethrough(inlines)
                | Frame::Link {
                    children: inlines, ..
                } => {
                    inlines.push(inline);
                    return;
                }
                Frame::Table { cell, .. } => {
                    cell.push(inline);
                    return;
                }
                _ => {}
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        for frame in self.stack.iter_mut().rev() {
            match frame {
                Frame::Root(blocks) | Frame::Quote(blocks) => {
                    blocks.push(block);
                    return;
                }
                Frame::List { current, .. } => {
                    current.push(block);
                    return;
                }
                _ => {}
            }
        }
    }

    fn finish(mut self) -> Vec<Block> {
        match self.stack.pop() {
            Some(Frame::Root(blocks)) => blocks,
            _ => Vec::new(),
        }
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

#[cfg(test)]
mod tests {
    use super::{Block, DiffKind, Inline, diff_lines, parse, parse_unified_diff};

    #[test]
    fn parses_heading_emphasis_and_code() {
        let blocks = parse("# Title\n\nHello **world** and `path`.\n\n```rs\nfn x() {}\n```\n");
        assert!(matches!(
            &blocks[0],
            Block::Heading { level: 1, inlines } if matches!(inlines.first(), Some(Inline::Text(t)) if t == "Title")
        ));
        assert!(blocks.iter().any(|block| matches!(block, Block::Code { language: Some(lang), text } if lang == "rs" && text.contains("fn x"))));
        let paragraph = blocks
            .iter()
            .find_map(|block| match block {
                Block::Paragraph { inlines } => Some(inlines),
                _ => None,
            })
            .expect("paragraph");
        assert!(paragraph.iter().any(|inline| matches!(inline, Inline::Strong(_))));
        assert!(paragraph.iter().any(|inline| matches!(inline, Inline::Code(code) if code == "path")));
    }

    #[test]
    fn parses_list_quote_and_table() {
        let blocks = parse(
            "- one\n- two\n\n> quoted\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n",
        );
        assert!(blocks.iter().any(|block| matches!(
            block,
            Block::List { ordered: false, items, .. } if items.len() == 2
        )));
        assert!(blocks.iter().any(|block| matches!(block, Block::Quote { .. })));
        assert!(blocks.iter().any(|block| matches!(
            block,
            Block::Table { headers, rows } if headers.len() == 2 && rows.len() == 1
        )));
    }

    #[test]
    fn incomplete_fence_does_not_panic() {
        let blocks = parse("```\nstill open");
        assert!(blocks.iter().any(|block| matches!(block, Block::Code { .. })));
    }

    #[test]
    fn unified_diff_marks_additions() {
        let lines = parse_unified_diff("--- a/notes.md\n+++ b/notes.md\n+Hello from Circulo.\n");
        assert!(lines.iter().any(|line| line.kind == DiffKind::Add && line.text.contains("Hello")));
        assert!(lines.iter().any(|line| line.kind == DiffKind::Header));
    }

    #[test]
    fn missing_diff_uses_new_content_as_adds() {
        let lines = diff_lines(None, "alpha\nbeta\n");
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.kind == DiffKind::Add));
    }
}
