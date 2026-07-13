pub mod compiler;
pub mod dedup;
pub mod extractor;
pub mod memory;
pub mod migration;
pub mod policy;
pub mod profile;
pub mod provider_extractor;
pub mod recall;
pub mod scope;
pub mod settings;

pub use compiler::{CompiledPersonaContext, PersonaPromptCompiler};
pub use dedup::{
    MemoryDedupKey, dedup_key_for_record, deduplicate_candidates, ensure_record_dedup_metadata,
    normalized_memory_content,
};
pub use extractor::MemoryRuleExtractor;
pub use memory::{
    MemoryCandidate, MemoryKind, MemoryRecallRequest, MemoryRecallResult, MemoryRecord,
    MemoryScope, MemorySensitivity, MemoryStatus, SCHEMA_VERSION, now_millis,
};
pub use migration::{MemoryMigrationResult, migrate_memory_record_value};
pub use policy::{MemoryPrivacyClassifier, MemoryWritePolicy, MemoryWritePolicyEngine};
pub use profile::{
    CompanionStrength, HumanProfile, PersonaConstraint, PersonaLayers, PersonaProfile,
    RelationshipFamiliarity, RelationshipState, yunxi_companion_strong,
};
pub use provider_extractor::ProviderMemoryExtractor;
pub use recall::MemoryRecallEngine;
pub use scope::MemoryScopeRouter;
pub use settings::{PersonaSettings, yunxi_home_dir};
