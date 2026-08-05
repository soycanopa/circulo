//! Shared token/compression engine.
//!
//! Used by the `circulo-mcp` orchestrator (token-optimizer tools) and by the
//! Fase 5 terminal filters. `compact_result` is **reversible**: the original
//! text is stored under `~/.circulo/cache/<hash>` and can be fetched with
//! `retrieve_original`.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

/// ~4 chars per token heuristic (CJK chars count as one token each).
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    let cjk = text
        .chars()
        .filter(|c| matches!(*c as u32, 0x4E00..=0x9FFF | 0x3000..=0x303F | 0xFF00..=0xFFEF))
        .count();
    (chars.saturating_sub(cjk)) / 4 + cjk + 1
}

const COMPACT_THRESHOLD_CHARS: usize = 2400;
const KEEP_HEAD_LINES: usize = 40;
const KEEP_TAIL_LINES: usize = 20;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionStats {
    pub original_bytes: usize,
    pub compacted_bytes: usize,
    pub saved_bytes: usize,
    pub original_ref: String,
    pub detected_kind: String,
}

thread_local! {
    /// Test/embedding override for the compaction cache location. Production
    /// code never sets this; tests point it at a fresh temp dir per test thread.
    static CACHE_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

fn cache_dir() -> Result<PathBuf, String> {
    let dir = match CACHE_DIR_OVERRIDE.with(|c| c.borrow().clone()) {
        Some(dir) => dir,
        None => {
            let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
            PathBuf::from(home).join(".circulo").join("cache")
        }
    };
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn hash_hex(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn cache_reference(text: &str) -> String {
    format!("circulo-cache://{}", hash_hex(text))
}

/// Persist the original text so `retrieve_original` can restore it later.
fn store_original(text: &str) -> Result<String, String> {
    let reference = cache_reference(text);
    let file = cache_dir()?.join(format!("{}.txt", hash_hex(text)));
    if !file.is_file() {
        std::fs::write(&file, text.as_bytes()).map_err(|err| err.to_string())?;
    }
    Ok(reference)
}

/// Fetch a previously compacted original by its `circulo-cache://<hash>` ref.
pub fn retrieve_original(reference: &str) -> Option<String> {
    let hash = reference.strip_prefix("circulo-cache://")?;
    let file = cache_dir().ok()?.join(format!("{hash}.txt"));
    std::fs::read_to_string(file).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactKind {
    JsonArray,
    GitStatus,
    GitDiff,
    TestOutput,
    LogLines,
    PathList,
    Prose,
}

pub fn detect_kind(content: &str) -> CompactKind {
    let trimmed = content.trim_start();
    if trimmed.starts_with('[') && trimmed.contains('"') {
        return CompactKind::JsonArray;
    }
    if content.contains("@@") && (content.contains("+++") || content.contains("diff --git")) {
        return CompactKind::GitDiff;
    }
    let first_lines: Vec<&str> = content.lines().take(6).collect();
    if first_lines.iter().any(|l| l.starts_with("On branch"))
        || first_lines.iter().any(|l| l.starts_with("Changes"))
        || first_lines.iter().any(|l| l.starts_with("Your branch"))
    {
        return CompactKind::GitStatus;
    }
    let has_test_markers = content
        .lines()
        .take(20)
        .any(|l| l.contains("passed") || l.contains("failed") || l.contains("Test run"));
    if has_test_markers || content.contains("✓") || content.contains("✗") {
        return CompactKind::TestOutput;
    }
    let lines = content.lines().count();
    let pathish = content
        .lines()
        .take(30)
        .filter(|l| l.starts_with('/') || l.starts_with("./") || l.ends_with('/'))
        .count();
    if pathish as f64 / (lines.min(30) as f64 + 0.001) > 0.6 {
        return CompactKind::PathList;
    }
    let unique = content
        .lines()
        .map(|l| l.trim_end())
        .collect::<std::collections::HashSet<_>>()
        .len();
    if lines > 12 && unique as f64 / (lines as f64) < 0.5 {
        return CompactKind::LogLines;
    }
    CompactKind::Prose
}

fn kind_label(kind: CompactKind) -> &'static str {
    match kind {
        CompactKind::JsonArray => "json-array",
        CompactKind::GitStatus => "git-status",
        CompactKind::GitDiff => "git-diff",
        CompactKind::TestOutput => "test-output",
        CompactKind::LogLines => "log-lines",
        CompactKind::PathList => "path-list",
        CompactKind::Prose => "prose",
    }
}

fn footer(reference: &str) -> String {
    format!(
        "\n\n[marcador-circulo output compactado por circulo-mcp — llama retrieve_original(\"{reference}\") para el texto completo]"
    )
}

/// Compress a large text block. Returns the compacted text plus stats when the
/// content was actually reduced; `None` when it was small enough to pass through.
pub fn compact_result(content: &str) -> Result<(String, Option<CompressionStats>), String> {
    let original_bytes = content.len();
    if original_bytes <= COMPACT_THRESHOLD_CHARS {
        return Ok((content.to_string(), None));
    }
    let kind = detect_kind(content);
    let reference = store_original(content)?;

    let compacted = match kind {
        CompactKind::JsonArray => compact_json_array(content),
        CompactKind::GitStatus => compact_git_status(content),
        CompactKind::GitDiff => compact_git_diff(content),
        CompactKind::TestOutput => compact_test_output(content),
        CompactKind::LogLines => compact_log_lines(content),
        CompactKind::PathList => compact_path_list(content),
        CompactKind::Prose => compact_prose(content),
    };

    let mut compacted = compacted;
    compacted.push_str(&footer(&reference));
    let compacted_bytes = compacted.len();

    let stats = CompressionStats {
        original_bytes,
        compacted_bytes,
        saved_bytes: original_bytes.saturating_sub(compacted_bytes),
        original_ref: reference,
        detected_kind: kind_label(kind).to_string(),
    };
    Ok((compacted, Some(stats)))
}

fn compact_json_array(content: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(Value::Array(items)) => {
            let total = items.len();
            let shown = items.iter().take(3).collect::<Vec<_>>();
            let mut out = format!("[{total} items] primeras {}\n", shown.len());
            for item in shown {
                let mut compact = serde_json::to_string(item).unwrap_or_default();
                if compact.len() > 300 {
                    compact.truncate(300);
                    compact.push('…');
                }
                out.push_str(&compact);
                out.push('\n');
            }
            out
        }
        _ => compact_prose(content),
    }
}

fn compact_git_status(content: &str) -> String {
    let mut out = String::new();
    let mut file_lines = Vec::new();
    for line in content.lines() {
        if line.starts_with("On branch")
            || line.starts_with("Your branch")
            || line.starts_with("Changes")
            || line.starts_with("Untracked")
            || line.trim().is_empty()
        {
            if !line.trim().is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        } else {
            file_lines.push(line);
        }
    }
    if !file_lines.is_empty() {
        out.push_str(&format!(
            "\n[{} archivos]\n",
            file_lines.len()
        ));
        for line in file_lines.iter().take(30) {
            out.push_str(line);
            out.push('\n');
        }
        if file_lines.len() > 30 {
            out.push_str(&format!("… {} más\n", file_lines.len() - 30));
        }
    }
    out
}

fn compact_git_diff(content: &str) -> String {
    let mut out = String::new();
    let mut files = 0usize;
    let mut hunks = 0usize;
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("diff --git") {
            files += 1;
            if files <= 6 {
                out.push_str(line);
                out.push('\n');
            }
        } else if line.starts_with("@@") {
            hunks += 1;
            if files <= 6 && hunks <= 12 {
                out.push_str(line);
                out.push('\n');
            }
        } else if line.starts_with('+') || line.starts_with('-') {
            if files <= 6 && hunks <= 12 {
                if out.chars().count() < 6000 {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
    out.push_str(&format!("\n[{files} archivos, {hunks} hunks — usa retrieve_original para el diff completo]\n"));
    out
}

fn compact_test_output(content: &str) -> String {
    let mut out = String::new();
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    for line in content.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if l.starts_with("FAILED")
            || l.starts_with("failed")
            || l.contains("✗")
            || l.contains("AssertionError")
            || l.starts_with("Error:")
            || l.starts_with("FAIL ")
            || l.starts_with("not ok")
        {
            failures.push(line);
            continue;
        }
        if l.starts_with("Tests:") || l.starts_with("Test run") || l.starts_with("Test Files") {
            summary.push(line);
        }
    }
    for line in summary.iter().take(10) {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&format!("\n[{} fallos]\n", failures.len()));
    for line in failures.iter().take(20) {
        out.push_str(line);
        out.push('\n');
    }
    if failures.len() > 20 {
        out.push_str(&format!("… {} fallos más\n", failures.len() - 20));
    }
    out
}

fn compact_log_lines(content: &str) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for line in content.lines() {
        *counts.entry(line.trim_end()).or_insert(0) += 1;
    }
    let mut entries: Vec<(&str, usize)> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut out = format!("[{} líneas, {} únicas]\n", content.lines().count(), entries.len());
    for (line, count) in entries.iter().take(15) {
        if *count > 1 {
            out.push_str(&format!("{count}× {line}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if entries.len() > 15 {
        out.push_str(&format!("… {} líneas únicas más\n", entries.len() - 15));
    }
    out
}

fn compact_path_list(content: &str) -> String {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut out = format!("[{} rutas]\n", lines.len());
    for line in lines.iter().take(40) {
        out.push_str(line);
        out.push('\n');
    }
    if lines.len() > 40 {
        out.push_str(&format!("… {} más\n", lines.len() - 40));
    }
    out
}

fn compact_prose(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut out = String::new();
    for line in lines.iter().take(KEEP_HEAD_LINES) {
        out.push_str(line);
        out.push('\n');
    }
    let omitted = total.saturating_sub(KEEP_HEAD_LINES + KEEP_TAIL_LINES);
    if omitted > 0 {
        out.push_str(&format!("\n… [{omitted} líneas omitidas — usa retrieve_original para el texto completo] …\n\n"));
    }
    for line in lines.iter().skip(total.saturating_sub(KEEP_TAIL_LINES)) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Extractive summary: keeps the head, a middle sample, and the tail.
pub fn summarize(content: &str, max_tokens: usize) -> (String, usize) {
    let original_tokens = estimate_tokens(content);
    if original_tokens <= max_tokens {
        return (content.to_string(), original_tokens);
    }
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let budget_lines = (max_tokens / 3).max(8);
    let head = budget_lines / 2;
    let tail = budget_lines - head;
    let mut out = String::new();
    for line in lines.iter().take(head) {
        out.push_str(line);
        out.push('\n');
    }
    let omitted = total.saturating_sub(head + tail);
    if omitted > 0 {
        out.push_str(&format!("… [{omitted} líneas omitidas] …\n"));
    }
    for line in lines.iter().skip(total.saturating_sub(tail)) {
        out.push_str(line);
        out.push('\n');
    }
    (out, original_tokens)
}

// ---------------------------------------------------------------------------
// Fase 5 — terminal output filters (command-aware compaction)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalFilterStats {
    pub original_bytes: usize,
    pub filtered_bytes: usize,
    pub saved_bytes: usize,
    pub applied: bool,
    /// The output to hand back to the agent (compacted or passthrough).
    pub filtered: String,
}

/// Filter a delegated terminal command's output before returning it to the
/// agent. Command-specific rules produce a terse summary; everything else falls
/// back to `compact_result`. The **raw** output is still streamed to the UI.
pub fn filter_terminal_output(command: &str, output: &str) -> TerminalFilterStats {
    let original_bytes = output.len();
    let command_key = command.trim().to_ascii_lowercase();

    let (filtered, applied) = if command_key.starts_with("git status") {
        (compact_git_status(output), true)
    } else if command_key.starts_with("git diff") {
        (compact_git_diff(output), true)
    } else if command_key.starts_with("ls") || command_key.starts_with("find ") || command_key.starts_with("rg --files") {
        (compact_path_list(output), true)
    } else {
        match compact_result(output) {
            Ok((text, Some(_))) => (text, true),
            Ok((text, None)) => (text, false),
            Err(_) => (output.to_string(), false),
        }
    };

    TerminalFilterStats {
        original_bytes,
        filtered_bytes: filtered.len(),
        saved_bytes: original_bytes.saturating_sub(filtered.len()),
        applied,
        filtered,
    }
}

/// Fallback byte-limit truncation used when the agent requests a limit.
pub fn truncate_bytes(content: &str, limit: usize) -> (String, bool) {
    if content.len() <= limit {
        return (content.to_string(), false);
    }
    let mut cut = limit;
    while cut > 0 && !content.is_char_boundary(cut) {
        cut -= 1;
    }
    (format!("{}\n… [salida truncada]", &content[..cut]), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Point the compaction cache at a fresh temp dir for the duration of `f`,
    /// so tests never touch the real `~/.circulo/cache` and never race on it.
    fn with_temp_cache<R>(f: impl FnOnce() -> R) -> R {
        let dir = tempfile::tempdir().unwrap();
        CACHE_DIR_OVERRIDE.with(|c| {
            *c.borrow_mut() = Some(dir.path().to_path_buf());
        });
        let result = f();
        CACHE_DIR_OVERRIDE.with(|c| {
            *c.borrow_mut() = None;
        });
        result
    }

    #[test]
    fn estimates_tokens() {
        assert!(estimate_tokens("hello world") >= 1);
        assert!(estimate_tokens("中文测试") >= 4);
        assert!(estimate_tokens(&"a".repeat(400)) >= 90);
    }

    #[test]
    fn small_content_is_passthrough() {
        let text = "short text".repeat(20);
        let (out, stats) = compact_result(&text).unwrap();
        assert_eq!(out, text);
        assert!(stats.is_none());
    }

    #[test]
    fn large_content_compacts_and_is_reversible() {
        with_temp_cache(|| {
            let mut text = String::new();
            for i in 0..600 {
                text.push_str(&format!("line {i} — some padding data here\n"));
            }
            let (out, stats) = compact_result(&text).unwrap();
            let stats = stats.expect("stats");
            assert!(out.len() < text.len());
            assert!(stats.original_bytes > stats.compacted_bytes);
            assert_eq!(stats.saved_bytes, stats.original_bytes - stats.compacted_bytes);
            let restored = retrieve_original(&stats.original_ref).expect("restorable");
            assert_eq!(restored, text);
        })
    }

    #[test]
    fn git_status_keeps_summary() {
        with_temp_cache(|| {
            let mut status = String::from(
                "On branch main\nYour branch is up to date\nChanges not staged for commit:\n",
            );
            for i in 0..300 {
                status.push_str(&format!("  modified: src/file_{i}.rs\n"));
            }
            let (out, stats) = compact_result(&status).unwrap();
            assert!(stats.is_some(), "large git status should compact");
            assert!(out.contains("[300 archivos]"));
            assert!(out.contains("On branch main"));
        })
    }

    #[test]
    fn json_arrays_compact() {
        with_temp_cache(|| {
            let items: Vec<serde_json::Value> = (0..200)
                .map(|i| serde_json::json!({ "id": i, "name": format!("item {i}") }))
                .collect();
            let text = serde_json::to_string(&items).unwrap();
            let (out, _) = compact_result(&text).unwrap();
            assert!(out.contains("[200 items]"));
            assert!(out.len() < text.len());
        })
    }

    #[test]
    fn terminal_filter_returns_stats() {
        let out = "total 0\ndrwxr-xr-x  2 user  staff   64 Jan  1 12:00 src\n".repeat(300);
        let stats = filter_terminal_output("ls -la", &out);
        assert!(stats.applied);
        assert!(stats.saved_bytes > 0);
        assert_eq!(
            stats.original_bytes - stats.filtered_bytes,
            stats.saved_bytes
        );
    }
}
