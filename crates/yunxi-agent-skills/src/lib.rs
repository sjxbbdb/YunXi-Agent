use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

pub const SKILL_FILE_NAME: &str = "SKILL.md";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: Option<String>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillInjection {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub name: String,
    pub arguments_json: Option<String>,
}

pub fn discover_skills(root: impl AsRef<Path>) -> AgentResult<Vec<SkillMetadata>> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|error| AgentError::Execution {
        message: format!(
            "failed to read skills directory {}: {error}",
            root.display()
        ),
    })? {
        let entry = entry.map_err(|error| AgentError::Execution {
            message: format!("failed to read skills directory entry: {error}"),
        })?;
        let skill_file = entry.path().join(SKILL_FILE_NAME);
        if !skill_file.is_file() {
            continue;
        }
        let content =
            std::fs::read_to_string(&skill_file).map_err(|error| AgentError::Execution {
                message: format!("failed to read skill {}: {error}", skill_file.display()),
            })?;
        skills.push(parse_skill_metadata(skill_file, &content));
    }
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

fn parse_skill_metadata(path: PathBuf, content: &str) -> SkillMetadata {
    let mut name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or("skill")
        .to_string();
    let mut description = None;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = value.trim().trim_matches('"').to_string();
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').to_string());
        }
    }

    SkillMetadata {
        name,
        description,
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn discovers_skill_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let skill_dir = temp.path().join("example");
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        std::fs::write(
            skill_dir.join(SKILL_FILE_NAME),
            "---\nname: example\ndescription: test skill\n---\n",
        )
        .expect("skill file");

        let skills = discover_skills(temp.path()).expect("skills");

        assert_eq!(skills[0].name, "example");
        assert_eq!(skills[0].description.as_deref(), Some("test skill"));
    }
}
