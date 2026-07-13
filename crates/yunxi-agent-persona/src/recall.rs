use crate::dedup::dedup_key_for_record;
use crate::memory::{
    MemoryKind, MemoryRecallRequest, MemoryRecallResult, MemoryRecord, MemoryScope, MemoryStatus,
};
use std::collections::BTreeMap;

const RECALL_RELEVANCE_THRESHOLD: f32 = 2.0;
const ALWAYS_ON_LIMIT: usize = 3;

#[derive(Clone, Debug, Default)]
pub struct MemoryRecallEngine;

impl MemoryRecallEngine {
    pub fn recall(
        &self,
        records: &[MemoryRecord],
        request: &MemoryRecallRequest,
    ) -> MemoryRecallResult {
        let (records, dropped_duplicates) = collapse_duplicate_records(records, request);
        let mut dropped_unrelated = 0;
        let mut always_on_count = 0;
        let mut scored = records
            .iter()
            .filter_map(|record| {
                if is_always_on_profile_record(record) {
                    if always_on_count < ALWAYS_ON_LIMIT {
                        always_on_count += 1;
                        return Some((
                            RECALL_RELEVANCE_THRESHOLD + record.importance,
                            record.clone(),
                        ));
                    }
                    dropped_unrelated += 1;
                    return None;
                }
                let score = score_record(record, request);
                if !request.query.trim().is_empty() && score >= RECALL_RELEVANCE_THRESHOLD {
                    Some((score, record.clone()))
                } else {
                    dropped_unrelated += 1;
                    None
                }
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.1.updated_at_millis.cmp(&left.1.updated_at_millis))
        });

        let mut records = Vec::new();
        let mut budget_used_chars = 0;
        let mut truncated = false;
        let mut dropped_by_budget = 0;
        for (_, record) in scored {
            if records.len() >= request.max_records {
                truncated = true;
                dropped_by_budget += 1;
                continue;
            }
            let len = record.content.chars().count();
            if budget_used_chars + len > request.budget_chars {
                truncated = true;
                dropped_by_budget += 1;
                continue;
            }
            budget_used_chars += len;
            records.push(record);
        }

        MemoryRecallResult {
            records,
            budget_used_chars,
            truncated,
            always_on_count,
            dropped_unrelated,
            dropped_by_budget,
            dropped_duplicates,
        }
    }
}

fn collapse_duplicate_records(
    records: &[MemoryRecord],
    request: &MemoryRecallRequest,
) -> (Vec<MemoryRecord>, usize) {
    let mut by_key: BTreeMap<String, MemoryRecord> = BTreeMap::new();
    let mut dropped_duplicates = 0;
    for record in records
        .iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .filter(|record| scope_matches(&record.scope, request.workspace_fingerprint.as_deref()))
    {
        let key = if record.dedup_key.trim().is_empty() {
            dedup_key_for_record(record).as_storage_key()
        } else {
            record.dedup_key.clone()
        };
        match by_key.get_mut(&key) {
            Some(existing) => {
                dropped_duplicates += 1;
                if should_replace_duplicate(existing, record) {
                    *existing = record.clone();
                }
            }
            None => {
                by_key.insert(key, record.clone());
            }
        }
    }
    (by_key.into_values().collect(), dropped_duplicates)
}

fn should_replace_duplicate(existing: &MemoryRecord, candidate: &MemoryRecord) -> bool {
    candidate.updated_at_millis > existing.updated_at_millis
        || (candidate.updated_at_millis == existing.updated_at_millis
            && candidate.revision > existing.revision)
        || (candidate.updated_at_millis == existing.updated_at_millis
            && candidate.revision == existing.revision
            && candidate.importance > existing.importance)
}

fn scope_matches(scope: &MemoryScope, workspace_fingerprint: Option<&str>) -> bool {
    match scope {
        MemoryScope::GlobalUser | MemoryScope::AgentIdentity | MemoryScope::Relationship => true,
        MemoryScope::Workspace { root_fingerprint } => {
            workspace_fingerprint == Some(root_fingerprint.as_str())
        }
    }
}

fn score_record(record: &MemoryRecord, request: &MemoryRecallRequest) -> f32 {
    let mut score = 0.0;
    let query = request.query.to_ascii_lowercase();
    let content = record.content.to_ascii_lowercase();
    for term in query.split_whitespace().filter(|term| term.len() >= 2) {
        if content.contains(term) {
            score += 2.0;
        }
    }
    if matches!(record.scope, MemoryScope::Workspace { .. }) {
        score += 0.5;
    }
    if kind_trigger_matches(record.kind, &query, &content) {
        score += 2.0;
    }
    if score > 0.0 {
        score += record.importance + record.confidence * 0.5;
    }
    score
}

fn is_always_on_profile_record(record: &MemoryRecord) -> bool {
    if !matches!(record.kind, MemoryKind::Preference) {
        return false;
    }
    if !matches!(
        record.scope,
        MemoryScope::GlobalUser | MemoryScope::Relationship
    ) {
        return false;
    }
    contains_any(
        &record.content,
        &[
            "中文", "英文", "语言", "回答", "交流", "称呼", "语气", "表达", "language", "reply",
            "answer", "tone",
        ],
    )
}

fn kind_trigger_matches(kind: MemoryKind, query: &str, content: &str) -> bool {
    match kind {
        MemoryKind::ProjectContext | MemoryKind::ToolTraceSummary => {
            contains_any(
                query,
                &[
                    "项目",
                    "仓库",
                    "代码",
                    "源码",
                    "workspace",
                    "repo",
                    "project",
                ],
            ) && contains_any(
                content,
                &[
                    "项目",
                    "仓库",
                    "代码",
                    "源码",
                    "workspace",
                    "repo",
                    "project",
                ],
            )
        }
        MemoryKind::PersonalFact => contains_any(
            query,
            &["我", "我的", "个人", "偏好", "profile", "me", "my"],
        ),
        MemoryKind::Goal => contains_any(query, &["目标", "计划", "下一步", "goal", "plan"]),
        MemoryKind::Correction => contains_any(
            query,
            &["要求", "约束", "不要", "纠正", "rule", "constraint"],
        ),
        MemoryKind::RelationshipNote | MemoryKind::EmotionalState => {
            contains_any(query, &["关系", "情绪", "感受", "relationship", "emotion"])
        }
        MemoryKind::Preference | MemoryKind::Event => false,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| value.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
}
