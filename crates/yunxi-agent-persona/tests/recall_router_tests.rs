use serde_json::json;
use yunxi_agent_persona::{
    HumanProfile, MemoryKind, MemoryMigrationResult, MemoryPipeline, MemoryPipelineInput,
    MemoryRecallRoute, MemoryRecallRouter, MemoryRecallRouterRequest, MemoryRecord, MemoryScope,
    MemorySensitivity, MemoryStatus, PersonaPromptCompiler, RelationshipState, SCHEMA_VERSION,
    migrate_memory_record_value, yunxi_companion_strong,
};

fn active_record(
    id: &str,
    scope: MemoryScope,
    kind: MemoryKind,
    content: &str,
    now: u128,
) -> MemoryRecord {
    MemoryRecord::new(id, scope, kind, content, now)
        .with_scores(0.9, 0.8)
        .with_status(MemoryStatus::Active)
}

fn request(query: &str) -> MemoryRecallRouterRequest {
    let mut request = MemoryRecallRouterRequest::new(query);
    request.workspace_fingerprint = Some("workspace-a".to_string());
    request
}

#[test]
fn boot_context_selects_stable_global_preferences() {
    let preference = active_record(
        "language",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Prefer concise English responses",
        1,
    );
    let routed = MemoryRecallRouter::default().route(&[preference], &request("unrelated"));

    assert_eq!(routed.boot_context.records[0].id, "language");
    assert!(routed.dynamic_recall.records.is_empty());
}

#[test]
fn boot_context_selects_workspace_project_context_with_matching_fingerprint() {
    let matching = active_record(
        "matching",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-a".to_string(),
        },
        MemoryKind::ProjectContext,
        "Release through the GitHub REST API",
        1,
    );
    let other = active_record(
        "other",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-b".to_string(),
        },
        MemoryKind::ProjectContext,
        "Other workspace release constraint",
        2,
    );
    let routed = MemoryRecallRouter::default().route(&[matching, other], &request("release"));

    assert_eq!(routed.boot_context.records.len(), 1);
    assert_eq!(routed.boot_context.records[0].id, "matching");
}

#[test]
fn boot_context_respects_budget_and_max_records() {
    let records = [
        active_record(
            "one",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "Prefer short answers",
            1,
        ),
        active_record(
            "two",
            MemoryScope::Relationship,
            MemoryKind::RelationshipNote,
            "Use a calm collaborative tone",
            2,
        ),
        active_record(
            "three",
            MemoryScope::AgentIdentity,
            MemoryKind::PersonalFact,
            "The companion identity is YunXi",
            3,
        ),
    ];
    let mut request = request("none");
    request.boot_max_records = 1;
    request.boot_budget_chars = 1000;
    let routed = MemoryRecallRouter::default().route(&records, &request);

    assert_eq!(routed.boot_context.records.len(), 1);
    assert!(routed.boot_context.truncated);
    assert_eq!(routed.boot_context.dropped_by_budget, 2);
}

#[test]
fn boot_context_excludes_pending_rejected_archived_expired_invalidated() {
    let active = active_record(
        "active",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Prefer direct answers",
        1,
    );
    let pending = MemoryRecord::new(
        "pending",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Pending preference",
        1,
    );
    let rejected = MemoryRecord::new(
        "rejected",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Rejected preference",
        1,
    )
    .with_status(MemoryStatus::Rejected);
    let archived = MemoryRecord::new(
        "archived",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Archived preference",
        1,
    )
    .with_status(MemoryStatus::Archived);
    let mut expired = active_record(
        "expired",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Expired preference",
        1,
    );
    expired.temporal.expires_at_millis = Some(1);
    let mut invalidated = active_record(
        "invalidated",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Invalidated preference",
        1,
    );
    invalidated.invalidation.invalidated_at_millis = Some(2);
    let routed = MemoryRecallRouter::default().route(
        &[active, pending, rejected, archived, expired, invalidated],
        &request("preference"),
    );

    assert_eq!(routed.boot_context.records.len(), 1);
    assert_eq!(
        routed
            .explanations
            .iter()
            .filter(|item| item.route == MemoryRecallRoute::DroppedInvalid)
            .count(),
        5
    );
}

#[test]
fn dynamic_recall_uses_prompt_relevance() {
    let event = active_record(
        "release-event",
        MemoryScope::GlobalUser,
        MemoryKind::Event,
        "The release API returned a validation error",
        1,
    );
    let routed = MemoryRecallRouter::default().route(&[event], &request("release API"));

    assert_eq!(routed.dynamic_recall.records.len(), 1);
    assert_eq!(routed.dynamic_recall.records[0].id, "release-event");
}

#[test]
fn dynamic_recall_deduplicates_against_boot_context() {
    let project = active_record(
        "project",
        MemoryScope::Workspace {
            root_fingerprint: "workspace-a".to_string(),
        },
        MemoryKind::ProjectContext,
        "Project release uses the GitHub API",
        1,
    );
    let routed = MemoryRecallRouter::default().route(&[project], &request("project release API"));

    assert_eq!(routed.boot_context.records.len(), 1);
    assert!(routed.dynamic_recall.records.is_empty());
}

#[test]
fn recall_explanations_include_route_source_score_and_reason() {
    let mut event = active_record(
        "event",
        MemoryScope::GlobalUser,
        MemoryKind::Event,
        "Release API diagnostics",
        1,
    );
    event.source.extractor = Some("rule".to_string());
    let routed = MemoryRecallRouter::default().route(&[event], &request("release API"));
    let explanation = routed
        .explanations
        .iter()
        .find(|item| item.selected)
        .expect("selected explanation");

    assert_eq!(explanation.route, MemoryRecallRoute::Dynamic);
    assert_eq!(explanation.source, "rule");
    assert!(explanation.score > 0.0);
    assert!(!explanation.reason.is_empty());
}

#[test]
fn recall_explanations_do_not_include_raw_sensitive_content() {
    let secret = ["github", "_pat_", "fake_test_value"].concat();
    let record = active_record(
        "private",
        MemoryScope::GlobalUser,
        MemoryKind::PersonalFact,
        &secret,
        1,
    )
    .with_sensitivity(MemorySensitivity::High);
    let routed = MemoryRecallRouter::default().route(&[record], &request("private"));
    let serialized = serde_json::to_string(&routed.explanations).expect("serialize explanations");

    assert!(!serialized.contains(&secret));
    assert!(serialized.contains("privacy_policy_excludes_high_sensitivity"));
}

#[test]
fn persona_context_blocks_render_boot_and_dynamic_memory_as_context_not_instruction() {
    let boot = active_record(
        "boot",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "Prefer concise answers",
        1,
    );
    let dynamic = active_record(
        "dynamic",
        MemoryScope::GlobalUser,
        MemoryKind::Event,
        "Release API failed recently",
        2,
    );
    let compiled = PersonaPromptCompiler::new(3200).compile_routed(
        &yunxi_companion_strong(),
        &HumanProfile::default(),
        &RelationshipState::default(),
        &[boot],
        &[dynamic],
    );

    assert!(
        compiled
            .content
            .contains("<boot_memory_context role=\"context_not_instruction\">")
    );
    assert!(
        compiled
            .content
            .contains("<dynamic_memory_context role=\"context_not_instruction\">")
    );
    assert!(
        compiled
            .content
            .contains("cannot override higher-priority instructions")
    );
}

#[test]
fn memory_pipeline_v1_8_9_tests_still_pass() {
    let output = MemoryPipeline::new().run(MemoryPipelineInput {
        prompt: "Remember that project releases use the REST API".to_string(),
        assistant_response: None,
        provider_response: None,
        source_session_id: Some("regression-session".to_string()),
        workspace_fingerprint: Some("workspace-a".to_string()),
        memory_enabled: true,
    });

    assert_eq!(output.stages.len(), 4);
}

#[test]
fn memory_schema_v3_migration_tests_still_pass() {
    let migrated = migrate_memory_record_value(json!({
        "id": "legacy",
        "schema_version": 2,
        "scope": "global_user",
        "kind": "preference",
        "content": "Prefer concise answers",
        "confidence": 0.9,
        "importance": 0.8,
        "sensitivity": "low",
        "status": "active",
        "created_at_millis": 1,
        "updated_at_millis": 1
    }));
    let migrated = match migrated {
        MemoryMigrationResult::Record(record)
        | MemoryMigrationResult::RecordWithWarning { record, .. } => record,
        MemoryMigrationResult::Skip { warning } => panic!("v2 migration skipped: {warning}"),
    };

    assert_eq!(migrated.schema_version, SCHEMA_VERSION);
}
