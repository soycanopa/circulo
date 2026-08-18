use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use circulo_core::{Task, TaskStatus, ToolCall, ToolCallStatus, ToolOutput};
use circulo_i18n::Catalog;
use circulo_markdown::{diff_lines, parse, Block, DiffKind, Inline};
use gpui::{
    div, prelude::*, px, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};
use serde_json::Value;

use crate::theme::{
    ACCENT, BG_SIDEBAR, BORDER, CODE_BG, DANGER, DIFF_ADD, DIFF_DEL, SUCCESS, TEXT, TEXT_MUTED,
};

pub fn human_tool_name(name: &str) -> String {
    name.replace('_', " ")
}

pub fn tool_context_line(input: &Value) -> Option<String> {
    const KEYS: &[&str] = &["path", "file_path", "file", "query", "pattern", "command"];
    KEYS.iter().find_map(|key| {
        input
            .get(*key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub fn ordered_tasks(tasks: &[Task]) -> Vec<&Task> {
    let mut items: Vec<&Task> = tasks.iter().collect();
    items.sort_by_key(|task| task.order);
    items
}

pub fn tool_status_key(status: ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "tool.status.pending",
        ToolCallStatus::Running => "tool.status.running",
        ToolCallStatus::Success => "tool.status.success",
        ToolCallStatus::Error => "tool.status.error",
    }
}

pub fn task_status_key(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "task.status.pending",
        TaskStatus::InProgress => "task.status.in_progress",
        TaskStatus::Completed => "task.status.completed",
        TaskStatus::Cancelled => "task.status.cancelled",
    }
}

pub fn render_text(content: &str) -> impl IntoElement {
    if content.is_empty() {
        return div().into_any_element();
    }

    let blocks = cached_parse(content);
    if let Some(text) = plain_paragraph_text(&blocks) {
        return div()
            .text_sm()
            .line_height(px(20.))
            .w_full()
            .min_w_0()
            .child(text)
            .into_any_element();
    }

    markdown_blocks(&blocks).into_any_element()
}

const PARSE_CACHE_CAP: usize = 128;

fn parse_cache() -> &'static Mutex<ParseCache> {
    static CACHE: OnceLock<Mutex<ParseCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ParseCache::default()))
}

#[derive(Default)]
struct ParseCache {
    entries: HashMap<String, Vec<Block>>,
    order: Vec<String>,
}

impl ParseCache {
    fn get(&self, content: &str) -> Option<Vec<Block>> {
        self.entries.get(content).cloned()
    }

    fn insert(&mut self, content: String, blocks: Vec<Block>) {
        if self.entries.contains_key(&content) {
            return;
        }
        if self.entries.len() >= PARSE_CACHE_CAP {
            let evict = self.order.len() / 2;
            for key in self.order.drain(..evict) {
                self.entries.remove(&key);
            }
        }
        self.order.push(content.clone());
        self.entries.insert(content, blocks);
    }
}

fn cached_parse(content: &str) -> Vec<Block> {
    if let Ok(cache) = parse_cache().lock() {
        if let Some(blocks) = cache.get(content) {
            return blocks;
        }
    }

    let blocks = parse(content);
    if let Ok(mut cache) = parse_cache().lock() {
        cache.insert(content.to_string(), blocks.clone());
    }
    blocks
}

fn plain_paragraph_text(blocks: &[Block]) -> Option<String> {
    if blocks.len() != 1 {
        return None;
    }
    let Block::Paragraph { inlines } = &blocks[0] else {
        return None;
    };
    if !inlines.iter().all(|inline| matches!(
        inline,
        Inline::Text(_) | Inline::SoftBreak | Inline::HardBreak
    )) {
        return None;
    }
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) => text.push_str(value),
            Inline::SoftBreak | Inline::HardBreak => text.push(' '),
            _ => return None,
        }
    }
    Some(text)
}

pub fn unsupported(catalog: &Catalog, message_index: usize, part_index: usize) -> gpui::AnyElement {
    div()
        .id(("unsupported", message_index * 32 + part_index))
        .px_2()
        .py_1()
        .rounded_md()
        .text_sm()
        .text_color(TEXT_MUTED)
        .child(catalog.get("part.unsupported").to_string())
        .into_any_element()
}

pub fn markdown_blocks(blocks: &[Block]) -> impl IntoElement {
    let mut col = div().flex().flex_col().gap_2().w_full().min_w_0();
    for (index, block) in blocks.iter().enumerate() {
        col = col.child(markdown_block(block, index));
    }
    col
}

fn markdown_block(block: &Block, index: usize) -> gpui::AnyElement {
    match block {
        Block::Heading { level, inlines } => div()
            .text_color(TEXT)
            .when(*level <= 2, |el| el.text_lg().font_weight(FontWeight::BOLD))
            .when(*level > 2, |el| el.font_weight(FontWeight::BOLD))
            .child(render_inlines(inlines))
            .into_any_element(),
        Block::Paragraph { inlines } => div()
            .w_full()
            .min_w_0()
            .text_sm()
            .text_color(TEXT)
            .child(render_inlines(inlines))
            .into_any_element(),
        Block::List {
            ordered,
            start,
            items,
        } => {
            let mut col = div().flex().flex_col().gap_1().w_full().pl(px(12.));
            for (offset, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    format!("{}.", start + offset as u64)
                } else {
                    "•".into()
                };
                col = col.child(
                    div()
                        .flex()
                        .gap_2()
                        .items_start()
                        .w_full()
                        .child(
                            div()
                                .flex_none()
                                .text_sm()
                                .text_color(TEXT_MUTED)
                                .child(marker),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(markdown_blocks(item)),
                        ),
                );
            }
            col.into_any_element()
        }
        Block::Quote { blocks } => div()
            .pl(px(12.))
            .border_l(px(2.))
            .border_color(BORDER)
            .text_color(TEXT_MUTED)
            .child(markdown_blocks(blocks))
            .into_any_element(),
        Block::Code { language: _, text } => {
            let mut body = div()
                .id(("code", index))
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .p_2()
                .rounded_md()
                .bg(CODE_BG)
                .font_family("Menlo")
                .text_sm()
                .overflow_x_scroll();
            for line in text.lines() {
                body = body.child(div().child(line.to_string()));
            }
            body.into_any_element()
        }
        Block::Table { headers, rows } => {
            let mut table = div()
                .id(("table", index))
                .w_full()
                .min_w_0()
                .overflow_x_scroll()
                .flex()
                .flex_col()
                .border_1()
                .border_color(BORDER);
            table = table.child(table_row(headers, true));
            for row in rows {
                table = table.child(table_row(row, false));
            }
            table.into_any_element()
        }
        Block::ThematicBreak => div().h(px(1.)).bg(BORDER).py_2().into_any_element(),
    }
}

fn table_row(cells: &[Vec<Inline>], header: bool) -> impl IntoElement {
    let mut row = div().flex().border_b_1().border_color(BORDER);
    for cell in cells {
        row = row.child(
            div()
                .flex_1()
                .px_2()
                .py_1()
                .text_sm()
                .when(header, |el| el.font_weight(FontWeight::BOLD))
                .child(render_inlines(cell)),
        );
    }
    row
}

fn render_inlines(inlines: &[Inline]) -> impl IntoElement {
    let flat = flatten_inlines(inlines, InlineStyle::default());
    if flat.len() == 1 && flat[0].1 == InlineStyle::default() {
        return div()
            .text_sm()
            .w_full()
            .min_w_0()
            .child(flat[0].0.clone());
    }

    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .w_full()
        .min_w_0()
        .overflow_hidden();
    for (text, style) in flatten_inlines(inlines, InlineStyle::default()) {
        for token in wrap_tokens(&text) {
            row = row.child(styled_span(&token, style));
        }
    }
    row
}

/// Split text into flex items so `flex_wrap` can break lines at word boundaries.
fn wrap_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            tokens.push(ch.to_string());
        } else {
            word.push(ch);
            if word.len() >= 32 {
                tokens.push(std::mem::take(&mut word));
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

#[derive(Clone, Copy, Default, PartialEq)]
struct InlineStyle {
    emphasis: bool,
    strong: bool,
    strike: bool,
    code: bool,
}

fn flatten_inlines(inlines: &[Inline], style: InlineStyle) -> Vec<(String, InlineStyle)> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push((text.clone(), style)),
            Inline::Code(text) => out.push((
                text.clone(),
                InlineStyle {
                    code: true,
                    ..style
                },
            )),
            Inline::SoftBreak | Inline::HardBreak => out.push((" ".into(), style)),
            Inline::Emphasis(children) => out.extend(flatten_inlines(
                children,
                InlineStyle {
                    emphasis: true,
                    ..style
                },
            )),
            Inline::Strong(children) => out.extend(flatten_inlines(
                children,
                InlineStyle {
                    strong: true,
                    ..style
                },
            )),
            Inline::Strikethrough(children) => out.extend(flatten_inlines(
                children,
                InlineStyle {
                    strike: true,
                    ..style
                },
            )),
            Inline::Link { dest, children } => {
                let mut nested = flatten_inlines(children, style);
                if nested.is_empty() {
                    nested.push((dest.clone(), style));
                } else {
                    nested.push((format!(" ({dest})"), style));
                }
                out.extend(nested);
            }
        }
    }
    out
}

fn styled_span(text: &str, style: InlineStyle) -> impl IntoElement {
    let mut el = div()
        .flex_none()
        .text_sm()
        .text_color(TEXT)
        .child(text.to_string());
    if style.strong {
        el = el.font_weight(FontWeight::BOLD);
    }
    if style.emphasis {
        el = el.italic();
    }
    if style.strike {
        el = el.line_through();
    }
    if style.code {
        el = el
            .px_1()
            .rounded_md()
            .bg(CODE_BG)
            .font_family("Menlo")
            .text_color(ACCENT);
    }
    el
}

pub fn tool_card(
    tool: &ToolCall,
    catalog: &Catalog,
    expanded: &HashSet<String>,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    use crate::icons::path;
    use crate::ui::disclosure::{
        activity_card, activity_card_body, activity_card_header, activity_detail_section,
    };

    let open = expanded.contains(&tool.id);
    let status = catalog.get(tool_status_key(tool.status)).to_string();
    let status_color = match tool.status {
        ToolCallStatus::Success => SUCCESS,
        ToolCallStatus::Error => DANGER,
        ToolCallStatus::Running => ACCENT,
        ToolCallStatus::Pending => TEXT_MUTED,
    };
    let context = tool_context_line(&tool.input).unwrap_or_default();

    let header = activity_card_header(
        ("tool-toggle", hash_id(&tool.id)),
        path::WRENCH,
        human_tool_name(&tool.name),
        if context.is_empty() { None } else { Some(context) },
        open,
        true,
        Some(
            div()
                .flex_none()
                .text_xs()
                .text_color(status_color)
                .child(status)
                .into_any_element(),
        ),
        on_click,
    );

    let body = open.then(|| {
        activity_card_body(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(activity_detail_section(
                    catalog.get("tool.input"),
                    div()
                        .w_full()
                        .min_w_0()
                        .text_xs()
                        .font_family("Menlo")
                        .text_color(TEXT_MUTED)
                        .child(tool.input.to_string()),
                ))
                .child(activity_detail_section(
                    catalog.get("tool.output"),
                    tool_output(tool.output.as_ref()),
                )),
        )
    });

    activity_card(("tool", hash_id(&tool.id)), header, body)
}

fn tool_output(output: Option<&ToolOutput>) -> impl IntoElement {
    match output {
        Some(ToolOutput::Text { content }) => div()
            .w_full()
            .min_w_0()
            .text_sm()
            .child(content.clone())
            .into_any_element(),
        Some(ToolOutput::Json { data }) => div()
            .w_full()
            .min_w_0()
            .text_xs()
            .font_family("Menlo")
            .child(data.to_string())
            .into_any_element(),
        Some(ToolOutput::Error { message }) => div()
            .text_sm()
            .text_color(DANGER)
            .child(message.clone())
            .into_any_element(),
        Some(ToolOutput::Diff {
            file_path,
            new_content,
            diff,
            ..
        }) => {
            let lines = diff_lines(diff.as_deref(), new_content);
            let mut col = div()
                .id(("diff", hash_id(file_path)))
                .flex()
                .flex_col()
                .h(px(220.))
                .overflow_y_scroll()
                .child(
                    div()
                        .text_xs()
                        .text_color(TEXT_MUTED)
                        .pb_1()
                        .child(file_path.clone()),
                );
            for line in lines {
                let (bg, prefix) = match line.kind {
                    DiffKind::Add => (Some(DIFF_ADD), "+"),
                    DiffKind::Delete => (Some(DIFF_DEL), "-"),
                    DiffKind::Header => (None, " "),
                    DiffKind::Context => (None, " "),
                };
                col = col.child(
                    div()
                        .font_family("Menlo")
                        .text_xs()
                        .when_some(bg, |el, color| el.bg(color))
                        .child(format!("{prefix}{}", line.text)),
                );
            }
            col.into_any_element()
        }
        None => div().into_any_element(),
    }
}

pub fn task_list(tasks: &[Task], catalog: &Catalog) -> impl IntoElement {
    let mut col = div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(BORDER);
    for task in ordered_tasks(tasks) {
        let status = catalog.get(task_status_key(task.status)).to_string();
        let color = match task.status {
            TaskStatus::Completed => SUCCESS,
            TaskStatus::Cancelled => TEXT_MUTED,
            TaskStatus::InProgress => ACCENT,
            TaskStatus::Pending => TEXT_MUTED,
        };
        col = col.child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .child(task.title.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(color)
                        .child(status),
                ),
        );
    }
    col
}

fn hash_id(value: &str) -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

#[cfg(test)]
mod tests {
    use super::{human_tool_name, ordered_tasks, tool_context_line};
    use circulo_core::{
        Message, MessagePart, MessageRole, MessageStatus, Question, QuestionStatus, QuestionType,
        Task, TaskStatus,
    };
    use circulo_i18n::Catalog;
    use serde_json::json;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
    }

    #[test]
    fn name_replaces_underscores() {
        assert_eq!(human_tool_name("edit_file"), "edit file");
    }

    #[test]
    fn context_prefers_path() {
        assert_eq!(
            tool_context_line(&json!({"path": "notes.md", "query": "x"})).as_deref(),
            Some("notes.md")
        );
    }

    #[test]
    fn tasks_sort_by_order() {
        let tasks = vec![
            Task {
                id: "b".into(),
                title: "second".into(),
                description: None,
                status: TaskStatus::InProgress,
                order: 1,
            },
            Task {
                id: "a".into(),
                title: "first".into(),
                description: None,
                status: TaskStatus::Completed,
                order: 0,
            },
        ];
        let ordered = ordered_tasks(&tasks);
        assert_eq!(ordered[0].title, "first");
        assert_eq!(ordered[1].title, "second");
    }

    #[test]
    fn question_part_is_unsupported_copy() {
        let catalog = Catalog::english();
        let fallback = catalog.get("part.unsupported");
        assert_ne!(fallback, "part.unsupported");
        let message = Message {
            id: Uuid::nil(),
            session_id: Uuid::nil(),
            role: MessageRole::Assistant,
            parts: vec![
                MessagePart::Text {
                    content: "Hello".into(),
                },
                MessagePart::Question {
                    question: Question {
                        id: "q1".into(),
                        prompt: "Choose one".into(),
                        question_type: QuestionType::Confirm,
                        options: None,
                        answer: None,
                        status: QuestionStatus::Pending,
                    },
                },
            ],
            status: MessageStatus::Complete,
            created_at: now(),
            is_streaming: false,
        };
        assert!(message
            .parts
            .iter()
            .any(|part| matches!(part, MessagePart::Question { .. })));
        assert!(fallback.contains("display"));
    }
}
