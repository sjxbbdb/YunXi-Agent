use crate::memory::{MemoryKind, MemorySensitivity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWritePolicy {
    Auto,
    RequireConfirmation,
    Discard,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPipelineLayer {
    L0RawTurn,
    L1StructuredFact,
    L2RelationshipEvent,
    L3ProfileSummary,
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryPolicyContext<'a> {
    pub kind: MemoryKind,
    pub sensitivity: MemorySensitivity,
    pub layer: MemoryPipelineLayer,
    pub confidence: f32,
    pub importance: f32,
    pub content: &'a str,
    pub source_is_clear: bool,
    pub explicitly_remembered: bool,
    pub memory_enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryPrivacyClassifier;

impl MemoryPrivacyClassifier {
    pub fn classify(&self, content: &str) -> MemorySensitivity {
        let lowered = content.to_ascii_lowercase();
        if contains_secret_marker(&lowered) {
            return MemorySensitivity::High;
        }
        if contains_sensitive_profile_marker(content, &lowered) {
            return MemorySensitivity::Medium;
        }
        MemorySensitivity::Low
    }

    pub fn contains_secret(&self, content: &str) -> bool {
        contains_secret_marker(&content.to_ascii_lowercase())
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryWritePolicyEngine {
    classifier: MemoryPrivacyClassifier,
}

impl MemoryWritePolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn policy_for(
        &self,
        kind: MemoryKind,
        sensitivity: MemorySensitivity,
        content: &str,
        memory_enabled: bool,
    ) -> MemoryWritePolicy {
        if !memory_enabled {
            return MemoryWritePolicy::Disabled;
        }
        if self.classifier.contains_secret(content) {
            return MemoryWritePolicy::Discard;
        }
        match sensitivity {
            MemorySensitivity::High => MemoryWritePolicy::RequireConfirmation,
            MemorySensitivity::Medium => MemoryWritePolicy::RequireConfirmation,
            MemorySensitivity::Low => match kind {
                MemoryKind::Preference | MemoryKind::Correction | MemoryKind::ProjectContext => {
                    MemoryWritePolicy::Auto
                }
                MemoryKind::ToolTraceSummary => MemoryWritePolicy::RequireConfirmation,
                MemoryKind::PersonalFact
                | MemoryKind::RelationshipNote
                | MemoryKind::EmotionalState
                | MemoryKind::Goal
                | MemoryKind::Event => MemoryWritePolicy::RequireConfirmation,
            },
        }
    }

    pub fn classify_content(&self, content: &str) -> MemorySensitivity {
        self.classifier.classify(content)
    }

    pub fn policy_for_context(&self, context: MemoryPolicyContext<'_>) -> MemoryWritePolicy {
        let baseline = self.policy_for(
            context.kind,
            context.sensitivity,
            context.content,
            context.memory_enabled,
        );
        if matches!(
            baseline,
            MemoryWritePolicy::Disabled | MemoryWritePolicy::Discard
        ) {
            return baseline;
        }
        if context.sensitivity != MemorySensitivity::Low {
            return MemoryWritePolicy::RequireConfirmation;
        }
        match context.layer {
            MemoryPipelineLayer::L0RawTurn => MemoryWritePolicy::Discard,
            MemoryPipelineLayer::L2RelationshipEvent => MemoryWritePolicy::RequireConfirmation,
            MemoryPipelineLayer::L3ProfileSummary => {
                if context.source_is_clear
                    && context.explicitly_remembered
                    && context.confidence >= 0.8
                    && context.importance >= 0.5
                {
                    MemoryWritePolicy::Auto
                } else {
                    MemoryWritePolicy::RequireConfirmation
                }
            }
            MemoryPipelineLayer::L1StructuredFact => baseline,
        }
    }
}

fn contains_secret_marker(lowered: &str) -> bool {
    lowered.contains("api key")
        || lowered.contains("apikey")
        || lowered.contains("authorization:")
        || lowered.contains("bearer ")
        || lowered.contains("password")
        || lowered.contains("token")
        || lowered.contains("secret")
        || lowered.contains("sk-")
        || lowered.contains("github_pat_")
        || lowered.contains("ghp_")
}

fn contains_sensitive_profile_marker(content: &str, lowered: &str) -> bool {
    lowered.contains("health")
        || lowered.contains("medical")
        || lowered.contains("finance")
        || lowered.contains("bank")
        || lowered.contains("emotion")
        || lowered.contains("relationship")
        || content.contains("情绪")
        || content.contains("关系")
        || content.contains("健康")
        || content.contains("财务")
        || content.contains("身份证")
        || content.contains("银行卡")
}
