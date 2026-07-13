use yunxi_agent_persona::{
    HumanProfile, MemoryRecord, MemoryScope, MemoryStatus, PersonaPromptCompiler,
    RelationshipState, yunxi_companion_strong,
};

#[test]
fn default_yunxi_persona_compiles_with_markers_and_memory_budget() {
    let profile = yunxi_companion_strong();
    let memory = MemoryRecord::new(
        "m1",
        MemoryScope::GlobalUser,
        yunxi_agent_persona::MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);

    let compiled = PersonaPromptCompiler::new(900).compile(
        &profile,
        &HumanProfile::default(),
        &RelationshipState::default(),
        &[memory],
    );

    assert_eq!(compiled.profile_id, "yunxi_companion_strong");
    assert!(compiled.content.contains("[YunXi persona context]"));
    assert!(compiled.content.contains("[YunXi memory context]"));
    assert!(compiled.content.contains("用户偏好使用中文回答"));
    assert_eq!(compiled.memory_count, 1);
    assert!(compiled.budget_used_chars <= compiled.budget_limit_chars + 30);
}
