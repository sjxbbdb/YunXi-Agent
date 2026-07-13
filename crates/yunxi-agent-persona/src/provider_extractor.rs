use crate::memory::{
    MemoryCandidate, MemoryKind, MemoryRecord, MemorySensitivity, MemoryStatus, now_millis,
};
use crate::policy::{MemoryWritePolicy, MemoryWritePolicyEngine};
use crate::scope::MemoryScopeRouter;
use serde::Deserialize;

#[derive(Clone, Debug, Default)]
pub struct ProviderMemoryExtractor {
    policy: MemoryWritePolicyEngine,
    scope_router: MemoryScopeRouter,
}

impl ProviderMemoryExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract_from_response(
        &self,
        response: &str,
        evidence: &str,
        source_session_id: Option<&str>,
        workspace_fingerprint: Option<&str>,
        memory_enabled: bool,
    ) -> Vec<MemoryCandidate> {
        let Some(payload) = parse_payload(response) else {
            return Vec::new();
        };
        let now = now_millis();
        payload
            .candidates
            .into_iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                let kind = parse_kind(&candidate.kind)?;
                let content = candidate.content.trim();
                if content.is_empty() {
                    return None;
                }
                let reason = candidate
                    .reason
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "provider:structured-memory-extraction".to_string());
                let classifier_sensitivity = self.policy.classify_content(content);
                let sensitivity = max_sensitivity(
                    classifier_sensitivity,
                    candidate
                        .sensitivity_hint
                        .as_deref()
                        .and_then(parse_sensitivity)
                        .unwrap_or(MemorySensitivity::Low),
                );
                let write_policy =
                    self.policy
                        .policy_for(kind, sensitivity, content, memory_enabled);
                let status = match write_policy {
                    MemoryWritePolicy::Auto => MemoryStatus::Active,
                    MemoryWritePolicy::RequireConfirmation => MemoryStatus::Pending,
                    MemoryWritePolicy::Discard | MemoryWritePolicy::Disabled => {
                        MemoryStatus::Rejected
                    }
                };
                let mut record = MemoryRecord::new(
                    format!("mem-{now}-provider-{index}"),
                    self.scope_router.route(
                        kind,
                        content,
                        &reason,
                        workspace_fingerprint,
                        candidate.scope_hint.as_deref(),
                    ),
                    kind,
                    content.to_string(),
                    now,
                )
                .with_scores(
                    candidate.confidence.unwrap_or(0.72),
                    candidate.importance.unwrap_or(0.55),
                )
                .with_sensitivity(sensitivity)
                .with_status(status);
                if let Some(source_session_id) = source_session_id {
                    record = record.with_source_session_id(source_session_id);
                }
                Some(MemoryCandidate {
                    proposed_record: record,
                    evidence: evidence.to_string(),
                    write_policy,
                    reason,
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct ProviderExtractionPayload {
    #[serde(default)]
    candidates: Vec<ProviderExtractionCandidate>,
}

#[derive(Clone, Debug, Deserialize)]
struct ProviderExtractionCandidate {
    kind: String,
    content: String,
    #[serde(default)]
    scope_hint: Option<String>,
    #[serde(default)]
    sensitivity_hint: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    importance: Option<f32>,
    #[serde(default)]
    reason: Option<String>,
}

fn parse_payload(response: &str) -> Option<ProviderExtractionPayload> {
    serde_json::from_str(response).ok().or_else(|| {
        extract_json_object(response).and_then(|value| serde_json::from_str(value).ok())
    })
}

fn extract_json_object(response: &str) -> Option<&str> {
    let trimmed = response.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end > start).then_some(&trimmed[start..=end])
}

fn parse_kind(value: &str) -> Option<MemoryKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "preference" => Some(MemoryKind::Preference),
        "personal_fact" | "personalfact" => Some(MemoryKind::PersonalFact),
        "relationship_note" | "relationshipnote" => Some(MemoryKind::RelationshipNote),
        "emotional_state" | "emotionalstate" => Some(MemoryKind::EmotionalState),
        "goal" => Some(MemoryKind::Goal),
        "project_context" | "projectcontext" => Some(MemoryKind::ProjectContext),
        "correction" => Some(MemoryKind::Correction),
        "event" => Some(MemoryKind::Event),
        "tool_trace_summary" | "tooltracesummary" => Some(MemoryKind::ToolTraceSummary),
        _ => None,
    }
}

fn parse_sensitivity(value: &str) -> Option<MemorySensitivity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Some(MemorySensitivity::Low),
        "medium" => Some(MemorySensitivity::Medium),
        "high" => Some(MemorySensitivity::High),
        _ => None,
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
