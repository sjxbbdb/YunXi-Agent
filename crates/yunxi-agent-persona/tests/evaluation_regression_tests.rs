use yunxi_agent_persona::{
    HumanProfile, MemoryKind, MemoryRecord, MemoryRuleExtractor, MemoryScope, MemoryStatus,
    MemoryWritePolicy, PersonaPromptCompiler, RelationshipGraphLite, RelationshipState,
    link_supersession_chain, yunxi_companion_strong,
};

#[test]
fn evaluation_persona_consistency_regression_is_automatic() {
    let profile = yunxi_companion_strong();
    let context = PersonaPromptCompiler::new(3200).compile(
        &profile,
        &HumanProfile::default(),
        &RelationshipState::default(),
        &[],
    );
    assert_eq!(profile.id, "yunxi_companion_strong");
    assert_eq!(profile.version, "2.3.3");
    assert!(context.content.contains("context_not_instruction"));
    assert!(context.content.contains("reply_style_guidance"));
    assert!(profile.layers.boundaries.contains("AGENTS.md"));
}

#[test]
fn evaluation_memory_precision_regression_tracks_allowed_and_forbidden_writes() {
    let extractor = MemoryRuleExtractor::new();
    let preference = extractor.extract("以后请用中文回答", None, None, Some("eval"), true);
    let secret = extractor.extract(
        "我的 API key 是 ghp_eval_secret_token",
        None,
        None,
        Some("eval"),
        true,
    );
    assert!(preference.iter().any(|candidate| {
        candidate.proposed_record.kind == MemoryKind::Preference
            && candidate.write_policy == MemoryWritePolicy::Auto
    }));
    assert!(
        secret
            .iter()
            .any(|candidate| candidate.write_policy == MemoryWritePolicy::Discard)
    );
}

#[test]
fn evaluation_relationship_continuity_regression_keeps_replacement_history() {
    let old = MemoryRecord::new(
        "old",
        MemoryScope::Relationship,
        MemoryKind::Preference,
        "old relationship preference",
        1,
    )
    .with_status(MemoryStatus::Active);
    let new = MemoryRecord::new(
        "new",
        MemoryScope::Relationship,
        MemoryKind::Preference,
        "new relationship preference",
        2,
    )
    .with_status(MemoryStatus::Active);
    let (old, new) = link_supersession_chain(&old, &new, 3);
    let graph = RelationshipGraphLite::from_records(&[old.clone(), new.clone()]);
    assert_eq!(old.invalidation.superseded_by.as_deref(), Some("new"));
    assert!(new.invalidation.supersedes.contains(&"old".to_string()));
    assert!(graph.edges.len() >= 2);
}
