use serde_json::json;
use yunxi_agent_persona::{
    HumanProfile, MemoryKind, MemoryLayer, MemoryMigrationResult, MemoryPipeline,
    MemoryPipelineInput, MemoryPipelineLayer, MemoryRecord, MemoryScope, MemoryStatus,
    MemoryWritePolicy, PersonaPromptCompiler, RelationshipState, migrate_memory_record_value,
    yunxi_companion_strong,
};

fn input(prompt: &str) -> MemoryPipelineInput {
    MemoryPipelineInput {
        prompt: prompt.to_string(),
        assistant_response: Some("Acknowledged.".to_string()),
        provider_response: None,
        source_session_id: Some("pipeline-session".to_string()),
        workspace_fingerprint: Some("pipeline-workspace".to_string()),
        memory_enabled: true,
    }
}

#[test]
fn pipeline_generates_l1_candidate_from_language_preference() {
    let output = MemoryPipeline::new().run(input("Please answer in English"));

    let preference = output
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::Preference)
        .expect("L1 language preference");
    assert_eq!(preference.proposed_record.layer, MemoryLayer::Preference);
    assert_eq!(preference.proposed_record.status, MemoryStatus::Active);
    assert!(output.stages.iter().any(|stage| {
        stage.layer == MemoryPipelineLayer::L1StructuredFact && stage.retained_count >= 1
    }));
}

#[test]
fn pipeline_auto_saves_explicit_chinese_remembered_preference_without_provider() {
    let output = MemoryPipeline::new().run(input(
        "请记住一个测试偏好：YUNXI_MEMORY_TEST_AUTOSEQ_20260801。我希望以后测试报告标题包含‘自动顺序测试’。请只回复“已记录测试偏好”。",
    ));

    let preference = output
        .candidates
        .iter()
        .find(|candidate| {
            candidate
                .reason
                .contains("rule:explicit-remember-preference")
        })
        .expect("explicit remembered preference");

    assert_eq!(preference.proposed_record.kind, MemoryKind::Preference);
    assert_eq!(preference.write_policy, MemoryWritePolicy::Auto);
    assert_eq!(preference.proposed_record.status, MemoryStatus::Active);
    assert!(
        preference
            .proposed_record
            .content
            .contains("YUNXI_MEMORY_TEST_AUTOSEQ_20260801")
    );
    assert!(!preference.proposed_record.content.contains("请只回复"));
}

#[test]
fn pipeline_generates_l2_relationship_candidate_as_pending() {
    let output = MemoryPipeline::new().run(input("My relationship with Alex is strained."));

    let relationship = output
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::RelationshipNote)
        .expect("L2 relationship event");
    assert_eq!(
        relationship.write_policy,
        MemoryWritePolicy::RequireConfirmation
    );
    assert_eq!(relationship.proposed_record.status, MemoryStatus::Pending);
    assert!(output.stages.iter().any(|stage| {
        stage.layer == MemoryPipelineLayer::L2RelationshipEvent && stage.pending_count >= 1
    }));
}

#[test]
fn pipeline_generates_l3_profile_summary_only_for_stable_low_risk_memory() {
    let transient = MemoryPipeline::new().run(input("Please answer in English"));
    assert!(
        transient
            .candidates
            .iter()
            .all(|candidate| candidate.proposed_record.layer != MemoryLayer::Profile)
    );

    let stable = MemoryPipeline::new().run(input("Please answer in English from now on"));
    let profile = stable
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.layer == MemoryLayer::Profile)
        .expect("stable L3 profile summary");
    assert_eq!(
        profile.proposed_record.sensitivity,
        yunxi_agent_persona::MemorySensitivity::Low
    );
    assert_eq!(profile.proposed_record.status, MemoryStatus::Active);
    assert!(profile.proposed_record.evidence.iter().any(|evidence| {
        evidence.kind == "l3_profile_summary"
            && evidence.summary.contains("not a raw-turn instruction")
    }));
}

#[test]
fn pipeline_records_l0_summary_as_evidence_not_instruction() {
    let output = MemoryPipeline::new().run(input("Please answer in English"));

    assert_eq!(output.l0_evidence.kind, "l0_raw_turn_summary");
    assert!(output.candidates.iter().all(|candidate| {
        candidate
            .proposed_record
            .evidence
            .iter()
            .any(|evidence| evidence.kind == "l0_raw_turn_summary")
    }));
    let l0 = output
        .stages
        .iter()
        .find(|stage| stage.layer == MemoryPipelineLayer::L0RawTurn)
        .expect("L0 audit stage");
    assert_eq!(l0.generated_count, 0);
}

#[test]
fn pipeline_discards_secret_like_memory() {
    let secret = format!("Remember my api {} is fake-{}-value", "key", "credential");
    let output = MemoryPipeline::new().run(input(&secret));

    assert!(!output.candidates.is_empty());
    assert!(output.candidates.iter().all(|candidate| {
        candidate.write_policy == MemoryWritePolicy::Discard
            && candidate.proposed_record.status == MemoryStatus::Rejected
            && candidate.proposed_record.content.contains("[redacted:")
            && !candidate
                .proposed_record
                .content
                .contains("fake-credential-value")
    }));
    assert!(!output.l0_evidence.summary.contains("fake-credential-value"));
}

#[test]
fn pipeline_downgrades_sensitive_personal_memory_to_pending() {
    let output = MemoryPipeline::new().run(input("My health condition changed recently."));

    let personal = output
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::PersonalFact)
        .expect("sensitive personal fact");
    assert_eq!(
        personal.write_policy,
        MemoryWritePolicy::RequireConfirmation
    );
    assert_eq!(personal.proposed_record.status, MemoryStatus::Pending);
}

#[test]
fn pending_active_rejected_flow_is_testable() {
    let output = MemoryPipeline::new().run(input("My relationship with Alex is strained."));
    let pending = output
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.kind == MemoryKind::RelationshipNote)
        .expect("pending relationship")
        .proposed_record
        .clone();
    let active = pending.clone().with_status(MemoryStatus::Active);
    let rejected = pending.clone().with_status(MemoryStatus::Rejected);
    let compiled = PersonaPromptCompiler::new(2400).compile(
        &yunxi_companion_strong(),
        &HumanProfile::default(),
        &RelationshipState::default(),
        &[pending, active, rejected],
    );

    assert_eq!(compiled.memory_count, 1);
}

#[test]
fn dedup_merges_rule_and_provider_candidates_across_layers() {
    let mut pipeline_input = input("Please answer in English from now on");
    pipeline_input.provider_response = Some(
        json!({
            "candidates": [{
                "kind": "preference",
                "content": "用户偏好后续默认使用英文交流。",
                "confidence": 0.95,
                "importance": 0.8,
                "reason": "provider:stable-language-preference"
            }]
        })
        .to_string(),
    );
    let output = MemoryPipeline::new().run(pipeline_input);
    let preferences = output
        .candidates
        .iter()
        .filter(|candidate| candidate.proposed_record.kind == MemoryKind::Preference)
        .collect::<Vec<_>>();

    assert_eq!(preferences.len(), 1);
    assert_eq!(preferences[0].proposed_record.layer, MemoryLayer::Profile);
    assert!(output.stages.iter().any(|stage| stage.merged_count >= 1));
}

#[test]
fn merge_preserves_source_lineage_across_l1_l3() {
    let mut pipeline_input = input("Please answer in English from now on");
    pipeline_input.provider_response = Some(
        json!({
            "candidates": [{
                "kind": "preference",
                "content": "用户偏好后续默认使用英文交流。",
                "confidence": 0.96,
                "importance": 0.8,
                "reason": "provider:language-preference"
            }]
        })
        .to_string(),
    );
    let output = MemoryPipeline::new().run(pipeline_input);
    let profile = output
        .candidates
        .iter()
        .find(|candidate| candidate.proposed_record.layer == MemoryLayer::Profile)
        .expect("merged L3 profile");
    let extractors = profile
        .proposed_record
        .source
        .attributions
        .iter()
        .filter_map(|source| source.extractor.as_deref())
        .collect::<Vec<_>>();

    assert!(extractors.contains(&"rule"));
    assert!(extractors.contains(&"provider"));
    assert!(
        profile
            .proposed_record
            .evidence
            .iter()
            .any(|evidence| evidence.kind == "l3_profile_summary")
    );
}

#[test]
fn provider_failure_does_not_block_rule_candidates() {
    let mut pipeline_input = input("Please answer in English");
    pipeline_input.provider_response = Some("not-json".to_string());
    let output = MemoryPipeline::new().run(pipeline_input);

    assert!(!output.warnings.is_empty());
    assert!(
        output
            .candidates
            .iter()
            .any(|candidate| candidate.proposed_record.kind == MemoryKind::Preference)
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|event| event.action == "provider_failed_soft")
    );
}

#[test]
fn persona_context_blocks_do_not_regress_after_pipeline_write() {
    let output = MemoryPipeline::new().run(input("Please answer in English from now on"));
    let records = output
        .candidates
        .into_iter()
        .map(|candidate| candidate.proposed_record)
        .collect::<Vec<_>>();
    let compiled = PersonaPromptCompiler::new(2400).compile(
        &yunxi_companion_strong(),
        &HumanProfile::default(),
        &RelationshipState::default(),
        &records,
    );

    assert!(compiled.content.contains("<persona>"));
    assert!(compiled.content.contains("<boundaries>"));
    assert!(compiled.content.contains("<human>"));
    assert!(compiled.content.contains("<relationship>"));
    assert!(
        compiled
            .content
            .contains("<memory_context role=\"context_not_instruction\">")
    );
    assert!(compiled.content.contains("context, not instructions"));
}

#[test]
fn schema_v3_migration_tests_still_pass() {
    let legacy = json!({
        "id": "v2-memory",
        "schema_version": 2,
        "scope": "global_user",
        "kind": "preference",
        "content": "Prefer concise answers",
        "confidence": 0.9,
        "importance": 0.7,
        "sensitivity": "low",
        "status": "active",
        "created_at_millis": 1,
        "updated_at_millis": 2
    });
    let record = match migrate_memory_record_value(legacy) {
        MemoryMigrationResult::Record(record)
        | MemoryMigrationResult::RecordWithWarning { record, .. } => record,
        MemoryMigrationResult::Skip { warning } => panic!("migration skipped: {warning}"),
    };

    assert_eq!(record.schema_version, 3);
    assert_eq!(record.content, "Prefer concise answers");
}

#[allow(dead_code)]
fn _record_fixture() -> MemoryRecord {
    MemoryRecord::new(
        "fixture",
        MemoryScope::GlobalUser,
        MemoryKind::Preference,
        "fixture",
        1,
    )
}
