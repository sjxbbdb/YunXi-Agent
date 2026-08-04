use async_trait::async_trait;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use yunxi_agent_companion::MailboxQuery;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentInput, AgentResult, ControlScope, ControlSource, MemoryExtractionMode,
};
use yunxi_agent_persona::{
    MemoryKind, MemoryRecord, MemoryScope, MemoryStatus, PersonaProfileStore, PersonaSettings,
    link_supersession_chain, now_millis,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderResponse, ProviderRole,
};
use yunxi_agent_runtime::{YunXiRuntimeBackend, general_companion_snapshot};
use yunxi_agent_storage::{
    FileCompanionMailboxStore, FilePersonaMemoryStore, InMemorySessionStore, PersonaMemoryScope,
};
use yunxi_agent_tools::NoopToolRuntime;

static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Default)]
struct CapturingProvider {
    messages: Arc<Mutex<Vec<ProviderMessage>>>,
}

#[derive(Clone, Default)]
struct LoveLetterProvider {
    love_letter_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl AgentProvider for LoveLetterProvider {
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse> {
        if request.input.prompt == "YunXi love letter composition" {
            self.love_letter_calls.fetch_add(1, Ordering::SeqCst);
            return Ok(ProviderResponse::assistant(
                r#"{"subject":"写给你","body":"我记得那些被认真确认过的小事，也珍惜我们一起把事情做好的时刻。"}"#,
            ));
        }
        Ok(ProviderResponse::assistant("正常回复不等待情书生成。"))
    }
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

#[tokio::test(flavor = "current_thread")]
async fn custom_persona_profile_reaches_runtime_provider_context() {
    let _guard = HOME_ENV_LOCK.lock().expect("home env lock");
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let previous_home = std::env::var_os("YUNXI_HOME");
    unsafe {
        std::env::set_var("YUNXI_HOME", home.path());
    }

    let source = home.path().join("custom-persona.json");
    fs::write(
        &source,
        r#"{
  "id": "runtime_custom",
  "display_name": "星河",
  "version": "1.0.0",
  "default_companion_strength": "strong",
  "layers": {
    "identity": "你是一个可靠的陪伴型 Agent。",
    "soul": "你珍视真实，也允许沉默存在。",
    "values": "诚实、尊重、边界感。",
    "voice": "使用中文，语气自然清晰。",
    "companion_style": "先理解，再帮助。",
    "work_style": "先检查，再改动。",
    "boundaries": "不编造记忆，不越过安全边界。",
    "addressing": "优先使用已确认的称呼。"
  },
  "constraints": []
}"#,
    )
    .expect("write persona fixture");
    let profile = PersonaProfileStore::import_file(&source).expect("import persona fixture");
    PersonaSettings {
        persona_enabled: true,
        memory_enabled: false,
        companion_enabled: false,
        love_letters_enabled: false,
        cloud_control_enabled: false,
        active_profile: profile.id,
    }
    .save()
    .expect("save custom persona settings");

    let provider = CapturingProvider::default();
    let captured = Arc::clone(&provider.messages);
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default());
    let result = Agent::new(AgentConfig::new(workspace.path()))
        .run_with_backend(&backend, AgentInput::text("你好"))
        .await;
    restore_env_var("YUNXI_HOME", previous_home);
    result.expect("custom persona runtime should complete");

    let system_context = captured
        .lock()
        .expect("messages lock")
        .iter()
        .filter(|message| message.role == ProviderRole::System)
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(system_context.contains("profile_id=\"runtime_custom\""));
    assert!(system_context.contains("<soul>你珍视真实，也允许沉默存在"));
}

#[tokio::test(flavor = "current_thread")]
async fn successful_reply_generates_encrypted_mailbox_item_without_memory_feedback() {
    let _guard = HOME_ENV_LOCK.lock().expect("home env lock");
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let previous_home = std::env::var_os("YUNXI_HOME");
    let previous_mailbox_key = std::env::var_os("YUNXI_MAILBOX_KEY_HEX");
    unsafe {
        std::env::set_var("YUNXI_HOME", home.path());
        std::env::set_var("YUNXI_MAILBOX_KEY_HEX", "11".repeat(32));
    }

    PersonaSettings {
        persona_enabled: true,
        memory_enabled: true,
        companion_enabled: true,
        love_letters_enabled: true,
        cloud_control_enabled: false,
        active_profile: "yunxi_companion_strong".to_string(),
    }
    .save()
    .expect("save settings");
    let memory_store = FilePersonaMemoryStore::for_workspace(workspace.path());
    let old = now_millis().saturating_sub(30 * 86_400_000);
    for record in [
        MemoryRecord::new(
            "letter-pref",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "用户喜欢真诚、克制的表达。",
            old,
        )
        .with_status(MemoryStatus::Active),
        MemoryRecord::new(
            "letter-relation",
            MemoryScope::Relationship,
            MemoryKind::RelationshipNote,
            "彼此已经建立了稳定的信任。",
            old + 1,
        )
        .with_status(MemoryStatus::Active),
        MemoryRecord::new(
            "letter-event",
            MemoryScope::Relationship,
            MemoryKind::Event,
            "一起完成了 YunXi 的一次重要集成测试。",
            old + 2,
        )
        .with_status(MemoryStatus::Active),
    ] {
        memory_store.append(&record).expect("append memory");
    }
    let memory_count_before = memory_store.list(PersonaMemoryScope::All).records.len();

    let provider = LoveLetterProvider::default();
    let love_letter_calls = Arc::clone(&provider.love_letter_calls);
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default());
    let mut config = AgentConfig::new(workspace.path())
        .with_memory_extraction_mode(MemoryExtractionMode::RuleOnly);
    config.companion.enabled = true;
    config.companion.love_letters.enabled = true;
    config.companion.love_letters.cooldown_min_days = 0;
    config.companion.love_letters.cooldown_max_days = 0;
    let result = Agent::new(config)
        .run_with_backend(&backend, AgentInput::text("今天也继续把事情做好。"))
        .await
        .expect("runtime completes");
    assert_eq!(
        result.final_response.as_deref(),
        Some("正常回复不等待情书生成。")
    );

    let mailbox = FileCompanionMailboxStore::for_workspace(workspace.path()).expect("mailbox");
    let item = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let page = mailbox.list(MailboxQuery::default()).expect("mailbox list");
            if let Some(item) = page.items.into_iter().next() {
                break item;
            }
            assert!(
                love_letter_calls.load(Ordering::SeqCst) <= 1,
                "worker must not duplicate model calls"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("love letter worker completes");
    let entry = mailbox
        .get(&item.item_id)
        .expect("mailbox get")
        .expect("mailbox entry");
    assert_eq!(entry.content.subject, "写给你");
    assert!(entry.content.body.contains("认真确认过的小事"));
    assert_eq!(love_letter_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        memory_store.list(PersonaMemoryScope::All).records.len(),
        memory_count_before,
        "generated letters must never feed back into long-term memory"
    );

    restore_env_var("YUNXI_MAILBOX_KEY_HEX", previous_mailbox_key);
    restore_env_var("YUNXI_HOME", previous_home);
}

async fn run_general_companion_scenario(workspace: &Path) -> AgentResult<()> {
    PersonaSettings {
        persona_enabled: true,
        memory_enabled: true,
        companion_enabled: false,
        love_letters_enabled: false,
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
    assert!(system_context.contains("yunxi_persona_context version=\"2.3.3\""));
    assert!(system_context.contains("reply_style_guidance"));
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
    assert_eq!(snapshot.version, env!("CARGO_PKG_VERSION"));
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
    assert!(snapshot.controls.memory_summary.contains("active=3"));
    assert!(snapshot.controls.memory_summary.contains("recallable=2"));
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
