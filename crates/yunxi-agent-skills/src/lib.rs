use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

pub const SKILL_FILE_NAME: &str = "SKILL.md";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: Option<String>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillPolicy {
    pub allow_implicit_invocation: Option<bool>,
    pub products: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<PathBuf>,
    pub icon_large: Option<PathBuf>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillDependencies {
    pub tools: Vec<SkillToolDependency>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillToolDependency {
    pub kind: String,
    pub value: String,
    pub description: Option<String>,
    pub transport: Option<String>,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillLoadError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillLoadOutcome {
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillLoadError>,
    pub disabled_paths: Vec<PathBuf>,
}

impl SkillLoadOutcome {
    pub fn enabled_skills(&self) -> Vec<&SkillMetadata> {
        self.skills
            .iter()
            .filter(|skill| !self.disabled_paths.contains(&skill.path))
            .collect()
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillInvocationResult {
    pub name: String,
    pub accepted: bool,
    pub output: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillCatalog {
    pub skills: Vec<SkillMetadata>,
}

impl SkillCatalog {
    pub fn from_root(root: impl AsRef<Path>) -> AgentResult<Self> {
        Ok(Self {
            skills: discover_skills(root)?,
        })
    }

    pub fn find(&self, name: &str) -> Option<&SkillMetadata> {
        self.skills.iter().find(|skill| skill.name == name)
    }

    pub fn render_instructions(&self) -> String {
        render_skill_instructions(&self.skills)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub skills: Vec<PathBuf>,
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: Vec<PathBuf>,
    pub interface: Option<PluginInterface>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub developer_name: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub website_url: Option<String>,
    pub brand_color: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: PluginManifest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicToolKind {
    ToolSearch,
    RequestUserInput,
    ViewImage,
    Skill,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DynamicToolMetadata {
    pub name: String,
    pub kind: DynamicToolKind,
    pub description: String,
    pub input_schema: Value,
    pub source: Option<String>,
}

pub fn default_dynamic_tools() -> Vec<DynamicToolMetadata> {
    vec![
        DynamicToolMetadata {
            name: "tool_search".to_string(),
            kind: DynamicToolKind::ToolSearch,
            description: "Search over deferred YunXi tool metadata.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
            source: Some("yunxi-agent-tools".to_string()),
        },
        DynamicToolMetadata {
            name: "request_user_input".to_string(),
            kind: DynamicToolKind::RequestUserInput,
            description: "Request concise input from an interactive host.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"prompt": {"type": "string"}},
                "required": ["prompt"]
            }),
            source: Some("yunxi-agent-tools".to_string()),
        },
        DynamicToolMetadata {
            name: "view_image".to_string(),
            kind: DynamicToolKind::ViewImage,
            description: "Inspect a local image file by path.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            source: Some("yunxi-agent-tools".to_string()),
        },
    ]
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

pub fn load_skill_injection(skill: &SkillMetadata) -> AgentResult<SkillInjection> {
    let content = std::fs::read_to_string(&skill.path).map_err(|error| AgentError::Execution {
        message: format!("failed to read skill {}: {error}", skill.path.display()),
    })?;
    Ok(SkillInjection {
        name: skill.name.clone(),
        content,
    })
}

pub fn render_skill_instructions(skills: &[SkillMetadata]) -> String {
    let mut lines = Vec::new();
    for skill in skills {
        let description = skill.description.as_deref().unwrap_or("No description");
        lines.push(format!("- {}: {description}", skill.name));
    }
    lines.join("\n")
}

pub fn load_plugin_manifest(plugin_root: impl AsRef<Path>) -> AgentResult<Option<PluginMetadata>> {
    let plugin_root = plugin_root.as_ref();
    let manifest_path = plugin_root.join(".codex-plugin").join("plugin.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&manifest_path).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to read plugin manifest {}: {error}",
                manifest_path.display()
            ),
        })?;
    let mut manifest = serde_json::from_str::<PluginManifest>(&content).map_err(|error| {
        AgentError::Execution {
            message: format!(
                "failed to parse plugin manifest {}: {error}",
                manifest_path.display()
            ),
        }
    })?;
    if manifest.name.trim().is_empty() {
        manifest.name = plugin_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("plugin")
            .to_string();
    }
    Ok(Some(PluginMetadata {
        root: plugin_root.to_path_buf(),
        manifest_path,
        manifest,
    }))
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

    #[test]
    fn skill_catalog_renders_instructions_and_loads_injection() {
        let temp = TempDir::new().expect("temp dir");
        let skill_dir = temp.path().join("writer");
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        std::fs::write(
            skill_dir.join(SKILL_FILE_NAME),
            "---\nname: writer\ndescription: writes reports\n---\n# Writer\n",
        )
        .expect("skill file");

        let catalog = SkillCatalog::from_root(temp.path()).expect("catalog");
        let skill = catalog.find("writer").expect("writer");
        let injection = load_skill_injection(skill).expect("injection");

        assert!(catalog.render_instructions().contains("writer"));
        assert!(injection.content.contains("# Writer"));
    }

    #[test]
    fn plugin_manifest_loads_from_codex_plugin_directory() {
        let temp = TempDir::new().expect("temp dir");
        let manifest_dir = temp.path().join(".codex-plugin");
        std::fs::create_dir_all(&manifest_dir).expect("manifest dir");
        std::fs::write(
            manifest_dir.join("plugin.json"),
            r#"{
                "name": "yunxi-plugin",
                "version": "0.1.0",
                "description": "fixture",
                "keywords": ["agent"],
                "skills": ["./skills"],
                "mcpServers": ["./mcp.json"],
                "interface": {"displayName": "YunXi Plugin"}
            }"#,
        )
        .expect("manifest");

        let plugin = load_plugin_manifest(temp.path())
            .expect("load")
            .expect("plugin");

        assert_eq!(plugin.manifest.name, "yunxi-plugin");
        assert_eq!(plugin.manifest.skills, vec![PathBuf::from("./skills")]);
    }

    #[test]
    fn dynamic_tools_include_codex_core_host_tools() {
        let names = default_dynamic_tools()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();

        assert!(names.contains(&"tool_search".to_string()));
        assert!(names.contains(&"request_user_input".to_string()));
        assert!(names.contains(&"view_image".to_string()));
    }
}
