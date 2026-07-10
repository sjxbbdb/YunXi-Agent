use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use yunxi_agent_core::{AgentError, AgentEvent, AgentResult, AgentRunStatus};

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
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

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
