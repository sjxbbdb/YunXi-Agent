use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

pub const DEFAULT_AGENTS_MD_FILENAME: &str = "AGENTS.md";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentsMdDocument {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoadedAgentsMd {
    pub documents: Vec<AgentsMdDocument>,
    pub user_instructions: Option<String>,
}

impl LoadedAgentsMd {
    pub fn combined_instructions(&self) -> String {
        let mut sections = Vec::new();
        if let Some(user_instructions) = &self.user_instructions {
            if !user_instructions.trim().is_empty() {
                sections.push(user_instructions.trim().to_string());
            }
        }
        for document in &self.documents {
            if !document.content.trim().is_empty() {
                sections.push(document.content.trim().to_string());
            }
        }
        sections.join("\n\n")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextFragment {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextBundle {
    pub agents_md: LoadedAgentsMd,
    pub fragments: Vec<ContextFragment>,
}

impl ContextBundle {
    pub fn prompt_prefix(&self) -> String {
        let mut parts = Vec::new();
        let agents = self.agents_md.combined_instructions();
        if !agents.is_empty() {
            parts.push(agents);
        }
        for fragment in &self.fragments {
            if !fragment.content.trim().is_empty() {
                parts.push(format!("{}:\n{}", fragment.name, fragment.content.trim()));
            }
        }
        parts.join("\n\n")
    }
}

pub fn load_agents_md_hierarchy(cwd: impl AsRef<Path>) -> AgentResult<LoadedAgentsMd> {
    load_agents_md_hierarchy_with_user(cwd, None)
}

pub fn load_agents_md_hierarchy_with_user(
    cwd: impl AsRef<Path>,
    user_instructions: Option<String>,
) -> AgentResult<LoadedAgentsMd> {
    let cwd = cwd.as_ref();
    let mut loaded = LoadedAgentsMd {
        documents: Vec::new(),
        user_instructions,
    };
    for directory in ancestor_directories(cwd)? {
        let path = directory.join(DEFAULT_AGENTS_MD_FILENAME);
        if path.is_file() {
            let content =
                std::fs::read_to_string(&path).map_err(|error| AgentError::Execution {
                    message: format!("failed to read {}: {error}", path.display()),
                })?;
            loaded.documents.push(AgentsMdDocument { path, content });
        }
    }
    Ok(loaded)
}

fn ancestor_directories(cwd: &Path) -> AgentResult<Vec<PathBuf>> {
    let canonical = std::fs::canonicalize(cwd).map_err(|error| AgentError::Execution {
        message: format!("failed to canonicalize {}: {error}", cwd.display()),
    })?;
    let mut directories = Vec::new();
    for ancestor in canonical.ancestors() {
        directories.push(ancestor.to_path_buf());
    }
    directories.reverse();
    Ok(directories)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn agents_md_loads_from_root_to_cwd() {
        let temp = TempDir::new().expect("temp dir");
        let child = temp.path().join("child");
        std::fs::create_dir_all(&child).expect("child dir");
        std::fs::write(temp.path().join(DEFAULT_AGENTS_MD_FILENAME), "root").expect("root file");
        std::fs::write(child.join(DEFAULT_AGENTS_MD_FILENAME), "child").expect("child file");

        let loaded = load_agents_md_hierarchy(&child).expect("loaded agents");

        assert_eq!(
            loaded
                .documents
                .iter()
                .map(|document| document.content.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "child"]
        );
    }
}
