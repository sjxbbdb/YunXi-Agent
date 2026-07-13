use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use yunxi_agent_core::{AgentError, AgentEvent, AgentResult, AgentRunStatus};
use yunxi_agent_multi_agent::{
    AgentGraphSessionMetadata, AgentId, AgentMetadata, AgentRole, AgentStatus,
};
use yunxi_agent_persona::{
    MemoryRecord, MemoryScope, MemoryStatus, PersonaSettings, yunxi_home_dir,
};
use yunxi_agent_protocol::{RuntimeEvent, from_jsonl_line, to_jsonl_line};

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn generate() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let counter = NEXT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!("yunxi-{millis}-{counter}"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub cwd: PathBuf,
    pub prompt: String,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
    pub status: AgentRunStatus,
    pub model: Option<String>,
    pub provider: Option<String>,
    #[serde(default)]
    pub parent_id: Option<SessionId>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub pinned: bool,
    pub created_at_millis: u128,
    #[serde(default = "now_millis")]
    pub updated_at_millis: u128,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    pub cwd: PathBuf,
    pub status: AgentRunStatus,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
    pub archived: bool,
    pub pinned: bool,
    pub parent_id: Option<SessionId>,
    pub title: Option<String>,
    pub prompt_preview: String,
    pub final_response_preview: Option<String>,
    pub event_count: usize,
    pub child_count: usize,
    pub model: Option<String>,
    pub provider: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThreadMetadata {
    pub id: SessionId,
    pub parent_id: Option<SessionId>,
    pub cwd: PathBuf,
    pub title: Option<String>,
    pub archived: bool,
    pub pinned: bool,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
}

/// Storage-owned, serializable session projection for a persisted agent graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionMetadata {
    pub id: SessionId,
    #[serde(default)]
    pub parent_id: Option<SessionId>,
    pub task: Option<String>,
    pub status: Option<AgentRunStatus>,
    pub title: Option<String>,
    pub archived: bool,
    pub pinned: bool,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
}

impl From<&SessionRecord> for AgentSessionMetadata {
    fn from(record: &SessionRecord) -> Self {
        Self {
            id: record.id.clone(),
            parent_id: record.parent_id.clone(),
            task: Some(record.prompt.clone()),
            status: Some(record.status),
            title: record.title.clone(),
            archived: record.archived,
            pinned: record.pinned,
            created_at_millis: record.created_at_millis,
            updated_at_millis: record.updated_at_millis,
        }
    }
}

impl From<&ThreadMetadata> for AgentSessionMetadata {
    fn from(thread: &ThreadMetadata) -> Self {
        Self {
            id: thread.id.clone(),
            parent_id: thread.parent_id.clone(),
            task: None,
            status: None,
            title: thread.title.clone(),
            archived: thread.archived,
            pinned: thread.pinned,
            created_at_millis: thread.created_at_millis,
            updated_at_millis: thread.updated_at_millis,
        }
    }
}

impl AgentSessionMetadata {
    pub fn to_agent_graph_session_metadata(&self) -> AgentGraphSessionMetadata {
        let agent_id = AgentId(self.id.0.clone());
        let parent_agent_id = self.parent_id.as_ref().map(|id| AgentId(id.0.clone()));
        AgentGraphSessionMetadata {
            session_id: self.id.0.clone(),
            parent_session_id: self.parent_id.as_ref().map(|id| id.0.clone()),
            agent: AgentMetadata {
                id: agent_id,
                parent_id: parent_agent_id,
                task: self
                    .task
                    .clone()
                    .or_else(|| self.title.clone())
                    .unwrap_or_else(|| self.id.0.clone()),
                status: match self.status {
                    Some(AgentRunStatus::Completed) => AgentStatus::Completed,
                    Some(AgentRunStatus::Failed) => AgentStatus::Failed,
                    Some(AgentRunStatus::Cancelled) => AgentStatus::Interrupted,
                    None => AgentStatus::Running,
                },
                role: Some(AgentRole::General),
                budget_tokens: None,
            },
            created_at_millis: self.created_at_millis,
            updated_at_millis: self.updated_at_millis,
        }
    }
}

/// A cycle-safe, storage-backed parent/child projection of session metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionGraphView {
    pub sessions: BTreeMap<SessionId, AgentSessionMetadata>,
    pub children: BTreeMap<SessionId, Vec<SessionId>>,
}

impl SessionGraphView {
    pub fn from_sessions(sessions: impl IntoIterator<Item = SessionRecord>) -> AgentResult<Self> {
        Self::from_metadata(sessions.into_iter().map(|record| (&record).into()))
    }

    pub fn from_threads(threads: impl IntoIterator<Item = ThreadMetadata>) -> AgentResult<Self> {
        Self::from_metadata(threads.into_iter().map(|thread| (&thread).into()))
    }

    pub fn from_metadata(
        sessions: impl IntoIterator<Item = AgentSessionMetadata>,
    ) -> AgentResult<Self> {
        let sessions = sessions
            .into_iter()
            .map(|session| (session.id.clone(), session))
            .collect();
        let mut view = Self {
            sessions,
            children: BTreeMap::new(),
        };
        view.validate()?;
        view.rebuild_children();
        Ok(view)
    }

    pub fn children_of(&self, id: &SessionId) -> Vec<AgentSessionMetadata> {
        self.children
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|child_id| self.sessions.get(child_id).cloned())
            .collect()
    }

    pub fn agent_graph_session_metadata(&self) -> Vec<AgentGraphSessionMetadata> {
        self.sessions
            .values()
            .map(AgentSessionMetadata::to_agent_graph_session_metadata)
            .collect()
    }

    pub fn validate(&self) -> AgentResult<()> {
        for id in self.sessions.keys() {
            let mut ancestors = BTreeSet::new();
            let mut current = Some(id);
            while let Some(current_id) = current {
                if !ancestors.insert(current_id.clone()) {
                    return Err(AgentError::Execution {
                        message: format!("cycle detected in session graph at {}", current_id.0),
                    });
                }
                current = self
                    .sessions
                    .get(current_id)
                    .and_then(|session| session.parent_id.as_ref());
            }
        }
        Ok(())
    }

    fn rebuild_children(&mut self) {
        for session in self.sessions.values() {
            if let Some(parent_id) = &session.parent_id {
                self.children
                    .entry(parent_id.clone())
                    .or_default()
                    .push(session.id.clone());
            }
        }
    }
}

impl ThreadMetadata {
    pub fn new(id: SessionId, cwd: impl Into<PathBuf>) -> Self {
        let now = now_millis();
        Self {
            id,
            parent_id: None,
            cwd: cwd.into(),
            title: None,
            archived: false,
            pinned: false,
            created_at_millis: now,
            updated_at_millis: now,
        }
    }

    pub fn with_parent(mut self, parent_id: SessionId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn archived(mut self, archived: bool) -> Self {
        self.archived = archived;
        self.updated_at_millis = now_millis();
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self.updated_at_millis = now_millis();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutItem {
    pub turn_id: String,
    pub event: AgentEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutRecord {
    pub thread: ThreadMetadata,
    pub prompt: String,
    pub items: Vec<RolloutItem>,
    pub final_response: Option<String>,
    pub status: AgentRunStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRolloutItem {
    pub turn_id: String,
    pub event: RuntimeEvent,
    pub estimated_bytes: usize,
    pub recorded_at_millis: u128,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRolloutRecord {
    pub thread: ThreadMetadata,
    pub items: Vec<RuntimeRolloutItem>,
    pub truncated: bool,
}

impl RuntimeRolloutRecord {
    pub fn replay_events(&self) -> Vec<RuntimeEvent> {
        self.items.iter().map(|item| item.event.clone()).collect()
    }

    pub fn to_jsonl(&self) -> AgentResult<String> {
        encode_runtime_rollout_jsonl(self.items.iter().map(|item| &item.event))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub session_id: SessionId,
    pub parent_session_id: Option<SessionId>,
    #[serde(default)]
    pub child_session_ids: Vec<SessionId>,
    #[serde(default)]
    pub child_session_count: usize,
    pub status: AgentRunStatus,
    pub rollout_items: usize,
    pub rollout_truncated: bool,
    pub archived: bool,
    pub pinned: bool,
    pub updated_at_millis: u128,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThreadStoreParitySnapshot {
    pub thread_id: String,
    pub session_id: Option<SessionId>,
    pub parent_session_id: Option<SessionId>,
    pub child_session_ids: Vec<SessionId>,
    pub forked_from_session_id: Option<SessionId>,
    pub rollout_items: usize,
    pub message_history_items: usize,
    pub runtime_events: usize,
    pub truncated: bool,
    pub compact_summary: Option<String>,
    pub lineage: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutParityRecord {
    pub input_items: usize,
    pub response_items: usize,
    pub tool_events: usize,
    pub runtime_events: usize,
    pub state_snapshot: RuntimeStateSnapshot,
}

impl RuntimeStateSnapshot {
    pub fn from_session_and_rollout(
        session: &SessionRecord,
        rollout: Option<&RuntimeRolloutRecord>,
    ) -> Self {
        let child_session_ids = child_session_ids_from_events(&session.events);
        Self {
            session_id: session.id.clone(),
            parent_session_id: session.parent_id.clone(),
            child_session_count: child_session_ids.len(),
            child_session_ids,
            status: session.status,
            rollout_items: rollout
                .map(|rollout| rollout.items.len())
                .unwrap_or_default(),
            rollout_truncated: rollout.map(|rollout| rollout.truncated).unwrap_or(false),
            archived: session.archived,
            pinned: session.pinned,
            updated_at_millis: session.updated_at_millis,
        }
    }
}

fn child_session_ids_from_events(events: &[AgentEvent]) -> Vec<SessionId> {
    let mut ids = BTreeSet::new();
    for event in events {
        if let AgentEvent::ChildAgentEvent {
            child_session_id, ..
        } = event
        {
            ids.insert(SessionId::new(child_session_id.clone()));
        }
    }
    ids.into_iter().collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RolloutBudget {
    pub max_items: usize,
    pub max_bytes: usize,
}

impl RolloutBudget {
    pub fn new(max_items: usize, max_bytes: usize) -> Self {
        Self {
            max_items: max_items.max(1),
            max_bytes: max_bytes.max(1),
        }
    }
}

impl Default for RolloutBudget {
    fn default() -> Self {
        Self {
            max_items: 1024,
            max_bytes: 4 * 1024 * 1024,
        }
    }
}

pub fn runtime_rollout_from_events(
    thread: ThreadMetadata,
    turn_id: impl Into<String>,
    events: Vec<RuntimeEvent>,
    budget: RolloutBudget,
) -> AgentResult<RuntimeRolloutRecord> {
    let turn_id = turn_id.into();
    let mut items = Vec::new();
    let mut total_bytes = 0usize;
    let mut truncated = false;
    for event in events {
        let line = to_jsonl_line(&event).map_err(|error| AgentError::Execution {
            message: format!("failed to encode runtime rollout event: {error}"),
        })?;
        let estimated_bytes = line.len();
        if items.len() >= budget.max_items
            || total_bytes.saturating_add(estimated_bytes) > budget.max_bytes
        {
            truncated = true;
            break;
        }
        total_bytes = total_bytes.saturating_add(estimated_bytes);
        items.push(RuntimeRolloutItem {
            turn_id: turn_id.clone(),
            event,
            estimated_bytes,
            recorded_at_millis: now_millis(),
        });
    }
    Ok(RuntimeRolloutRecord {
        thread,
        items,
        truncated,
    })
}

pub fn encode_runtime_rollout_jsonl<'a>(
    events: impl IntoIterator<Item = &'a RuntimeEvent>,
) -> AgentResult<String> {
    let mut lines = Vec::new();
    for event in events {
        lines.push(to_jsonl_line(event).map_err(|error| AgentError::Execution {
            message: format!("failed to encode runtime rollout JSONL: {error}"),
        })?);
    }
    Ok(lines.join("\n"))
}

pub fn decode_runtime_rollout_jsonl(jsonl: &str) -> AgentResult<Vec<RuntimeEvent>> {
    let mut events = Vec::new();
    for line in jsonl.lines().filter(|line| !line.trim().is_empty()) {
        events.push(
            from_jsonl_line(line).map_err(|error| AgentError::Execution {
                message: format!("failed to decode runtime rollout JSONL: {error}"),
            })?,
        );
    }
    Ok(events)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryItemKind {
    User,
    Assistant,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistoryItem {
    pub session_id: SessionId,
    pub kind: HistoryItemKind,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionHistory {
    pub sessions: Vec<SessionRecord>,
    pub items: Vec<HistoryItem>,
}

impl SessionHistory {
    pub fn from_sessions(sessions: Vec<SessionRecord>) -> Self {
        let mut items = Vec::new();
        for session in &sessions {
            items.push(HistoryItem {
                session_id: session.id.clone(),
                kind: HistoryItemKind::User,
                content: session.prompt.clone(),
            });
            if let Some(final_response) = &session.final_response {
                items.push(HistoryItem {
                    session_id: session.id.clone(),
                    kind: HistoryItemKind::Assistant,
                    content: final_response.clone(),
                });
            }
        }
        Self { sessions, items }
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryLoadOptions {
    pub max_sessions: usize,
}

impl HistoryLoadOptions {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            max_sessions: max_sessions.max(1),
        }
    }
}

impl Default for HistoryLoadOptions {
    fn default() -> Self {
        Self { max_sessions: 128 }
    }
}

impl From<SessionRecord> for RolloutRecord {
    fn from(record: SessionRecord) -> Self {
        let thread = record.thread_metadata();
        Self {
            thread,
            prompt: record.prompt,
            items: record
                .events
                .into_iter()
                .enumerate()
                .map(|(index, event)| RolloutItem {
                    turn_id: format!("turn-{index}"),
                    event,
                })
                .collect(),
            final_response: record.final_response,
            status: record.status,
        }
    }
}

impl SessionRecord {
    pub fn new(
        cwd: impl Into<PathBuf>,
        prompt: impl Into<String>,
        final_response: Option<String>,
        events: Vec<AgentEvent>,
    ) -> Self {
        let now = now_millis();
        Self {
            id: SessionId::generate(),
            cwd: cwd.into(),
            prompt: prompt.into(),
            final_response,
            events,
            status: AgentRunStatus::Completed,
            model: None,
            provider: None,
            parent_id: None,
            title: None,
            archived: false,
            pinned: false,
            created_at_millis: now,
            updated_at_millis: now,
        }
    }

    pub fn with_status(mut self, status: AgentRunStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    pub fn with_provider(mut self, provider: Option<String>) -> Self {
        self.provider = provider;
        self
    }

    pub fn with_parent_id(mut self, parent_id: SessionId) -> Self {
        self.parent_id = Some(parent_id);
        self.updated_at_millis = now_millis();
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self.updated_at_millis = now_millis();
        self
    }

    pub fn with_archived(mut self, archived: bool) -> Self {
        self.archived = archived;
        self.updated_at_millis = now_millis();
        self
    }

    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self.updated_at_millis = now_millis();
        self
    }

    pub fn touch(mut self) -> Self {
        self.updated_at_millis = now_millis();
        self
    }

    pub fn forked_from(mut self, parent_id: SessionId) -> Self {
        let now = now_millis();
        let title = self
            .title
            .clone()
            .unwrap_or_else(|| preview_title(&self.prompt));
        self.id = SessionId::generate();
        self.parent_id = Some(parent_id);
        self.title = Some(format!("Fork of {title}"));
        self.archived = false;
        self.pinned = false;
        self.created_at_millis = now;
        self.updated_at_millis = now;
        self
    }

    pub fn thread_metadata(&self) -> ThreadMetadata {
        ThreadMetadata {
            id: self.id.clone(),
            parent_id: self.parent_id.clone(),
            cwd: self.cwd.clone(),
            title: self.title.clone(),
            archived: self.archived,
            pinned: self.pinned,
            created_at_millis: self.created_at_millis,
            updated_at_millis: self.updated_at_millis,
        }
    }

    pub fn agent_session_metadata(&self) -> AgentSessionMetadata {
        self.into()
    }

    pub fn summary_with_child_count(&self, child_count: usize) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            cwd: self.cwd.clone(),
            status: self.status,
            created_at_millis: self.created_at_millis,
            updated_at_millis: self.updated_at_millis,
            archived: self.archived,
            pinned: self.pinned,
            parent_id: self.parent_id.clone(),
            title: self.title.clone(),
            prompt_preview: preview_for_summary(&self.prompt),
            final_response_preview: self.final_response.as_deref().map(preview_for_summary),
            event_count: self.events.len(),
            child_count,
            model: self.model.clone(),
            provider: self.provider.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilePersonaMemoryStore {
    workspace_root: PathBuf,
    global_root: PathBuf,
    workspace_fingerprint: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PersonaMemoryLoad {
    pub records: Vec<MemoryRecord>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersonaMemoryScope {
    All,
    Global,
    Workspace,
    Pending,
}

impl FilePersonaMemoryStore {
    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        let workspace_root = cwd.as_ref().join(".yunxi").join("memory");
        let global_root = yunxi_home_dir().join("memory");
        let workspace_fingerprint = workspace_fingerprint(cwd.as_ref());
        Self {
            workspace_root,
            global_root,
            workspace_fingerprint,
        }
    }

    pub fn workspace_fingerprint(&self) -> &str {
        &self.workspace_fingerprint
    }

    pub fn settings(&self) -> PersonaSettings {
        PersonaSettings::load()
    }

    pub fn append(&self, record: &MemoryRecord) -> AgentResult<()> {
        let path = self.path_for_record(record);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| AgentError::Execution {
                message: format!(
                    "failed to create memory directory {}: {error}",
                    parent.display()
                ),
            })?;
        }
        let line = serde_json::to_string(record).map_err(|error| AgentError::Execution {
            message: format!("failed to serialize memory {}: {error}", record.id),
        })?;
        let mut content = line;
        content.push('\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, content.as_bytes()))
            .map_err(|error| AgentError::Execution {
                message: format!("failed to append memory {}: {error}", path.display()),
            })
    }

    pub fn list(&self, scope: PersonaMemoryScope) -> PersonaMemoryLoad {
        let mut load = PersonaMemoryLoad::default();
        for path in self.paths_for_scope(scope) {
            read_memory_jsonl(&path, &mut load.records, &mut load.warnings);
        }
        load.records = latest_records(load.records);
        load.records.sort_by(|left, right| {
            right
                .updated_at_millis
                .cmp(&left.updated_at_millis)
                .then_with(|| left.id.cmp(&right.id))
        });
        load
    }

    pub fn search(&self, query: &str, scope: PersonaMemoryScope) -> PersonaMemoryLoad {
        let mut load = self.list(scope);
        let query = query.to_ascii_lowercase();
        load.records.retain(|record| {
            query.trim().is_empty()
                || record.content.to_ascii_lowercase().contains(&query)
                || record.id.to_ascii_lowercase().contains(&query)
        });
        load
    }

    pub fn show(&self, id: &str) -> PersonaMemoryLoad {
        let mut load = self.list(PersonaMemoryScope::All);
        load.records.retain(|record| record.id == id);
        load
    }

    pub fn update_status(
        &self,
        id: &str,
        status: MemoryStatus,
    ) -> AgentResult<Option<MemoryRecord>> {
        let load = self.list(PersonaMemoryScope::All);
        let Some(record) = load.records.into_iter().find(|record| record.id == id) else {
            return Ok(None);
        };
        let updated = record.with_status(status);
        self.append(&updated)?;
        Ok(Some(updated))
    }

    pub fn clear_workspace(&self) -> AgentResult<usize> {
        let records = self.list(PersonaMemoryScope::Workspace).records;
        let mut count = 0;
        for record in records
            .into_iter()
            .filter(|record| record.status == MemoryStatus::Active)
        {
            let archived = record.with_status(MemoryStatus::Archived);
            self.append(&archived)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn active_records(&self) -> PersonaMemoryLoad {
        let mut load = self.list(PersonaMemoryScope::All);
        load.records
            .retain(|record| record.status == MemoryStatus::Active);
        load
    }

    fn path_for_record(&self, record: &MemoryRecord) -> PathBuf {
        let pending = record.status == MemoryStatus::Pending;
        match &record.scope {
            MemoryScope::Workspace { .. } => {
                if pending {
                    self.workspace_root.join("pending.jsonl")
                } else {
                    self.workspace_root.join("workspace-memory.jsonl")
                }
            }
            _ => {
                if pending {
                    self.global_root.join("pending.jsonl")
                } else {
                    self.global_root.join("global-memory.jsonl")
                }
            }
        }
    }

    fn paths_for_scope(&self, scope: PersonaMemoryScope) -> Vec<PathBuf> {
        let global = [
            self.global_root.join("global-memory.jsonl"),
            self.global_root.join("pending.jsonl"),
        ];
        let workspace = [
            self.workspace_root.join("workspace-memory.jsonl"),
            self.workspace_root.join("pending.jsonl"),
        ];
        match scope {
            PersonaMemoryScope::All => global.into_iter().chain(workspace).collect(),
            PersonaMemoryScope::Global => global.into_iter().collect(),
            PersonaMemoryScope::Workspace => workspace.into_iter().collect(),
            PersonaMemoryScope::Pending => vec![
                self.global_root.join("pending.jsonl"),
                self.workspace_root.join("pending.jsonl"),
            ],
        }
    }
}

pub fn workspace_fingerprint(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical
        .to_string_lossy()
        .to_ascii_lowercase()
        .hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn read_memory_jsonl(path: &Path, records: &mut Vec<MemoryRecord>, warnings: &mut Vec<String>) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<MemoryRecord>(line) {
            Ok(record) => records.push(record),
            Err(error) => warnings.push(format!(
                "failed to parse memory line {} in {}: {error}",
                index + 1,
                path.display()
            )),
        }
    }
}

fn latest_records(records: Vec<MemoryRecord>) -> Vec<MemoryRecord> {
    let mut by_id = BTreeMap::new();
    for record in records {
        let replace = by_id.get(&record.id).is_none_or(|existing: &MemoryRecord| {
            existing.updated_at_millis <= record.updated_at_millis
        });
        if replace {
            by_id.insert(record.id.clone(), record);
        }
    }
    by_id.into_values().collect()
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save(&self, record: SessionRecord) -> AgentResult<SessionId>;
    async fn load(&self, id: &SessionId) -> AgentResult<Option<SessionRecord>>;
    async fn list(&self) -> AgentResult<Vec<SessionRecord>>;

    async fn history(
        &self,
        id: &SessionId,
        options: HistoryLoadOptions,
    ) -> AgentResult<Option<SessionHistory>> {
        let mut records = Vec::new();
        let mut seen = HashSet::new();
        let mut current_id = Some(id.clone());

        while let Some(id) = current_id {
            if records.len() >= options.max_sessions {
                break;
            }
            if !seen.insert(id.clone()) {
                return Err(AgentError::Execution {
                    message: format!("cycle detected while loading session history at {}", id.0),
                });
            }
            let Some(record) = self.load(&id).await? else {
                if records.is_empty() {
                    return Ok(None);
                }
                break;
            };
            current_id = record.parent_id.clone();
            records.push(record);
        }

        records.reverse();
        Ok(Some(SessionHistory::from_sessions(records)))
    }

    async fn update(&self, record: SessionRecord) -> AgentResult<()> {
        self.save(record).await.map(|_| ())
    }

    async fn archive(&self, id: &SessionId, archived: bool) -> AgentResult<Option<SessionRecord>> {
        let Some(record) = self.load(id).await? else {
            return Ok(None);
        };
        let record = record.with_archived(archived);
        self.update(record.clone()).await?;
        Ok(Some(record))
    }

    async fn pin(&self, id: &SessionId, pinned: bool) -> AgentResult<Option<SessionRecord>> {
        let Some(record) = self.load(id).await? else {
            return Ok(None);
        };
        let record = record.with_pinned(pinned);
        self.update(record.clone()).await?;
        Ok(Some(record))
    }

    async fn fork(&self, id: &SessionId) -> AgentResult<Option<SessionRecord>> {
        let Some(record) = self.load(id).await? else {
            return Ok(None);
        };
        let forked = record.forked_from(id.clone());
        self.save(forked.clone()).await?;
        Ok(Some(forked))
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemorySessionStore {
    records: Arc<Mutex<Vec<SessionRecord>>>,
}

impl InMemorySessionStore {
    fn lock_records(&self) -> AgentResult<std::sync::MutexGuard<'_, Vec<SessionRecord>>> {
        self.records.lock().map_err(|_| AgentError::Execution {
            message: "session store lock was poisoned".to_string(),
        })
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn save(&self, record: SessionRecord) -> AgentResult<SessionId> {
        let id = record.id.clone();
        self.lock_records()?.push(record);
        Ok(id)
    }

    async fn load(&self, id: &SessionId) -> AgentResult<Option<SessionRecord>> {
        let record = self
            .lock_records()?
            .iter()
            .find(|record| &record.id == id)
            .cloned();
        Ok(record)
    }

    async fn list(&self) -> AgentResult<Vec<SessionRecord>> {
        Ok(self.lock_records()?.clone())
    }

    async fn update(&self, record: SessionRecord) -> AgentResult<()> {
        let mut records = self.lock_records()?;
        if let Some(existing) = records.iter_mut().find(|existing| existing.id == record.id) {
            *existing = record;
        } else {
            records.push(record);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSessionStore {
    root: PathBuf,
}

impl FileSessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        Self::new(cwd.as_ref().join(".yunxi").join("sessions"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn record_path(&self, id: &SessionId) -> PathBuf {
        self.root.join(format!("{}.json", id.0))
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn save(&self, record: SessionRecord) -> AgentResult<SessionId> {
        std::fs::create_dir_all(&self.root).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to create session directory {}: {error}",
                self.root.display()
            ),
        })?;

        let id = record.id.clone();
        let path = self.record_path(&id);
        let content =
            serde_json::to_string_pretty(&record).map_err(|error| AgentError::Execution {
                message: format!("failed to serialize session {}: {error}", id.0),
            })?;
        std::fs::write(&path, content).map_err(|error| AgentError::Execution {
            message: format!("failed to write session {}: {error}", path.display()),
        })?;
        Ok(id)
    }

    async fn load(&self, id: &SessionId) -> AgentResult<Option<SessionRecord>> {
        let path = self.record_path(id);
        if !path.is_file() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|error| AgentError::Execution {
            message: format!("failed to read session {}: {error}", path.display()),
        })?;
        let record = serde_json::from_str::<SessionRecord>(&content).map_err(|error| {
            AgentError::Execution {
                message: format!("failed to parse session {}: {error}", path.display()),
            }
        })?;
        Ok(Some(record))
    }

    async fn list(&self) -> AgentResult<Vec<SessionRecord>> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }

        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.root).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to read session directory {}: {error}",
                self.root.display()
            ),
        })? {
            let entry = entry.map_err(|error| AgentError::Execution {
                message: format!("failed to read session directory entry: {error}"),
            })?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let content =
                std::fs::read_to_string(&path).map_err(|error| AgentError::Execution {
                    message: format!("failed to read session {}: {error}", path.display()),
                })?;
            let record = serde_json::from_str::<SessionRecord>(&content).map_err(|error| {
                AgentError::Execution {
                    message: format!("failed to parse session {}: {error}", path.display()),
                }
            })?;
            records.push(record);
        }
        records.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        Ok(records)
    }
}

fn preview_title(prompt: &str) -> String {
    const MAX: usize = 48;
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "untitled session".to_string();
    }
    if trimmed.chars().count() <= MAX {
        return trimmed.to_string();
    }

    let mut value = trimmed
        .chars()
        .take(MAX.saturating_sub(3))
        .collect::<String>();
    value.push_str("...");
    value
}

fn preview_for_summary(value: &str) -> String {
    const MAX: usize = 120;
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX {
        return trimmed.to_string();
    }

    let mut preview = trimmed
        .chars()
        .take(MAX.saturating_sub(3))
        .collect::<String>();
    preview.push_str("...");
    preview
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yunxi_agent_protocol::{ResponseItem, RuntimeEvent, ThreadId, TurnId};

    #[test]
    fn runtime_rollout_round_trips_jsonl() {
        let events = vec![
            RuntimeEvent::ThreadStarted {
                thread_id: ThreadId("thread-1".to_string()),
            },
            RuntimeEvent::Item {
                thread_id: ThreadId("thread-1".to_string()),
                turn_id: TurnId("turn-1".to_string()),
                item: ResponseItem::Reasoning {
                    content: "thinking".to_string(),
                },
            },
        ];

        let jsonl = encode_runtime_rollout_jsonl(events.iter()).expect("jsonl");
        let decoded = decode_runtime_rollout_jsonl(&jsonl).expect("decoded");

        assert_eq!(decoded, events);
    }

    #[test]
    fn runtime_rollout_applies_budget() {
        let thread = ThreadMetadata::new(SessionId::new("thread-1"), ".");
        let events = vec![
            RuntimeEvent::ThreadStarted {
                thread_id: ThreadId("thread-1".to_string()),
            },
            RuntimeEvent::TurnCompleted {
                thread_id: ThreadId("thread-1".to_string()),
                turn_id: TurnId("turn-1".to_string()),
            },
        ];

        let rollout = runtime_rollout_from_events(
            thread,
            "turn-1",
            events,
            RolloutBudget::new(1, usize::MAX),
        )
        .expect("rollout");

        assert!(rollout.truncated);
        assert_eq!(rollout.items.len(), 1);
    }

    #[test]
    fn session_graph_view_rebuilds_parent_child_relationships() {
        let mut root = SessionRecord::new(".", "root", None, vec![]);
        root.id = SessionId::new("root");
        let mut child =
            SessionRecord::new(".", "child", None, vec![]).with_parent_id(root.id.clone());
        child.id = SessionId::new("child");

        let view = SessionGraphView::from_sessions([root.clone(), child.clone()])
            .expect("session graph should rebuild");

        assert_eq!(
            view.children_of(&root.id),
            vec![child.agent_session_metadata()]
        );
    }

    #[test]
    fn session_graph_view_rejects_cycles() {
        let mut left = SessionRecord::new(".", "left", None, vec![]);
        left.id = SessionId::new("left");
        left.parent_id = Some(SessionId::new("right"));
        let mut right = SessionRecord::new(".", "right", None, vec![]);
        right.id = SessionId::new("right");
        right.parent_id = Some(SessionId::new("left"));

        let error = SessionGraphView::from_sessions([left, right])
            .expect_err("cyclic session graph should be rejected");

        assert!(format!("{error}").contains("cycle detected"));
    }

    #[test]
    fn session_graph_view_exports_multi_agent_metadata() {
        let mut root = SessionRecord::new(".", "root task", None, vec![]);
        root.id = SessionId::new("root");
        let mut child =
            SessionRecord::new(".", "child task", None, vec![]).with_parent_id(root.id.clone());
        child.id = SessionId::new("child");

        let metadata = SessionGraphView::from_sessions([root, child])
            .expect("session graph")
            .agent_graph_session_metadata();

        assert_eq!(metadata.len(), 2);
        assert!(metadata.iter().any(|item| item.session_id == "child"
            && item.parent_session_id.as_deref() == Some("root")
            && item.agent.task == "child task"));
    }
}
