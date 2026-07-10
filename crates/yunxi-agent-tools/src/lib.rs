use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, ApprovalMode, SandboxMode};
use yunxi_agent_exec::{ExecCommand, ExecLifecycleEvent, ExecManager};
use yunxi_agent_mcp::{
    InMemoryMcpRuntime, McpRuntime, McpSessionManager, McpToolInvocation,
    load_in_memory_runtime_seed, load_workspace_mcp_configs,
};
use yunxi_agent_multi_agent::{AgentId, InMemoryAgentRegistry, MultiAgentCommand};
use yunxi_agent_patch::{PatchFileChangeKind, apply_patch_detailed};
use yunxi_agent_sandbox::{
    ApprovalRequirement, ExecutionPolicy, NetworkPolicy, PolicyDecision, PolicyEvaluation,
    SandboxRequirement,
};
use yunxi_agent_skills::{
    DynamicToolKind, DynamicToolMetadata, SkillCatalog, SkillInvocation, SkillInvocationResult,
    load_skill_injection, workspace_dynamic_tools,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: Option<String>,
    pub cwd: PathBuf,
    pub kind: ToolRequestKind,
    pub policy: ToolPolicy,
}

impl ToolRequest {
    pub fn shell(cwd: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            id: None,
            cwd: cwd.into(),
            kind: ToolRequestKind::Shell {
                command: command.into(),
            },
            policy: ToolPolicy::trusted(),
        }
    }

    pub fn patch(cwd: impl Into<PathBuf>, patch: impl Into<String>) -> Self {
        Self {
            id: None,
            cwd: cwd.into(),
            kind: ToolRequestKind::Patch {
                patch: patch.into(),
            },
            policy: ToolPolicy::trusted(),
        }
    }

    pub fn with_policy(mut self, policy: ToolPolicy) -> Self {
        self.policy = policy;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolRequestKind {
    Shell {
        command: String,
    },
    Patch {
        patch: String,
    },
    Mcp {
        server: String,
        tool: String,
        arguments_json: Option<String>,
    },
    Skill {
        name: String,
        arguments_json: Option<String>,
    },
    MultiAgent {
        action: String,
        arguments_json: Option<String>,
    },
    ToolSearch {
        query: String,
    },
    RequestUserInput {
        prompt: String,
    },
    ViewImage {
        path: String,
    },
}

impl ToolRequestKind {
    pub fn tool_name(&self) -> ToolName {
        match self {
            Self::Shell { .. } => ToolName::Shell,
            Self::Patch { .. } => ToolName::Patch,
            Self::Mcp { .. } => ToolName::Mcp,
            Self::Skill { .. } => ToolName::Skill,
            Self::MultiAgent { .. } => ToolName::MultiAgent,
            Self::ToolSearch { .. } => ToolName::ToolSearch,
            Self::RequestUserInput { .. } => ToolName::RequestUserInput,
            Self::ViewImage { .. } => ToolName::ViewImage,
        }
    }

    fn policy_command(&self) -> Option<String> {
        match self {
            Self::Shell { command } => Some(command.clone()),
            Self::Patch { .. } => Some("apply_patch > workspace".to_string()),
            Self::Mcp { server, tool, .. } => Some(format!("mcp {server} {tool}")),
            Self::Skill { name, .. } => Some(format!("skill {name}")),
            Self::MultiAgent { action, .. } => Some(format!("multi_agent {action}")),
            Self::ToolSearch { query } => Some(format!("tool_search {query}")),
            Self::RequestUserInput { prompt } => Some(format!("request_user_input {prompt}")),
            Self::ViewImage { path } => Some(format!("view_image {path}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    Shell,
    Patch,
    Mcp,
    Skill,
    MultiAgent,
    ToolSearch,
    RequestUserInput,
    ViewImage,
}

impl ToolName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Patch => "patch",
            Self::Mcp => "mcp",
            Self::Skill => "skill",
            Self::MultiAgent => "multi_agent",
            Self::ToolSearch => "tool_search",
            Self::RequestUserInput => "request_user_input",
            Self::ViewImage => "view_image",
        }
    }
}

impl Default for ToolName {
    fn default() -> Self {
        Self::Shell
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: ToolName,
    pub description: String,
    pub parameters: Value,
    pub model_visible: bool,
}

impl ToolSpec {
    pub fn new(
        name: ToolName,
        description: impl Into<String>,
        parameters: Value,
        model_visible: bool,
    ) -> Self {
        Self {
            name,
            description: description.into(),
            parameters,
            model_visible,
        }
    }

    pub fn openai_tool_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name.as_str(),
                "description": self.description.clone(),
                "parameters": self.parameters.clone(),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DynamicToolSpec {
    pub name: String,
    pub kind: DynamicToolKind,
    pub description: String,
    pub parameters: Value,
    pub source: Option<String>,
}

impl DynamicToolSpec {
    pub fn from_metadata(metadata: DynamicToolMetadata) -> Self {
        Self {
            name: metadata.name,
            kind: metadata.kind,
            description: metadata.description,
            parameters: metadata.input_schema,
            source: metadata.source,
        }
    }

    pub fn openai_tool_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name.clone(),
                "description": self.description.clone(),
                "parameters": self.parameters.clone(),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolRegistry {
    specs: BTreeMap<ToolName, ToolSpec>,
    dynamic_specs: BTreeMap<String, DynamicToolSpec>,
}

impl ToolRegistry {
    pub fn new(specs: impl IntoIterator<Item = ToolSpec>) -> Self {
        Self {
            specs: specs
                .into_iter()
                .map(|spec| (spec.name, spec))
                .collect::<BTreeMap<_, _>>(),
            dynamic_specs: BTreeMap::new(),
        }
    }

    pub fn spec(&self, name: ToolName) -> Option<&ToolSpec> {
        self.specs.get(&name)
    }

    pub fn specs(&self) -> impl Iterator<Item = &ToolSpec> {
        self.specs.values()
    }

    pub fn dynamic_specs(&self) -> impl Iterator<Item = &DynamicToolSpec> {
        self.dynamic_specs.values()
    }

    pub fn with_dynamic_tools(
        mut self,
        tools: impl IntoIterator<Item = DynamicToolMetadata>,
    ) -> Self {
        for tool in tools {
            let spec = DynamicToolSpec::from_metadata(tool);
            if self
                .specs
                .values()
                .any(|fixed| fixed.name.as_str() == spec.name.as_str())
            {
                continue;
            }
            self.dynamic_specs.insert(spec.name.clone(), spec);
        }
        self
    }

    pub fn model_visible_specs(&self) -> Vec<&ToolSpec> {
        self.specs
            .values()
            .filter(|spec| spec.model_visible)
            .collect()
    }

    pub fn openai_tools_json(&self) -> Vec<Value> {
        let mut tools = self
            .model_visible_specs()
            .into_iter()
            .map(ToolSpec::openai_tool_json)
            .collect::<Vec<_>>();
        tools.extend(
            self.dynamic_specs
                .values()
                .map(DynamicToolSpec::openai_tool_json),
        );
        tools
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        default_tool_registry()
    }
}

pub fn default_tool_registry() -> ToolRegistry {
    ToolRegistry::new([
        shell_tool_spec(),
        patch_tool_spec(),
        mcp_tool_spec(),
        skill_tool_spec(),
        multi_agent_tool_spec(),
        tool_search_tool_spec(),
        request_user_input_tool_spec(),
        view_image_tool_spec(),
    ])
}

pub fn workspace_tool_registry(cwd: impl AsRef<Path>) -> AgentResult<ToolRegistry> {
    let cwd = cwd.as_ref();
    let mut dynamic_tools = workspace_dynamic_tools(cwd)?;
    dynamic_tools.extend(workspace_mcp_dynamic_tools(cwd)?);
    Ok(default_tool_registry().with_dynamic_tools(dynamic_tools))
}

fn workspace_mcp_dynamic_tools(cwd: &Path) -> AgentResult<Vec<DynamicToolMetadata>> {
    load_workspace_mcp_configs(cwd).map(|servers| {
        servers
            .into_iter()
            .filter(|server| server.enabled)
            .map(|server| DynamicToolMetadata {
                name: format!("mcp__{}", sanitize_dynamic_tool_name(&server.name)),
                kind: DynamicToolKind::Mcp,
                description: format!("Call or discover YunXi MCP server {}.", server.name),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "server": {"type": "string"},
                        "tool": {"type": "string"},
                        "arguments_json": {"type": "string"}
                    },
                    "required": ["tool"],
                    "additionalProperties": false
                }),
                source: Some(format!("workspace-mcp:{}", server.name)),
            })
            .collect()
    })
}

fn sanitize_dynamic_tool_name(name: &str) -> String {
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
    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "server".to_string()
    } else {
        sanitized
    }
}

fn shell_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Shell,
        "Run a shell command inside the configured YunXi workspace.",
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute with the platform shell."
                }
            },
            "required": ["command"],
            "additionalProperties": false
        }),
        true,
    )
}

fn patch_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Patch,
        "Apply a constrained YunXi patch operation inside the configured workspace.",
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["write", "delete", "move"],
                    "description": "Patch operation to apply."
                },
                "path": {
                    "type": "string",
                    "description": "Workspace-relative file path."
                },
                "from": {
                    "type": "string",
                    "description": "Workspace-relative source path for move operations."
                },
                "content": {
                    "type": "string",
                    "description": "File content for write operations."
                }
            },
            "required": ["op", "path"],
            "additionalProperties": false
        }),
        true,
    )
}

fn mcp_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Mcp,
        "Call a registered YunXi MCP server tool.",
        json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server name."
                },
                "tool": {
                    "type": "string",
                    "description": "Tool name on the MCP server."
                },
                "arguments_json": {
                    "type": "string",
                    "description": "Optional serialized JSON object for tool arguments."
                }
            },
            "required": ["server", "tool"],
            "additionalProperties": false
        }),
        true,
    )
}

fn skill_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Skill,
        "Invoke a registered YunXi skill by name.",
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name."
                },
                "arguments_json": {
                    "type": "string",
                    "description": "Optional serialized JSON object for skill arguments."
                }
            },
            "required": ["name"],
            "additionalProperties": false
        }),
        true,
    )
}

fn multi_agent_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::MultiAgent,
        "Coordinate YunXi sub-agent lifecycle actions.",
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "wait", "send_message", "follow_up", "interrupt", "list"]
                },
                "arguments_json": {
                    "type": "string",
                    "description": "Optional serialized JSON object for action arguments."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        }),
        true,
    )
}

fn tool_search_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::ToolSearch,
        "Search available YunXi tools and workspace file metadata.",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Tool or file metadata query."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
        true,
    )
}

fn request_user_input_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::RequestUserInput,
        "Ask the interactive host for concise user input.",
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Question to present to the user."
                }
            },
            "required": ["prompt"],
            "additionalProperties": false
        }),
        true,
    )
}

fn view_image_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::ViewImage,
        "Inspect a local image file from the workspace.",
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Local image path."
                }
            },
            "required": ["path"],
            "additionalProperties": false
        }),
        true,
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRoute {
    pub name: ToolName,
    pub model_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDispatch {
    pub request: ToolRequest,
    pub route: ToolRoute,
    pub trace: ToolDispatchTrace,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDispatchTrace {
    pub request_id: Option<String>,
    pub tool_name: ToolName,
    pub route_status: ToolRouteStatus,
    pub policy_decision: ToolPolicyDecision,
    pub policy_evaluation: PolicyEvaluation,
}

impl ToolDispatchTrace {
    pub fn summary(&self) -> String {
        let id = self.request_id.as_deref().unwrap_or("none");
        let policy = match &self.policy_decision {
            ToolPolicyDecision::Approved => "approved".to_string(),
            ToolPolicyDecision::Declined { reason } => format!("declined:{reason}"),
        };
        let sandbox = &self.policy_evaluation.sandbox_backend.backend;
        let network = &self.policy_evaluation.network_decision;
        let escalation = self
            .policy_evaluation
            .escalation_request
            .as_ref()
            .map(|request| {
                format!(
                    ", escalation={:?}/{:?}",
                    request.required_sandbox, request.required_network
                )
            })
            .unwrap_or_default();
        format!(
            "Tool dispatch routed {} (id={id}, route={:?}, policy={policy}, sandbox={sandbox:?}, network={network:?}{escalation})",
            self.tool_name, self.route_status
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRouteStatus {
    Routed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicyDecision {
    Approved,
    Declined { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolRouter {
    registry: ToolRegistry,
}

impl ToolRouter {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn route(&self, request: ToolRequest) -> AgentResult<ToolDispatch> {
        let tool_name = request.kind.tool_name();
        let spec = self
            .registry
            .spec(tool_name)
            .ok_or_else(|| AgentError::Execution {
                message: format!("tool is not registered in YunXi router: {tool_name}"),
            })?;
        let route = ToolRoute {
            name: tool_name,
            model_visible: spec.model_visible,
        };
        let policy_evaluation = request.policy.evaluation_for(&request);
        let policy_decision = ToolPolicy::decision_from_evaluation(&policy_evaluation);
        let trace = ToolDispatchTrace {
            request_id: request.id.clone(),
            tool_name,
            route_status: ToolRouteStatus::Routed,
            policy_decision,
            policy_evaluation,
        };
        Ok(ToolDispatch {
            request,
            route,
            trace,
        })
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new(default_tool_registry())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolResponse {
    pub id: Option<String>,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub changed_files: Vec<ToolFileChange>,
    pub lifecycle_events: Vec<ExecLifecycleEvent>,
}

impl ToolResponse {
    pub fn completed(
        id: Option<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
        changed_files: Vec<ToolFileChange>,
    ) -> Self {
        Self {
            id,
            status: ToolStatus::Completed,
            output: Some(output.into()),
            error: None,
            exit_code,
            changed_files,
            lifecycle_events: Vec::new(),
        }
    }

    pub fn failed(
        id: Option<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
        changed_files: Vec<ToolFileChange>,
    ) -> Self {
        Self {
            id,
            status: ToolStatus::Failed,
            output: Some(output.into()),
            error: None,
            exit_code,
            changed_files,
            lifecycle_events: Vec::new(),
        }
    }

    pub fn declined(id: Option<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            status: ToolStatus::Declined,
            output: None,
            error: Some(message.into()),
            exit_code: None,
            changed_files: Vec::new(),
            lifecycle_events: Vec::new(),
        }
    }

    pub fn with_lifecycle_events(mut self, events: Vec<ExecLifecycleEvent>) -> Self {
        self.lifecycle_events = events;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolFileChange {
    pub path: PathBuf,
    pub kind: ToolFileChangeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFileChangeKind {
    Added,
    Deleted,
    Updated,
    Moved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub approval: ApprovalDecision,
    pub sandbox: SandboxPolicy,
    pub workspace_root: Option<PathBuf>,
    pub network: NetworkPolicy,
    pub execution_policy: ExecutionPolicy,
}

impl ToolPolicy {
    pub fn trusted() -> Self {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            approval: ApprovalDecision::Approved,
            sandbox: SandboxPolicy::DangerFullAccess,
            workspace_root: Some(workspace_root.clone()),
            network: NetworkPolicy::Inherit,
            execution_policy: ExecutionPolicy {
                approval: ApprovalRequirement::PreApproved,
                sandbox: SandboxRequirement::DangerFullAccess,
                network: NetworkPolicy::Inherit,
                workspace_root,
            },
        }
    }

    pub fn from_config(config: &AgentConfig) -> Self {
        let execution_policy = ExecutionPolicy::from_config(config);
        Self {
            approval: match config.approval_mode {
                ApprovalMode::Never => ApprovalDecision::Approved,
                ApprovalMode::OnFailure => ApprovalDecision::OnFailure,
                ApprovalMode::OnRequest | ApprovalMode::Untrusted => ApprovalDecision::Required,
            },
            sandbox: match config.sandbox_mode {
                SandboxMode::ReadOnly => SandboxPolicy::ReadOnly,
                SandboxMode::WorkspaceWrite => SandboxPolicy::WorkspaceWrite,
                SandboxMode::DangerFullAccess => SandboxPolicy::DangerFullAccess,
            },
            workspace_root: Some(config.cwd.clone()),
            network: execution_policy.network,
            execution_policy,
        }
    }

    pub fn decision_for(&self, request: &ToolRequest) -> ToolPolicyDecision {
        Self::decision_from_evaluation(&self.evaluation_for(request))
    }

    pub fn evaluation_for(&self, request: &ToolRequest) -> PolicyEvaluation {
        let mut policy = self.execution_policy.clone();
        if let Some(workspace_root) = self.workspace_root.clone() {
            policy.workspace_root = workspace_root;
        }
        policy.evaluate(&request.cwd, request.kind.policy_command().as_deref())
    }

    fn decision_from_evaluation(evaluation: &PolicyEvaluation) -> ToolPolicyDecision {
        match &evaluation.decision {
            PolicyDecision::Allowed => ToolPolicyDecision::Approved,
            PolicyDecision::Blocked { reason } => ToolPolicyDecision::Declined {
                reason: reason.clone(),
            },
        }
    }

    fn denial_for(&self, request: &ToolRequest) -> Option<String> {
        match self.evaluation_for(request).decision {
            PolicyDecision::Allowed => None,
            PolicyDecision::Blocked { reason } => Some(reason),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    OnFailure,
    Required,
    Declined { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxPolicy {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[async_trait]
pub trait ToolRuntime: Send + Sync {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NoopToolRuntime;

#[async_trait]
impl ToolRuntime for NoopToolRuntime {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse> {
        Ok(ToolResponse::declined(
            request.id,
            "YunXi tool execution is not wired in this runtime slice",
        ))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShellToolRuntime;

#[async_trait]
impl ToolRuntime for ShellToolRuntime {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse> {
        if let Some(reason) = request.policy.denial_for(&request) {
            return Ok(ToolResponse::declined(request.id, reason));
        }

        match request.kind {
            ToolRequestKind::Shell { command } => run_shell(request.id, request.cwd, command).await,
            ToolRequestKind::Patch { patch } => run_patch(request.id, request.cwd, patch),
            ToolRequestKind::ToolSearch { query } => {
                run_tool_search(request.id, request.cwd, query)
            }
            ToolRequestKind::ViewImage { path } => run_view_image(request.id, request.cwd, path),
            ToolRequestKind::RequestUserInput { prompt } => Ok(ToolResponse::declined(
                request.id,
                format!("request_user_input requires an interactive host: {prompt}"),
            )),
            ToolRequestKind::Mcp { .. }
            | ToolRequestKind::Skill { .. }
            | ToolRequestKind::MultiAgent { .. } => Ok(ToolResponse::declined(
                request.id,
                "YunXi has registered this tool but the specialized runtime is not attached",
            )),
        }
    }
}

#[derive(Clone)]
pub struct CompositeToolRuntime {
    shell: ShellToolRuntime,
    mcp: Arc<dyn McpRuntime>,
    agents: InMemoryAgentRegistry,
    skill_roots: Vec<PathBuf>,
}

impl std::fmt::Debug for CompositeToolRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompositeToolRuntime")
            .field("shell", &self.shell)
            .field("agents", &self.agents)
            .field("skill_roots", &self.skill_roots)
            .finish_non_exhaustive()
    }
}

impl Default for CompositeToolRuntime {
    fn default() -> Self {
        Self {
            shell: ShellToolRuntime,
            mcp: Arc::new(InMemoryMcpRuntime::default()),
            agents: InMemoryAgentRegistry::default(),
            skill_roots: default_skill_roots(),
        }
    }
}

impl CompositeToolRuntime {
    pub fn with_mcp_runtime<R>(mut self, runtime: R) -> Self
    where
        R: McpRuntime + 'static,
    {
        self.mcp = Arc::new(runtime);
        self
    }

    pub fn with_agent_registry(mut self, registry: InMemoryAgentRegistry) -> Self {
        self.agents = registry;
        self
    }

    pub fn with_skill_roots(mut self, roots: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.skill_roots = roots.into_iter().map(Into::into).collect();
        self
    }

    pub fn agent_registry(&self) -> &InMemoryAgentRegistry {
        &self.agents
    }
}

#[async_trait]
impl ToolRuntime for CompositeToolRuntime {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse> {
        if let Some(reason) = request.policy.denial_for(&request) {
            return Ok(ToolResponse::declined(request.id, reason));
        }

        match request.kind.clone() {
            ToolRequestKind::Mcp {
                server,
                tool,
                arguments_json,
            } => {
                run_mcp_tool(
                    Arc::clone(&self.mcp),
                    request.id,
                    request.cwd,
                    server,
                    tool,
                    arguments_json,
                )
                .await
            }
            ToolRequestKind::Skill {
                name,
                arguments_json,
            } => run_skill(
                request.id,
                request.cwd,
                self.skill_roots.clone(),
                name,
                arguments_json,
            ),
            ToolRequestKind::MultiAgent {
                action,
                arguments_json,
            } => run_multi_agent(request.id, &self.agents, action, arguments_json),
            _ => self.shell.execute(request).await,
        }
    }
}

async fn run_shell(id: Option<String>, cwd: PathBuf, command: String) -> AgentResult<ToolResponse> {
    let before = WorkspaceSnapshot::capture(&cwd)?;
    let exec_command = ExecCommand::observed_shell(cwd.clone(), command).with_id(id.clone());
    let exec_trace = ExecManager::default().run(exec_command).await?;

    let changed_files = before.diff(&WorkspaceSnapshot::capture(&cwd)?);
    let combined = exec_trace.summary.aggregated_output.clone();
    let exit_code = exec_trace.summary.exit_code;
    if !exec_trace.summary.timed_out && exit_code == Some(0) {
        Ok(
            ToolResponse::completed(id, combined, exit_code, changed_files)
                .with_lifecycle_events(exec_trace.events),
        )
    } else {
        Ok(ToolResponse::failed(id, combined, exit_code, changed_files)
            .with_lifecycle_events(exec_trace.events))
    }
}

fn run_patch(id: Option<String>, cwd: PathBuf, patch: String) -> AgentResult<ToolResponse> {
    let report = match apply_patch_detailed(&cwd, &patch) {
        Ok(report) => report,
        Err(error) => {
            let output = serde_json::to_string(&error).map_err(|error| AgentError::Execution {
                message: format!("failed to serialize patch diagnostics: {error}"),
            })?;
            return Ok(ToolResponse::failed(id, output, None, Vec::new()));
        }
    };
    let changed_files = report
        .changed_files
        .into_iter()
        .map(|change| ToolFileChange {
            path: change.path,
            kind: match change.kind {
                PatchFileChangeKind::Added => ToolFileChangeKind::Added,
                PatchFileChangeKind::Updated => ToolFileChangeKind::Updated,
                PatchFileChangeKind::Deleted => ToolFileChangeKind::Deleted,
                PatchFileChangeKind::Moved => ToolFileChangeKind::Moved,
            },
        })
        .collect();

    let output = serde_json::to_string(&serde_json::json!({
        "status": "applied",
        "diagnostics": report.diagnostics
    }))
    .map_err(|error| AgentError::Execution {
        message: format!("failed to serialize patch report: {error}"),
    })?;
    Ok(ToolResponse::completed(id, output, Some(0), changed_files))
}

fn run_tool_search(id: Option<String>, cwd: PathBuf, query: String) -> AgentResult<ToolResponse> {
    let query = query.trim().to_string();
    let mut matches = Vec::new();
    if !query.is_empty() && cwd.is_dir() {
        collect_file_matches(&cwd, &cwd, &query, 50, &mut matches)?;
    }
    let query_lower = query.to_ascii_lowercase();
    let registry = workspace_tool_registry(&cwd)?;
    let tool_matches = registry
        .specs()
        .filter(|spec| {
            spec.name.as_str().contains(&query_lower)
                || spec.description.to_ascii_lowercase().contains(&query_lower)
        })
        .map(|spec| {
            json!({
                "name": spec.name.as_str(),
                "description": spec.description.clone(),
                "kind": "builtin",
                "source": "yunxi-agent-tools"
            })
        })
        .chain(
            registry
                .dynamic_specs()
                .filter(|spec| {
                    spec.name.contains(&query_lower)
                        || spec.description.to_ascii_lowercase().contains(&query_lower)
                })
                .map(|spec| {
                    json!({
                        "name": spec.name.clone(),
                        "description": spec.description.clone(),
                        "kind": spec.kind,
                        "source": spec.source.clone()
                    })
                }),
        )
        .collect::<Vec<_>>();
    let output = serde_json::to_string(&json!({
        "query": query,
        "matches": matches
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "tools": tool_matches
    }))
    .map_err(|error| AgentError::Execution {
        message: format!("failed to serialize tool_search output: {error}"),
    })?;
    Ok(ToolResponse::completed(id, output, Some(0), Vec::new()))
}

fn run_view_image(id: Option<String>, cwd: PathBuf, path: String) -> AgentResult<ToolResponse> {
    let requested = PathBuf::from(path);
    let full_path = if requested.is_absolute() {
        requested
    } else {
        cwd.join(requested)
    };
    if !full_path.is_file() {
        return Ok(ToolResponse::declined(
            id,
            format!("image file does not exist: {}", full_path.display()),
        ));
    }
    Ok(ToolResponse::completed(
        id,
        format!("image file available: {}", full_path.display()),
        Some(0),
        Vec::new(),
    ))
}

async fn run_mcp_tool(
    runtime: Arc<dyn McpRuntime>,
    id: Option<String>,
    cwd: PathBuf,
    server: String,
    tool: String,
    arguments_json: Option<String>,
) -> AgentResult<ToolResponse> {
    let invocation = McpToolInvocation {
        server,
        tool,
        arguments_json,
    };
    let workspace_runtime = load_workspace_mcp_runtime(&cwd)?;
    let workspace_session = McpSessionManager::from_workspace(&cwd)?;
    let result = match workspace_runtime {
        Some(workspace_runtime) => workspace_runtime.call_tool(invocation).await,
        None => match workspace_session {
            Some(workspace_session) => {
                let _ = workspace_session.initialize(&invocation.server).await;
                workspace_session.call_tool(invocation).await
            }
            None => runtime.call_tool(invocation).await,
        },
    };
    match result {
        Ok(result) => Ok(ToolResponse::completed(
            id,
            result.content,
            Some(0),
            Vec::new(),
        )),
        Err(error) => Ok(ToolResponse::failed(
            id,
            error.to_string(),
            None,
            Vec::new(),
        )),
    }
}

fn load_workspace_mcp_runtime(cwd: &Path) -> AgentResult<Option<InMemoryMcpRuntime>> {
    let seed_path = cwd.join(".yunxi").join("mcp-runtime.json");
    if !seed_path.is_file() {
        return Ok(None);
    }
    Ok(Some(load_in_memory_runtime_seed(seed_path)?))
}

fn run_skill(
    id: Option<String>,
    cwd: PathBuf,
    skill_roots: Vec<PathBuf>,
    name: String,
    arguments_json: Option<String>,
) -> AgentResult<ToolResponse> {
    let invocation = SkillInvocation {
        name: name.clone(),
        arguments_json,
    };
    for root in resolve_skill_roots(&cwd, skill_roots) {
        let catalog = SkillCatalog::from_root(&root)?;
        if let Some(skill) = catalog.find(&name) {
            let injection = load_skill_injection(skill)?;
            let result = SkillInvocationResult {
                name,
                accepted: true,
                output: injection.content,
            };
            let output =
                serde_json::to_string(&json!({"invocation": invocation, "result": result}))
                    .map_err(|error| AgentError::Execution {
                        message: format!("failed to serialize skill invocation result: {error}"),
                    })?;
            return Ok(ToolResponse::completed(id, output, Some(0), Vec::new()));
        }
    }

    Ok(ToolResponse::failed(
        id,
        format!("skill is not registered in YunXi runtime: {name}"),
        None,
        Vec::new(),
    ))
}

fn run_multi_agent(
    id: Option<String>,
    registry: &InMemoryAgentRegistry,
    action: String,
    arguments_json: Option<String>,
) -> AgentResult<ToolResponse> {
    let command = parse_multi_agent_command(&action, arguments_json.as_deref())?;
    match registry.execute(command) {
        Ok(result) => {
            let output = serde_json::to_string(&result).map_err(|error| AgentError::Execution {
                message: format!("failed to serialize multi-agent result: {error}"),
            })?;
            Ok(ToolResponse::completed(id, output, Some(0), Vec::new()))
        }
        Err(error) => Ok(ToolResponse::failed(
            id,
            error.to_string(),
            None,
            Vec::new(),
        )),
    }
}

fn parse_multi_agent_command(
    action: &str,
    arguments_json: Option<&str>,
) -> AgentResult<MultiAgentCommand> {
    let arguments = parse_arguments_object(arguments_json)?;
    match action {
        "spawn" => Ok(MultiAgentCommand::Spawn {
            task: required_string(&arguments, "task")?,
            parent_id: optional_string(&arguments, "parent_id").map(AgentId),
        }),
        "wait" => Ok(MultiAgentCommand::Wait {
            id: AgentId(required_string(&arguments, "id")?),
        }),
        "send_message" | "sendMessage" | "message" => Ok(MultiAgentCommand::SendMessage {
            id: AgentId(required_string(&arguments, "id")?),
            message: required_string(&arguments, "message")?,
        }),
        "follow_up" | "followUp" => Ok(MultiAgentCommand::FollowUp {
            id: AgentId(required_string(&arguments, "id")?),
            task: required_string(&arguments, "task")?,
        }),
        "interrupt" => Ok(MultiAgentCommand::Interrupt {
            id: AgentId(required_string(&arguments, "id")?),
        }),
        "list" => Ok(MultiAgentCommand::List),
        _ => Err(AgentError::Execution {
            message: format!("unsupported multi-agent action: {action}"),
        }),
    }
}

fn parse_arguments_object(arguments_json: Option<&str>) -> AgentResult<Value> {
    let Some(arguments_json) = arguments_json
        .map(str::trim)
        .filter(|json| !json.is_empty())
    else {
        return Ok(json!({}));
    };
    let value =
        serde_json::from_str::<Value>(arguments_json).map_err(|error| AgentError::Execution {
            message: format!("failed to parse tool arguments JSON: {error}"),
        })?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(AgentError::Execution {
            message: "tool arguments JSON must be an object".to_string(),
        })
    }
}

fn required_string(arguments: &Value, name: &str) -> AgentResult<String> {
    optional_string(arguments, name).ok_or_else(|| AgentError::Execution {
        message: format!("missing required multi-agent argument: {name}"),
    })
}

fn optional_string(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn default_skill_roots() -> Vec<PathBuf> {
    [".codex/skills", ".yunxi/skills", "skills"]
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

fn resolve_skill_roots(cwd: &Path, roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots
        .into_iter()
        .map(|root| {
            if root.is_absolute() {
                root
            } else {
                cwd.join(root)
            }
        })
        .collect()
}

fn collect_file_matches(
    root: &Path,
    dir: &Path,
    query: &str,
    limit: usize,
    matches: &mut Vec<PathBuf>,
) -> AgentResult<()> {
    if matches.len() >= limit {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|error| AgentError::Execution {
        message: format!(
            "failed to read tool_search directory {}: {error}",
            dir.display()
        ),
    })? {
        if matches.len() >= limit {
            break;
        }
        let entry = entry.map_err(|error| AgentError::Execution {
            message: format!("failed to read tool_search entry: {error}"),
        })?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_entry(&name) {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| AgentError::Execution {
            message: format!("failed to read metadata for {}: {error}", path.display()),
        })?;
        if metadata.is_dir() {
            collect_file_matches(root, &path, query, limit, matches)?;
        } else if metadata.is_file() && name.contains(query) {
            matches.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn platform_shell(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.args(["-c", command]);
    process
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct WorkspaceSnapshot {
    files: BTreeMap<PathBuf, FileState>,
}

impl WorkspaceSnapshot {
    fn capture(root: &Path) -> AgentResult<Self> {
        let mut snapshot = Self::default();
        if !root.is_dir() {
            return Ok(snapshot);
        }
        snapshot.capture_dir(root, root)?;
        Ok(snapshot)
    }

    fn capture_dir(&mut self, root: &Path, dir: &Path) -> AgentResult<()> {
        for entry in std::fs::read_dir(dir).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to read workspace directory {}: {error}",
                dir.display()
            ),
        })? {
            let entry = entry.map_err(|error| AgentError::Execution {
                message: format!("failed to read workspace entry: {error}"),
            })?;
            let path = entry.path();
            let name = entry.file_name();
            if should_skip_entry(&name.to_string_lossy()) {
                continue;
            }
            let metadata = entry.metadata().map_err(|error| AgentError::Execution {
                message: format!("failed to read metadata for {}: {error}", path.display()),
            })?;
            if metadata.is_dir() {
                self.capture_dir(root, &path)?;
            } else if metadata.is_file() {
                let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                self.files
                    .insert(relative, FileState::from_metadata(metadata));
            }
        }
        Ok(())
    }

    fn diff(&self, after: &Self) -> Vec<ToolFileChange> {
        let paths = self
            .files
            .keys()
            .chain(after.files.keys())
            .cloned()
            .collect::<BTreeSet<_>>();

        paths
            .into_iter()
            .filter_map(
                |path| match (self.files.get(&path), after.files.get(&path)) {
                    (None, Some(_)) => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Added,
                    }),
                    (Some(_), None) => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Deleted,
                    }),
                    (Some(before), Some(after)) if before != after => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Updated,
                    }),
                    _ => None,
                },
            )
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileState {
    len: u64,
    modified_millis: u128,
}

impl FileState {
    fn from_metadata(metadata: std::fs::Metadata) -> Self {
        let modified_millis = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        Self {
            len: metadata.len(),
            modified_millis,
        }
    }
}

fn should_skip_entry(name: &str) -> bool {
    matches!(name, ".git" | ".yunxi" | "target" | "vendor")
}
