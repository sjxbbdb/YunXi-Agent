use std::path::PathBuf;

use serde_json::Value;
use yunxi_agent_core::AgentResult;
use yunxi_agent_sandbox::{
    ApprovalCacheEntry, ApprovalKey, ApprovalRequirement, CachedApprovalDecision,
    SessionApprovalCache,
};
use yunxi_agent_storage::SessionId;
use yunxi_agent_tools::{ToolRequest, ToolRequestKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApprovalCacheProbe {
    pub(crate) tool_name: String,
    pub(crate) key: String,
    pub(crate) decision: CachedApprovalDecision,
    pub(crate) reused: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeSessionDriver {
    session_id: SessionId,
    approval_cache: SessionApprovalCache,
}

impl RuntimeSessionDriver {
    pub(crate) fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            approval_cache: SessionApprovalCache::default(),
        }
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub(crate) fn apply_approval_cache(
        &mut self,
        request: &mut ToolRequest,
    ) -> AgentResult<ApprovalCacheProbe> {
        let key = approval_key_for_request(request);
        if let Some(entry) = self
            .approval_cache
            .entries
            .iter()
            .find(|entry| entry.key == key)
            .cloned()
        {
            if matches!(entry.decision, CachedApprovalDecision::Approved) {
                mark_preapproved(request);
            }
            return Ok(ApprovalCacheProbe {
                tool_name: key.tool_name.clone(),
                key: approval_key_summary(&key),
                decision: entry.decision,
                reused: true,
            });
        }

        let decision = if request
            .policy
            .execution_policy
            .approval
            .is_approved_without_prompt()
        {
            CachedApprovalDecision::Approved
        } else {
            CachedApprovalDecision::AskAgain
        };
        self.approval_cache.remember(ApprovalCacheEntry {
            key: key.clone(),
            decision,
            reason: Some("non-interactive runtime policy decision".to_string()),
        });
        Ok(ApprovalCacheProbe {
            tool_name: key.tool_name.clone(),
            key: approval_key_summary(&key),
            decision,
            reused: false,
        })
    }
}

fn mark_preapproved(request: &mut ToolRequest) {
    request.policy.approval = yunxi_agent_tools::ApprovalDecision::Approved;
    request.policy.execution_policy.approval = ApprovalRequirement::PreApproved;
}

fn approval_key_for_request(request: &ToolRequest) -> ApprovalKey {
    let (command, target_paths) = approval_command_and_paths(request);
    ApprovalKey {
        tool_name: request.kind.tool_name().to_string(),
        command,
        cwd: Some(request.cwd.clone()),
        target_paths,
        sandbox: Some(request.policy.execution_policy.sandbox),
        network: Some(request.policy.execution_policy.network),
    }
}

fn approval_command_and_paths(request: &ToolRequest) -> (Option<String>, Vec<PathBuf>) {
    match &request.kind {
        ToolRequestKind::Shell { command } => (Some(command.clone()), Vec::new()),
        ToolRequestKind::Patch { patch } => (
            Some("apply_patch".to_string()),
            serde_json::from_str::<Value>(patch)
                .ok()
                .and_then(|value| value.get("path").and_then(Value::as_str).map(PathBuf::from))
                .into_iter()
                .collect(),
        ),
        ToolRequestKind::Mcp { server, tool, .. } => {
            (Some(format!("mcp {server} {tool}")), Vec::new())
        }
        ToolRequestKind::Skill { name, .. } => (Some(format!("skill {name}")), Vec::new()),
        ToolRequestKind::MultiAgent { action, .. } => {
            (Some(format!("multi_agent {action}")), Vec::new())
        }
        ToolRequestKind::ToolSearch { query } => (Some(format!("tool_search {query}")), Vec::new()),
        ToolRequestKind::RequestUserInput { prompt } => {
            (Some(format!("request_user_input {prompt}")), Vec::new())
        }
        ToolRequestKind::ViewImage { path } => (Some(format!("view_image {path}")), Vec::new()),
    }
}

fn approval_key_summary(key: &ApprovalKey) -> String {
    let command = key.command.as_deref().unwrap_or("none");
    let cwd = key
        .cwd
        .as_ref()
        .map(|cwd| cwd.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    format!("{}:{command}:{cwd}", key.tool_name)
}
