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
    Mcp,
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillRuntimeCatalog {
    pub core_assets: Vec<SkillMetadata>,
    pub workspace_skills: Vec<SkillMetadata>,
    pub plugin_skills: Vec<SkillMetadata>,
    pub plugin_manifests: Vec<PluginManifest>,
    pub dynamic_tools: Vec<DynamicToolMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionToolExecution {
    pub tool_name: String,
    pub source: Option<String>,
    pub model_visible_schema: Value,
    pub status: ExtensionToolStatus,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionToolStatus {
    Discovered,
    Dispatched,
    Completed,
    Failed,
    Missing,
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDiscoveryOutcome {
    pub plugins: Vec<PluginMetadata>,
    pub errors: Vec<SkillLoadError>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDynamicToolSeed {
    pub plugin: PluginMetadata,
    pub skill_roots: Vec<PathBuf>,
    pub dynamic_tools: Vec<DynamicToolMetadata>,
}

impl PluginMetadata {
    pub fn resolved_skill_roots(&self) -> Vec<PathBuf> {
        self.manifest
            .skills
            .iter()
            .map(|path| resolve_plugin_path(&self.root, path))
            .collect()
    }

    pub fn resolved_mcp_server_paths(&self) -> Vec<PathBuf> {
        self.manifest
            .mcp_servers
            .iter()
            .map(|path| resolve_plugin_path(&self.root, path))
            .collect()
    }

    pub fn dynamic_tool_seed(&self) -> AgentResult<PluginDynamicToolSeed> {
        let mut dynamic_tools = dynamic_tools_for_plugin(self)?;
        for skill_root in self.resolved_skill_roots() {
            let skills = discover_skills(&skill_root)?;
            dynamic_tools.extend(dynamic_tools_for_skills_with_source(
                &skills,
                Some(format!("plugin:{}", self.manifest.name)),
            ));
        }
        Ok(PluginDynamicToolSeed {
            plugin: self.clone(),
            skill_roots: self.resolved_skill_roots(),
            dynamic_tools,
        })
    }
}

pub fn discover_plugins(root: impl AsRef<Path>) -> AgentResult<PluginDiscoveryOutcome> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Ok(PluginDiscoveryOutcome::default());
    }

    let mut outcome = PluginDiscoveryOutcome::default();
    if let Some(plugin) = load_plugin_manifest(root)? {
        outcome.plugins.push(plugin);
    }

    for entry in std::fs::read_dir(root).map_err(|error| AgentError::Execution {
        message: format!(
            "failed to read plugin directory {}: {error}",
            root.display()
        ),
    })? {
        let entry = entry.map_err(|error| AgentError::Execution {
            message: format!("failed to read plugin directory entry: {error}"),
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match load_plugin_manifest(&path) {
            Ok(Some(plugin)) => outcome.plugins.push(plugin),
            Ok(None) => {}
            Err(error) => outcome.errors.push(SkillLoadError {
                path,
                message: error.to_string(),
            }),
        }
    }

    outcome
        .plugins
        .sort_by(|left, right| left.manifest.name.cmp(&right.manifest.name));
    Ok(outcome)
}

pub fn dynamic_tools_for_skills(skills: &[SkillMetadata]) -> Vec<DynamicToolMetadata> {
    dynamic_tools_for_skills_with_source(skills, None)
}

pub fn dynamic_tools_for_plugins(
    plugins: &[PluginMetadata],
) -> AgentResult<Vec<DynamicToolMetadata>> {
    let mut tools = Vec::new();
    for plugin in plugins {
        tools.extend(plugin.dynamic_tool_seed()?.dynamic_tools);
    }
    dedupe_dynamic_tools(tools)
}

pub fn workspace_dynamic_tools(root: impl AsRef<Path>) -> AgentResult<Vec<DynamicToolMetadata>> {
    let root = root.as_ref();
    let mut tools = Vec::new();
    for skill_root in default_workspace_skill_roots(root) {
        tools.extend(dynamic_tools_for_skills(&discover_skills(skill_root)?));
    }
    for plugin_root in default_workspace_plugin_roots(root) {
        let outcome = discover_plugins(plugin_root)?;
        tools.extend(dynamic_tools_for_plugins(&outcome.plugins)?);
    }
    dedupe_dynamic_tools(tools)
}

fn dynamic_tools_for_plugin(plugin: &PluginMetadata) -> AgentResult<Vec<DynamicToolMetadata>> {
    let mut tools = vec![DynamicToolMetadata {
        name: format!("plugin__{}", sanitize_tool_name(&plugin.manifest.name)),
        kind: DynamicToolKind::Plugin,
        description: plugin
            .manifest
            .description
            .clone()
            .or_else(|| {
                plugin
                    .manifest
                    .interface
                    .as_ref()
                    .and_then(|interface| interface.short_description.clone())
            })
            .unwrap_or_else(|| format!("YunXi plugin {}", plugin.manifest.name)),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Optional plugin-local discovery query."
                }
            },
            "additionalProperties": false
        }),
        source: Some(format!("plugin:{}", plugin.root.display())),
    }];

    for path in plugin.resolved_mcp_server_paths() {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("server");
        tools.push(DynamicToolMetadata {
            name: format!(
                "mcp__{}__{}",
                sanitize_tool_name(&plugin.manifest.name),
                sanitize_tool_name(stem)
            ),
            kind: DynamicToolKind::Mcp,
            description: format!(
                "Discover or call MCP server metadata from plugin {}.",
                plugin.manifest.name
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tool": {"type": "string"},
                    "arguments_json": {"type": "string"}
                },
                "additionalProperties": false
            }),
            source: Some(path.display().to_string()),
        });
    }

    dedupe_dynamic_tools(tools)
}

fn dynamic_tools_for_skills_with_source(
    skills: &[SkillMetadata],
    source_prefix: Option<String>,
) -> Vec<DynamicToolMetadata> {
    skills
        .iter()
        .map(|skill| DynamicToolMetadata {
            name: format!("skill__{}", sanitize_tool_name(&skill.name)),
            kind: DynamicToolKind::Skill,
            description: skill
                .description
                .clone()
                .unwrap_or_else(|| format!("Invoke YunXi skill {}", skill.name)),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Skill name override. Defaults to this dynamic tool's skill."
                    },
                    "arguments_json": {
                        "type": "string",
                        "description": "Optional serialized JSON object for skill arguments."
                    }
                },
                "additionalProperties": false
            }),
            source: Some(match &source_prefix {
                Some(prefix) => format!("{prefix}:skill:{}", skill.name),
                None => format!("skill:{}", skill.path.display()),
            }),
        })
        .collect()
}

fn dedupe_dynamic_tools(tools: Vec<DynamicToolMetadata>) -> AgentResult<Vec<DynamicToolMetadata>> {
    let mut by_name = std::collections::BTreeMap::new();
    for tool in tools {
        by_name.entry(tool.name.clone()).or_insert(tool);
    }
    Ok(by_name.into_values().collect())
}

fn default_workspace_skill_roots(root: &Path) -> Vec<PathBuf> {
    [".codex/skills", ".yunxi/skills", "skills"]
        .into_iter()
        .map(|path| root.join(path))
        .collect()
}

fn default_workspace_plugin_roots(root: &Path) -> Vec<PathBuf> {
    [".codex/plugins", ".yunxi/plugins", "plugins"]
        .into_iter()
        .map(|path| root.join(path))
        .collect()
}

fn resolve_plugin_path(plugin_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        plugin_root.join(path)
    }
}

fn sanitize_tool_name(name: &str) -> String {
    let mut sanitized = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "tool".to_string()
    } else {
        sanitized.chars().take(48).collect()
    }
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

    #[test]
    fn workspace_dynamic_tools_include_skills_and_plugin_mcp() {
        let temp = TempDir::new().expect("temp dir");
        let workspace_skill = temp.path().join(".yunxi/skills/writer");
        std::fs::create_dir_all(&workspace_skill).expect("workspace skill dir");
        std::fs::write(
            workspace_skill.join(SKILL_FILE_NAME),
            "---\nname: writer\ndescription: writes reports\n---\n# Writer\n",
        )
        .expect("workspace skill");

        let plugin_root = temp.path().join(".yunxi/plugins/research");
        let plugin_skill = plugin_root.join("skills/searcher");
        let plugin_manifest = plugin_root.join(".codex-plugin");
        std::fs::create_dir_all(&plugin_skill).expect("plugin skill dir");
        std::fs::create_dir_all(&plugin_manifest).expect("plugin manifest dir");
        std::fs::write(
            plugin_manifest.join("plugin.json"),
            r#"{
                "name": "research-pack",
                "description": "research plugin",
                "skills": ["skills"],
                "mcpServers": ["mcp/research.json"]
            }"#,
        )
        .expect("plugin manifest");
        std::fs::write(
            plugin_skill.join(SKILL_FILE_NAME),
            "---\nname: searcher\ndescription: searches notes\n---\n# Searcher\n",
        )
        .expect("plugin skill");

        let tools = workspace_dynamic_tools(temp.path()).expect("dynamic tools");
        let names = tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>();

        assert!(names.contains(&"skill__writer".to_string()));
        assert!(names.contains(&"skill__searcher".to_string()));
        assert!(names.contains(&"plugin__research-pack".to_string()));
        assert!(names.contains(&"mcp__research-pack__research".to_string()));
    }
}
