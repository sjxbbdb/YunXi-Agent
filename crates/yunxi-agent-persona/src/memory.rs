use crate::policy::MemoryWritePolicy;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    GlobalUser,
    Workspace { root_fingerprint: String },
    AgentIdentity,
    Relationship,
}

impl MemoryScope {
    pub fn label(&self) -> String {
        match self {
            Self::GlobalUser => "global_user".to_string(),
            Self::Workspace { root_fingerprint } => format!("workspace:{root_fingerprint}"),
            Self::AgentIdentity => "agent_identity".to_string(),
            Self::Relationship => "relationship".to_string(),
        }
    }

    pub fn is_workspace_match(&self, root_fingerprint: &str) -> bool {
        matches!(self, Self::Workspace { root_fingerprint: current } if current == root_fingerprint)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Preference,
    PersonalFact,
    RelationshipNote,
    EmotionalState,
    Goal,
    ProjectContext,
    Correction,
    Event,
    ToolTraceSummary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySensitivity {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Pending,
    Rejected,
    Archived,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub schema_version: u32,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<String>,
    pub confidence: f32,
    pub importance: f32,
    pub sensitivity: MemorySensitivity,
    pub status: MemoryStatus,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
}

impl MemoryRecord {
    pub fn new(
        id: impl Into<String>,
        scope: MemoryScope,
        kind: MemoryKind,
        content: impl Into<String>,
        now: u128,
    ) -> Self {
        Self {
            id: id.into(),
            schema_version: SCHEMA_VERSION,
            scope,
            kind,
            content: content.into(),
            source_session_id: None,
            confidence: 0.75,
            importance: 0.5,
            sensitivity: MemorySensitivity::Low,
            status: MemoryStatus::Pending,
            created_at_millis: now,
            updated_at_millis: now,
        }
    }

    pub fn with_source_session_id(mut self, source_session_id: impl Into<String>) -> Self {
        self.source_session_id = Some(source_session_id.into());
        self
    }

    pub fn with_scores(mut self, confidence: f32, importance: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    pub fn with_sensitivity(mut self, sensitivity: MemorySensitivity) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    pub fn with_status(mut self, status: MemoryStatus) -> Self {
        self.status = status;
        self.updated_at_millis = now_millis();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub proposed_record: MemoryRecord,
    pub evidence: String,
    pub write_policy: MemoryWritePolicy,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryRecallRequest {
    pub query: String,
    pub workspace_fingerprint: Option<String>,
    pub max_records: usize,
    pub budget_chars: usize,
}

impl MemoryRecallRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            workspace_fingerprint: None,
            max_records: 8,
            budget_chars: 1200,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecallResult {
    pub records: Vec<MemoryRecord>,
    pub budget_used_chars: usize,
    pub truncated: bool,
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
