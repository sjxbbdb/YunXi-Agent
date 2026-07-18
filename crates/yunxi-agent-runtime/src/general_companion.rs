use serde::{Deserialize, Serialize};
use yunxi_agent_core::{
    AgentConfig, AgentResult, CompanionSettings, ControlScope, ControlSnapshot, ControlSource,
};
use yunxi_agent_persona::{PersonaSettings, SCHEMA_VERSION, yunxi_companion_strong};

use crate::control_snapshot;

/// One auditable runtime view over the capabilities that make up the general
/// companion agent. The default runtime remains YunXi-owned and local; the
/// upstream Codex source checkout is not part of this path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneralCompanionSnapshot {
    pub version: String,
    pub runtime_owner: String,
    pub upstream_codex_required: bool,
    pub persona_profile_id: String,
    pub persona_enabled: bool,
    pub memory_enabled: bool,
    pub memory_schema_version: u32,
    pub relationship_read_only: bool,
    pub companion_enabled: bool,
    pub proactive_default_off: bool,
    pub cloud_control_enabled: bool,
    pub controls: ControlSnapshot,
}

pub fn general_companion_snapshot(config: &AgentConfig) -> AgentResult<GeneralCompanionSnapshot> {
    let settings = PersonaSettings::load();
    let profile = yunxi_companion_strong();
    let controls = control_snapshot(config)?;
    let relationship_read_only = controls
        .scope(ControlScope::Relationship)
        .is_some_and(|scope| {
            scope.enabled.is_none() && scope.source == ControlSource::ReadOnlyHistory
        });

    Ok(GeneralCompanionSnapshot {
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_owner: "yunxi".to_string(),
        upstream_codex_required: false,
        persona_profile_id: profile.id,
        persona_enabled: settings.persona_enabled,
        memory_enabled: settings.memory_enabled,
        memory_schema_version: SCHEMA_VERSION,
        relationship_read_only,
        companion_enabled: config.companion.enabled,
        proactive_default_off: !CompanionSettings::default().enabled,
        cloud_control_enabled: config.companion.cloud_control_enabled,
        controls,
    })
}
