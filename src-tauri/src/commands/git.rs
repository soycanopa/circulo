use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileStatus {
    pub path: String,
    /// One of `created`, `modified`, `deleted`, `untracked`.
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
}

/// Diff shaped like the frontend `SessionDiff` so the review panel renders it
/// without changes. `status` stays within created/modified/deleted/unchanged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
    pub status: String,
    pub generated: bool,
}

#[tauri::command]
pub async fn git_status_cmd(project_path: String) -> Result<GitStatus, String> {
    let project = PathBuf::from(&project_path);
    if !project.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || parse_git_status(&project))
        .await
        .map_err(|err| format!("Git status task failed: {err}"))?
}

#[tauri::command]
pub async fn git_file_diff_cmd(
    project_path: String,
    path: String,
) -> Result<GitFileDiff, String> {
    let project = PathBuf::from(&project_path);
    if !project.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    // Security: keep reads inside the project root.
    let candidate = project.join(&path);
    if !candidate.starts_with(&project) {
        return Err("Path escapes the project root".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || load_file_diff(&project, &path, &candidate))
        .await
        .map_err(|err| format!("Git diff task failed: {err}"))?
}

fn run_git(project: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .map_err(|err| format!("Failed to run git: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args.join(" "), stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn classify_status(x: char, y: char) -> &'static str {
    match (x, y) {
        ('?', _) => "untracked",
        ('A', _) | (_, 'A') => "created",
        ('D', _) | (_, 'D') => "deleted",
        _ => "modified",
    }
}

fn clean_path(raw: &str) -> String {
    // porcelain quotes paths containing special chars: "foo bar"
    let unquoted = if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    };
    // renames report "old -> new"; surface the current path
    unquoted
        .rsplit(" -> ")
        .next()
        .unwrap_or(&unquoted)
        .to_string()
}

fn parse_git_status(project: &Path) -> Result<GitStatus, String> {
    let stdout = run_git(project, &["status", "--porcelain=v1", "-b"])?;
    let mut branch = String::new();
    let mut files = Vec::new();
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch = rest.split("...").next().unwrap_or(rest).trim().to_string();
            continue;
        }
        if line.len() < 3 {
            continue;
        }
        let flags = &line[..2];
        let x = flags.chars().next().unwrap_or(' ');
        let y = flags.chars().nth(1).unwrap_or(' ');
        files.push(GitFileStatus {
            path: clean_path(line[2..].trim()),
            status: classify_status(x, y).to_string(),
            staged: x != ' ' && x != '?',
        });
    }
    Ok(GitStatus { branch, files })
}

fn load_file_diff(project: &Path, path: &str, candidate: &Path) -> Result<GitFileDiff, String> {
    let status = status_for_path(project, path)?.unwrap_or("untracked".to_string());
    let (old_text, new_text, status) = match status.as_str() {
        "untracked" | "created" => {
            let work = std::fs::read_to_string(candidate)
                .map_err(|err| format!("Failed to read {}: {err}", candidate.display()))?;
            (String::new(), work, "created".to_string())
        }
        "deleted" => (
            read_head_file(project, path)?.unwrap_or_default(),
            String::new(),
            "deleted".to_string(),
        ),
        _ => (
            read_head_file(project, path)?.unwrap_or_default(),
            std::fs::read_to_string(candidate).unwrap_or_default(),
            "modified".to_string(),
        ),
    };
    Ok(GitFileDiff {
        path: path.to_string(),
        old_text,
        new_text,
        status,
        generated: is_generated_file(path),
    })
}

fn status_for_path(project: &Path, path: &str) -> Result<Option<String>, String> {
    let stdout = run_git(project, &["status", "--porcelain=v1", "--", path])?;
    let line = stdout.lines().next().unwrap_or("");
    if line.len() < 2 {
        return Ok(None);
    }
    let x = line.chars().next().unwrap_or(' ');
    let y = line.chars().nth(1).unwrap_or(' ');
    Ok(Some(classify_status(x, y).to_string()))
}

fn read_head_file(project: &Path, path: &str) -> Result<Option<String>, String> {
    match run_git(project, &["show", &format!("HEAD:{path}")]) {
        Ok(content) => Ok(Some(content)),
        Err(_) => Ok(None),
    }
}

/// Mirrors `GENERATED_FILE_RE` in `src/lib/diff-tools.ts`.
fn is_generated_file(path: &str) -> bool {
    const GENERATED_SEGMENTS: [&str; 13] = [
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
        "dist",
        "build",
        "out",
        "coverage",
        ".next",
        ".nuxt",
        "node_modules",
        "target",
    ];
    let lower = path.to_lowercase();
    if lower.split('/').any(|seg| GENERATED_SEGMENTS.contains(&seg)) {
        return true;
    }
    lower.contains(".min.") || lower.ends_with(".map")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_porcelain_flags() {
        assert_eq!(classify_status('?', '?'), "untracked");
        assert_eq!(classify_status('A', ' '), "created");
        assert_eq!(classify_status(' ', 'A'), "created");
        assert_eq!(classify_status('D', ' '), "deleted");
        assert_eq!(classify_status(' ', 'D'), "deleted");
        assert_eq!(classify_status('M', ' '), "modified");
        assert_eq!(classify_status(' ', 'M'), "modified");
        assert_eq!(classify_status('R', ' '), "modified");
    }

    #[test]
    fn cleans_quoted_and_renamed_paths() {
        assert_eq!(clean_path("\"foo bar.txt\""), "foo bar.txt");
        assert_eq!(clean_path("old.rs -> new.rs"), "new.rs");
        assert_eq!(clean_path("plain.rs"), "plain.rs");
    }

    #[test]
    fn detects_generated_files() {
        assert!(is_generated_file("node_modules/pkg/index.js"));
        assert!(is_generated_file("dist/app.js"));
        assert!(is_generated_file("package-lock.json"));
        assert!(is_generated_file("src/app.min.css"));
        assert!(is_generated_file("src/out.map"));
        assert!(!is_generated_file("src/App.tsx"));
        assert!(!is_generated_file("docs/notes.md"));
    }
}
