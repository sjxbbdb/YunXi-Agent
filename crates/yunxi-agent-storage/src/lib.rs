use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use yunxi_agent_core::{AgentError, AgentEvent, AgentResult};

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
        }
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
