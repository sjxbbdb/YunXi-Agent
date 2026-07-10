use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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
    pub created_at_millis: u128,
}

impl SessionRecord {
    pub fn new(
        cwd: impl Into<PathBuf>,
        prompt: impl Into<String>,
        final_response: Option<String>,
        events: Vec<AgentEvent>,
    ) -> Self {
        Self {
            id: SessionId::generate(),
            cwd: cwd.into(),
            prompt: prompt.into(),
            final_response,
            events,
            status: AgentRunStatus::Completed,
            model: None,
            provider: None,
            created_at_millis: now_millis(),
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
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save(&self, record: SessionRecord) -> AgentResult<SessionId>;
    async fn load(&self, id: &SessionId) -> AgentResult<Option<SessionRecord>>;
    async fn list(&self) -> AgentResult<Vec<SessionRecord>>;
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

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
