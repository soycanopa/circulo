use std::path::Path;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShSearchResultDto {
    pub id: String,
    pub skill_id: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct SkillsShSearchResponse {
    skills: Vec<SkillsShSearchResultDto>,
}

pub async fn search_skills_sh(query: &str) -> Result<Vec<SkillsShSearchResultDto>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let response = reqwest::Client::new()
        .get("https://skills.sh/api/search")
        .query(&[("q", trimmed)])
        .send()
        .await
        .map_err(|err| format!("No se pudo conectar con skills.sh: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "skills.sh respondió con estado {}",
            response.status()
        ));
    }

    let body: SkillsShSearchResponse = response
        .json()
        .await
        .map_err(|err| format!("Respuesta inválida de skills.sh: {err}"))?;

    Ok(body.skills)
}

pub fn install_skills_package(
    package: &str,
    scope: &str,
    project_path: Option<&Path>,
) -> Result<String, String> {
    let mut command = Command::new("npx");
    command.args(["skills", "add", package, "-a", "opencode", "-y"]);

    match scope {
        "global" => {
            command.arg("-g");
        }
        "project" => {
            let path = project_path.ok_or_else(|| "Project path required".to_string())?;
            if !path.is_dir() {
                return Err(format!("Not a directory: {}", path.display()));
            }
            command.current_dir(path);
        }
        _ => return Err("Invalid skill scope".to_string()),
    }

    let output = command
        .output()
        .map_err(|err| format!("Failed to run skills CLI: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        return Ok(if stdout.is_empty() {
            "Skill installed successfully".to_string()
        } else {
            stdout
        });
    }

    let message = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("skills add failed with status {}", output.status)
    };
    Err(message)
}