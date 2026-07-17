use crate::dedup::{deduplicate_candidates, status_for_policy};
use crate::extractor::MemoryRuleExtractor;
use crate::memory::{
    MemoryCandidate, MemoryEvidence, MemoryKind, MemoryLayer, MemorySensitivity, MemoryStatus,
};
use crate::policy::{
    MemoryPipelineLayer, MemoryPolicyContext, MemoryPrivacyClassifier, MemoryWritePolicy,
    MemoryWritePolicyEngine,
};
use crate::provider_extractor::ProviderMemoryExtractor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct MemoryPipeline {
    rule_extractor: MemoryRuleExtractor,
    provider_extractor: ProviderMemoryExtractor,
    policy: MemoryWritePolicyEngine,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryPipelineInput {
    pub prompt: String,
    pub assistant_response: Option<String>,
    pub provider_response: Option<String>,
    pub source_session_id: Option<String>,
    pub workspace_fingerprint: Option<String>,
    pub memory_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryPipelineStageResult {
    pub layer: MemoryPipelineLayer,
    pub observed_count: usize,
    pub generated_count: usize,
    pub retained_count: usize,
    pub pending_count: usize,
    pub rejected_count: usize,
    pub merged_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryPipelineDiagnostic {
    pub layer: MemoryPipelineLayer,
    pub candidate_id: Option<String>,
    pub action: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryPipelineOutput {
    pub stages: Vec<MemoryPipelineStageResult>,
    pub candidates: Vec<MemoryCandidate>,
    pub diagnostics: Vec<MemoryPipelineDiagnostic>,
    pub warnings: Vec<String>,
    pub l0_evidence: MemoryEvidence,
}

impl MemoryPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&self, input: MemoryPipelineInput) -> MemoryPipelineOutput {
        let l0_evidence = raw_turn_evidence(&input);
        let explicit = explicitly_stable_memory(&input.prompt);
        let mut warnings = Vec::new();
        let mut candidates = self.rule_extractor.extract(
            &input.prompt,
            input.assistant_response.as_deref(),
            input.source_session_id.as_deref(),
            input.workspace_fingerprint.as_deref(),
            input.memory_enabled,
        );
        if let Some(response) = input.provider_response.as_deref() {
            match self.provider_extractor.extract_from_response_checked(
                response,
                &input.prompt,
                input.source_session_id.as_deref(),
                input.workspace_fingerprint.as_deref(),
                input.memory_enabled,
            ) {
                Ok(mut provider_candidates) => {
                    candidates.append(&mut provider_candidates);
                }
                Err(error) => {
                    warnings.push(error);
                }
            }
        }

        for candidate in &mut candidates {
            candidate.ensure_v3_provenance();
            attach_evidence(candidate, &l0_evidence);
            apply_layer_policy(&self.policy, candidate, explicit, input.memory_enabled);
            redact_secret_candidate(candidate);
        }

        let generated_count = candidates.len();
        let l1_generated_count = candidates
            .iter()
            .filter(|candidate| {
                layer_for_kind(candidate.proposed_record.kind)
                    == MemoryPipelineLayer::L1StructuredFact
            })
            .count();
        let l2_generated_count = generated_count.saturating_sub(l1_generated_count);
        let mut candidates = deduplicate_candidates(candidates);
        let mut promoted_ids = Vec::new();
        for candidate in &mut candidates {
            if qualifies_for_l3(candidate, explicit) {
                candidate.proposed_record.layer = MemoryLayer::Profile;
                let evidence = MemoryEvidence {
                    kind: "l3_profile_summary".to_string(),
                    summary: "Stable low-risk fact promoted from structured memory; not a raw-turn instruction."
                        .to_string(),
                    source_session_id: input.source_session_id.clone(),
                    source_turn_id: None,
                    source_event_id: None,
                };
                attach_evidence(candidate, &evidence);
                candidate.reason = append_reason(&candidate.reason, "pipeline:l3-profile-summary");
                apply_layer_policy(&self.policy, candidate, explicit, input.memory_enabled);
                promoted_ids.push(candidate.proposed_record.id.clone());
            }
        }

        let stages = stage_results(
            &candidates,
            l1_generated_count,
            l2_generated_count,
            &promoted_ids,
        );
        let diagnostics = candidates
            .iter()
            .map(|candidate| MemoryPipelineDiagnostic {
                layer: candidate_layer(candidate, &promoted_ids),
                candidate_id: Some(candidate.proposed_record.id.clone()),
                action: diagnostic_action(candidate).to_string(),
                reason: candidate.reason.clone(),
            })
            .chain(warnings.iter().map(|warning| MemoryPipelineDiagnostic {
                layer: MemoryPipelineLayer::L1StructuredFact,
                candidate_id: None,
                action: "provider_failed_soft".to_string(),
                reason: warning.clone(),
            }))
            .collect();

        MemoryPipelineOutput {
            stages,
            candidates,
            diagnostics,
            warnings,
            l0_evidence,
        }
    }
}

fn raw_turn_evidence(input: &MemoryPipelineInput) -> MemoryEvidence {
    let classifier = MemoryPrivacyClassifier;
    let joined = format!(
        "user: {} | assistant: {}",
        input.prompt.trim(),
        input.assistant_response.as_deref().unwrap_or("").trim()
    );
    let summary = if classifier.contains_secret(&joined) {
        "[redacted: secret-like raw turn omitted]".to_string()
    } else {
        compact(&joined, 240)
    };
    MemoryEvidence {
        kind: "l0_raw_turn_summary".to_string(),
        summary,
        source_session_id: input.source_session_id.clone(),
        source_turn_id: None,
        source_event_id: None,
    }
}

fn attach_evidence(candidate: &mut MemoryCandidate, evidence: &MemoryEvidence) {
    if !candidate.proposed_record.evidence.contains(evidence) {
        candidate.proposed_record.evidence.push(evidence.clone());
    }
}

fn apply_layer_policy(
    policy: &MemoryWritePolicyEngine,
    candidate: &mut MemoryCandidate,
    explicitly_remembered: bool,
    memory_enabled: bool,
) {
    let layer = if candidate.reason.contains("pipeline:l3-profile-summary") {
        MemoryPipelineLayer::L3ProfileSummary
    } else {
        layer_for_kind(candidate.proposed_record.kind)
    };
    candidate.write_policy = policy.policy_for_context(MemoryPolicyContext {
        kind: candidate.proposed_record.kind,
        sensitivity: candidate.proposed_record.sensitivity,
        layer,
        confidence: candidate.proposed_record.confidence,
        importance: candidate.proposed_record.importance,
        content: &candidate.proposed_record.content,
        source_is_clear: candidate.proposed_record.source.extractor.is_some()
            && candidate.proposed_record.source.session_id.is_some(),
        explicitly_remembered,
        memory_enabled,
    });
    candidate.proposed_record.status = status_for_policy(candidate.write_policy);
}

fn redact_secret_candidate(candidate: &mut MemoryCandidate) {
    if candidate.write_policy != MemoryWritePolicy::Discard {
        return;
    }
    candidate.proposed_record.content = "[redacted: secret-like memory discarded]".to_string();
    candidate.evidence = "[redacted: secret-like evidence omitted]".to_string();
    candidate
        .proposed_record
        .evidence
        .retain(|evidence| !MemoryPrivacyClassifier.contains_secret(&evidence.summary));
    candidate.proposed_record.dedup_key.clear();
    candidate.proposed_record.ensure_dedup_metadata();
}

fn qualifies_for_l3(candidate: &MemoryCandidate, explicitly_remembered: bool) -> bool {
    explicitly_remembered
        && candidate.proposed_record.kind == MemoryKind::Preference
        && candidate.proposed_record.sensitivity == MemorySensitivity::Low
        && candidate.proposed_record.confidence >= 0.8
        && candidate.proposed_record.importance >= 0.5
        && candidate.proposed_record.source.extractor.is_some()
        && candidate.write_policy == MemoryWritePolicy::Auto
}

fn stage_results(
    candidates: &[MemoryCandidate],
    l1_generated_count: usize,
    l2_generated_count: usize,
    promoted_ids: &[String],
) -> Vec<MemoryPipelineStageResult> {
    let l1 = candidates
        .iter()
        .filter(|candidate| {
            candidate_layer(candidate, promoted_ids) == MemoryPipelineLayer::L1StructuredFact
        })
        .collect::<Vec<_>>();
    let l2 = candidates
        .iter()
        .filter(|candidate| {
            candidate_layer(candidate, promoted_ids) == MemoryPipelineLayer::L2RelationshipEvent
        })
        .collect::<Vec<_>>();
    let l3 = candidates
        .iter()
        .filter(|candidate| {
            candidate_layer(candidate, promoted_ids) == MemoryPipelineLayer::L3ProfileSummary
        })
        .collect::<Vec<_>>();
    vec![
        MemoryPipelineStageResult {
            layer: MemoryPipelineLayer::L0RawTurn,
            observed_count: 1,
            generated_count: 0,
            retained_count: 0,
            pending_count: 0,
            rejected_count: 0,
            merged_count: 0,
        },
        counted_stage(
            MemoryPipelineLayer::L1StructuredFact,
            l1_generated_count,
            &l1,
            l1_generated_count.saturating_sub(l1.len().saturating_add(l3.len())),
        ),
        counted_stage(
            MemoryPipelineLayer::L2RelationshipEvent,
            l2_generated_count,
            &l2,
            l2_generated_count.saturating_sub(l2.len()),
        ),
        counted_stage(MemoryPipelineLayer::L3ProfileSummary, l3.len(), &l3, 0),
    ]
}

fn counted_stage(
    layer: MemoryPipelineLayer,
    generated_count: usize,
    candidates: &[&MemoryCandidate],
    merged_count: usize,
) -> MemoryPipelineStageResult {
    MemoryPipelineStageResult {
        layer,
        observed_count: generated_count,
        generated_count,
        retained_count: candidates.len(),
        pending_count: candidates
            .iter()
            .filter(|candidate| candidate.proposed_record.status == MemoryStatus::Pending)
            .count(),
        rejected_count: candidates
            .iter()
            .filter(|candidate| candidate.proposed_record.status == MemoryStatus::Rejected)
            .count(),
        merged_count,
    }
}

fn candidate_layer(candidate: &MemoryCandidate, promoted_ids: &[String]) -> MemoryPipelineLayer {
    if promoted_ids.contains(&candidate.proposed_record.id) {
        MemoryPipelineLayer::L3ProfileSummary
    } else {
        layer_for_kind(candidate.proposed_record.kind)
    }
}

fn layer_for_kind(kind: MemoryKind) -> MemoryPipelineLayer {
    match kind {
        MemoryKind::RelationshipNote | MemoryKind::EmotionalState | MemoryKind::Event => {
            MemoryPipelineLayer::L2RelationshipEvent
        }
        _ => MemoryPipelineLayer::L1StructuredFact,
    }
}

fn diagnostic_action(candidate: &MemoryCandidate) -> &'static str {
    match candidate.write_policy {
        MemoryWritePolicy::Auto => "auto_saved",
        MemoryWritePolicy::RequireConfirmation => "pending",
        MemoryWritePolicy::Discard => "discarded",
        MemoryWritePolicy::Disabled => "disabled",
    }
}

fn explicitly_stable_memory(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    prompt.contains("以后")
        || prompt.contains("记住")
        || lower.contains("from now on")
        || lower.contains("remember that")
        || lower.contains("always ")
        || lower.contains("by default")
}

fn append_reason(existing: &str, marker: &str) -> String {
    if existing.contains(marker) {
        existing.to_string()
    } else {
        compact(&format!("{existing} | {marker}"), 180)
    }
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
