use std::path::PathBuf;
use std::process::Command;

/// Resolve a CLI binary when running as a macOS .app (minimal PATH).
pub fn resolve_binary(program: &str, env_override: &str) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var(env_override) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_file() {
                return Ok(candidate);
            }
            return Err(format!(
                "{env_override} points to an invalid path: {trimmed}"
            ));
        }
    }

    if let Some(path) = find_on_path_env(program) {
        return Ok(path);
    }

    for candidate in default_candidates(program) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    if let Some(path) = resolve_via_login_shell(program) {
        return Ok(path);
    }

    Err(format!(
        "Could not find `{program}`. Install it or set {env_override} to the full binary path."
    ))
}

pub fn resolve_opencode() -> Result<PathBuf, String> {
    resolve_binary("opencode", "OPENCODE_BIN")
}

pub fn resolve_cursor_agent() -> Result<PathBuf, String> {
    resolve_binary("cursor-agent", "CURSOR_AGENT_BIN")
}

pub fn resolve_grok() -> Result<PathBuf, String> {
    resolve_binary("grok", "GROK_BIN")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn find_on_path_env(program: &str) -> Option<PathBuf> {
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn default_candidates(program: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if program == "opencode" {
        if let Some(home) = home_dir() {
            paths.push(home.join(".opencode/bin/opencode"));
        }
    }

    if program == "grok" {
        if let Some(home) = home_dir() {
            paths.push(home.join(".grok/bin/grok"));
        }
    }

    if let Some(home) = home_dir() {
        paths.push(home.join(".local/bin").join(program));
        paths.push(home.join("bin").join(program));
    }

    paths.push(PathBuf::from("/opt/homebrew/bin").join(program));
    paths.push(PathBuf::from("/usr/local/bin").join(program));
    paths.push(PathBuf::from("/usr/bin").join(program));

    paths
}

fn resolve_via_login_shell(program: &str) -> Option<PathBuf> {
    let output = Command::new("/bin/zsh")
        .args(["-lic", &format!("command -v {program}")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(path);
    candidate.is_file().then_some(candidate)
}
