use yunxi_agent_persona::{
    MemoryKind, MemoryPrivacyClassifier, MemoryRecallEngine, MemoryRecallRequest, MemoryRecord,
    MemoryScope, MemorySensitivity, MemoryStatus, MemoryWritePolicy, MemoryWritePolicyEngine,
};

#[test]
fn privacy_classifier_treats_tokens_as_high_sensitivity() {
    let classifier = MemoryPrivacyClassifier;

    assert_eq!(
        classifier.classify("Authorization: Bearer <redacted>"),
        MemorySensitivity::High
    );
    assert!(classifier.contains_secret("github_pat_ marker"));
}

#[test]
fn write_policy_auto_saves_low_risk_preferences_but_not_secrets() {
    let engine = MemoryWritePolicyEngine::new();

    assert_eq!(
        engine.policy_for(
            MemoryKind::Preference,
            MemorySensitivity::Low,
            "用户偏好中文回答",
            true,
        ),
        MemoryWritePolicy::Auto
    );
    assert_eq!(
        engine.policy_for(
            MemoryKind::Preference,
            MemorySensitivity::High,
            "api key <redacted>",
            true,
        ),
        MemoryWritePolicy::Discard
    );
    assert_eq!(
        engine.policy_for(
            MemoryKind::Preference,
            MemorySensitivity::Low,
            "用户偏好中文回答",
            false,
        ),
        MemoryWritePolicy::Disabled
    );
}

#[test]
fn recall_filters_pending_records_and_respects_workspace_scope() {
    let active = MemoryRecord::new(
        "active",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-a".to_string(),
        },
        MemoryKind::ProjectContext,
        "项目要求使用 REST API 发布",
        1,
    )
    .with_status(MemoryStatus::Active);
    let pending = MemoryRecord::new(
        "pending",
        MemoryScope::GlobalUser,
        MemoryKind::PersonalFact,
        "用户个人事实",
        2,
    );
    let other_workspace = MemoryRecord::new(
        "other",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-b".to_string(),
        },
        MemoryKind::ProjectContext,
        "其他项目上下文",
        3,
    )
    .with_status(MemoryStatus::Active);

    let request = MemoryRecallRequest {
        query: "REST API".to_string(),
        workspace_fingerprint: Some("workspace-a".to_string()),
        max_records: 8,
        budget_chars: 1000,
    };
    let result = MemoryRecallEngine.recall(&[active, pending, other_workspace], &request);

    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].id, "active");
}
