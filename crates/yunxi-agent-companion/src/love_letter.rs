use serde::{Deserialize, Serialize};
use yunxi_agent_core::LoveLetterSettings;
use yunxi_agent_persona::{
    MemoryKind, MemoryPrivacyClassifier, MemoryRecord, MemorySensitivity, MemoryStatus,
};

use crate::CompanionRelationshipStage;

pub const LOVE_LETTER_SCHEMA_VERSION: u32 = 1;
const DAY_MILLIS: u128 = 86_400_000;
const MAX_SELECTED_MEMORIES: usize = 12;
const MAX_SELECTED_MEMORY_CHARS: usize = 4_800;
const MAX_SINGLE_MEMORY_CHARS: usize = 600;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoveLetterMemoryReference {
    pub id: String,
    pub revision: u32,
    pub updated_at_millis: u128,
    pub kind: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoveLetterMemorySelection {
    pub references: Vec<LoveLetterMemoryReference>,
    pub contents: Vec<String>,
    pub memory_revision: String,
    pub earliest_updated_at_millis: Option<u128>,
    pub latest_updated_at_millis: Option<u128>,
}

impl LoveLetterMemorySelection {
    pub fn new_memory_count_since(&self, last_generated_at_millis: Option<u128>) -> usize {
        let Some(last_generated_at_millis) = last_generated_at_millis else {
            return self.references.len();
        };
        self.references
            .iter()
            .filter(|memory| memory.updated_at_millis > last_generated_at_millis)
            .count()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoveLetterMemorySelector;

impl LoveLetterMemorySelector {
    pub fn select(&self, records: &[MemoryRecord], now_millis: u128) -> LoveLetterMemorySelection {
        let privacy = MemoryPrivacyClassifier;
        let mut candidates = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Active)
            .filter(|record| record.is_recallable_at(now_millis))
            .filter(|record| record.sensitivity != MemorySensitivity::High)
            .filter(|record| !privacy.contains_secret(&record.content))
            .filter(|record| eligible_kind(record.kind))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            memory_score(right)
                .total_cmp(&memory_score(left))
                .then_with(|| right.updated_at_millis.cmp(&left.updated_at_millis))
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut references = Vec::new();
        let mut contents = Vec::new();
        let mut used_chars = 0usize;
        for record in candidates.into_iter().take(MAX_SELECTED_MEMORIES) {
            let remaining = MAX_SELECTED_MEMORY_CHARS.saturating_sub(used_chars);
            if remaining == 0 {
                break;
            }
            let content = truncate_chars(
                record.content.trim(),
                remaining.min(MAX_SINGLE_MEMORY_CHARS),
            );
            if content.is_empty() {
                continue;
            }
            used_chars = used_chars.saturating_add(content.chars().count());
            references.push(LoveLetterMemoryReference {
                id: record.id.clone(),
                revision: record.revision,
                updated_at_millis: record.updated_at_millis,
                kind: memory_kind_label(record.kind).to_string(),
                content_hash: format!("{:016x}", stable_hash64(record.content.as_bytes())),
            });
            contents.push(content);
        }
        let earliest_updated_at_millis = references
            .iter()
            .map(|memory| memory.updated_at_millis)
            .min();
        let latest_updated_at_millis = references
            .iter()
            .map(|memory| memory.updated_at_millis)
            .max();
        let revision_seed = references
            .iter()
            .map(|memory| format!("{}:{}:{}", memory.id, memory.revision, memory.content_hash))
            .collect::<Vec<_>>()
            .join("|");
        LoveLetterMemorySelection {
            references,
            contents,
            memory_revision: format!("memory:{:016x}", stable_hash64(revision_seed.as_bytes())),
            earliest_updated_at_millis,
            latest_updated_at_millis,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoveLetterEligibilityInput<'a> {
    pub settings: &'a LoveLetterSettings,
    pub companion_enabled: bool,
    pub persona_enabled: bool,
    pub memory_enabled: bool,
    pub relationship_stage: CompanionRelationshipStage,
    pub owner_scope: &'a str,
    pub profile_consistency_key: &'a str,
    pub selection: &'a LoveLetterMemorySelection,
    pub last_generated_at_millis: Option<u128>,
    pub generated_today: u32,
    pub has_open_task: bool,
    pub retry_after_millis: Option<u128>,
    pub now_millis: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoveLetterEligibilityDecision {
    Eligible {
        idempotency_key: String,
        cooldown_days: u32,
    },
    Ineligible {
        reason: LoveLetterIneligibilityReason,
        retry_at_millis: Option<u128>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoveLetterIneligibilityReason {
    Disabled,
    CompanionUnavailable,
    PersonaUnavailable,
    MemoryUnavailable,
    RelationshipTooNew,
    InsufficientMemories,
    NoNewMemories,
    OpenTask,
    DailyLimit,
    Cooldown,
    ErrorBackoff,
}

#[derive(Clone, Debug, Default)]
pub struct LoveLetterEligibilityPolicy;

impl LoveLetterEligibilityPolicy {
    pub fn decide(&self, input: &LoveLetterEligibilityInput<'_>) -> LoveLetterEligibilityDecision {
        let ineligible = |reason, retry_at_millis| LoveLetterEligibilityDecision::Ineligible {
            reason,
            retry_at_millis,
        };
        if !input.settings.enabled {
            return ineligible(LoveLetterIneligibilityReason::Disabled, None);
        }
        if !input.companion_enabled {
            return ineligible(LoveLetterIneligibilityReason::CompanionUnavailable, None);
        }
        if !input.persona_enabled || input.profile_consistency_key.trim().is_empty() {
            return ineligible(LoveLetterIneligibilityReason::PersonaUnavailable, None);
        }
        if !input.memory_enabled {
            return ineligible(LoveLetterIneligibilityReason::MemoryUnavailable, None);
        }
        if input.relationship_stage == CompanionRelationshipStage::New {
            return ineligible(LoveLetterIneligibilityReason::RelationshipTooNew, None);
        }
        if input.selection.references.len() < input.settings.minimum_active_memories {
            return ineligible(LoveLetterIneligibilityReason::InsufficientMemories, None);
        }
        if input.has_open_task {
            return ineligible(LoveLetterIneligibilityReason::OpenTask, None);
        }
        if input.generated_today >= input.settings.max_per_day {
            return ineligible(LoveLetterIneligibilityReason::DailyLimit, None);
        }
        if let Some(retry_at_millis) = input.retry_after_millis
            && input.now_millis < retry_at_millis
        {
            return ineligible(
                LoveLetterIneligibilityReason::ErrorBackoff,
                Some(retry_at_millis),
            );
        }
        let new_memory_count = input
            .selection
            .new_memory_count_since(input.last_generated_at_millis);
        if new_memory_count < input.settings.minimum_new_memories {
            return ineligible(LoveLetterIneligibilityReason::NoNewMemories, None);
        }

        let cooldown_days = deterministic_cooldown_days(
            input.settings,
            input.owner_scope,
            input.profile_consistency_key,
            &input.selection.memory_revision,
        );
        let base_millis = input
            .last_generated_at_millis
            .or(input.selection.earliest_updated_at_millis)
            .unwrap_or(input.now_millis);
        let eligible_at_millis =
            base_millis.saturating_add(u128::from(cooldown_days).saturating_mul(DAY_MILLIS));
        if input.now_millis < eligible_at_millis {
            return ineligible(
                LoveLetterIneligibilityReason::Cooldown,
                Some(eligible_at_millis),
            );
        }

        let window = input.now_millis / DAY_MILLIS;
        let seed = format!(
            "{}|{}|{}|{}",
            input.owner_scope,
            input.profile_consistency_key,
            input.selection.memory_revision,
            window
        );
        LoveLetterEligibilityDecision::Eligible {
            idempotency_key: format!("love-letter:{:016x}", stable_hash64(seed.as_bytes())),
            cooldown_days,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoveLetterTaskState {
    Pending,
    Generating,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoveLetterTask {
    pub schema_version: u32,
    pub task_id: String,
    pub idempotency_key: String,
    pub owner_scope: String,
    pub state: LoveLetterTaskState,
    pub profile_id: String,
    pub profile_consistency_key: String,
    pub memory_revision: String,
    pub memory_references: Vec<LoveLetterMemoryReference>,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
    pub generation_attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_started_at_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_elapsed_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mailbox_item_id: Option<String>,
}

impl LoveLetterTask {
    pub fn pending(
        idempotency_key: impl Into<String>,
        owner_scope: impl Into<String>,
        profile_id: impl Into<String>,
        profile_consistency_key: impl Into<String>,
        selection: &LoveLetterMemorySelection,
        now_millis: u128,
    ) -> Self {
        let idempotency_key = idempotency_key.into();
        Self {
            schema_version: LOVE_LETTER_SCHEMA_VERSION,
            task_id: format!("letter-{:016x}", stable_hash64(idempotency_key.as_bytes())),
            idempotency_key,
            owner_scope: owner_scope.into(),
            state: LoveLetterTaskState::Pending,
            profile_id: profile_id.into(),
            profile_consistency_key: profile_consistency_key.into(),
            memory_revision: selection.memory_revision.clone(),
            memory_references: selection.references.clone(),
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
            generation_attempts: 0,
            generation_started_at_millis: None,
            generation_elapsed_millis: None,
            lease_expires_at_millis: None,
            generated_at_millis: None,
            retry_after_millis: None,
            last_error_label: None,
            mailbox_item_id: None,
        }
    }
}

fn deterministic_cooldown_days(
    settings: &LoveLetterSettings,
    owner_scope: &str,
    profile_consistency_key: &str,
    memory_revision: &str,
) -> u32 {
    let minimum = settings.cooldown_min_days.min(settings.cooldown_max_days);
    let maximum = settings.cooldown_min_days.max(settings.cooldown_max_days);
    let span = maximum.saturating_sub(minimum).saturating_add(1);
    let seed = format!("{owner_scope}|{profile_consistency_key}|{memory_revision}");
    minimum.saturating_add((stable_hash64(seed.as_bytes()) % u64::from(span)) as u32)
}

fn eligible_kind(kind: MemoryKind) -> bool {
    matches!(
        kind,
        MemoryKind::Preference
            | MemoryKind::PersonalFact
            | MemoryKind::RelationshipNote
            | MemoryKind::Goal
            | MemoryKind::Event
    )
}

fn memory_score(record: &MemoryRecord) -> f32 {
    let kind_weight = match record.kind {
        MemoryKind::RelationshipNote => 0.35,
        MemoryKind::Event => 0.3,
        MemoryKind::Preference => 0.25,
        MemoryKind::PersonalFact => 0.2,
        MemoryKind::Goal => 0.15,
        _ => 0.0,
    };
    record.importance.clamp(0.0, 1.0) * 0.5 + record.confidence.clamp(0.0, 1.0) * 0.25 + kind_weight
}

fn memory_kind_label(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Preference => "preference",
        MemoryKind::PersonalFact => "personal_fact",
        MemoryKind::RelationshipNote => "relationship_note",
        MemoryKind::EmotionalState => "emotional_state",
        MemoryKind::Goal => "goal",
        MemoryKind::ProjectContext => "project_context",
        MemoryKind::Correction => "correction",
        MemoryKind::Event => "event",
        MemoryKind::ToolTraceSummary => "tool_trace_summary",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

pub fn stable_hash64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use yunxi_agent_persona::{MemoryScope, MemoryTemporal};

    fn memory(id: &str, kind: MemoryKind, content: &str, updated: u128) -> MemoryRecord {
        let mut record = MemoryRecord::new(id, MemoryScope::Relationship, kind, content, updated)
            .with_status(MemoryStatus::Active);
        record.updated_at_millis = updated;
        record.temporal = MemoryTemporal {
            observed_at_millis: updated,
            valid_from_millis: Some(updated),
            ..MemoryTemporal::default()
        };
        record
    }

    fn settings() -> LoveLetterSettings {
        LoveLetterSettings {
            enabled: true,
            cooldown_min_days: 0,
            cooldown_max_days: 0,
            ..LoveLetterSettings::default()
        }
    }

    #[test]
    fn selector_excludes_pending_high_secret_tool_and_expired_memories() {
        let now = 10_000;
        let active = memory("active", MemoryKind::RelationshipNote, "一起走过的雨天", 1);
        let mut pending = memory("pending", MemoryKind::Event, "pending", 2);
        pending.status = MemoryStatus::Pending;
        let mut high = memory("high", MemoryKind::PersonalFact, "private", 3);
        high.sensitivity = MemorySensitivity::High;
        let secret = memory("secret", MemoryKind::PersonalFact, "API key: secret", 4);
        let tool = memory("tool", MemoryKind::ToolTraceSummary, "shell output", 5);
        let mut expired = memory("expired", MemoryKind::Goal, "old goal", 6);
        expired.temporal.expires_at_millis = Some(now);

        let selection =
            LoveLetterMemorySelector.select(&[active, pending, high, secret, tool, expired], now);

        assert_eq!(selection.references.len(), 1);
        assert_eq!(selection.references[0].id, "active");
    }

    #[test]
    fn eligibility_is_deterministic_and_requires_new_memory() {
        let selection = LoveLetterMemorySelector.select(
            &[
                memory("a", MemoryKind::Preference, "喜欢安静", 1),
                memory("b", MemoryKind::RelationshipNote, "信任逐渐建立", 2),
                memory("c", MemoryKind::Event, "共同完成一次测试", 3),
            ],
            100,
        );
        let config = settings();
        let input = LoveLetterEligibilityInput {
            settings: &config,
            companion_enabled: true,
            persona_enabled: true,
            memory_enabled: true,
            relationship_stage: CompanionRelationshipStage::Familiar,
            owner_scope: "workspace",
            profile_consistency_key: "profile:1",
            selection: &selection,
            last_generated_at_millis: None,
            generated_today: 0,
            has_open_task: false,
            retry_after_millis: None,
            now_millis: 100,
        };
        let first = LoveLetterEligibilityPolicy.decide(&input);
        let second = LoveLetterEligibilityPolicy.decide(&input);
        assert_eq!(first, second);
        assert!(matches!(
            first,
            LoveLetterEligibilityDecision::Eligible { .. }
        ));

        let no_new = LoveLetterEligibilityPolicy.decide(&LoveLetterEligibilityInput {
            last_generated_at_millis: Some(100),
            ..input
        });
        assert!(matches!(
            no_new,
            LoveLetterEligibilityDecision::Ineligible {
                reason: LoveLetterIneligibilityReason::NoNewMemories,
                ..
            }
        ));
    }
}
