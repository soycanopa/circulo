//! Lightweight semantic tools for the `circulo-mcp` orchestrator.
//!
//! A pragmatic v1 index: per-language symbol extraction via regex plus
//! word-boundary reference search, bounded to the project root. (A full
//! tree-sitter incremental index is a planned upgrade — see docs/QA.md.)

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Serialize;

const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", ".git", ".svn", ".hg", "__pycache__",
    ".venv", "venv", "vendor", ".next", ".turbo", ".cargo", ".cache",
];

const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "icns", "pdf", "woff", "woff2",
    "ttf", "eot", "mp4", "mp3", "zip", "gz", "tar", "lockb", "bin",
];

fn is_indexable(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    if SKIP_EXTS.contains(&ext) {
        return false;
    }
    for dir in SKIP_DIRS {
        if path.components().any(|c| c.as_os_str() == *dir) {
            return false;
        }
    }
    true
}

fn symbol_patterns_for(ext: &str) -> Vec<(&'static str, &'static str)> {
    match ext {
        "rs" => vec![
            (r"^\s*pub(?:\([^)]*\))?\s+fn\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*fn\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*(?:pub\s+)?struct\s+([A-Za-z_]\w*)", "struct"),
            (r"^\s*(?:pub\s+)?enum\s+([A-Za-z_]\w*)", "enum"),
            (r"^\s*(?:pub\s+)?trait\s+([A-Za-z_]\w*)", "trait"),
            (r"^\s*(?:pub\s+)?impl\s*<[^>]*>\s+([A-Za-z_]\w*)", "impl"),
            (r"^\s*(?:pub\s+)?impl\s+([A-Za-z_]\w*)", "impl"),
            (r"^\s*(?:pub\s+)?const\s+([A-Za-z_]\w*)", "const"),
            (r"^\s*(?:pub\s+)?static\s+([A-Za-z_]\w*)", "static"),
        ],
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => vec![
            (r"^\s*export\s+(?:default\s+)?(?:async\s+)?function\s+([A-Za-z_$]\w*)", "fn"),
            (r"^\s*(?:async\s+)?function\s+([A-Za-z_$]\w*)", "fn"),
            (r"^\s*export\s+(?:default\s+)?class\s+([A-Za-z_$]\w*)", "class"),
            (r"^\s*class\s+([A-Za-z_$]\w*)", "class"),
            (r"^\s*export\s+(?:const|let|var)\s+([A-Za-z_$]\w*)", "const"),
            (r"^\s*(?:const|let|var)\s+([A-Za-z_$]\w*)", "const"),
            (r"^\s*interface\s+([A-Za-z_$]\w*)", "interface"),
            (r"^\s*type\s+([A-Za-z_$]\w*)\s*=", "type"),
        ],
        "py" => vec![
            (r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*class\s+([A-Za-z_]\w*)", "class"),
        ],
        "go" => vec![
            (r"^\s*func\s+(?:\([^)]*\)\s+)?([A-Za-z_]\w*)", "fn"),
            (r"^\s*type\s+([A-Za-z_]\w*)\s+struct", "struct"),
            (r"^\s*type\s+([A-Za-z_]\w*)\s+interface", "interface"),
            (r"^\s*(?:var|const)\s+([A-Za-z_]\w*)", "const"),
        ],
        "c" | "h" | "cpp" | "hpp" | "cc" => vec![
            (r"^\s*(?:static\s+)?(?:inline\s+)?(?:[\w:*<>,\s]+?)\s+([A-Za-z_]\w*)\s*\(", "fn"),
            (r"^\s*class\s+([A-Za-z_]\w*)", "class"),
            (r"^\s*struct\s+([A-Za-z_]\w*)", "struct"),
            (r"^\s*#define\s+([A-Za-z_]\w*)", "const"),
        ],
        "swift" => vec![
            (r"^\s*(?:public\s+|private\s+|internal\s+|fileprivate\s+|open\s+)*func\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*(?:public\s+|private\s+|internal\s+|open\s+)*class\s+([A-Za-z_]\w*)", "class"),
            (r"^\s*(?:public\s+|private\s+|internal\s+|open\s+)*struct\s+([A-Za-z_]\w*)", "struct"),
            (r"^\s*(?:public\s+|private\s+|internal\s+|open\s+)*enum\s+([A-Za-z_]\w*)", "enum"),
            (r"^\s*(?:public\s+|private\s+|internal\s+|open\s+)*let\s+([A-Za-z_]\w*)", "const"),
        ],
        "kt" | "kts" => vec![
            (r"^\s*(?:public\s+|private\s+|internal\s+|protected\s+)*(?:suspend\s+)?fun\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*(?:public\s+|private\s+|internal\s+|data\s+|sealed\s+)*class\s+([A-Za-z_]\w*)", "class"),
            (r"^\s*(?:public\s+|private\s+|internal\s+)*interface\s+([A-Za-z_]\w*)", "interface"),
        ],
        "sh" | "bash" | "zsh" | "fish" => vec![
            (r"^\s*function\s+([A-Za-z_]\w*)", "fn"),
            (r"^\s*([A-Za-z_]\w*)\s*\(\)\s*\{", "fn"),
            (r"^\s*export\s+([A-Za-z_]\w*)=", "const"),
        ],
        "md" | "mdx" => vec![(r"^##+\s+(.+)$", "heading")],
        _ => vec![],
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolMatch {
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceMatch {
    pub file: String,
    pub line: usize,
    pub text: String,
}

fn walk_source_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() == 0 {
                return true;
            }
            if name.starts_with('.') {
                return false;
            }
            !SKIP_DIRS.contains(&name.as_ref())
        })
        .flatten()
    {
        if files.len() >= max_files {
            break;
        }
        if entry.file_type().is_file() && is_indexable(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }
    files
}

fn relative_display(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| file.to_string_lossy().into_owned())
}

/// Find declarations matching `name` (case-insensitive substring on symbol).
pub fn find_symbol(root: &Path, name: &str) -> Vec<SymbolMatch> {
    let query = name.to_lowercase();
    let mut out = Vec::new();
    for file in walk_source_files(root, 400) {
        let Some(ext) = file.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let patterns = symbol_patterns_for(ext);
        if patterns.is_empty() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (pattern, kind) in patterns {
            let Ok(re) = Regex::new(pattern) else {
                continue;
            };
            for (line_idx, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if let Some(caps) = re.captures(trimmed) {
                    let Some(symbol) = caps.get(1).map(|m| m.as_str().to_string()) else {
                        continue;
                    };
                    if symbol.to_lowercase().contains(&query) {
                        out.push(SymbolMatch {
                            file: relative_display(root, &file),
                            line: line_idx + 1,
                            kind: kind.to_string(),
                            name: symbol,
                            signature: line.trim().to_string(),
                        });
                    }
                    // Only the first pattern that matches a line wins.
                    break;
                }
            }
        }
    }
    out
}

/// Find all (case-insensitive, word-boundary) usages of `name`.
pub fn get_references(root: &Path, name: &str) -> Vec<ReferenceMatch> {
    let mut out = Vec::new();
    if name.trim().is_empty() {
        return out;
    }
    let Ok(word_re) = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(name.trim()))) else {
        return out;
    };
    for file in walk_source_files(root, 400) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (line_idx, line) in text.lines().enumerate() {
            if word_re.is_match(line) {
                out.push(ReferenceMatch {
                    file: relative_display(root, &file),
                    line: line_idx + 1,
                    text: line.trim().to_string(),
                });
            }
        }
    }
    out
}

/// List symbols in a single file (absolute path must stay under `root`).
pub fn outline(root: &Path, file: &Path) -> Result<Vec<SymbolMatch>, String> {
    let canonical_root = root.canonicalize().map_err(|err| err.to_string())?;
    let canonical_file = file.canonicalize().map_err(|err| err.to_string())?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err("File escapes project root".to_string());
    }
    let Some(ext) = file.extension().and_then(|e| e.to_str()) else {
        return Ok(Vec::new());
    };
    let patterns = symbol_patterns_for(ext);
    let text = std::fs::read_to_string(&canonical_file).map_err(|err| err.to_string())?;
    let mut out = Vec::new();
    for (pattern, kind) in patterns {
        let Ok(re) = Regex::new(pattern) else {
            continue;
        };
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if let Some(caps) = re.captures(trimmed) {
                if let Some(symbol) = caps.get(1).map(|m| m.as_str().to_string()) {
                    out.push(SymbolMatch {
                        file: relative_display(root, &canonical_file),
                        line: line_idx + 1,
                        kind: kind.to_string(),
                        name: symbol,
                        signature: line.trim().to_string(),
                    });
                }
                break;
            }
        }
    }
    Ok(out)
}

/// Resolve the project root for the sidecar, preferring `CIRCULO_PROJECT_ROOT`.
pub fn project_root() -> PathBuf {
    std::env::var("CIRCULO_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rust_symbols() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn handle_prompt() {}\nfn private_helper() {}\nstruct Config {}\n",
        )
        .unwrap();
        let matches = find_symbol(dir.path(), "handle");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "handle_prompt");
        assert_eq!(matches[0].kind, "fn");
        assert_eq!(matches[0].line, 1);
    }

    #[test]
    fn finds_references_with_word_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.rs"),
            "use foo;\nlet foo_handle = 1;\nfoo();\nfoobar();\n",
        )
        .unwrap();
        let refs = get_references(dir.path(), "foo");
        let lines: Vec<usize> = refs.iter().map(|r| r.line).collect();
        assert!(lines.contains(&3), "plain foo() line");
        assert!(!lines.contains(&4), "foobar must not match");
    }

    #[test]
    fn outline_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.ts"), "export function a() {}\nexport const b = 1;\ninterface C {}\n")
            .unwrap();
        let symbols = outline(dir.path(), &dir.path().join("x.ts")).unwrap();
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"C"));
    }

    #[test]
    fn outline_rejects_outside_paths() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "x").unwrap();
        let err = outline(dir.path(), &outside.path().join("secret.txt")).unwrap_err();
        assert!(err.contains("escapes"));
    }
}
