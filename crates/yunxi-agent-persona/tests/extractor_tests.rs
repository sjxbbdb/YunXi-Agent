use yunxi_agent_persona::{
    MemoryKind, MemoryRuleExtractor, MemoryScope, MemoryStatus, MemoryWritePolicy,
    ProviderMemoryExtractor,
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
