pub mod compiler;
pub mod dedup;
pub mod extractor;
pub mod memory;
pub mod merge;
pub mod migration;
pub mod pipeline;
pub mod policy;
pub mod profile;
pub mod provider_extractor;
pub mod recall;
pub mod recall_router;
pub mod relationship_graph;
pub mod scope;
pub mod settings;

pub use compiler::{CompiledPersonaContext, PersonaPromptCompiler};
pub use dedup::{
    MemoryDedupKey, dedup_key_for_record, deduplicate_candidates, ensure_record_dedup_metadata,
    normalized_memory_content,
};
pub use extractor::MemoryRuleExtractor;
pub use memory::{
    MemoryCandidate, MemoryEntityRef, MemoryEntityType, MemoryEvidence, MemoryInvalidation,
    MemoryKind, MemoryLayer, MemoryRecallRequest, MemoryRecallResult, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemorySourceAttribution, MemoryStatus, MemoryTemporal,
    SCHEMA_VERSION, now_millis,
};
pub use merge::{
    MemoryMergeResult, MemoryMergeStrategy, memory_conflict_family,
    merge_equivalent_memory_records, merge_memory_candidates,
};
pub use migration::{MemoryMigrationResult, migrate_memory_record_value};
pub use pipeline::{
    MemoryPipeline, MemoryPipelineDiagnostic, MemoryPipelineInput, MemoryPipelineOutput,
    MemoryPipelineStageResult,
};
pub use policy::{
    MemoryPipelineLayer, MemoryPolicyContext, MemoryPrivacyClassifier, MemoryWritePolicy,
    MemoryWritePolicyEngine,
};
pub use profile::{
    CompanionStrength, HumanProfile, PersonaConstraint, PersonaLayers, PersonaProfile,
    RelationshipFamiliarity, RelationshipState, yunxi_companion_strong,
};
pub use provider_extractor::ProviderMemoryExtractor;
pub use recall::MemoryRecallEngine;
pub use recall_router::{
    MemoryRecallExplanation, MemoryRecallRoute, MemoryRecallRouter, MemoryRecallRouterRequest,
    MemoryRecallRouterResult,
};
pub use relationship_graph::{
    MemoryGraphEdge, MemoryGraphNode, MemoryGraphRelation, RelationshipGraphLite,
    is_relationship_timeline_query, link_supersession_chain, temporal_ordering_time,
};
pub use scope::MemoryScopeRouter;
pub use settings::{PersonaSettings, yunxi_home_dir};
