use crate::memory::{
    MemoryEntityRef, MemoryEntityType, MemoryKind, MemoryRecord, MemoryScope, MemoryStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryGraphRelation {
    Preference,
    Correction,
    Supersedes,
    ConflictsWith,
    RelationshipNote,
    ProjectContext,
    Goal,
    EmotionalState,
    Event,
    RelatedTo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryGraphNode {
    pub entity_type: MemoryEntityType,
    pub id: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryGraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: MemoryGraphRelation,
    pub memory_id: String,
    pub event_at_millis: Option<u128>,
    pub observed_at_millis: u128,
    pub updated_at_millis: u128,
    pub created_at_millis: u128,
    pub valid_from_millis: Option<u128>,
    pub expires_at_millis: Option<u128>,
    pub invalidated_at_millis: Option<u128>,
    pub supersedes: Vec<String>,
    pub superseded_by: Option<String>,
    pub conflicts_with: Vec<String>,
    pub active: bool,
}

impl MemoryGraphEdge {
    pub fn ordering_time(&self) -> u128 {
        self.event_at_millis.unwrap_or_else(|| {
            if self.observed_at_millis > 0 {
                self.observed_at_millis
            } else if self.updated_at_millis > 0 {
                self.updated_at_millis
            } else {
                self.created_at_millis
            }
        })
    }

    pub fn is_active_at(&self, at_millis: u128) -> bool {
        self.active
            && self
                .valid_from_millis
                .is_none_or(|valid_from| valid_from <= at_millis)
            && self
                .expires_at_millis
                .is_none_or(|expires_at| expires_at > at_millis)
            && self.invalidated_at_millis.is_none()
            && self.superseded_by.is_none()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationshipGraphLite {
    pub nodes: Vec<MemoryGraphNode>,
    pub edges: Vec<MemoryGraphEdge>,
}

impl RelationshipGraphLite {
    pub fn from_records(records: &[MemoryRecord]) -> Self {
        let mut nodes = BTreeMap::<String, MemoryGraphNode>::new();
        let mut edges = Vec::new();

        for record in records {
            let endpoints = endpoints(record);
            for node in &endpoints {
                nodes.entry(node.id.clone()).or_insert_with(|| node.clone());
            }

            let base_relation = relation_for_kind(record.kind);
            edges.push(edge_for_record(
                record,
                &endpoints[0].id,
                &endpoints[1].id,
                base_relation,
                "fact",
            ));
            for superseded_id in &record.invalidation.supersedes {
                let target = memory_node(superseded_id);
                nodes
                    .entry(target.id.clone())
                    .or_insert_with(|| target.clone());
                edges.push(edge_for_record(
                    record,
                    &endpoints[0].id,
                    &target.id,
                    MemoryGraphRelation::Supersedes,
                    "supersedes",
                ));
            }
            for conflict_id in &record.invalidation.conflicts_with {
                let target = memory_node(conflict_id);
                nodes
                    .entry(target.id.clone())
                    .or_insert_with(|| target.clone());
                edges.push(edge_for_record(
                    record,
                    &endpoints[0].id,
                    &target.id,
                    MemoryGraphRelation::ConflictsWith,
                    "conflict",
                ));
            }
        }

        Self {
            nodes: nodes.into_values().collect(),
            edges,
        }
    }

    pub fn events_by_time(&self) -> Vec<&MemoryGraphEdge> {
        let mut edges = self.edges.iter().collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            right
                .ordering_time()
                .cmp(&left.ordering_time())
                .then_with(|| left.id.cmp(&right.id))
        });
        edges
    }

    pub fn active_edges_at(&self, at_millis: u128) -> Vec<&MemoryGraphEdge> {
        self.events_by_time()
            .into_iter()
            .filter(|edge| edge.is_active_at(at_millis))
            .collect()
    }

    pub fn relation_for_memory(&self, memory_id: &str) -> Option<MemoryGraphRelation> {
        self.edges
            .iter()
            .find(|edge| {
                edge.memory_id == memory_id
                    && !matches!(
                        edge.relation,
                        MemoryGraphRelation::Supersedes | MemoryGraphRelation::ConflictsWith
                    )
            })
            .map(|edge| edge.relation)
    }
}

pub fn link_supersession_chain(
    existing: &MemoryRecord,
    incoming: &MemoryRecord,
    at_millis: u128,
) -> (MemoryRecord, MemoryRecord) {
    let mut old = existing.clone();
    let mut new = incoming.clone();
    old.invalidation.superseded_by = Some(new.id.clone());
    old.invalidation.invalidated_at_millis = Some(at_millis);
    old.updated_at_millis = old.updated_at_millis.max(at_millis);
    old.revision = old.revision.saturating_add(1).max(2);
    if !new.invalidation.supersedes.contains(&old.id) {
        new.invalidation.supersedes.push(old.id.clone());
    }
    new.temporal.valid_from_millis = Some(
        new.temporal
            .valid_from_millis
            .unwrap_or(at_millis)
            .max(at_millis),
    );
    new.updated_at_millis = new.updated_at_millis.max(at_millis);
    old.ensure_dedup_metadata();
    new.ensure_dedup_metadata();
    (old, new)
}

pub fn is_relationship_timeline_query(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    [
        "关系",
        "情绪",
        "感受",
        "以前",
        "之前",
        "后来",
        "变化",
        "relationship",
        "emotion",
        "feel",
        "before",
        "after",
        "changed",
        "timeline",
        "history",
    ]
    .iter()
    .any(|needle| query.contains(needle) || lower.contains(needle))
}

pub fn temporal_ordering_time(record: &MemoryRecord) -> u128 {
    record.temporal.event_at_millis.unwrap_or_else(|| {
        if record.temporal.observed_at_millis > 0 {
            record.temporal.observed_at_millis
        } else if record.updated_at_millis > 0 {
            record.updated_at_millis
        } else {
            record.created_at_millis
        }
    })
}

fn relation_for_kind(kind: MemoryKind) -> MemoryGraphRelation {
    match kind {
        MemoryKind::Preference => MemoryGraphRelation::Preference,
        MemoryKind::Correction => MemoryGraphRelation::Correction,
        MemoryKind::RelationshipNote => MemoryGraphRelation::RelationshipNote,
        MemoryKind::ProjectContext => MemoryGraphRelation::ProjectContext,
        MemoryKind::Goal => MemoryGraphRelation::Goal,
        MemoryKind::EmotionalState => MemoryGraphRelation::EmotionalState,
        MemoryKind::Event => MemoryGraphRelation::Event,
        MemoryKind::PersonalFact | MemoryKind::ToolTraceSummary => MemoryGraphRelation::RelatedTo,
    }
}

fn endpoints(record: &MemoryRecord) -> [MemoryGraphNode; 2] {
    match record.entities.as_slice() {
        [from, to, ..] => [entity_node(from), entity_node(to)],
        [from] => [entity_node(from), scope_node(&record.scope)],
        [] => [scope_node(&record.scope), memory_node(&record.id)],
    }
}

fn entity_node(entity: &MemoryEntityRef) -> MemoryGraphNode {
    MemoryGraphNode {
        entity_type: entity.entity_type,
        id: entity.id.clone(),
        label: entity.label.clone(),
    }
}

fn scope_node(scope: &MemoryScope) -> MemoryGraphNode {
    let (entity_type, id) = match scope {
        MemoryScope::GlobalUser => (MemoryEntityType::User, "user:global".to_string()),
        MemoryScope::Workspace { root_fingerprint } => (
            MemoryEntityType::Workspace,
            format!("workspace:{root_fingerprint}"),
        ),
        MemoryScope::AgentIdentity => (MemoryEntityType::Agent, "agent:yunxi".to_string()),
        MemoryScope::Relationship => (
            MemoryEntityType::Relationship,
            "relationship:user-yunxi".to_string(),
        ),
    };
    MemoryGraphNode {
        entity_type,
        id,
        label: None,
    }
}

fn memory_node(memory_id: &str) -> MemoryGraphNode {
    MemoryGraphNode {
        entity_type: MemoryEntityType::Relationship,
        id: format!("memory:{memory_id}"),
        label: None,
    }
}

fn edge_for_record(
    record: &MemoryRecord,
    from: &str,
    to: &str,
    relation: MemoryGraphRelation,
    suffix: &str,
) -> MemoryGraphEdge {
    MemoryGraphEdge {
        id: format!("{}:{suffix}", record.id),
        from: from.to_string(),
        to: to.to_string(),
        relation,
        memory_id: record.id.clone(),
        event_at_millis: record.temporal.event_at_millis,
        observed_at_millis: record.temporal.observed_at_millis,
        updated_at_millis: record.updated_at_millis,
        created_at_millis: record.created_at_millis,
        valid_from_millis: record.temporal.valid_from_millis,
        expires_at_millis: record.temporal.expires_at_millis,
        invalidated_at_millis: record.invalidation.invalidated_at_millis,
        supersedes: record.invalidation.supersedes.clone(),
        superseded_by: record.invalidation.superseded_by.clone(),
        conflicts_with: record.invalidation.conflicts_with.clone(),
        active: record.status == MemoryStatus::Active,
    }
}
