use crate::memory::{
    MemoryRecallRequest, MemoryRecallResult, MemoryRecord, MemoryScope, MemoryStatus,
};

#[derive(Clone, Debug, Default)]
pub struct MemoryRecallEngine;

impl MemoryRecallEngine {
    pub fn recall(
        &self,
        records: &[MemoryRecord],
        request: &MemoryRecallRequest,
    ) -> MemoryRecallResult {
        let mut scored = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Active)
            .filter(|record| scope_matches(&record.scope, request.workspace_fingerprint.as_deref()))
            .map(|record| (score_record(record, request), record.clone()))
            .filter(|(score, _)| *score > 0.0 || request.query.trim().is_empty())
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
        for (_, record) in scored {
            if records.len() >= request.max_records {
                truncated = true;
                break;
            }
            let len = record.content.chars().count();
            if budget_used_chars + len > request.budget_chars {
                truncated = true;
                break;
            }
            budget_used_chars += len;
            records.push(record);
        }

        MemoryRecallResult {
            records,
            budget_used_chars,
            truncated,
        }
    }
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
    let mut score = record.importance * 3.0 + record.confidence;
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
    score
}
