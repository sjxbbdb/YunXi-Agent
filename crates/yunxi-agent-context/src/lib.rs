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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
}

impl ConversationMessage {
    pub fn new(role: ConversationRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ConversationRole::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ConversationRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ConversationRole::Assistant, content)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextWindowBudget {
    pub context_window_tokens: Option<i64>,
    pub auto_compact_threshold_tokens: Option<i64>,
}

impl ContextWindowBudget {
    pub fn new(
        context_window_tokens: Option<i64>,
        auto_compact_threshold_tokens: Option<i64>,
    ) -> Self {
        Self {
            context_window_tokens,
            auto_compact_threshold_tokens,
        }
    }

    pub fn status(&self, messages: &[ConversationMessage]) -> ContextWindowStatus {
        let active_context_tokens = estimate_messages_tokens(messages);
        let auto_compact_scope_tokens = active_context_tokens;
        let auto_compact_scope_limit = self.auto_compact_threshold_tokens;
        let full_context_window_limit = self.context_window_tokens;
        let full_context_window_limit_reached =
            full_context_window_limit.is_some_and(|limit| active_context_tokens >= limit);
        let token_limit_reached = auto_compact_scope_limit
            .is_some_and(|limit| auto_compact_scope_tokens >= limit)
            || full_context_window_limit_reached;
        let scope_remaining = auto_compact_scope_limit
            .map(|limit| limit.saturating_sub(auto_compact_scope_tokens).max(0));
        let full_remaining = full_context_window_limit
            .map(|limit| limit.saturating_sub(active_context_tokens).max(0));
        let tokens_until_compaction = match (scope_remaining, full_remaining) {
            (Some(scope), Some(full)) => Some(scope.min(full)),
            (scope, full) => scope.or(full),
        };

        ContextWindowStatus {
            active_context_tokens,
            auto_compact_scope_tokens,
            auto_compact_scope_limit,
            full_context_window_limit,
            tokens_until_compaction,
            full_context_window_limit_reached,
            token_limit_reached,
        }
    }

    fn compact_target_tokens(&self) -> Option<i64> {
        let limit = match (
            self.auto_compact_threshold_tokens,
            self.context_window_tokens,
        ) {
            (Some(auto), Some(full)) => Some(auto.min(full)),
            (Some(auto), None) => Some(auto),
            (None, Some(full)) => Some(full),
            (None, None) => None,
        }?;
        Some(((limit as f64) * 0.8).floor().max(1.0) as i64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextWindowStatus {
    pub active_context_tokens: i64,
    pub auto_compact_scope_tokens: i64,
    pub auto_compact_scope_limit: Option<i64>,
    pub full_context_window_limit: Option<i64>,
    pub tokens_until_compaction: Option<i64>,
    pub full_context_window_limit_reached: bool,
    pub token_limit_reached: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RestoredHistory {
    pub messages: Vec<ConversationMessage>,
    pub status: ContextWindowStatus,
    pub compacted: bool,
    pub dropped_messages: usize,
}

impl RestoredHistory {
    pub fn empty(budget: ContextWindowBudget) -> Self {
        Self {
            messages: Vec::new(),
            status: budget.status(&[]),
            compacted: false,
            dropped_messages: 0,
        }
    }
}

pub fn restore_history_for_prompt(
    messages: Vec<ConversationMessage>,
    budget: ContextWindowBudget,
) -> RestoredHistory {
    let status = budget.status(&messages);
    if !status.token_limit_reached {
        return RestoredHistory {
            messages,
            status,
            compacted: false,
            dropped_messages: 0,
        };
    }

    let target_tokens = budget.compact_target_tokens().unwrap_or(1);
    let mut kept_reversed = Vec::new();
    let mut kept_tokens = 0i64;
    for message in messages.iter().rev() {
        let tokens = estimate_message_tokens(message);
        if !kept_reversed.is_empty() && kept_tokens.saturating_add(tokens) > target_tokens {
            break;
        }
        kept_tokens = kept_tokens.saturating_add(tokens);
        kept_reversed.push(message.clone());
    }
    kept_reversed.reverse();

    let dropped_messages = messages.len().saturating_sub(kept_reversed.len());
    let mut restored_messages = Vec::new();
    if dropped_messages > 0 {
        restored_messages.push(ConversationMessage::system(compaction_summary(
            &messages[..dropped_messages],
        )));
    }
    restored_messages.extend(kept_reversed);

    RestoredHistory {
        messages: restored_messages,
        status,
        compacted: dropped_messages > 0,
        dropped_messages,
    }
}

pub fn estimate_messages_tokens(messages: &[ConversationMessage]) -> i64 {
    messages
        .iter()
        .map(estimate_message_tokens)
        .fold(0, i64::saturating_add)
}

pub fn estimate_message_tokens(message: &ConversationMessage) -> i64 {
    approx_token_count(&message.content).saturating_add(4)
}

pub fn approx_token_count(text: &str) -> i64 {
    let bytes = text.as_bytes().len() as i64;
    if bytes == 0 {
        return 0;
    }
    bytes.saturating_add(3) / 4
}

fn compaction_summary(messages: &[ConversationMessage]) -> String {
    let mut lines = vec![format!(
        "Compacted {} earlier history message(s) to stay within the context window.",
        messages.len()
    )];
    for message in messages.iter().take(6) {
        lines.push(format!(
            "- {:?}: {}",
            message.role,
            preview_for_summary(&message.content)
        ));
    }
    if messages.len() > 6 {
        lines.push(format!(
            "- ... {} more message(s) omitted",
            messages.len() - 6
        ));
    }
    lines.join("\n")
}

fn preview_for_summary(content: &str) -> String {
    const MAX: usize = 96;
    let trimmed = content.trim().replace('\n', " ");
    if trimmed.chars().count() <= MAX {
        return trimmed;
    }
    let mut preview = trimmed
        .chars()
        .take(MAX.saturating_sub(3))
        .collect::<String>();
    preview.push_str("...");
    preview
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

    #[test]
    fn context_window_status_reports_compaction_pressure() {
        let budget = ContextWindowBudget::new(Some(30), Some(20));
        let messages = vec![
            ConversationMessage::user("short prompt"),
            ConversationMessage::assistant("short answer"),
        ];

        let status = budget.status(&messages);

        assert!(!status.token_limit_reached);
        assert_eq!(
            status.tokens_until_compaction,
            Some(20 - estimate_messages_tokens(&messages))
        );
    }

    #[test]
    fn restore_history_compacts_oldest_messages_when_budget_is_exceeded() {
        let budget = ContextWindowBudget::new(Some(24), Some(18));
        let messages = vec![
            ConversationMessage::user("first user message with a lot of older context"),
            ConversationMessage::assistant("first assistant answer with detail"),
            ConversationMessage::user("second user message with more detail"),
            ConversationMessage::assistant("newest assistant answer"),
        ];

        let restored = restore_history_for_prompt(messages, budget);

        assert!(restored.compacted);
        assert!(restored.dropped_messages > 0);
        assert_eq!(
            restored.messages.first().map(|message| message.role),
            Some(ConversationRole::System)
        );
        assert!(restored.messages[0].content.contains("Compacted"));
        assert_eq!(
            restored
                .messages
                .last()
                .map(|message| message.content.as_str()),
            Some("newest assistant answer")
        );
    }
}
