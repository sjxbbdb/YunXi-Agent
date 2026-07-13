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

#[test]
fn recall_keeps_global_language_preference_but_drops_unrelated_workspace_memory() {
    let language = MemoryRecord::new(
        "language",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let unrelated_project = MemoryRecord::new(
        "project",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-a".to_string(),
        },
        MemoryKind::ProjectContext,
        "项目发布必须使用 GitHub API。",
        2,
    )
    .with_status(MemoryStatus::Active);

    let request = MemoryRecallRequest {
        query: "请计算 2+2".to_string(),
        workspace_fingerprint: Some("workspace-a".to_string()),
        max_records: 8,
        budget_chars: 1000,
    };
    let result = MemoryRecallEngine.recall(&[language, unrelated_project], &request);

    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].id, "language");
    assert_eq!(result.always_on_count, 1);
    assert_eq!(result.dropped_unrelated, 1);
}

#[test]
fn recall_empty_query_does_not_return_all_active_records() {
    let language = MemoryRecord::new(
        "language",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let unrelated = MemoryRecord::new(
        "unrelated",
        MemoryScope::GlobalUser,
        MemoryKind::Event,
        "用户曾经测试过一个普通事件。",
        2,
    )
    .with_status(MemoryStatus::Active);

    let request = MemoryRecallRequest {
        query: String::new(),
        workspace_fingerprint: Some("workspace-a".to_string()),
        max_records: 8,
        budget_chars: 1000,
    };
    let result = MemoryRecallEngine.recall(&[language, unrelated], &request);

    assert_eq!(
        result
            .records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>(),
        vec!["language"]
    );
    assert_eq!(result.dropped_unrelated, 1);
}

#[test]
fn recall_reports_budget_truncation() {
    let first = MemoryRecord::new(
        "language-1",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let second = MemoryRecord::new(
        "language-2",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "用户偏好回答保持简洁。",
        2,
    )
    .with_status(MemoryStatus::Active);

    let request = MemoryRecallRequest {
        query: "语言偏好".to_string(),
        workspace_fingerprint: Some("workspace-a".to_string()),
        max_records: 1,
        budget_chars: 1000,
    };
    let result = MemoryRecallEngine.recall(&[first, second], &request);

    assert_eq!(result.records.len(), 1);
    assert!(result.truncated);
    assert_eq!(result.dropped_by_budget, 1);
}
