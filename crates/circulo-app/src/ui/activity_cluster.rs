//! Waku-style activity clusters: reasoning, tools, and tasks folded behind one
//! disclosure once assistant text arrives.

use std::collections::HashSet;

use circulo_core::MessagePart;
use circulo_i18n::Catalog;
use gpui::{div, prelude::*, px, FontWeight, IntoElement, ParentElement, SharedString, Styled};
use crate::shell::AppShell;
use crate::parts::{task_list, tool_card, unsupported};
use crate::ui::disclosure::{disclosure_header, disclosure_rail};
use crate::ui::reasoning_block::reasoning_block;
use crate::ui::shimmer_text::shimmer_text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSegment {
    Text { part_index: usize },
    Activity { part_indices: Vec<usize> },
}

pub fn message_segments(parts: &[MessagePart]) -> Vec<MessageSegment> {
    let mut segments = Vec::new();
    let mut activity = Vec::new();

    let flush_activity = |segments: &mut Vec<MessageSegment>, activity: &mut Vec<usize>| {
        if !activity.is_empty() {
            segments.push(MessageSegment::Activity {
                part_indices: std::mem::take(activity),
            });
        }
    };

    for (index, part) in parts.iter().enumerate() {
        match part {
            MessagePart::Text { .. } => {
                flush_activity(&mut segments, &mut activity);
                segments.push(MessageSegment::Text { part_index: index });
            }
            MessagePart::Reasoning { .. }
            | MessagePart::ToolCall { .. }
            | MessagePart::TaskList { .. } => activity.push(index),
            MessagePart::Question { .. } => {
                flush_activity(&mut segments, &mut activity);
                segments.push(MessageSegment::Text { part_index: index });
            }
        }
    }
    flush_activity(&mut segments, &mut activity);
    segments
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActivityCounts {
    plan_steps: usize,
    thoughts: usize,
    commands: usize,
    file_edits: usize,
    file_reads: usize,
    file_searches: usize,
    tool_calls: usize,
}

fn count_unit(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn tool_bucket(name: &str) -> &'static str {
    let name = name.to_lowercase();
    if name.contains("todo") || name.contains("plan") || name.contains("task") {
        "plan"
    } else if name.contains("read") {
        "read"
    } else if name.contains("write")
        || name.contains("edit")
        || name.contains("patch")
        || name.contains("replace")
    {
        "edit"
    } else if name.contains("grep")
        || name.contains("search")
        || name.contains("glob")
        || name.contains("find")
    {
        "search"
    } else if name.contains("bash")
        || name.contains("shell")
        || name.contains("command")
        || name.contains("run")
    {
        "command"
    } else {
        "tool"
    }
}

fn reasoning_thought_count(content: &str) -> usize {
    let lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    lines.max(1)
}

pub fn activity_counts(parts: &[MessagePart], indices: &[usize]) -> ActivityCounts {
    let mut counts = ActivityCounts::default();
    for index in indices {
        match &parts[*index] {
            MessagePart::Reasoning { content, .. } => {
                counts.thoughts += reasoning_thought_count(content);
            }
            MessagePart::TaskList { tasks } => {
                counts.plan_steps += tasks.len().max(1);
            }
            MessagePart::ToolCall { tool_call } => match tool_bucket(&tool_call.name) {
                "plan" => counts.plan_steps += 1,
                "read" => counts.file_reads += 1,
                "edit" => counts.file_edits += 1,
                "search" => counts.file_searches += 1,
                "command" => counts.commands += 1,
                _ => counts.tool_calls += 1,
            },
            _ => {}
        }
    }
    counts
}

pub fn activity_summary_label(counts: ActivityCounts, live: bool, catalog: &Catalog) -> String {
    if live && counts == ActivityCounts::default() {
        return catalog.get("activity.summary.working").to_string();
    }

    let mut parts = Vec::new();
    if counts.plan_steps > 0 {
        parts.push(count_unit(
            counts.plan_steps,
            catalog.get("activity.unit.plan_step"),
            catalog.get("activity.unit.plan_steps"),
        ));
    }
    if counts.thoughts > 0 {
        parts.push(count_unit(
            counts.thoughts,
            catalog.get("activity.unit.thought"),
            catalog.get("activity.unit.thoughts"),
        ));
    }
    if counts.commands > 0 {
        parts.push(count_unit(
            counts.commands,
            catalog.get("activity.unit.command"),
            catalog.get("activity.unit.commands"),
        ));
    }
    if counts.file_edits > 0 {
        parts.push(count_unit(
            counts.file_edits,
            catalog.get("activity.unit.file_edit"),
            catalog.get("activity.unit.file_edits"),
        ));
    }
    if counts.file_reads > 0 {
        parts.push(count_unit(
            counts.file_reads,
            catalog.get("activity.unit.file_read"),
            catalog.get("activity.unit.file_reads"),
        ));
    }
    if counts.tool_calls > 0 {
        parts.push(count_unit(
            counts.tool_calls,
            catalog.get("activity.unit.tool_call"),
            catalog.get("activity.unit.tool_calls"),
        ));
    }
    if counts.file_searches > 0 {
        parts.push(count_unit(
            counts.file_searches,
            catalog.get("activity.unit.file_search"),
            catalog.get("activity.unit.file_searches"),
        ));
    }

    if parts.is_empty() {
        return if live {
            catalog.get("activity.summary.working").to_string()
        } else {
            catalog.get("activity.summary.worked").to_string()
        };
    }

    let details = parts.join(" • ");
    if live {
        details
    } else {
        format!("{} {details}", catalog.get("activity.summary.ran_prefix"))
    }
}

pub fn cluster_key(message_id: circulo_core::Uuid, segment_index: usize) -> String {
    format!("{message_id}:{segment_index}")
}

pub fn cluster_open(
    key: &str,
    live: bool,
    expanded: &HashSet<String>,
    collapsed_live: &HashSet<String>,
) -> bool {
    if collapsed_live.contains(key) {
        return false;
    }
    if expanded.contains(key) {
        return true;
    }
    live
}

#[allow(clippy::too_many_arguments)]
pub fn activity_cluster(
    message_id: circulo_core::Uuid,
    message_index: usize,
    segment_index: usize,
    part_indices: &[usize],
    parts: &[MessagePart],
    live: bool,
    streaming: bool,
    catalog: &Catalog,
    expanded: &HashSet<String>,
    collapsed_live: &HashSet<String>,
    expanded_tools: &HashSet<String>,
    expanded_reasoning: &HashSet<String>,
    cx: &mut gpui::Context<AppShell>,
) -> impl IntoElement {
    let key = cluster_key(message_id, segment_index);
    let open = cluster_open(&key, live, expanded, collapsed_live);
    let counts = activity_counts(parts, part_indices);
    let summary = activity_summary_label(counts, live, catalog);
    let toggle_key = key.clone();

    let header_label = if live {
        shimmer_text(summary).into_any_element()
    } else {
        div()
            .font_weight(FontWeight::MEDIUM)
            .child(summary)
            .into_any_element()
    };

    let header = disclosure_header(
        SharedString::from(format!("activity-cluster-{key}")),
        header_label,
        open,
        cx.listener(move |this, _, _, cx| {
            if live {
                if this.collapsed_live_activity_clusters.contains(&toggle_key) {
                    this.collapsed_live_activity_clusters.remove(&toggle_key);
                } else {
                    this.collapsed_live_activity_clusters.insert(toggle_key.clone());
                }
            } else if this.expanded_activity_clusters.contains(&toggle_key) {
                this.expanded_activity_clusters.remove(&toggle_key);
            } else {
                this.expanded_activity_clusters.insert(toggle_key.clone());
            }
            cx.notify();
        }),
    );

    div()
        .w_full()
        .min_w_0()
        .mt(px(10.))
        .mb(px(20.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(header)
        .when(open, |cluster| {
            cluster.child(disclosure_rail(render_activity_items(
                message_index,
                part_indices,
                parts,
                live,
                streaming,
                catalog,
                expanded_tools,
                expanded_reasoning,
                cx,
            )))
        })
}

fn render_activity_items(
    message_index: usize,
    part_indices: &[usize],
    parts: &[MessagePart],
    live: bool,
    streaming: bool,
    catalog: &Catalog,
    expanded_tools: &HashSet<String>,
    expanded_reasoning: &HashSet<String>,
    cx: &mut gpui::Context<AppShell>,
) -> impl IntoElement {
    let last_reasoning_index = part_indices.iter().rev().find_map(|index| {
        matches!(parts[*index], MessagePart::Reasoning { .. }).then_some(*index)
    });

    let mut col = div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .w_full()
        .min_w_0()
        .overflow_hidden();
    for part_index in part_indices {
        let element = match &parts[*part_index] {
            MessagePart::Reasoning {
                id,
                content,
                visible,
            } => {
                let toggle_id = id.clone();
                let reasoning_live =
                    live && streaming && last_reasoning_index == Some(*part_index);
                reasoning_block(
                    id,
                    content,
                    *visible,
                    reasoning_live,
                    streaming,
                    catalog,
                    expanded_reasoning,
                    cx.listener(move |this, _, _, cx| {
                        if !this.expanded_reasoning.remove(&toggle_id) {
                            this.expanded_reasoning.insert(toggle_id.clone());
                        }
                        cx.notify();
                    }),
                )
                .into_any_element()
            }
            MessagePart::ToolCall { tool_call } => {
                let id = tool_call.id.clone();
                tool_card(
                    tool_call,
                    catalog,
                    expanded_tools,
                    cx.listener(move |this, _, _, cx| {
                        if !this.expanded_tools.remove(&id) {
                            this.expanded_tools.insert(id.clone());
                        }
                        cx.notify();
                    }),
                )
                .into_any_element()
            }
            MessagePart::TaskList { tasks } => task_list(tasks, catalog).into_any_element(),
            MessagePart::Text { .. } | MessagePart::Question { .. } => {
                unsupported(catalog, message_index, *part_index)
            }
        };
        col = col.child(
            div()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .child(element),
        );
    }
    col
}

#[cfg(test)]
mod tests {
    use super::{activity_counts, activity_summary_label, message_segments, ActivityCounts};
    use circulo_core::{MessagePart, Task, TaskStatus, ToolCall, ToolCallStatus};
    use circulo_i18n::Catalog;
    use serde_json::json;

    #[test]
    fn segments_split_on_text_boundaries() {
        let parts = vec![
            MessagePart::Reasoning {
                id: "r1".into(),
                content: String::new(),
                visible: true,
            },
            MessagePart::ToolCall {
                tool_call: ToolCall {
                    id: "t1".into(),
                    name: "read".into(),
                    status: ToolCallStatus::Success,
                    input: json!({}),
                    output: None,
                    started_at: None,
                    finished_at: None,
                },
            },
            MessagePart::Text {
                content: "Answer".into(),
            },
            MessagePart::ToolCall {
                tool_call: ToolCall {
                    id: "t2".into(),
                    name: "grep".into(),
                    status: ToolCallStatus::Success,
                    input: json!({}),
                    output: None,
                    started_at: None,
                    finished_at: None,
                },
            },
        ];
        let segments = message_segments(&parts);
        assert_eq!(segments.len(), 3);
    }

    #[test]
    fn summary_lists_activity_units() {
        let catalog = Catalog::english();
        let label = activity_summary_label(
            ActivityCounts {
                plan_steps: 3,
                thoughts: 33,
                commands: 23,
                file_edits: 21,
                file_reads: 7,
                file_searches: 5,
                tool_calls: 5,
            },
            false,
            &catalog,
        );
        assert!(label.contains("3 plan steps"));
        assert!(label.contains("33 thoughts"));
        assert!(label.contains("23 commands"));
    }
}
