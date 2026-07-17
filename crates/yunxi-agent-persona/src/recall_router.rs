use crate::dedup::dedup_key_for_record;
use crate::memory::{
    MemoryKind, MemoryLayer, MemoryRecallRequest, MemoryRecallResult, MemoryRecord, MemoryScope,
    MemorySensitivity, now_millis,
};
use crate::recall::{MemoryRecallEngine, RECALL_RELEVANCE_THRESHOLD, score_record};
use crate::relationship_graph::{
    MemoryGraphRelation, RelationshipGraphLite, is_relationship_timeline_query,
    temporal_ordering_time,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const BOOT_MIN_CONFIDENCE: f32 = 0.6;
const BOOT_MIN_IMPORTANCE: f32 = 0.4;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRecallRoute {
    Boot,
    Dynamic,
    DroppedDuplicate,
    DroppedUnrelated,
    DroppedBudget,
    DroppedInvalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecallExplanation {
    pub memory_id: String,
    pub route: MemoryRecallRoute,
    pub score: f32,
    pub selected: bool,
    pub reason: String,
    pub source: String,
    pub layer: MemoryLayer,
    pub scope: String,
    pub kind: MemoryKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<MemoryGraphRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryRecallRouterRequest {
    pub query: String,
    pub workspace_fingerprint: Option<String>,
    pub boot_budget_chars: usize,
    pub dynamic_budget_chars: usize,
    pub boot_max_records: usize,
    pub dynamic_max_records: usize,
}

impl MemoryRecallRouterRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            workspace_fingerprint: None,
            boot_budget_chars: 1000,
            dynamic_budget_chars: 1200,
            boot_max_records: 6,
            dynamic_max_records: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecallRouterResult {
    pub boot_context: MemoryRecallResult,
    pub dynamic_recall: MemoryRecallResult,
    pub explanations: Vec<MemoryRecallExplanation>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryRecallRouter {
    engine: MemoryRecallEngine,
}

impl MemoryRecallRouter {
    pub fn route(
        &self,
        records: &[MemoryRecord],
        request: &MemoryRecallRouterRequest,
    ) -> MemoryRecallRouterResult {
        let now = now_millis();
        let graph = RelationshipGraphLite::from_records(records);
        let timeline_query = is_relationship_timeline_query(&request.query);
        let mut explanations = Vec::new();
        let mut unique = BTreeMap::<String, MemoryRecord>::new();
        let mut historical_candidates = Vec::new();

        for record in records {
            if !record.is_recallable_at(now) {
                if timeline_query
                    && is_graph_memory(record)
                    && record.status == crate::memory::MemoryStatus::Active
                    && record.sensitivity != MemorySensitivity::High
                    && scope_matches(&record.scope, request.workspace_fingerprint.as_deref())
                {
                    historical_candidates.push(record.clone());
                    continue;
                }
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedInvalid,
                    0.0,
                    false,
                    invalid_reason(record, now),
                    &graph,
                    Some(invalid_reason(record, now)),
                ));
                continue;
            }
            if record.sensitivity == MemorySensitivity::High {
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedInvalid,
                    0.0,
                    false,
                    "privacy_policy_excludes_high_sensitivity",
                    &graph,
                    None,
                ));
                continue;
            }
            if !scope_matches(&record.scope, request.workspace_fingerprint.as_deref()) {
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedUnrelated,
                    0.0,
                    false,
                    "workspace_scope_mismatch",
                    &graph,
                    None,
                ));
                continue;
            }

            let key = record_key(record);
            match unique.get_mut(&key) {
                Some(existing) if should_replace(existing, record) => {
                    explanations.push(explanation(
                        existing,
                        MemoryRecallRoute::DroppedDuplicate,
                        0.0,
                        false,
                        "superseded_by_newer_duplicate",
                        &graph,
                        Some("superseded_by_newer_fact"),
                    ));
                    *existing = record.clone();
                }
                Some(_) => explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedDuplicate,
                    0.0,
                    false,
                    "older_duplicate",
                    &graph,
                    Some("superseded_by_newer_fact"),
                )),
                None => {
                    unique.insert(key, record.clone());
                }
            }
        }

        let unique_records = unique.into_values().collect::<Vec<_>>();
        let duplicate_drops = explanations
            .iter()
            .filter(|item| item.route == MemoryRecallRoute::DroppedDuplicate)
            .count();
        let mut boot_candidates = unique_records
            .iter()
            .filter(|record| {
                is_boot_candidate(record) && !(timeline_query && is_graph_memory(record))
            })
            .map(|record| (boot_score(record), record.clone()))
            .collect::<Vec<_>>();
        boot_candidates.sort_by(compare_scored);

        let mut boot_context = MemoryRecallResult::default();
        for (score, record) in boot_candidates {
            let content_chars = record.content.chars().count();
            if boot_context.records.len() >= request.boot_max_records
                || boot_context.budget_used_chars + content_chars > request.boot_budget_chars
            {
                boot_context.truncated = true;
                boot_context.dropped_by_budget += 1;
                explanations.push(explanation(
                    &record,
                    MemoryRecallRoute::DroppedBudget,
                    score,
                    false,
                    "boot_budget_or_record_limit",
                    &graph,
                    None,
                ));
                continue;
            }
            boot_context.budget_used_chars += content_chars;
            if record.kind == MemoryKind::Preference {
                boot_context.always_on_count += 1;
            }
            explanations.push(explanation(
                &record,
                MemoryRecallRoute::Boot,
                score,
                true,
                if is_graph_memory(&record) {
                    "active_relation_edge"
                } else {
                    "stable_boot_context"
                },
                &graph,
                Some("active_at_recall_time"),
            ));
            boot_context.records.push(record);
        }

        let boot_keys = boot_context
            .records
            .iter()
            .map(record_key)
            .collect::<BTreeSet<_>>();
        let mut dynamic_candidates = unique_records
            .iter()
            .filter(|record| !boot_keys.contains(&record_key(record)))
            .cloned()
            .collect::<Vec<_>>();
        dynamic_candidates.extend(historical_candidates);
        let mut dynamic_request = MemoryRecallRequest::new(request.query.clone());
        dynamic_request.workspace_fingerprint = request.workspace_fingerprint.clone();
        dynamic_request.max_records = request.dynamic_max_records;
        dynamic_request.budget_chars = request.dynamic_budget_chars;
        let mut dynamic_recall = if timeline_query {
            timeline_recall(&dynamic_candidates, &dynamic_request)
        } else {
            self.engine
                .recall_prompt_relevant(&dynamic_candidates, &dynamic_request)
        };
        if request.boot_max_records == 0 {
            dynamic_recall.dropped_duplicates += duplicate_drops;
        } else {
            boot_context.dropped_duplicates += duplicate_drops;
        }
        let dynamic_ids = dynamic_recall
            .records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<BTreeSet<_>>();

        for record in &dynamic_candidates {
            let score = if timeline_query && is_graph_memory(record) {
                1.0
            } else {
                score_record(record, &dynamic_request)
            };
            if dynamic_ids.contains(record.id.as_str()) {
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::Dynamic,
                    score,
                    true,
                    if timeline_query {
                        "temporal_event_match"
                    } else if is_graph_memory(record) {
                        "active_relation_edge"
                    } else {
                        "prompt_relevance_match"
                    },
                    &graph,
                    Some(if timeline_query && !record.is_recallable_at(now) {
                        invalid_reason(record, now)
                    } else if timeline_query {
                        "ordered_by_event_observed_updated_created_time"
                    } else {
                        "active_at_recall_time"
                    }),
                ));
            } else if timeline_query || score >= RECALL_RELEVANCE_THRESHOLD {
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedBudget,
                    score,
                    false,
                    "dynamic_budget_or_record_limit",
                    &graph,
                    Some(if record.is_recallable_at(now) {
                        "active_at_recall_time"
                    } else {
                        invalid_reason(record, now)
                    }),
                ));
            } else {
                explanations.push(explanation(
                    record,
                    MemoryRecallRoute::DroppedUnrelated,
                    score,
                    false,
                    "below_dynamic_relevance_threshold",
                    &graph,
                    None,
                ));
            }
        }

        MemoryRecallRouterResult {
            boot_context,
            dynamic_recall,
            explanations,
        }
    }
}

fn is_boot_candidate(record: &MemoryRecord) -> bool {
    record.confidence >= BOOT_MIN_CONFIDENCE
        && record.importance >= BOOT_MIN_IMPORTANCE
        && matches!(
            record.layer,
            MemoryLayer::Profile
                | MemoryLayer::Preference
                | MemoryLayer::Relationship
                | MemoryLayer::Workspace
        )
        && matches!(
            record.kind,
            MemoryKind::Preference
                | MemoryKind::PersonalFact
                | MemoryKind::RelationshipNote
                | MemoryKind::ProjectContext
                | MemoryKind::Correction
        )
}

fn boot_score(record: &MemoryRecord) -> f32 {
    let layer_bonus = match record.layer {
        MemoryLayer::Preference | MemoryLayer::Workspace => 1.0,
        MemoryLayer::Profile | MemoryLayer::Relationship => 0.75,
        _ => 0.0,
    };
    record.importance * 2.0 + record.confidence + layer_bonus
}

fn compare_scored(left: &(f32, MemoryRecord), right: &(f32, MemoryRecord)) -> std::cmp::Ordering {
    right
        .0
        .partial_cmp(&left.0)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| right.1.updated_at_millis.cmp(&left.1.updated_at_millis))
        .then_with(|| left.1.id.cmp(&right.1.id))
}

fn record_key(record: &MemoryRecord) -> String {
    if record.dedup_key.trim().is_empty() {
        dedup_key_for_record(record).as_storage_key()
    } else {
        record.dedup_key.clone()
    }
}

fn should_replace(existing: &MemoryRecord, candidate: &MemoryRecord) -> bool {
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

fn explanation(
    record: &MemoryRecord,
    route: MemoryRecallRoute,
    score: f32,
    selected: bool,
    reason: &str,
    graph: &RelationshipGraphLite,
    temporal_reason: Option<&str>,
) -> MemoryRecallExplanation {
    MemoryRecallExplanation {
        memory_id: record.id.clone(),
        route,
        score,
        selected,
        reason: reason.to_string(),
        source: source_label(record),
        layer: record.layer,
        scope: record.scope.label(),
        kind: record.kind,
        relation: graph.relation_for_memory(&record.id),
        temporal_reason: temporal_reason.map(str::to_string),
    }
}

fn timeline_recall(records: &[MemoryRecord], request: &MemoryRecallRequest) -> MemoryRecallResult {
    let mut candidates = records
        .iter()
        .filter(|record| is_graph_memory(record))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        temporal_ordering_time(right)
            .cmp(&temporal_ordering_time(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut result = MemoryRecallResult::default();
    for record in candidates {
        let chars = record.content.chars().count();
        if result.records.len() >= request.max_records
            || result.budget_used_chars + chars > request.budget_chars
        {
            result.truncated = true;
            result.dropped_by_budget += 1;
            continue;
        }
        result.budget_used_chars += chars;
        result.records.push(record);
    }
    result
}

fn is_graph_memory(record: &MemoryRecord) -> bool {
    matches!(
        record.kind,
        MemoryKind::Preference
            | MemoryKind::Correction
            | MemoryKind::RelationshipNote
            | MemoryKind::EmotionalState
            | MemoryKind::Goal
            | MemoryKind::ProjectContext
            | MemoryKind::Event
    )
}

fn invalid_reason(record: &MemoryRecord, now: u128) -> &'static str {
    if record.invalidation.superseded_by.is_some() {
        "superseded_by_newer_fact"
    } else if record
        .temporal
        .expires_at_millis
        .is_some_and(|expires_at| expires_at <= now)
    {
        "expired_relation_edge"
    } else if record.status == crate::memory::MemoryStatus::Pending
        && !record.invalidation.conflicts_with.is_empty()
    {
        "conflict_pending_confirmation"
    } else {
        "inactive_expired_or_invalidated"
    }
}

fn source_label(record: &MemoryRecord) -> String {
    match record.source.extractor.as_deref() {
        Some("provider") => "provider".to_string(),
        Some("rule") => "rule".to_string(),
        Some(_) => "extractor".to_string(),
        None if record.source_session_id.is_some() || record.source.session_id.is_some() => {
            "session".to_string()
        }
        None => "stored".to_string(),
    }
}
