use serde_json::json;
use yunxi_agent_persona::{
    MemoryEntityRef, MemoryEntityType, MemoryGraphRelation, MemoryKind, MemoryMigrationResult,
    MemoryPipeline, MemoryPipelineInput, MemoryRecallRoute, MemoryRecallRouter,
    MemoryRecallRouterRequest, MemoryRecord, MemoryScope, MemorySensitivity, MemoryStatus,
    RelationshipGraphLite, migrate_memory_record_value,
};

fn active(id: &str, kind: MemoryKind, content: &str, at: u128) -> MemoryRecord {
    MemoryRecord::new(id, MemoryScope::Relationship, kind, content, at)
        .with_scores(0.9, 0.8)
        .with_status(MemoryStatus::Active)
}

fn timeline_request() -> MemoryRecallRouterRequest {
    let mut request = MemoryRecallRouterRequest::new("relationship timeline before and after");
    request.boot_max_records = 0;
    request.dynamic_max_records = 20;
    request.dynamic_budget_chars = 10_000;
    request
}

#[test]
fn relationship_graph_builds_nodes_and_edges_from_memory_entities() {
    let mut record = active(
        "relationship-1",
        MemoryKind::RelationshipNote,
        "The relationship became more trusting",
        10,
    );
    record.entities = vec![
        MemoryEntityRef {
            entity_type: MemoryEntityType::User,
            id: "user:primary".to_string(),
            label: Some("User".to_string()),
        },
        MemoryEntityRef {
            entity_type: MemoryEntityType::Agent,
            id: "agent:yunxi".to_string(),
            label: Some("YunXi".to_string()),
        },
    ];

    let graph = RelationshipGraphLite::from_records(&[record]);

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from, "user:primary");
    assert_eq!(graph.edges[0].to, "agent:yunxi");
    assert_eq!(
        graph.edges[0].relation,
        MemoryGraphRelation::RelationshipNote
    );
}

#[test]
fn relationship_graph_orders_events_by_time() {
    let mut observed = active("observed", MemoryKind::Event, "Observed event", 100);
    observed.temporal.observed_at_millis = 300;
    let mut event = active("event", MemoryKind::Event, "Explicit event", 200);
    event.temporal.event_at_millis = Some(400);
    let created = active("created", MemoryKind::Event, "Created event", 250);

    let graph = RelationshipGraphLite::from_records(&[observed, event, created]);
    let ids = graph
        .events_by_time()
        .into_iter()
        .filter(|edge| edge.id.ends_with(":fact"))
        .map(|edge| edge.memory_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["event", "observed", "created"]);
}

#[test]
fn old_superseded_fact_is_not_recalled_as_active_boot_context() {
    let mut old = active(
        "old-language",
        MemoryKind::Preference,
        "Prefer Chinese replies",
        10,
    );
    old.invalidation.superseded_by = Some("new-language".to_string());
    old.invalidation.invalidated_at_millis = Some(20);
    let new = active(
        "new-language",
        MemoryKind::Preference,
        "Prefer English replies",
        20,
    );
    let routed = MemoryRecallRouter::default().route(
        &[old, new],
        &MemoryRecallRouterRequest::new("current preference"),
    );

    assert_eq!(routed.boot_context.records.len(), 1);
    assert_eq!(routed.boot_context.records[0].id, "new-language");
    assert!(routed.explanations.iter().any(|explanation| {
        explanation.memory_id == "old-language"
            && explanation.reason == "superseded_by_newer_fact"
            && !explanation.selected
    }));
}

#[test]
fn relationship_events_are_recalled_by_time_for_dynamic_query() {
    let mut first = active(
        "first",
        MemoryKind::RelationshipNote,
        "We started collaborating",
        10,
    );
    first.temporal.event_at_millis = Some(100);
    let mut latest = active(
        "latest",
        MemoryKind::EmotionalState,
        "The collaboration feels trusting",
        20,
    );
    latest.temporal.event_at_millis = Some(200);

    let routed = MemoryRecallRouter::default().route(&[first, latest], &timeline_request());
    let ids = routed
        .dynamic_recall
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["latest", "first"]);
}

#[test]
fn expired_relationship_edge_is_excluded_from_active_recall() {
    let mut expired = active(
        "expired",
        MemoryKind::RelationshipNote,
        "An expired relationship state",
        1,
    );
    expired.temporal.expires_at_millis = Some(2);
    let graph = RelationshipGraphLite::from_records(&[expired.clone()]);
    let routed = MemoryRecallRouter::default()
        .route(&[expired], &MemoryRecallRouterRequest::new("current state"));

    assert!(graph.active_edges_at(u128::MAX).is_empty());
    assert!(routed.boot_context.records.is_empty());
    assert!(routed.dynamic_recall.records.is_empty());
    assert!(
        routed
            .explanations
            .iter()
            .any(|item| item.reason == "expired_relation_edge")
    );
}

#[test]
fn invalidated_memory_remains_in_history_but_not_active_recall() {
    let mut invalidated = active(
        "invalidated",
        MemoryKind::RelationshipNote,
        "Historical relationship state",
        1,
    );
    invalidated.invalidation.invalidated_at_millis = Some(2);
    let graph = RelationshipGraphLite::from_records(&[invalidated.clone()]);
    let routed = MemoryRecallRouter::default().route(
        &[invalidated],
        &MemoryRecallRouterRequest::new("current state"),
    );

    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.memory_id == "invalidated")
    );
    assert!(graph.active_edges_at(u128::MAX).is_empty());
    assert!(routed.boot_context.records.is_empty());
}

#[test]
fn graph_explanations_include_relation_and_temporal_reason() {
    let mut event = active(
        "event",
        MemoryKind::RelationshipNote,
        "Relationship changed after a repair",
        1,
    );
    event.temporal.event_at_millis = Some(10);
    let routed = MemoryRecallRouter::default().route(&[event], &timeline_request());
    let explanation = routed
        .explanations
        .iter()
        .find(|item| item.selected)
        .expect("selected graph explanation");

    assert_eq!(
        explanation.relation,
        Some(MemoryGraphRelation::RelationshipNote)
    );
    assert_eq!(explanation.reason, "temporal_event_match");
    assert_eq!(
        explanation.temporal_reason.as_deref(),
        Some("ordered_by_event_observed_updated_created_time")
    );
}

#[test]
fn graph_explanations_do_not_include_raw_sensitive_content() {
    let secret = ["github", "_pat_", "relationship_test_value"].concat();
    let record = active("sensitive", MemoryKind::RelationshipNote, &secret, 1)
        .with_sensitivity(MemorySensitivity::High);
    let routed = MemoryRecallRouter::default().route(&[record], &timeline_request());
    let json = serde_json::to_string(&routed.explanations).expect("serialize explanations");

    assert!(!json.contains(&secret));
    assert!(json.contains("privacy_policy_excludes_high_sensitivity"));
}

#[test]
fn pending_conflict_explanation_is_not_recalled_as_history() {
    let mut pending = MemoryRecord::new(
        "pending-conflict",
        MemoryScope::Relationship,
        MemoryKind::RelationshipNote,
        "A relationship claim awaiting confirmation",
        1,
    );
    pending
        .invalidation
        .conflicts_with
        .push("active-relationship".to_string());
    let routed = MemoryRecallRouter::default().route(&[pending], &timeline_request());

    assert!(routed.dynamic_recall.records.is_empty());
    assert!(
        routed
            .explanations
            .iter()
            .any(|item| { item.reason == "conflict_pending_confirmation" && !item.selected })
    );
}

#[test]
fn schema_v3_migration_still_preserves_invalidation_fields() {
    let migrated = migrate_memory_record_value(json!({
        "id": "v3-history",
        "schema_version": 3,
        "scope": "relationship",
        "kind": "relationship_note",
        "content": "Historical state",
        "confidence": 0.9,
        "importance": 0.8,
        "sensitivity": "low",
        "status": "active",
        "created_at_millis": 1,
        "updated_at_millis": 3,
        "invalidation": {
            "supersedes": ["older"],
            "superseded_by": "newer",
            "conflicts_with": ["conflict"],
            "invalidated_at_millis": 3,
            "expires_reason": "relationship changed"
        }
    }));
    let record = match migrated {
        MemoryMigrationResult::Record(record)
        | MemoryMigrationResult::RecordWithWarning { record, .. } => record,
        MemoryMigrationResult::Skip { warning } => panic!("migration skipped: {warning}"),
    };

    assert_eq!(record.invalidation.supersedes, vec!["older"]);
    assert_eq!(record.invalidation.superseded_by.as_deref(), Some("newer"));
    assert_eq!(record.invalidation.conflicts_with, vec!["conflict"]);
    assert_eq!(record.invalidation.invalidated_at_millis, Some(3));
    assert_eq!(
        record.invalidation.expires_reason.as_deref(),
        Some("relationship changed")
    );
}

#[test]
fn boot_context_and_l0_l3_pipeline_regressions_still_pass() {
    let output = MemoryPipeline::new().run(MemoryPipelineInput {
        prompt: "Remember that I prefer English from now on".to_string(),
        source_session_id: Some("relationship-graph-regression".to_string()),
        memory_enabled: true,
        ..MemoryPipelineInput::default()
    });
    assert_eq!(output.stages.len(), 4);
    assert!(
        output
            .candidates
            .iter()
            .any(|candidate| candidate.proposed_record.status == MemoryStatus::Active)
    );
    let records = output
        .candidates
        .into_iter()
        .map(|candidate| candidate.proposed_record)
        .collect::<Vec<_>>();
    let routed = MemoryRecallRouter::default().route(
        &records,
        &MemoryRecallRouterRequest::new("current preference"),
    );
    assert!(!routed.boot_context.records.is_empty());
    assert!(routed.dynamic_recall.records.is_empty());
    assert!(
        routed
            .explanations
            .iter()
            .any(|item| item.route == MemoryRecallRoute::Boot)
    );
}
