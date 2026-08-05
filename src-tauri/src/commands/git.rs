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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchInfo {
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranches {
    /// Current branch name, or short HEAD SHA when detached.
    pub current: String,
    pub detached: bool,
    pub local: Vec<GitBranchInfo>,
    pub remote: Vec<GitBranchInfo>,
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

#[tauri::command]
pub async fn git_branches_cmd(project_path: String) -> Result<GitBranches, String> {
    let project = PathBuf::from(&project_path);
    if !project.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || list_branches(&project))
        .await
        .map_err(|err| format!("Git branches task failed: {err}"))?
}

#[tauri::command]
pub async fn git_checkout_cmd(
    project_path: String,
    name: String,
) -> Result<GitBranches, String> {
    let project = PathBuf::from(&project_path);
    if !project.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    let name = name.trim().to_string();
    if !is_valid_branch_name(&name) {
        return Err("Branch name must not be empty or contain whitespace".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        checkout_branch(&project, &name)?;
        list_branches(&project)
    })
    .await
    .map_err(|err| format!("Git checkout task failed: {err}"))?
}

/// Resolve the checkout args for a branch. Remote branches create a local
/// tracking branch with the short name (`origin/x` → `git checkout -b x origin/x`).
fn checkout_args(name: &str) -> Vec<String> {
    match name.strip_prefix("origin/") {
        Some(short) if !short.is_empty() && !short.contains('/') => {
            vec![
                "checkout".to_string(),
                "-b".to_string(),
                short.to_string(),
                name.to_string(),
            ]
        }
        _ => vec!["checkout".to_string(), name.to_string()],
    }
}

fn checkout_branch(project: &Path, name: &str) -> Result<(), String> {
    // If the remote's short name already exists as a local branch, a plain
    // checkout wins (git would otherwise refuse `-b` with an existing branch).
    let short = name
        .strip_prefix("origin/")
        .filter(|s| !s.is_empty() && !s.contains('/'));
    if let Some(short) = short {
        if !run_git(project, &["branch", "--list", short])?
            .trim()
            .is_empty()
        {
            run_git(project, &["checkout", short])?;
            return Ok(());
        }
    }
    let args = checkout_args(name);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git(project, &refs)?;
    Ok(())
}

#[tauri::command]
pub async fn git_create_branch_cmd(
    project_path: String,
    name: String,
    start_point: Option<String>,
) -> Result<GitBranches, String> {
    let project = PathBuf::from(&project_path);
    if !project.is_dir() {
        return Err("Project path is not a directory".to_string());
    }
    let name = name.trim().to_string();
    if !is_valid_branch_name(&name) {
        return Err("Branch name must not be empty or contain whitespace".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        match start_point {
            Some(start) => run_git(&project, &["checkout", "-b", &name, &start])?,
            None => run_git(&project, &["checkout", "-b", &name])?,
        };
        list_branches(&project)
    })
    .await
    .map_err(|err| format!("Git create branch task failed: {err}"))?
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

fn list_branches(project: &Path) -> Result<GitBranches, String> {
    let current_raw = run_git(project, &["branch", "--show-current"])?;
    let current = current_raw.trim().to_string();
    let detached = current.is_empty();
    let display_current = if detached {
        run_git(project, &["rev-parse", "--short", "HEAD"])?.trim().to_string()
    } else {
        current.clone()
    };

    let local_raw = run_git(project, &["branch", "--format=%(HEAD)%(refname:short)"])?;
    let mut local = Vec::new();
    for line in local_raw.lines() {
        let Some((name, is_current)) = parse_local_branch_line(line) else {
            continue;
        };
        let upstream = if !detached && name == current {
            branch_upstream(project, &name)
        } else {
            None
        };
        local.push(GitBranchInfo {
            name,
            current: is_current,
            remote: false,
            upstream,
        });
    }

    let remote_raw = run_git(project, &["branch", "-r", "--format=%(refname:short)"])?;
    let mut remote = Vec::new();
    for line in remote_raw.lines() {
        let name = line.trim();
        // Skip `origin/HEAD` and bare remote refs (e.g. a lone `origin`).
        if name.is_empty() || name.ends_with("/HEAD") || !name.contains('/') {
            continue;
        }
        remote.push(GitBranchInfo {
            name: name.to_string(),
            current: false,
            remote: true,
            upstream: None,
        });
    }

    Ok(GitBranches {
        current: display_current,
        detached,
        local,
        remote,
    })
}

/// Parse one `git branch --format=%(HEAD)%(refname:short)` line: `*main` or ` main`.
/// Returns `(name, is_current)`, or `None` for blank lines.
fn parse_local_branch_line(line: &str) -> Option<(String, bool)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (is_current, name) = match line.strip_prefix('*') {
        Some(rest) => (true, rest.trim()),
        None => (false, line.trim()),
    };
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), is_current))
}

/// Validates a user-supplied branch name for `git create branch`.
fn is_valid_branch_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(char::is_whitespace)
}

fn branch_upstream(project: &Path, name: &str) -> Option<String> {
    let out = run_git(
        project,
        &["rev-parse", "--abbrev-ref", &format!("{name}@{{upstream}}")],
    )
    .ok()?;
    let trimmed = out.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

    #[test]
    fn parses_local_branch_lines() {
        assert_eq!(
            parse_local_branch_line("*main"),
            Some(("main".to_string(), true))
        );
        assert_eq!(
            parse_local_branch_line(" feat/x"),
            Some(("feat/x".to_string(), false))
        );
        assert_eq!(parse_local_branch_line(""), None);
        assert_eq!(parse_local_branch_line("  "), None);
    }

    #[test]
    fn validates_branch_names() {
        assert!(is_valid_branch_name("feat/cool"));
        assert!(is_valid_branch_name("release-1.2.3"));
        assert!(!is_valid_branch_name(""));
        assert!(!is_valid_branch_name("  "));
        assert!(!is_valid_branch_name("has space"));
        assert!(!is_valid_branch_name("tab\there"));
    }

    #[test]
    fn resolves_checkout_args_for_remotes() {
        assert_eq!(checkout_args("main"), vec!["checkout", "main"]);
        assert_eq!(
            checkout_args("origin/feature"),
            vec!["checkout", "-b", "feature", "origin/feature"]
        );
        // Nested remote path is checked out directly (rare, but safe).
        assert_eq!(
            checkout_args("origin/team/nested"),
            vec!["checkout", "origin/team/nested"]
        );
    }

    #[test]
    fn lists_and_switches_branches_in_real_repo() {
        let result = run_real_repo_test();
        if let Err(err) = result {
            // Sandboxed environments may block writes outside the project.
            if err.contains("Operation not permitted") {
                eprintln!("skipping git integration test (sandboxed temp dir): {err}");
                return;
            }
            panic!("{err}");
        }
    }

    fn run_real_repo_test() -> Result<(), String> {
        let work = tempfile::tempdir().map_err(|err| err.to_string())?;
        let remote = tempfile::tempdir().map_err(|err| err.to_string())?;
        let project = work.path();
        run_git(remote.path(), &["init", "--bare"])?;
        run_git(project, &["init", "-b", "main"])?;
        run_git(project, &["config", "user.email", "test@example.com"])?;
        run_git(project, &["config", "user.name", "Test"])?;
        std::fs::write(project.join("file.txt"), "hello").map_err(|err| err.to_string())?;
        run_git(project, &["add", "."])?;
        run_git(project, &["commit", "-m", "init"])?;

        run_git(project, &["branch", "feature/x"])?;
        run_git(
            project,
            &["remote", "add", "origin", remote.path().to_str().unwrap()],
        )?;
        run_git(project, &["push", "origin", "main"])?;
        run_git(project, &["branch", "--set-upstream-to=origin/main", "main"])?;

        let branches = list_branches(project)?;
        assert_eq!(branches.current, "main");
        assert!(!branches.detached);
        let main_local = branches
            .local
            .iter()
            .find(|branch| branch.name == "main")
            .expect("main local branch");
        assert!(main_local.current);
        assert_eq!(main_local.upstream.as_deref(), Some("origin/main"));
        assert!(branches
            .local
            .iter()
            .any(|branch| branch.name == "feature/x" && !branch.current));
        assert!(branches
            .remote
            .iter()
            .any(|branch| branch.name == "origin/main" && branch.remote));

        // Switch to the other local branch and confirm the list updates.
        checkout_branch(project, "feature/x")?;
        let after = list_branches(project)?;
        assert_eq!(after.current, "feature/x");
        assert!(after
            .local
            .iter()
            .find(|branch| branch.name == "feature/x")
            .unwrap()
            .current);

        // Checking out a remote branch whose local already exists just switches.
        checkout_branch(project, "origin/main")?;
        let tracked = list_branches(project)?;
        assert_eq!(tracked.current, "main");

        Ok(())
    }
}
