//! Skills management — search skills.sh, list installed skills, install and
//! delete. Skills are `SKILL.md` files with `name`/`description` frontmatter,
//! installed either to the project (`.opencode/skills/<name>/`) or globally
//! (`~/.config/opencode/skills/<name>/`).
//!
//! skills.sh exposes a documented `/api/v1` API (https://skills.sh/docs/api)
//! that requires a Vercel OIDC token. When the user provides one we use the
//! official endpoints (semantic search + detail with SKILL.md contents). Without
//! a token we fall back to the public `/api/search` endpoint and GitHub raw
//! fetching — both unauthenticated. Every command reports which mode ran.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Public search endpoint — undocumented but unauthenticated.
const SKILLS_PUBLIC_SEARCH_URL: &str = "https://skills.sh/api/search";
/// Official API base — requires `Authorization: Bearer <VERCEL_OIDC_TOKEN>`.
const SKILLS_API_BASE: &str = "https://skills.sh/api/v1";
const SEARCH_TIMEOUT_MS: u64 = 15_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchResult {
    /// Stable `{source}/{slug}` id (skills.sh).
    pub id: String,
    pub skill_id: String,
    pub slug: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
    #[serde(default)]
    pub description: String,
    /// GitHub repo URL / well-known base (from the authenticated API).
    #[serde(default)]
    pub install_url: Option<String>,
    /// Link to the skill page on skills.sh.
    #[serde(default)]
    pub url: Option<String>,
    /// "github" | "well-known" (from the authenticated API).
    #[serde(default)]
    pub source_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchResponse {
    pub skills: Vec<SkillSearchResult>,
    pub count: usize,
    /// "authenticated" when the official `/api/v1` API answered, "public" when
    /// the unauthenticated fallback was used.
    pub mode: String,
    /// True when the chosen skills.sh path failed or changed shape.
    pub degraded: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    pub name: String,
    pub description: String,
    /// "project" | "global"
    pub scope: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillListResponse {
    pub skills: Vec<InstalledSkill>,
}

#[derive(Debug, Deserialize)]
struct SkillSearchApiResponse {
    #[serde(default)]
    skills: Vec<SkillSearchApiItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillSearchApiItem {
    id: String,
    skill_id: String,
    name: String,
    #[serde(default)]
    installs: u64,
    source: String,
    #[serde(default)]
    description: String,
}

/// Official `/api/v1/skills/search` response.
#[derive(Debug, Deserialize)]
struct SkillSearchV1Response {
    #[serde(default)]
    data: Vec<SkillSearchV1Item>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillSearchV1Item {
    id: String,
    slug: String,
    name: String,
    #[serde(default)]
    installs: u64,
    source: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    install_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
}

/// Official `/api/v1/skills/{id}` detail response (SKILL.md contents included).
#[derive(Debug, Deserialize)]
struct SkillDetailV1Response {
    id: String,
    #[serde(default)]
    files: Option<Vec<SkillFileV1>>,
}

#[derive(Debug, Deserialize)]
struct SkillFileV1 {
    path: String,
    contents: String,
}

fn global_skills_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("opencode").join("skills"))
}

fn project_skills_dir(project_path: Option<&str>) -> Option<PathBuf> {
    project_path.map(|p| PathBuf::from(p).join(".opencode").join("skills"))
}

/// Search skills.sh. Uses the official `/api/v1/skills/search` endpoint when an
/// OIDC token is provided; otherwise falls back to the public `/api/search`
/// endpoint. Either path degrades gracefully instead of failing hard.
#[tauri::command]
pub async fn search_skills_cmd(
    query: String,
    limit: Option<usize>,
    oidc_token: Option<String>,
) -> Result<SkillSearchResponse, String> {
    let q = query.trim().to_string();
    let limit = limit.unwrap_or(10).clamp(1, 100);

    let token = oidc_token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());

    if let Some(token) = token {
        return match search_skills_v1(&q, limit, token).await {
            Ok(Some(response)) => Ok(response),
            Ok(None) => Ok(SkillSearchResponse {
                skills: Vec::new(),
                count: 0,
                mode: "authenticated".to_string(),
                degraded: true,
                error: Some(
                    "skills.sh authenticated API unavailable — check your Vercel OIDC token"
                        .to_string(),
                ),
            }),
            Err(err) => Ok(SkillSearchResponse {
                skills: Vec::new(),
                count: 0,
                mode: "authenticated".to_string(),
                degraded: true,
                error: Some(err),
            }),
        };
    }

    search_skills_public(&q, limit).await
}

async fn search_skills_public(query: &str, limit: usize) -> Result<SkillSearchResponse, String> {
    let url = format!("{SKILLS_PUBLIC_SEARCH_URL}?q={}&limit={}", urlencode(query), limit);
    let response = http_get_json(&url, None).await;
    match response {
        Ok(body) => match serde_json::from_str::<SkillSearchApiResponse>(&body) {
            Ok(parsed) => {
                let skills: Vec<SkillSearchResult> = parsed
                    .skills
                    .into_iter()
                    .map(|s| SkillSearchResult {
                        id: s.id.clone(),
                        skill_id: s.skill_id.clone(),
                        slug: s.skill_id,
                        name: s.name,
                        installs: s.installs,
                        source: s.source,
                        description: s.description,
                        install_url: None,
                        url: None,
                        source_type: None,
                    })
                    .collect();
                Ok(SkillSearchResponse {
                    count: skills.len(),
                    skills,
                    mode: "public".to_string(),
                    degraded: false,
                    error: None,
                })
            }
            Err(err) => Ok(SkillSearchResponse {
                skills: Vec::new(),
                count: 0,
                mode: "public".to_string(),
                degraded: true,
                error: Some(format!("skills.sh returned an unexpected response: {err}")),
            }),
        },
        Err(err) => Ok(SkillSearchResponse {
            skills: Vec::new(),
            count: 0,
            mode: "public".to_string(),
            degraded: true,
            error: Some(format!("skills.sh search failed: {err}")),
        }),
    }
}

/// Official search with an OIDC token. `None` means the endpoint rejected the
/// token (401) or was unreachable.
async fn search_skills_v1(
    query: &str,
    limit: usize,
    token: &str,
) -> Result<Option<SkillSearchResponse>, String> {
    let url = format!("{SKILLS_API_BASE}/skills/search?q={}&limit={}", urlencode(query), limit);
    let body = match http_get_json(&url, Some(token)).await {
        Ok(body) => body,
        Err(err) => {
            // Distinguish "rejected token" from transient failures.
            if err.contains("401") || err.contains("authentication_required") {
                return Ok(None);
            }
            return Err(err);
        }
    };
    match serde_json::from_str::<SkillSearchV1Response>(&body) {
        Ok(parsed) => {
            let skills: Vec<SkillSearchResult> = parsed
                .data
                .into_iter()
                .filter(|s| !s.slug.is_empty())
                .map(|s| SkillSearchResult {
                    id: s.id.clone(),
                    skill_id: s.slug.clone(),
                    slug: s.slug,
                    name: s.name,
                    installs: s.installs,
                    source: s.source,
                    description: s.description,
                    install_url: s.install_url,
                    url: s.url,
                    source_type: s.source_type,
                })
                .collect();
            Ok(Some(SkillSearchResponse {
                count: skills.len(),
                skills,
                mode: "authenticated".to_string(),
                degraded: false,
                error: None,
            }))
        }
        Err(err) => Err(format!(
            "skills.sh authenticated API returned an unexpected response: {err}"
        )),
    }
}

/// List installed skills: project `.opencode/skills` + global
/// `~/.config/opencode/skills`.
#[tauri::command]
pub fn list_skills_cmd(project_path: Option<String>) -> Result<SkillListResponse, String> {
    let mut skills = Vec::new();
    let mut scan = |dir: Option<PathBuf>, scope: &str| {
        let Some(dir) = dir else { return };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let skill_dir = entry.path();
            let skill_md = skill_dir.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(content) = std::fs::read_to_string(&skill_md) {
                let (fm_name, description) = parse_frontmatter(&content);
                skills.push(InstalledSkill {
                    name: if fm_name.is_empty() { name } else { fm_name },
                    description,
                    scope: scope.to_string(),
                    path: skill_md.display().to_string(),
                });
            }
        }
    };
    scan(project_skills_dir(project_path.as_deref()), "project");
    scan(Some(global_skills_dir()?), "global");
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(SkillListResponse { skills })
}

/// Install a skill from skills.sh into the project or globally. With an OIDC
/// token the official `/api/v1/skills/{id}` endpoint supplies the SKILL.md
/// (handles well-known/domain sources and avoids GitHub API limits); without it
/// the GitHub tree is walked for `SKILL.md`.
#[tauri::command]
pub async fn install_skill_cmd(
    name: String,
    source: String,
    target: String,
    project_path: Option<String>,
    id: Option<String>,
    oidc_token: Option<String>,
) -> Result<InstalledSkill, String> {
    let name = sanitize_skill_name(&name)?;
    let token = oidc_token
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let content = match token {
        Some(token) => match id.as_deref().filter(|i| !i.is_empty()) {
            Some(id) => match fetch_skill_md_v1(id, token).await {
                Ok(content) => content,
                Err(err) if err.contains("401") || err.contains("not found") => {
                    fetch_skill_md_github(&name, &source).await?
                }
                Err(err) => return Err(err),
            },
            None => fetch_skill_md_github(&name, &source).await?,
        },
        None => fetch_skill_md_github(&name, &source).await?,
    };
    let (_, description) = parse_frontmatter(&content);

    let dest = match target.as_str() {
        "project" => {
            let root = project_path
                .ok_or_else(|| "Project path is required to install a project skill".to_string())?;
            let dest = PathBuf::from(&root).join(".opencode").join("skills").join(&name);
            ensure_within_root(&dest, &PathBuf::from(&root))?;
            dest
        }
        "global" => global_skills_dir()?.join(&name),
        other => return Err(format!("Unknown install target: {other}")),
    };

    std::fs::create_dir_all(&dest).map_err(|err| format!("Could not create skill dir: {err}"))?;
    std::fs::write(dest.join("SKILL.md"), content)
        .map_err(|err| format!("Could not write SKILL.md: {err}"))?;

    Ok(InstalledSkill {
        name,
        description,
        scope: target,
        path: dest.join("SKILL.md").display().to_string(),
    })
}

/// Delete an installed skill (project or global scope).
#[tauri::command]
pub fn delete_skill_cmd(
    name: String,
    scope: String,
    project_path: Option<String>,
) -> Result<(), String> {
    let name = sanitize_skill_name(&name)?;
    let dir = match scope.as_str() {
        "project" => {
            let root = project_path
                .ok_or_else(|| "Project path is required to delete a project skill".to_string())?;
            let dir = PathBuf::from(&root).join(".opencode").join("skills").join(&name);
            ensure_within_root(&dir, &PathBuf::from(&root))?;
            dir
        }
        "global" => global_skills_dir()?.join(&name),
        other => return Err(format!("Unknown scope: {other}")),
    };
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|err| format!("Could not delete skill: {err}"))?;
    }
    Ok(())
}

fn sanitize_skill_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Skill name must not be empty".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains("..")
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        return Err("Invalid skill name".to_string());
    }
    Ok(trimmed.to_string())
}

fn ensure_within_root(dest: &Path, root: &Path) -> Result<(), String> {
    if !dest.starts_with(root) {
        return Err("Skill path escapes the project root".to_string());
    }
    Ok(())
}

/// Resolve the raw SKILL.md for `source` (`owner/repo`) + `name` (skill id) by
/// walking the repo tree and finding the skill's folder.
async fn fetch_skill_md_github(name: &str, source: &str) -> Result<String, String> {
    if !source.contains('/') {
        return Err(format!(
            "Invalid skill source '{source}' — expected owner/repo"
        ));
    }

    let tree_url = format!(
        "https://api.github.com/repos/{}/git/trees/HEAD?recursive=1",
        source
    );
    let tree_json = http_get_json(&tree_url, None).await?;

    let parsed: serde_json::Value = serde_json::from_str(&tree_json)
        .map_err(|err| format!("Could not parse GitHub tree response: {err}"))?;
    let tree = parsed
        .get("tree")
        .and_then(|t| t.as_array())
        .ok_or_else(|| "GitHub tree response has no 'tree' array".to_string())?;

    let suffix = format!("{name}/SKILL.md");
    let mut candidates: Vec<String> = Vec::new();
    let mut preferred: Option<String> = None;
    for node in tree {
        let path = match node.get("path").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => continue,
        };
        if !path.ends_with(&suffix) {
            continue;
        }
        candidates.push(path.to_string());
        if path.contains("skills/") && preferred.is_none() {
            preferred = Some(path.to_string());
        }
    }

    if candidates.is_empty() {
        return Err(format!(
            "No SKILL.md found for '{name}' in {source} (is the skill id correct?)"
        ));
    }
    if candidates.len() > 1 && preferred.is_none() {
        return Err(format!(
            "Multiple SKILL.md candidates for '{name}' in {source}: {} — install it from the source repo directly",
            candidates.join(", ")
        ));
    }
    let path = preferred.or_else(|| candidates.first().cloned()).unwrap();

    let raw_url = format!(
        "https://raw.githubusercontent.com/{}/HEAD/{}",
        source, path
    );
    http_get_json(&raw_url, None).await
}

/// Fetch SKILL.md via the official `/api/v1/skills/{id}` detail endpoint. Works
/// for GitHub *and* well-known (domain) sources.
async fn fetch_skill_md_v1(id: &str, token: &str) -> Result<String, String> {
    let url = format!("{SKILLS_API_BASE}/skills/{}", urlencode_path(id));
    let body = http_get_json(&url, Some(token)).await?;
    let parsed: SkillDetailV1Response = serde_json::from_str(&body)
        .map_err(|err| format!("Could not parse skill detail response: {err}"))?;
    let skill_md = parsed
        .files
        .as_deref()
        .and_then(|files| files.iter().find(|f| f.path == "SKILL.md"))
        .ok_or_else(|| format!("Skill '{}' has no SKILL.md file", parsed.id))?;
    Ok(skill_md.contents.clone())
}

/// GET a URL, optionally with a Bearer token. Maps HTTP status codes to
/// readable errors so callers can branch on 401/auth failures.
async fn http_get_json(url: &str, token: Option<&str>) -> Result<String, String> {
    let url = url.to_string();
    let token = token.map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_millis(SEARCH_TIMEOUT_MS))
            .build();
        let mut req = agent.get(&url).set("Accept", "application/json");
        if let Some(token) = &token {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }
        match req.call() {
            Ok(res) => res
                .into_string()
                .map_err(|err| format!("Could not read response: {err}")),
            Err(ureq::Error::Status(code, res)) => {
                let detail = res
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(300)
                    .collect::<String>();
                Err(format!("HTTP {code}: {detail}"))
            }
            Err(err) => Err(format!("Request failed: {err}")),
        }
    })
    .await
    .map_err(|err| format!("HTTP task failed: {err}"))?
}

fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Percent-encode every path segment of a skills.sh `{source}/{slug}` id.
fn urlencode_path(id: &str) -> String {
    id.split('/')
        .map(urlencode)
        .collect::<Vec<_>>()
        .join("/")
}

/// Minimal `name:`/`description:` frontmatter extraction from a SKILL.md file.
fn parse_frontmatter(content: &str) -> (String, String) {
    let Some(body) = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) else {
        return (String::new(), String::new());
    };
    let Some(end) = body.find("\n---").or_else(|| body.find("\r\n---")) else {
        return (String::new(), String::new());
    };
    let mut name = String::new();
    let mut description = String::new();
    for line in body[..end].lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = value.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().trim_matches('"').trim_matches('\'').to_string();
        }
    }
    (name, description)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter() {
        let content = "---\nname: tdd\ndescription: \"Test-driven development\"\n---\n# TDD\n";
        let (name, description) = parse_frontmatter(content);
        assert_eq!(name, "tdd");
        assert_eq!(description, "Test-driven development");
    }

    #[test]
    fn empty_without_frontmatter() {
        let (name, description) = parse_frontmatter("# plain skill\n");
        assert_eq!(name, "");
        assert_eq!(description, "");
    }

    #[test]
    fn sanitizes_names() {
        assert!(sanitize_skill_name("tdd").is_ok());
        assert!(sanitize_skill_name("a/b").is_err());
        assert!(sanitize_skill_name("../x").is_err());
        assert!(sanitize_skill_name("").is_err());
    }

    #[test]
    fn urlencodes() {
        assert_eq!(urlencode("tdd js"), "tdd%20js");
        assert_eq!(urlencode("hello-world"), "hello-world");
    }

    #[test]
    fn urlencodes_path_segments() {
        assert_eq!(urlencode_path("mattpocock/skills/tdd"), "mattpocock/skills/tdd");
        assert_eq!(urlencode_path("some owner/repo name/skill"), "some%20owner/repo%20name/skill");
    }

    #[test]
    fn parses_public_search_response() {
        // Real shape returned by the unauthenticated /api/search endpoint.
        let body = r#"{
            "query": "react",
            "searchType": "fuzzy",
            "skills": [
                {
                    "id": "vercel-labs/agent-skills/vercel-react-best-practices",
                    "skillId": "vercel-react-best-practices",
                    "name": "vercel-react-best-practices",
                    "installs": 608939,
                    "source": "vercel-labs/agent-skills"
                }
            ],
            "count": 1,
            "duration_ms": 500
        }"#;
        let parsed: SkillSearchApiResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.skills.len(), 1);
        let item = &parsed.skills[0];
        assert_eq!(item.id, "vercel-labs/agent-skills/vercel-react-best-practices");
        assert_eq!(item.skill_id, "vercel-react-best-practices");
        assert_eq!(item.installs, 608939);
    }

    #[test]
    fn parses_v1_search_response() {
        // Real shape returned by the authenticated /api/v1/skills/search endpoint.
        let body = r#"{
            "data": [
                {
                    "id": "expo/skills/react-native",
                    "slug": "react-native",
                    "name": "React Native",
                    "source": "expo/skills",
                    "installs": 3842,
                    "sourceType": "github",
                    "installUrl": "https://github.com/expo/skills",
                    "url": "https://skills.sh/expo/skills/react-native"
                }
            ],
            "query": "react native",
            "searchType": "semantic",
            "count": 1,
            "durationMs": 142
        }"#;
        let parsed: SkillSearchV1Response = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.data.len(), 1);
        let item = &parsed.data[0];
        assert_eq!(item.slug, "react-native");
        assert_eq!(item.source_type.as_deref(), Some("github"));
        assert!(item.install_url.is_some());
    }

    #[test]
    fn parses_v1_detail_response() {
        // Real shape returned by the authenticated /api/v1/skills/{id} endpoint.
        let body = r#"{
            "id": "vercel-labs/skills/find-skills",
            "source": "vercel-labs/skills",
            "slug": "find-skills",
            "installs": 24531,
            "hash": "abc123",
            "files": [
                { "path": "SKILL.md", "contents": "---\nname: Find Skills\n---\n# Find Skills" },
                { "path": "examples/x.ts", "contents": "// example" }
            ]
        }"#;
        let parsed: SkillDetailV1Response = serde_json::from_str(body).unwrap();
        let files = parsed.files.expect("files");
        let skill_md = files
            .iter()
            .find(|f| f.path == "SKILL.md")
            .expect("SKILL.md file");
        assert!(skill_md.contents.contains("Find Skills"));
    }

    #[test]
    fn rejects_paths_outside_root() {
        let root = PathBuf::from("/tmp/proj");
        assert!(ensure_within_root(&root.join(".opencode/skills/x"), &root).is_ok());
        assert!(ensure_within_root(&PathBuf::from("/tmp/other"), &root).is_err());
    }
}
