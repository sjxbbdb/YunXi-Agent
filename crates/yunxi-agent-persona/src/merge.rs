use crate::dedup::{max_policy, status_for_policy};
use crate::memory::{
    MemoryCandidate, MemoryInvalidation, MemoryKind, MemoryRecord, MemorySensitivity, MemorySource,
    MemoryStatus, MemoryTemporal,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryMergeStrategy {
    PreserveExisting,
    PromoteIncoming,
    CombineNonConflicting,
    ConflictRequiresConfirmation,
}

impl MemoryMergeStrategy {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::PreserveExisting => "preserve_existing",
            Self::PromoteIncoming => "promote_incoming",
            Self::CombineNonConflicting => "combine_non_conflicting",
            Self::ConflictRequiresConfirmation => "conflict_requires_confirmation",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryMergeResult {
    pub record: MemoryRecord,
    pub strategy: MemoryMergeStrategy,
    pub summary: String,
}

pub fn merge_equivalent_memory_records(
    existing: &MemoryRecord,
    incoming: &MemoryRecord,
    now_millis: u128,
) -> MemoryMergeResult {
    let decision = content_merge_decision(existing, incoming);
    let mut merged = match decision.strategy {
        MemoryMergeStrategy::PromoteIncoming => incoming.clone(),
        _ => existing.clone(),
    };

    merged.content = decision.content;
    merged.confidence = existing.confidence.max(incoming.confidence);
    merged.importance = existing.importance.max(incoming.importance);
    merged.sensitivity = max_sensitivity(existing.sensitivity, incoming.sensitivity);
    merged.status = merged_status(existing.status, incoming.status, merged.sensitivity);
    merged.source_session_id = merge_source_session_id(decision.strategy, existing, incoming);
    merged.entities = merge_unique(&existing.entities, &incoming.entities);
    merged.evidence = merge_unique(&existing.evidence, &incoming.evidence);
    merged.source = merge_source(decision.strategy, existing, incoming);
    merged.temporal = merge_temporal(existing, incoming);
    merged.invalidation = merge_invalidation(existing, incoming);
    merged.created_at_millis = existing.created_at_millis.min(incoming.created_at_millis);
    merged.updated_at_millis = now_millis;
    merged.revision = existing
        .revision
        .max(incoming.revision)
        .saturating_add(1)
        .max(2);
    merged.merged_count = existing
        .merged_count
        .saturating_add(incoming.merged_count)
        .max(2);
    merged.ensure_dedup_metadata();

    MemoryMergeResult {
        record: merged,
        strategy: decision.strategy,
        summary: decision.summary,
    }
}

pub fn merge_memory_candidates(existing: &mut MemoryCandidate, incoming: MemoryCandidate) {
    let candidate_revision = existing
        .proposed_record
        .revision
        .max(incoming.proposed_record.revision);
    let candidate_merged_count = existing
        .proposed_record
        .merged_count
        .max(incoming.proposed_record.merged_count);
    let now = existing
        .proposed_record
        .updated_at_millis
        .max(incoming.proposed_record.updated_at_millis);
    let result =
        merge_equivalent_memory_records(&existing.proposed_record, &incoming.proposed_record, now);
    existing.proposed_record = result.record;
    // Candidate dedup happens before durable persistence, so it must not
    // consume a storage revision or inflate the persisted merge counter.
    existing.proposed_record.revision = candidate_revision.max(1);
    existing.proposed_record.merged_count = candidate_merged_count.max(1);
    existing.write_policy = max_policy(existing.write_policy, incoming.write_policy);
    existing.proposed_record.status = status_for_policy(existing.write_policy);
    existing.evidence = merge_short(&existing.evidence, &incoming.evidence, 240);
    existing.reason = merge_short(&existing.reason, &incoming.reason, 180);
}

pub fn memory_conflict_family(record: &MemoryRecord) -> Option<String> {
    if record.kind != MemoryKind::Preference {
        return None;
    }
    let key = if record.dedup_key.trim().is_empty() {
        crate::dedup::dedup_key_for_record(record).as_storage_key()
    } else {
        record.dedup_key.clone()
    };
    if key.ends_with("|language:zh") || key.ends_with("|language:en") {
        return Some(format!("{}|preference|language", record.scope.label()));
    }
    None
}

struct ContentDecision {
    content: String,
    strategy: MemoryMergeStrategy,
    summary: String,
}

fn content_merge_decision(existing: &MemoryRecord, incoming: &MemoryRecord) -> ContentDecision {
    let existing_content = existing.content.trim();
    let incoming_content = incoming.content.trim();
    if normalized_for_compare(existing_content) == normalized_for_compare(incoming_content) {
        return ContentDecision {
            content: existing.content.clone(),
            strategy: MemoryMergeStrategy::PreserveExisting,
            summary: "equivalent content; preserved existing".to_string(),
        };
    }

    let existing_generic = is_generic_language_preference(existing);
    let incoming_generic = is_generic_language_preference(incoming);
    if existing_generic && !incoming_generic {
        return ContentDecision {
            content: incoming.content.clone(),
            strategy: MemoryMergeStrategy::PromoteIncoming,
            summary: "incoming has richer language preference details".to_string(),
        };
    }
    if !existing_generic && incoming_generic {
        return ContentDecision {
            content: existing.content.clone(),
            strategy: MemoryMergeStrategy::PreserveExisting,
            summary: "incoming is a generic duplicate; preserved richer existing".to_string(),
        };
    }

    let existing_detail = detail_keyword_count(existing_content);
    let incoming_detail = detail_keyword_count(incoming_content);
    if existing_detail > 0
        && incoming_detail > 0
        && !content_contains(existing_content, incoming_content)
    {
        let combined = combine_contents(existing_content, incoming_content);
        return ContentDecision {
            content: combined,
            strategy: MemoryMergeStrategy::CombineNonConflicting,
            summary: "combined non-conflicting memory details".to_string(),
        };
    }

    let existing_score = information_score(existing_content);
    let incoming_score = information_score(incoming_content);
    if incoming_score > existing_score + 16 {
        return ContentDecision {
            content: incoming.content.clone(),
            strategy: MemoryMergeStrategy::PromoteIncoming,
            summary: "incoming has higher information score".to_string(),
        };
    }

    ContentDecision {
        content: existing.content.clone(),
        strategy: MemoryMergeStrategy::PreserveExisting,
        summary: "existing content is at least as informative".to_string(),
    }
}

fn is_generic_language_preference(record: &MemoryRecord) -> bool {
    memory_conflict_family(record).is_some() && detail_keyword_count(&record.content) == 0
}

fn detail_keyword_count(content: &str) -> usize {
    [
        "简洁",
        "关键细节",
        "保留细节",
        "保留关键",
        "不要太长",
        "别太长",
        "详细",
        "分点",
        "步骤",
        "示例",
        "准确",
        "保留上下文",
    ]
    .into_iter()
    .filter(|keyword| content.contains(keyword))
    .count()
}

fn information_score(content: &str) -> usize {
    let cjk = content.chars().filter(|ch| is_cjk(*ch)).count();
    let ascii_words = content
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .count();
    cjk + ascii_words.saturating_mul(3) + detail_keyword_count(content).saturating_mul(40)
}

fn content_contains(left: &str, right: &str) -> bool {
    let left = normalized_for_compare(left);
    let right = normalized_for_compare(right);
    left.contains(&right) || right.contains(&left)
}

fn combine_contents(existing: &str, incoming: &str) -> String {
    let incoming = trim_memory_sentence_end(incoming);
    let combined = format!(
        "{}；新补充：{}。",
        trim_memory_sentence_end(existing),
        incoming
    );
    compact(&combined, 260)
}

fn trim_memory_sentence_end(value: &str) -> &str {
    value.trim_end_matches(|ch| matches!(ch, '。' | '.' | '；' | ';'))
}

fn merge_source_session_id(
    strategy: MemoryMergeStrategy,
    existing: &MemoryRecord,
    incoming: &MemoryRecord,
) -> Option<String> {
    match strategy {
        MemoryMergeStrategy::PromoteIncoming => incoming
            .source_session_id
            .clone()
            .or_else(|| existing.source_session_id.clone()),
        MemoryMergeStrategy::PreserveExisting
        | MemoryMergeStrategy::CombineNonConflicting
        | MemoryMergeStrategy::ConflictRequiresConfirmation => existing
            .source_session_id
            .clone()
            .or_else(|| incoming.source_session_id.clone()),
    }
}

fn merge_source(
    strategy: MemoryMergeStrategy,
    existing: &MemoryRecord,
    incoming: &MemoryRecord,
) -> MemorySource {
    let (primary, secondary) = match strategy {
        MemoryMergeStrategy::PromoteIncoming => (&incoming.source, &existing.source),
        MemoryMergeStrategy::PreserveExisting
        | MemoryMergeStrategy::CombineNonConflicting
        | MemoryMergeStrategy::ConflictRequiresConfirmation => (&existing.source, &incoming.source),
    };
    let mut merged = primary.clone();
    if merged.extractor.is_none() {
        merged.extractor.clone_from(&secondary.extractor);
    }
    if merged.session_id.is_none() {
        merged.session_id.clone_from(&secondary.session_id);
    }
    if merged.workspace_fingerprint.is_none() {
        merged
            .workspace_fingerprint
            .clone_from(&secondary.workspace_fingerprint);
    }
    if merged.provider.is_none() {
        merged.provider.clone_from(&secondary.provider);
    }
    if merged.rule_id.is_none() {
        merged.rule_id.clone_from(&secondary.rule_id);
    }
    merged.attributions = merge_unique(&primary.attributions, &secondary.attributions);
    for attribution in [
        primary.primary_attribution(),
        secondary.primary_attribution(),
    ] {
        if !merged.attributions.contains(&attribution)
            && (attribution.extractor.is_some()
                || attribution.session_id.is_some()
                || attribution.workspace_fingerprint.is_some()
                || attribution.provider.is_some()
                || attribution.rule_id.is_some())
        {
            merged.attributions.push(attribution);
        }
    }
    merged.ensure_primary_attribution();
    merged
}

fn merge_temporal(existing: &MemoryRecord, incoming: &MemoryRecord) -> MemoryTemporal {
    MemoryTemporal {
        observed_at_millis: existing
            .temporal
            .observed_at_millis
            .min(incoming.temporal.observed_at_millis),
        event_at_millis: existing
            .temporal
            .event_at_millis
            .or(incoming.temporal.event_at_millis),
        valid_from_millis: min_option(
            existing.temporal.valid_from_millis,
            incoming.temporal.valid_from_millis,
        ),
        expires_at_millis: min_option(
            existing.temporal.expires_at_millis,
            incoming.temporal.expires_at_millis,
        ),
    }
}

fn merge_invalidation(existing: &MemoryRecord, incoming: &MemoryRecord) -> MemoryInvalidation {
    MemoryInvalidation {
        supersedes: merge_unique(
            &existing.invalidation.supersedes,
            &incoming.invalidation.supersedes,
        ),
        superseded_by: existing
            .invalidation
            .superseded_by
            .clone()
            .or_else(|| incoming.invalidation.superseded_by.clone()),
        conflicts_with: merge_unique(
            &existing.invalidation.conflicts_with,
            &incoming.invalidation.conflicts_with,
        ),
        expires_reason: existing
            .invalidation
            .expires_reason
            .clone()
            .or_else(|| incoming.invalidation.expires_reason.clone()),
        invalidated_at_millis: min_option(
            existing.invalidation.invalidated_at_millis,
            incoming.invalidation.invalidated_at_millis,
        ),
    }
}

fn merge_unique<T: Clone + PartialEq>(left: &[T], right: &[T]) -> Vec<T> {
    let mut merged = left.to_vec();
    for item in right {
        if !merged.contains(item) {
            merged.push(item.clone());
        }
    }
    merged
}

fn min_option<T: Ord + Copy>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn merged_status(
    existing: MemoryStatus,
    incoming: MemoryStatus,
    sensitivity: MemorySensitivity,
) -> MemoryStatus {
    if existing == MemoryStatus::Pending || incoming == MemoryStatus::Pending {
        return MemoryStatus::Pending;
    }
    if matches!(
        sensitivity,
        MemorySensitivity::Medium | MemorySensitivity::High
    ) {
        return MemoryStatus::Pending;
    }
    if existing == MemoryStatus::Active || incoming == MemoryStatus::Active {
        MemoryStatus::Active
    } else {
        incoming
    }
}

fn max_sensitivity(left: MemorySensitivity, right: MemorySensitivity) -> MemorySensitivity {
    match (left, right) {
        (MemorySensitivity::High, _) | (_, MemorySensitivity::High) => MemorySensitivity::High,
        (MemorySensitivity::Medium, _) | (_, MemorySensitivity::Medium) => {
            MemorySensitivity::Medium
        }
        _ => MemorySensitivity::Low,
    }
}

fn merge_short(left: &str, right: &str, max_chars: usize) -> String {
    if left == right || right.trim().is_empty() {
        return compact(left, max_chars);
    }
    if left.trim().is_empty() {
        return compact(right, max_chars);
    }
    compact(&format!("{left} | {right}"), max_chars)
}

fn compact(value: &str, max_chars: usize) -> String {
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

fn normalized_for_compare(content: &str) -> String {
    let mut out = String::new();
    for ch in content.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if is_cjk(ch) {
            out.push(ch);
        }
    }
    out
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
        || ('\u{3400}'..='\u{4dbf}').contains(&ch)
        || ('\u{f900}'..='\u{faff}').contains(&ch)
}
