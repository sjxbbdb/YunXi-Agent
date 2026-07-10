use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

pub const DEFAULT_AGENTS_MD_FILENAME: &str = "AGENTS.md";
pub const COMPACT_PROMPT_TEMPLATE: &str = include_str!("../assets/prompts/compact_prompt.md");
pub const COMPACT_SUMMARY_PREFIX_TEMPLATE: &str =
    include_str!("../assets/prompts/compact_summary_prefix.md");
pub const APPLY_PATCH_TOOL_INSTRUCTIONS: &str =
    include_str!("../assets/prompts/apply_patch_tool_instructions.md");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptAsset {
    pub name: String,
    pub content: String,
}

impl PromptAsset {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }
}

pub fn built_in_prompt_assets() -> Vec<PromptAsset> {
    vec![
        PromptAsset::new("compact.prompt", COMPACT_PROMPT_TEMPLATE),
        PromptAsset::new("compact.summary_prefix", COMPACT_SUMMARY_PREFIX_TEMPLATE),
        PromptAsset::new(
            "tools.apply_patch.instructions",
            APPLY_PATCH_TOOL_INSTRUCTIONS,
        ),
    ]
}

pub fn built_in_prompt_asset(name: &str) -> Option<PromptAsset> {
    built_in_prompt_assets()
        .into_iter()
        .find(|asset| asset.name == name)
}

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarkedContextFragment {
    pub name: String,
    pub role: ConversationRole,
    pub start_marker: String,
    pub end_marker: String,
    pub body: String,
    pub priority: i32,
}

impl MarkedContextFragment {
    pub fn new(
        name: impl Into<String>,
        role: ConversationRole,
        start_marker: impl Into<String>,
        end_marker: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            role,
            start_marker: start_marker.into(),
            end_marker: end_marker.into(),
            body: body.into(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn render(&self) -> String {
        if self.start_marker.is_empty() || self.end_marker.is_empty() {
            return self.body.clone();
        }
        format!("{}{}{}", self.start_marker, self.body, self.end_marker)
    }

    pub fn matches_text(&self, text: &str) -> bool {
        matches_marked_text(&self.start_marker, &self.end_marker, text)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptAssembly {
    pub assets: Vec<PromptAsset>,
    pub fragments: Vec<MarkedContextFragment>,
    pub messages: Vec<ConversationMessage>,
}

impl PromptAssembly {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_asset(mut self, asset: PromptAsset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn with_fragment(mut self, fragment: MarkedContextFragment) -> Self {
        self.fragments.push(fragment);
        self.fragments
            .sort_by(|left, right| right.priority.cmp(&left.priority));
        self
    }

    pub fn with_message(mut self, message: ConversationMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn from_context_bundle(bundle: &ContextBundle, user_prompt: impl Into<String>) -> Self {
        let mut assembly = Self::new();
        let prefix = bundle.prompt_prefix();
        if !prefix.is_empty() {
            assembly = assembly.with_message(ConversationMessage::system(prefix));
        }
        assembly.with_message(ConversationMessage::user(user_prompt))
    }

    pub fn into_messages(mut self) -> Vec<ConversationMessage> {
        let mut messages = Vec::new();
        for asset in self.assets {
            if !asset.content.trim().is_empty() {
                messages.push(ConversationMessage::system(asset.content));
            }
        }
        for fragment in self.fragments.drain(..) {
            messages.push(ConversationMessage::new(fragment.role, fragment.render()));
        }
        messages.extend(self.messages);
        messages
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextWindowPhase {
    Normal,
    Pressure,
    NeedsCompaction,
    Compacted,
}

impl ContextWindowStatus {
    pub fn phase(&self, compacted: bool) -> ContextWindowPhase {
        if compacted {
            ContextWindowPhase::Compacted
        } else if self.token_limit_reached {
            ContextWindowPhase::NeedsCompaction
        } else if self
            .tokens_until_compaction
            .is_some_and(|remaining| remaining <= self.active_context_tokens.max(1) / 5)
        {
            ContextWindowPhase::Pressure
        } else {
            ContextWindowPhase::Normal
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptDebugSnapshot {
    pub assets: Vec<String>,
    pub fragments: Vec<String>,
    pub message_count: usize,
    pub estimated_tokens: i64,
    pub phase: ContextWindowPhase,
}

impl PromptDebugSnapshot {
    pub fn from_assembly(assembly: &PromptAssembly, budget: ContextWindowBudget) -> Self {
        let messages = assembly.clone().into_messages();
        let status = budget.status(&messages);
        Self {
            assets: assembly
                .assets
                .iter()
                .map(|asset| asset.name.clone())
                .collect(),
            fragments: assembly
                .fragments
                .iter()
                .map(|fragment| fragment.name.clone())
                .collect(),
            message_count: messages.len(),
            estimated_tokens: status.active_context_tokens,
            phase: status.phase(false),
        }
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactTrigger {
    Manual,
    TokenBudget,
    Resume,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompactRequest {
    pub messages: Vec<ConversationMessage>,
    pub budget: ContextWindowBudget,
    pub trigger: CompactTrigger,
    pub prompt_template: String,
}

impl CompactRequest {
    pub fn new(
        messages: Vec<ConversationMessage>,
        budget: ContextWindowBudget,
        trigger: CompactTrigger,
    ) -> Self {
        Self {
            messages,
            budget,
            trigger,
            prompt_template: COMPACT_PROMPT_TEMPLATE.to_string(),
        }
    }

    pub fn prompt(&self) -> String {
        let mut prompt = self.prompt_template.trim().to_string();
        prompt.push_str("\n\nConversation to compact:\n");
        for message in &self.messages {
            prompt.push_str(&format!(
                "\n{:?}: {}",
                message.role,
                preview_for_summary(&message.content)
            ));
        }
        prompt
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompactSummary {
    pub trigger: CompactTrigger,
    pub summary: String,
    pub retained_messages: Vec<ConversationMessage>,
    pub dropped_messages: usize,
    pub status: ContextWindowStatus,
}

pub fn compact_messages(request: CompactRequest) -> CompactSummary {
    let restored = restore_history_for_prompt(request.messages.clone(), request.budget);
    let summary = if restored.compacted {
        restored
            .messages
            .first()
            .map(|message| message.content.clone())
            .unwrap_or_else(|| compaction_summary(&request.messages))
    } else {
        compaction_summary(&request.messages)
    };
    CompactSummary {
        trigger: request.trigger,
        summary,
        retained_messages: restored.messages,
        dropped_messages: restored.dropped_messages,
        status: restored.status,
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

pub fn matches_marked_text(start_marker: &str, end_marker: &str, text: &str) -> bool {
    if start_marker.is_empty() || end_marker.is_empty() {
        return false;
    }
    let trimmed_start = text.trim_start();
    let starts = trimmed_start
        .get(..start_marker.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(start_marker));
    let trimmed_end = text.trim_end();
    let ends = trimmed_end
        .get(trimmed_end.len().saturating_sub(end_marker.len())..)
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(end_marker));
    starts && ends
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileMention {
    pub raw: String,
    pub path: PathBuf,
}

pub fn extract_file_mentions(text: &str) -> Vec<FileMention> {
    let mut mentions = Vec::new();
    let mut seen = BTreeSet::new();
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | ';' | ':' | ')' | '(' | '[' | ']'
            )
        });
        let candidate = trimmed.strip_prefix('@').unwrap_or(trimmed);
        if !looks_like_path(candidate) {
            continue;
        }
        let path = PathBuf::from(candidate);
        if seen.insert(path.clone()) {
            mentions.push(FileMention {
                raw: trimmed.to_string(),
                path,
            });
        }
    }
    mentions
}

pub fn search_workspace_files(
    root: impl AsRef<Path>,
    query: &str,
    limit: usize,
) -> AgentResult<Vec<PathBuf>> {
    let root = root.as_ref();
    let mut matches = Vec::new();
    if !root.is_dir() || query.trim().is_empty() || limit == 0 {
        return Ok(matches);
    }
    search_workspace_files_inner(root, root, query, limit, &mut matches)?;
    Ok(matches)
}

fn search_workspace_files_inner(
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
        message: format!("failed to read search directory {}: {error}", dir.display()),
    })? {
        if matches.len() >= limit {
            break;
        }
        let entry = entry.map_err(|error| AgentError::Execution {
            message: format!("failed to read search directory entry: {error}"),
        })?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_search_entry(&name) {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| AgentError::Execution {
            message: format!("failed to read metadata for {}: {error}", path.display()),
        })?;
        if metadata.is_dir() {
            search_workspace_files_inner(root, &path, query, limit, matches)?;
        } else if metadata.is_file() && name.contains(query) {
            matches.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.contains('.')
}

fn should_skip_search_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".yunxi" | ".codegraph" | "target" | "vendor" | "extracted"
    )
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

    #[test]
    fn prompt_assets_include_compact_and_patch_templates() {
        let assets = built_in_prompt_assets();

        assert!(assets.iter().any(|asset| asset.name == "compact.prompt"));
        assert!(
            built_in_prompt_asset("tools.apply_patch.instructions")
                .expect("patch asset")
                .content
                .contains("*** Begin Patch")
        );
    }

    #[test]
    fn marked_fragments_render_and_match_case_insensitive_markers() {
        let fragment = MarkedContextFragment::new(
            "environment",
            ConversationRole::User,
            "<environment_context>",
            "</environment_context>",
            "\nworkspace ready\n",
        );

        let rendered = fragment.render();

        assert!(fragment.matches_text(&rendered));
        assert!(matches_marked_text(
            "<ENVIRONMENT_CONTEXT>",
            "</ENVIRONMENT_CONTEXT>",
            &rendered
        ));
    }

    #[test]
    fn compact_request_builds_handoff_prompt_and_summary() {
        let request = CompactRequest::new(
            vec![
                ConversationMessage::user("please continue the extraction"),
                ConversationMessage::assistant("patch layer completed"),
            ],
            ContextWindowBudget::new(Some(20), Some(12)),
            CompactTrigger::TokenBudget,
        );

        let prompt = request.prompt();
        let summary = compact_messages(request);

        assert!(prompt.contains("Conversation to compact"));
        assert_eq!(summary.trigger, CompactTrigger::TokenBudget);
        assert!(!summary.retained_messages.is_empty());
    }

    #[test]
    fn file_mentions_are_extracted_and_deduplicated() {
        let mentions =
            extract_file_mentions("edit @src/lib.rs and `docs/report.md`, then src/lib.rs");

        assert_eq!(
            mentions
                .iter()
                .map(|mention| mention.path.clone())
                .collect::<Vec<_>>(),
            vec![PathBuf::from("src/lib.rs"), PathBuf::from("docs/report.md")]
        );
    }

    #[test]
    fn workspace_file_search_skips_heavy_directories() {
        let temp = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(temp.path().join("src")).expect("src dir");
        std::fs::create_dir_all(temp.path().join("target")).expect("target dir");
        std::fs::write(temp.path().join("src/lib.rs"), "").expect("src file");
        std::fs::write(temp.path().join("target/lib.rs"), "").expect("target file");

        let matches = search_workspace_files(temp.path(), "lib", 10).expect("search");

        assert_eq!(matches, vec![PathBuf::from("src/lib.rs")]);
    }
}
