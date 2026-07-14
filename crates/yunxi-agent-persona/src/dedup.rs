use crate::memory::{MemoryCandidate, MemoryKind, MemoryRecord, MemorySensitivity, MemoryStatus};
use crate::merge::merge_memory_candidates;
use crate::policy::MemoryWritePolicy;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MemoryDedupKey {
    pub scope_label: String,
    pub kind: MemoryKind,
    pub normalized_content: String,
}

impl MemoryDedupKey {
    pub fn as_storage_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.scope_label,
            memory_kind_key(self.kind),
            self.normalized_content
        )
    }
}

pub fn dedup_key_for_record(record: &MemoryRecord) -> MemoryDedupKey {
    MemoryDedupKey {
        scope_label: record.scope.label(),
        kind: record.kind,
        normalized_content: normalized_memory_content(record.kind, &record.content),
    }
}

pub fn normalized_memory_content(kind: MemoryKind, content: &str) -> String {
    if matches!(kind, MemoryKind::Preference) {
        if contains_any(content, &["中文", "汉语", "Chinese", "Mandarin"]) {
            return "language:zh".to_string();
        }
        if contains_any(content, &["英文", "英语", "English"]) {
            return "language:en".to_string();
        }
    }
    normalize_text(content)
}

pub fn ensure_record_dedup_metadata(record: &mut MemoryRecord) {
    if record.dedup_key.trim().is_empty() {
        record.dedup_key = dedup_key_for_record(record).as_storage_key();
    }
    if record.revision == 0 {
        record.revision = 1;
    }
    if record.merged_count == 0 {
        record.merged_count = 1;
    }
}

pub fn deduplicate_candidates(candidates: Vec<MemoryCandidate>) -> Vec<MemoryCandidate> {
    let mut by_key: BTreeMap<String, MemoryCandidate> = BTreeMap::new();
    let mut order = Vec::new();
    for mut candidate in candidates {
        ensure_record_dedup_metadata(&mut candidate.proposed_record);
        let key = candidate.proposed_record.dedup_key.clone();
        if let Some(existing) = by_key.get_mut(&key) {
            merge_memory_candidates(existing, candidate);
        } else {
            order.push(key.clone());
            by_key.insert(key, candidate);
        }
    }
    order
        .into_iter()
        .filter_map(|key| by_key.remove(&key))
        .collect()
}

pub fn max_sensitivity(left: MemorySensitivity, right: MemorySensitivity) -> MemorySensitivity {
    match (left, right) {
        (MemorySensitivity::High, _) | (_, MemorySensitivity::High) => MemorySensitivity::High,
        (MemorySensitivity::Medium, _) | (_, MemorySensitivity::Medium) => {
            MemorySensitivity::Medium
        }
        _ => MemorySensitivity::Low,
    }
}

pub fn max_policy(left: MemoryWritePolicy, right: MemoryWritePolicy) -> MemoryWritePolicy {
    if policy_rank(left) >= policy_rank(right) {
        left
    } else {
        right
    }
}

pub fn status_for_policy(policy: MemoryWritePolicy) -> MemoryStatus {
    match policy {
        MemoryWritePolicy::Auto => MemoryStatus::Active,
        MemoryWritePolicy::RequireConfirmation => MemoryStatus::Pending,
        MemoryWritePolicy::Discard | MemoryWritePolicy::Disabled => MemoryStatus::Rejected,
    }
}

fn policy_rank(policy: MemoryWritePolicy) -> u8 {
    match policy {
        MemoryWritePolicy::Auto => 1,
        MemoryWritePolicy::RequireConfirmation => 2,
        MemoryWritePolicy::Disabled => 3,
        MemoryWritePolicy::Discard => 4,
    }
}

fn normalize_text(content: &str) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for ch in content.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_space = false;
        } else if ch.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
                last_space = true;
            }
        } else if is_cjk(ch) {
            out.push(ch);
            last_space = false;
        }
    }
    out.trim().to_string()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| value.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
        || ('\u{3400}'..='\u{4dbf}').contains(&ch)
        || ('\u{f900}'..='\u{faff}').contains(&ch)
}

fn memory_kind_key(kind: MemoryKind) -> &'static str {
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
