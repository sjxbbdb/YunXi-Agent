use async_trait::async_trait;
use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentInput, AgentResult, ControlScope, ControlSource, MemoryExtractionMode,
};
use yunxi_agent_persona::{
    MemoryKind, MemoryRecord, MemoryScope, MemoryStatus, PersonaSettings, link_supersession_chain,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderResponse, ProviderRole,
};
use yunxi_agent_runtime::{YunXiRuntimeBackend, general_companion_snapshot};
use yunxi_agent_storage::{FilePersonaMemoryStore, InMemorySessionStore, PersonaMemoryScope};
use yunxi_agent_tools::NoopToolRuntime;

static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Default)]
struct CapturingProvider {
    messages: Arc<Mutex<Vec<ProviderMessage>>>,
}

#[async_trait]
impl AgentProvider for CapturingProvider {
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse> {
        *self.messages.lock().expect("messages lock") = request.messages;
        Ok(ProviderResponse::assistant("general companion captured"))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn general_companion_closes_cross_session_memory_relationship_and_control_chain() {
    let _guard = HOME_ENV_LOCK.lock().expect("home env lock");
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let previous_home = std::env::var_os("YUNXI_HOME");
    unsafe {
        std::env::set_var("YUNXI_HOME", home.path());
    }

    let result = run_general_companion_scenario(workspace.path()).await;
    restore_env_var("YUNXI_HOME", previous_home);
    result.expect("general companion integration should complete");
}

async fn run_general_companion_scenario(workspace: &Path) -> AgentResult<()> {
    PersonaSettings {
        persona_enabled: true,
        memory_enabled: true,
        companion_enabled: false,
        cloud_control_enabled: false,
        active_profile: "yunxi_companion_strong".to_string(),
    }
    .save()
    .expect("save isolated settings");

    let first_backend = YunXiRuntimeBackend::with_parts(
        CapturingProvider::default(),
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    Agent::new(
        AgentConfig::new(workspace).with_memory_extraction_mode(MemoryExtractionMode::RuleOnly),
    )
    .run_with_backend(&first_backend, AgentInput::text("以后请用中文回答"))
    .await?;

    let memory_store = FilePersonaMemoryStore::for_workspace(workspace);
    let old = MemoryRecord::new(
        "relationship-old",
        MemoryScope::Relationship,
        MemoryKind::RelationshipNote,
        "Alex relationship is strained",
        10,
    )
    .with_status(MemoryStatus::Active);
    let new = MemoryRecord::new(
        "relationship-new",
        MemoryScope::Relationship,
        MemoryKind::RelationshipNote,
        "Alex relationship is trusting",
        20,
    )
    .with_status(MemoryStatus::Active);
    let (old, new) = link_supersession_chain(&old, &new, 30);
    memory_store.append(&old)?;
    memory_store.append(&new)?;

    let second_provider = CapturingProvider::default();
    let captured = Arc::clone(&second_provider.messages);
    let second_backend = YunXiRuntimeBackend::with_parts(
        second_provider,
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let config =
        AgentConfig::new(workspace).with_memory_extraction_mode(MemoryExtractionMode::RuleOnly);
    Agent::new(config.clone())
        .run_with_backend(
            &second_backend,
            AgentInput::text("What is the current Alex relationship and language preference?"),
        )
        .await?;

    let system_context = captured
        .lock()
        .expect("messages lock")
        .iter()
        .filter(|message| message.role == ProviderRole::System)
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(system_context.contains("yunxi_persona_context version=\"2.0.6\""));
    assert!(system_context.contains("Alex relationship is trusting"));
    assert!(!system_context.contains("Alex relationship is strained"));

    let loaded = memory_store.list(PersonaMemoryScope::All);
    assert!(loaded.records.iter().any(|record| {
        record.kind == MemoryKind::Preference && record.status == MemoryStatus::Active
    }));
    assert_eq!(
        loaded
            .records
            .iter()
            .find(|record| record.id == "relationship-old")
            .and_then(|record| record.invalidation.superseded_by.as_deref()),
        Some("relationship-new")
    );

    let snapshot = general_companion_snapshot(&config)?;
    assert_eq!(snapshot.version, "2.1.3");
    assert_eq!(snapshot.runtime_owner, "yunxi");
    assert!(!snapshot.upstream_codex_required);
    assert_eq!(snapshot.persona_profile_id, "yunxi_companion_strong");
    assert!(snapshot.persona_enabled);
    assert!(snapshot.memory_enabled);
    assert_eq!(snapshot.memory_schema_version, 3);
    assert!(snapshot.relationship_read_only);
    assert!(!snapshot.companion_enabled);
    assert!(snapshot.proactive_default_off);
    assert!(!snapshot.cloud_control_enabled);
    assert!(snapshot.controls.memory_summary.contains("active=2"));
    assert!(
        snapshot
            .controls
            .scope(ControlScope::Relationship)
            .is_some_and(|scope| scope.source == ControlSource::ReadOnlyHistory)
    );
    Ok(())
}

fn restore_env_var(name: &str, value: Option<OsString>) {
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
