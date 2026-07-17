use crate::dedup::dedup_key_for_record;
use crate::policy::{MemoryPrivacyClassifier, MemoryWritePolicy};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 3;

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
    Profile,
    Preference,
    Relationship,
    Workspace,
    Episode,
    ToolTrace,
    #[default]
    Unknown,
}

impl MemoryLayer {
    pub fn for_kind(kind: MemoryKind) -> Self {
        match kind {
            MemoryKind::Preference => Self::Preference,
            MemoryKind::PersonalFact => Self::Profile,
            MemoryKind::RelationshipNote | MemoryKind::EmotionalState => Self::Relationship,
            MemoryKind::ProjectContext | MemoryKind::Correction => Self::Workspace,
            MemoryKind::Goal | MemoryKind::Event => Self::Episode,
            MemoryKind::ToolTraceSummary => Self::ToolTrace,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEntityType {
    User,
    Agent,
    Workspace,
    Project,
    Tool,
    Person,
    Relationship,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntityRef {
    #[serde(default)]
    pub entity_type: MemoryEntityType,
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryTemporal {
    #[serde(default)]
    pub observed_at_millis: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_at_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_millis: Option<u128>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryEvidence {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemorySourceAttribution {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extractor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
}

impl MemorySourceAttribution {
    fn is_empty(&self) -> bool {
        self.extractor.is_none()
            && self.session_id.is_none()
            && self.workspace_fingerprint.is_none()
            && self.provider.is_none()
            && self.rule_id.is_none()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemorySource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extractor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributions: Vec<MemorySourceAttribution>,
}

impl MemorySource {
    pub fn primary_attribution(&self) -> MemorySourceAttribution {
        MemorySourceAttribution {
            extractor: self.extractor.clone(),
            session_id: self.session_id.clone(),
            workspace_fingerprint: self.workspace_fingerprint.clone(),
            provider: self.provider.clone(),
            rule_id: self.rule_id.clone(),
        }
    }

    pub fn ensure_primary_attribution(&mut self) {
        let attribution = self.primary_attribution();
        if !attribution.is_empty() && !self.attributions.contains(&attribution) {
            self.attributions.push(attribution);
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryInvalidation {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts_with: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalidated_at_millis: Option<u128>,
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
    #[serde(default)]
    pub dedup_key: String,
    #[serde(default = "default_revision")]
    pub revision: u32,
    #[serde(default = "default_merged_count")]
    pub merged_count: u32,
    #[serde(default)]
    pub layer: MemoryLayer,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<MemoryEntityRef>,
    #[serde(default)]
    pub temporal: MemoryTemporal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<MemoryEvidence>,
    #[serde(default)]
    pub source: MemorySource,
    #[serde(default)]
    pub invalidation: MemoryInvalidation,
}

impl MemoryRecord {
    pub fn new(
        id: impl Into<String>,
        scope: MemoryScope,
        kind: MemoryKind,
        content: impl Into<String>,
        now: u128,
    ) -> Self {
        let mut record = Self {
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
            dedup_key: String::new(),
            revision: 1,
            merged_count: 1,
            layer: MemoryLayer::for_kind(kind),
            entities: Vec::new(),
            temporal: MemoryTemporal {
                observed_at_millis: now,
                valid_from_millis: Some(now),
                ..MemoryTemporal::default()
            },
            evidence: Vec::new(),
            source: MemorySource::default(),
            invalidation: MemoryInvalidation::default(),
        };
        record.ensure_dedup_metadata();
        record
    }

    pub fn with_source_session_id(mut self, source_session_id: impl Into<String>) -> Self {
        let source_session_id = source_session_id.into();
        self.source_session_id = Some(source_session_id.clone());
        self.source.session_id = Some(source_session_id);
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

    pub fn with_dedup_metadata(mut self) -> Self {
        self.ensure_dedup_metadata();
        self
    }

    pub fn ensure_dedup_metadata(&mut self) {
        self.schema_version = SCHEMA_VERSION;
        if self.layer == MemoryLayer::Unknown {
            self.layer = MemoryLayer::for_kind(self.kind);
        }
        if self.temporal.observed_at_millis == 0 {
            self.temporal.observed_at_millis = self.created_at_millis;
        }
        if self.temporal.valid_from_millis.is_none() {
            self.temporal.valid_from_millis = Some(self.created_at_millis);
        }
        if self.source.session_id.is_none() {
            self.source.session_id.clone_from(&self.source_session_id);
        }
        if self.source.workspace_fingerprint.is_none()
            && let MemoryScope::Workspace { root_fingerprint } = &self.scope
        {
            self.source.workspace_fingerprint = Some(root_fingerprint.clone());
        }
        self.source.ensure_primary_attribution();
        if self.dedup_key.trim().is_empty() {
            self.dedup_key = dedup_key_for_record(self).as_storage_key();
        }
        if self.revision == 0 {
            self.revision = 1;
        }
        if self.merged_count == 0 {
            self.merged_count = 1;
        }
    }

    pub fn is_recallable_at(&self, now_millis: u128) -> bool {
        self.status == MemoryStatus::Active
            && self
                .temporal
                .valid_from_millis
                .is_none_or(|valid_from| valid_from <= now_millis)
            && self
                .temporal
                .expires_at_millis
                .is_none_or(|expires_at| expires_at > now_millis)
            && self.invalidation.invalidated_at_millis.is_none()
            && self.invalidation.superseded_by.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub proposed_record: MemoryRecord,
    pub evidence: String,
    pub write_policy: MemoryWritePolicy,
    pub reason: String,
}

impl MemoryCandidate {
    pub fn ensure_v3_provenance(&mut self) {
        let extractor = if self.reason.starts_with("provider:") {
            "provider"
        } else {
            "rule"
        };
        self.proposed_record.source.extractor = Some(extractor.to_string());
        if extractor == "rule" && !self.reason.trim().is_empty() {
            self.proposed_record.source.rule_id = Some(self.reason.clone());
        }
        self.proposed_record.ensure_dedup_metadata();

        let evidence_summary = if MemoryPrivacyClassifier.contains_secret(&self.evidence) {
            "[redacted: secret-like evidence omitted]".to_string()
        } else {
            compact_evidence(&self.evidence, 240)
        };
        if evidence_summary.is_empty() {
            return;
        }
        let evidence = MemoryEvidence {
            kind: format!("{extractor}_candidate"),
            summary: evidence_summary,
            source_session_id: self.proposed_record.source_session_id.clone(),
            source_turn_id: None,
            source_event_id: None,
        };
        if !self.proposed_record.evidence.contains(&evidence) {
            self.proposed_record.evidence.push(evidence);
        }
    }
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecallResult {
    pub records: Vec<MemoryRecord>,
    pub budget_used_chars: usize,
    pub truncated: bool,
    #[serde(default)]
    pub always_on_count: usize,
    #[serde(default)]
    pub dropped_unrelated: usize,
    #[serde(default)]
    pub dropped_by_budget: usize,
    #[serde(default)]
    pub dropped_duplicates: usize,
}

fn default_revision() -> u32 {
    1
}

fn default_merged_count() -> u32 {
    1
}

fn compact_evidence(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
