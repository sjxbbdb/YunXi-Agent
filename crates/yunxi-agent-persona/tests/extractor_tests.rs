use yunxi_agent_persona::{
    MemoryCandidate, MemoryKind, MemoryRecord, MemoryRuleExtractor, MemoryScope, MemorySensitivity,
    MemoryStatus, MemoryWritePolicy, ProviderMemoryExtractor, deduplicate_candidates,
};

#[test]
fn rule_extractor_routes_language_preference_to_global_user_scope() {
    let candidates = MemoryRuleExtractor::new().extract(
        "以后请用中文回答",
        None,
        Some("session-1"),
        Some("workspace-a"),
        true,
    );

    let preference = candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::Preference)
        .expect("language preference candidate");

    assert_eq!(preference.proposed_record.scope, MemoryScope::GlobalUser);
    assert_eq!(preference.proposed_record.status, MemoryStatus::Active);
}

#[test]
fn rule_extractor_routes_project_hard_constraints_to_workspace_scope() {
    let candidates = MemoryRuleExtractor::new().extract(
        "当前项目的硬性要求是推送必须走 GitHub API",
        None,
        Some("session-1"),
        Some("workspace-a"),
        true,
    );

    let project = candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::ProjectContext)
        .expect("project context candidate");

    assert_eq!(
        project.proposed_record.scope,
        MemoryScope::Workspace {
            root_fingerprint: "workspace-a".to_string()
        }
    );
    assert_eq!(project.proposed_record.status, MemoryStatus::Active);
}

#[test]
fn provider_extractor_preserves_policy_and_overrides_wrong_scope_hints() {
    let response = r#"{
        "candidates": [
            {
                "kind": "preference",
                "content": "用户偏好后续默认使用中文交流。",
                "scope_hint": "workspace",
                "sensitivity_hint": "low",
                "confidence": 0.9,
                "importance": 0.7,
                "reason": "provider:language-preference"
            },
            {
                "kind": "personal_fact",
                "content": "用户的 api key 是 sk-test-secret",
                "scope_hint": "global_user",
                "sensitivity_hint": "low",
                "reason": "provider:secret"
            }
        ]
    }"#;

    let candidates = ProviderMemoryExtractor::new().extract_from_response(
        response,
        "evidence",
        Some("session-1"),
        Some("workspace-a"),
        true,
    );

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].proposed_record.scope, MemoryScope::GlobalUser);
    assert_eq!(candidates[0].write_policy, MemoryWritePolicy::Auto);
    assert_eq!(candidates[1].write_policy, MemoryWritePolicy::Discard);
    assert_eq!(candidates[1].proposed_record.status, MemoryStatus::Rejected);
}

#[test]
fn rule_extractor_deduplicates_equivalent_chinese_language_preferences() {
    let candidates = MemoryRuleExtractor::new().extract(
        "以后请用中文回答",
        None,
        Some("session-1"),
        Some("workspace-a"),
        true,
    );

    let language_preferences = candidates
        .iter()
        .filter(|candidate| candidate.proposed_record.kind == MemoryKind::Preference)
        .collect::<Vec<_>>();

    assert_eq!(language_preferences.len(), 1);
    assert_eq!(
        language_preferences[0].proposed_record.dedup_key,
        "global_user|preference|language:zh"
    );
}

#[test]
fn unified_candidate_dedup_merges_provider_and_rule_language_preferences() {
    let now = 10;
    let rule = MemoryCandidate {
        proposed_record: MemoryRecord::new(
            "rule",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "用户偏好使用中文回答。",
            now,
        )
        .with_status(MemoryStatus::Active),
        evidence: "用中文回答".to_string(),
        write_policy: MemoryWritePolicy::Auto,
        reason: "rule:language-preference-direct".to_string(),
    };
    let provider = MemoryCandidate {
        proposed_record: MemoryRecord::new(
            "provider",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "用户偏好默认中文交流。",
            now + 1,
        )
        .with_scores(0.95, 0.9)
        .with_status(MemoryStatus::Active),
        evidence: "默认中文交流".to_string(),
        write_policy: MemoryWritePolicy::Auto,
        reason: "provider:language-preference".to_string(),
    };

    let candidates = deduplicate_candidates(vec![rule, provider]);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].proposed_record.confidence, 0.95);
    assert!(
        candidates[0]
            .reason
            .contains("provider:language-preference")
    );
}

#[test]
fn dedup_keeps_project_constraints_separate_from_language_preferences() {
    let candidates = MemoryRuleExtractor::new().extract(
        "以后请用中文回答，当前项目硬性要求是推送必须走 GitHub API",
        None,
        Some("session-1"),
        Some("workspace-a"),
        true,
    );

    assert!(candidates.iter().any(|candidate| {
        candidate.proposed_record.kind == MemoryKind::Preference
            && candidate.proposed_record.scope == MemoryScope::GlobalUser
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.proposed_record.kind == MemoryKind::ProjectContext
            && matches!(
                candidate.proposed_record.scope,
                MemoryScope::Workspace { .. }
            )
    }));
}

#[test]
fn dedup_keeps_secret_like_candidate_discard_policy() {
    let now = 20;
    let low_risk = MemoryCandidate {
        proposed_record: MemoryRecord::new(
            "low",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "用户偏好使用中文回答。",
            now,
        )
        .with_status(MemoryStatus::Active),
        evidence: "中文".to_string(),
        write_policy: MemoryWritePolicy::Auto,
        reason: "rule:language-preference".to_string(),
    };
    let secret_like = MemoryCandidate {
        proposed_record: MemoryRecord::new(
            "secret",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "用户偏好使用中文回答，api key 是 sk-test-secret。",
            now + 1,
        )
        .with_sensitivity(MemorySensitivity::High)
        .with_status(MemoryStatus::Rejected),
        evidence: "api key redacted".to_string(),
        write_policy: MemoryWritePolicy::Discard,
        reason: "provider:secret".to_string(),
    };

    let candidates = deduplicate_candidates(vec![low_risk, secret_like]);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].write_policy, MemoryWritePolicy::Discard);
    assert_eq!(candidates[0].proposed_record.status, MemoryStatus::Rejected);
    assert_eq!(
        candidates[0].proposed_record.sensitivity,
        MemorySensitivity::High
    );
}
