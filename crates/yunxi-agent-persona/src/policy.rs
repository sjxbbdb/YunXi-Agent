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
