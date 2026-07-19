use std::ffi::OsString;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentMessageSequence,
    AgentMessageStreamPhase, AgentRunApprovalDecision, AgentRunControl, AgentRunStatus,
    AgentRunUserInputResponse, ApprovalMode, CommandStatus, CompanionSettings, FileChangeKind,
    MemoryExtractionMode, PatchStatus, SandboxMode,
};
use yunxi_agent_persona::{MemoryKind, MemoryRecord, MemoryScope, MemoryStatus};
use yunxi_agent_protocol::{
    ProtocolRole, ResponseItem, ResponseStatus, StreamEvent, ThreadId, ToolCall, TurnId,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderResponse, ProviderRole,
    ProviderStream, ProviderStreamEventSink, ProviderToolCall, StaticProvider,
};
use yunxi_agent_runtime::{
    YunXiRuntimeBackend, control_snapshot, protocol_stream_events_to_agent_events,
};
use yunxi_agent_storage::{
    FileControlStore, FilePersonaMemoryStore, InMemorySessionStore, PersonaMemoryScope, SessionId,
    SessionRecord, SessionStore,
};
use yunxi_agent_tools::{CompositeToolRuntime, NoopToolRuntime, ShellToolRuntime};

#[tokio::test(flavor = "current_thread")]
async fn companion_is_disabled_by_default() {
    let workspace = TempDir::new().expect("workspace");
    let backend = YunXiRuntimeBackend::with_parts(
        StaticProvider::default(),
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let result = Agent::new(AgentConfig::new(workspace.path()))
        .run_with_backend(&backend, AgentInput::text("reminder due"))
        .await
        .expect("runtime should complete");
    assert!(!result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Message { content, .. } if content.contains("[提醒]")
    )));
}

#[tokio::test(flavor = "current_thread")]
async fn companion_check_emits_reasoned_plan_without_tool_execution() {
    let workspace = TempDir::new().expect("workspace");
    let backend = YunXiRuntimeBackend::with_parts(
        StaticProvider::default(),
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let config = AgentConfig {
        companion: CompanionSettings {
            enabled: true,
            allow_tool_requests: true,
            ..CompanionSettings::default()
        },
        ..AgentConfig::new(workspace.path())
    };
    let result = Agent::new(config)
        .run_with_backend(&backend, AgentInput::text("tool request: open notes"))
        .await
        .expect("runtime should complete");
    let messages = result
        .events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::Message { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("需要确认的工具建议"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("不会自动执行"))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn companion_plan_is_recorded_and_visible_in_control_snapshot() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let workspace_path = workspace.path().to_path_buf();
    let snapshot = with_yunxi_home(home.path(), async move {
        let backend = YunXiRuntimeBackend::with_parts(
            StaticProvider::default(),
            NoopToolRuntime,
            InMemorySessionStore::default(),
        );
        let config = AgentConfig {
            companion: CompanionSettings {
                enabled: true,
                ..CompanionSettings::default()
            },
            ..AgentConfig::new(&workspace_path)
        };
        Agent::new(config.clone())
            .run_with_backend(&backend, AgentInput::text("reminder due"))
            .await
            .expect("runtime should complete");
        let history = FileControlStore::for_workspace(&workspace_path)
            .companion_history()
            .expect("companion history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].trigger, "reminder_due");
        control_snapshot(&config).expect("control snapshot")
    })
    .await;

    assert!(snapshot.companion_enabled);
    assert!(
        snapshot
            .scope(yunxi_agent_core::ControlScope::Companion)
            .is_some_and(|state| state.summary == "history_records=1")
    );
}

static MEMORY_ENV_LOCK: Mutex<()> = Mutex::new(());

async fn with_yunxi_home<T>(home: &Path, future: impl Future<Output = T>) -> T {
    let _guard = MEMORY_ENV_LOCK.lock().expect("memory env lock");
    let previous_home = std::env::var_os("YUNXI_HOME");
    let previous_companion = std::env::var_os("YUNXI_COMPANION_ENABLED");
    unsafe {
        std::env::set_var("YUNXI_HOME", home);
        std::env::remove_var("YUNXI_COMPANION_ENABLED");
    }
    let output = future.await;
    restore_env_var("YUNXI_HOME", previous_home);
    restore_env_var("YUNXI_COMPANION_ENABLED", previous_companion);
    output
}

async fn with_memory_enabled_home<T>(home: &Path, future: impl Future<Output = T>) -> T {
    let _guard = MEMORY_ENV_LOCK.lock().expect("memory env lock");
    let previous_home = std::env::var_os("YUNXI_HOME");
    let previous_memory = std::env::var_os("YUNXI_MEMORY_ENABLED");
    unsafe {
        std::env::set_var("YUNXI_HOME", home);
        std::env::set_var("YUNXI_MEMORY_ENABLED", "1");
    }
    let output = future.await;
    restore_env_var("YUNXI_HOME", previous_home);
    restore_env_var("YUNXI_MEMORY_ENABLED", previous_memory);
    output
}

#[tokio::test(flavor = "current_thread")]
async fn runtime_emits_boot_and_dynamic_recall_summaries() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let workspace_path = workspace.path().to_path_buf();
    let result = with_memory_enabled_home(home.path(), async move {
        let store = FilePersonaMemoryStore::for_workspace(&workspace_path);
        let preference = MemoryRecord::new(
            "boot-preference",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "Prefer concise English responses",
            1,
        )
        .with_scores(0.9, 0.8)
        .with_status(MemoryStatus::Active);
        store.append(&preference).expect("append boot memory");

        let backend = YunXiRuntimeBackend::with_parts(
            StaticProvider::default(),
            NoopToolRuntime,
            InMemorySessionStore::default(),
        );
        Agent::new(AgentConfig::new(&workspace_path))
            .run_with_backend(&backend, AgentInput::text("release API status"))
            .await
    })
    .await
    .expect("runtime should complete");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryRecall { scope, count: 1, .. } if scope == "boot"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryRecall { scope, .. } if scope == "dynamic"
    )));
}

fn restore_env_var(name: &str, value: Option<OsString>) {
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}

#[cfg(windows)]
fn long_running_shell_command() -> &'static str {
    "ping -n 6 127.0.0.1 > nul"
}

#[cfg(not(windows))]
fn long_running_shell_command() -> &'static str {
    "sleep 5"
}

#[tokio::test]
async fn yunxi_runtime_runs_without_codex_backend() {
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(StaticProvider::default(), NoopToolRuntime, store.clone());
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("explain this project"))
        .await
        .expect("yunxi runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("YunXi autonomous runtime accepted prompt: explain this project")
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ThreadStarted { .. }))
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Started { prompt } if prompt == "explain this project"
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[derive(Clone, Default)]
struct PatchCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for PatchCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "patch result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Patch {
            id: Some("patch-1".to_string()),
            patch: r#"{"op":"write","path":"runtime-patch.txt","content":"patched"}"#.to_string(),
        }))
    }
}

#[derive(Clone, Default)]
struct CapturingProvider {
    messages: Arc<Mutex<Vec<ProviderMessage>>>,
}

#[async_trait::async_trait]
impl AgentProvider for CapturingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        *self.messages.lock().expect("messages lock") =
            request.messages.iter().cloned().collect::<Vec<_>>();
        Ok(ProviderResponse::assistant("captured"))
    }
}

#[derive(Clone)]
struct MemoryExtractionFixtureProvider {
    memory_response: &'static str,
    memory_delay: Duration,
    memory_calls: Arc<AtomicUsize>,
}

impl MemoryExtractionFixtureProvider {
    fn new(memory_response: &'static str) -> Self {
        Self {
            memory_response,
            memory_delay: Duration::ZERO,
            memory_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.memory_delay = delay;
        self
    }

    fn memory_calls(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.memory_calls)
    }
}

#[async_trait::async_trait]
impl AgentProvider for MemoryExtractionFixtureProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if request.input.prompt == "YunXi memory extraction" {
            self.memory_calls.fetch_add(1, Ordering::SeqCst);
            if !self.memory_delay.is_zero() {
                tokio::time::sleep(self.memory_delay).await;
            }
            return Ok(ProviderResponse::assistant(self.memory_response));
        }
        Ok(ProviderResponse::assistant("fixture main response"))
    }
}

#[tokio::test]
async fn yunxi_runtime_injects_agents_md_before_user_prompt() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("AGENTS.md"), "Use YunXi instructions").expect("agents file");
    let provider = CapturingProvider::default();
    let messages = Arc::clone(&provider.messages);
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default());
    let agent = Agent::new(AgentConfig::new(temp.path()));

    agent
        .run_with_backend(&backend, AgentInput::text("hello"))
        .await
        .expect("runtime should complete");

    let captured = messages.lock().expect("messages lock").clone();
    assert_eq!(
        captured.first().map(|message| message.role),
        Some(ProviderRole::System)
    );
    assert!(
        captured
            .first()
            .is_some_and(|message| message.content.contains("Use YunXi instructions"))
    );
    assert_eq!(
        captured.last().map(|message| message.role),
        Some(ProviderRole::User)
    );
}

#[tokio::test]
async fn yunxi_runtime_injects_mentioned_workspace_file_context() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(temp.path().join("src")).expect("src dir");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn marker() {}\n").expect("source file");
    let provider = CapturingProvider::default();
    let messages = Arc::clone(&provider.messages);
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default());
    let agent = Agent::new(AgentConfig::new(temp.path()));

    agent
        .run_with_backend(&backend, AgentInput::text("inspect @src/lib.rs"))
        .await
        .expect("runtime should complete");

    let captured = messages.lock().expect("messages lock").clone();
    assert!(captured.iter().any(|message| {
        message.role == ProviderRole::System
            && message.content.contains("Mentioned workspace file context")
            && message.content.contains("pub fn marker")
    }));
    assert_eq!(
        captured.last().map(|message| message.content.as_str()),
        Some("inspect @src/lib.rs")
    );
}

#[tokio::test]
async fn yunxi_runtime_records_parent_session_metadata() {
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(StaticProvider::default(), NoopToolRuntime, store.clone());
    let agent = Agent::new(
        AgentConfig::new(PathBuf::from("."))
            .with_parent_session_id("parent-session")
            .with_session_title("Resume parent-session")
            .with_approval_mode(ApprovalMode::Never),
    );

    agent
        .run_with_backend(&backend, AgentInput::text("continue work"))
        .await
        .expect("yunxi runtime should complete");

    let sessions = store.list().await.expect("session list");
    assert_eq!(sessions.len(), 1);
    assert_eq!(
        sessions[0].parent_id.as_ref().map(|id| id.0.as_str()),
        Some("parent-session")
    );
    assert_eq!(sessions[0].title.as_deref(), Some("Resume parent-session"));
}

#[tokio::test]
async fn yunxi_runtime_restores_parent_session_history_before_current_prompt() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let mut root = SessionRecord::new(
        temp.path(),
        "root prompt",
        Some("root answer".to_string()),
        vec![],
    );
    root.id = SessionId::new("root");
    store.save(root.clone()).await.expect("save root");

    let mut child = SessionRecord::new(
        temp.path(),
        "child prompt",
        Some("child answer".to_string()),
        vec![],
    )
    .with_parent_id(root.id.clone());
    child.id = SessionId::new("child");
    store.save(child.clone()).await.expect("save child");

    let provider = CapturingProvider::default();
    let messages = Arc::clone(&provider.messages);
    let backend = YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, store);
    let agent = Agent::new(
        AgentConfig::new(temp.path())
            .with_parent_session_id("child")
            .with_approval_mode(ApprovalMode::Never),
    );

    agent
        .run_with_backend(&backend, AgentInput::text("continue work"))
        .await
        .expect("runtime should complete");

    let captured = messages.lock().expect("messages lock").clone();
    assert!(
        captured
            .iter()
            .any(|message| message.content == "root prompt")
    );
    assert!(
        captured
            .iter()
            .any(|message| message.content == "root answer")
    );
    assert!(
        captured
            .iter()
            .any(|message| message.content == "child prompt")
    );
    assert!(
        captured
            .iter()
            .any(|message| message.content == "child answer")
    );
    assert_eq!(
        captured.last().map(|message| message.content.as_str()),
        Some("continue work")
    );
}

#[tokio::test]
async fn yunxi_runtime_injects_mentioned_file_context_before_restored_history() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("notes.md"), "mentioned file marker").expect("notes file");
    let store = InMemorySessionStore::default();
    let mut root = SessionRecord::new(
        temp.path(),
        "root prompt",
        Some("root answer".to_string()),
        vec![],
    );
    root.id = SessionId::new("root");
    store.save(root).await.expect("save root");

    let provider = CapturingProvider::default();
    let messages = Arc::clone(&provider.messages);
    let backend = YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, store);
    let agent = Agent::new(
        AgentConfig::new(temp.path())
            .with_parent_session_id("root")
            .with_approval_mode(ApprovalMode::Never),
    );

    agent
        .run_with_backend(&backend, AgentInput::text("inspect @notes.md"))
        .await
        .expect("runtime should complete");

    let captured = messages.lock().expect("messages lock").clone();
    let file_index = captured
        .iter()
        .position(|message| message.content.contains("mentioned file marker"))
        .expect("mentioned file context");
    let history_index = captured
        .iter()
        .position(|message| message.content == "root prompt")
        .expect("restored history prompt");

    assert!(file_index < history_index);
    assert_eq!(
        captured.last().map(|message| message.content.as_str()),
        Some("inspect @notes.md")
    );
}

#[tokio::test]
async fn yunxi_runtime_compacts_restored_history_when_budget_is_exceeded() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let mut root = SessionRecord::new(
        temp.path(),
        "root prompt with enough words to exceed a tiny compact budget",
        Some("root answer with enough words to exceed a tiny compact budget".to_string()),
        vec![],
    );
    root.id = SessionId::new("root");
    store.save(root.clone()).await.expect("save root");

    let provider = CapturingProvider::default();
    let messages = Arc::clone(&provider.messages);
    let backend = YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, store);
    let agent = Agent::new(
        AgentConfig::new(temp.path())
            .with_parent_session_id("root")
            .with_context_window_tokens(16)
            .with_auto_compact_threshold_tokens(12)
            .with_approval_mode(ApprovalMode::Never),
    );

    agent
        .run_with_backend(&backend, AgentInput::text("continue compacted work"))
        .await
        .expect("runtime should complete");

    let captured = messages.lock().expect("messages lock").clone();
    assert!(captured.iter().any(
        |message| message.role == ProviderRole::System && message.content.contains("Compacted")
    ));
    assert_eq!(
        captured.last().map(|message| message.content.as_str()),
        Some("continue compacted work")
    );
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_patch_tool() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(PatchCallingProvider, ShellToolRuntime, store.clone());
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("patch a file"))
        .await
        .expect("yunxi runtime should complete patch loop");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("runtime-patch.txt")).expect("patched file"),
        "patched"
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::PatchCompleted {
            status: PatchStatus::Completed
        }
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::FileChanged {
            path,
            kind: FileChangeKind::Add
        } if path == "runtime-patch.txt"
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn yunxi_runtime_rejects_empty_prompt() {
    let backend = YunXiRuntimeBackend::default();
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));

    let error = agent
        .run_with_backend(&backend, AgentInput::text("   "))
        .await
        .expect_err("empty prompt should be rejected");

    assert!(matches!(error, AgentError::EmptyPrompt));
}

#[derive(Clone, Default)]
struct ShellCallingProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl AgentProvider for ShellCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "tool said: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
            id: Some("shell-1".to_string()),
            command: "echo yunxi-tool".to_string(),
        }))
    }
}

#[derive(Clone, Default)]
struct StreamingShellCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for StreamingShellCallingProvider {
    async fn complete(
        &self,
        _request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        Ok(ProviderResponse::assistant(
            "streaming fallback should not be used",
        ))
    }

    async fn stream(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> yunxi_agent_core::AgentResult<ProviderStream> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderStream::from_events(vec![
                StreamEvent::ResponseStarted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    metadata: None,
                },
                StreamEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: ResponseItem::Message {
                        role: yunxi_agent_protocol::ProtocolRole::Assistant,
                        content: format!("streamed tool said: {}", tool_message.content.trim()),
                    },
                },
                StreamEvent::ResponseCompleted {
                    thread_id,
                    turn_id,
                    status: ResponseStatus::Completed,
                },
            ]));
        }

        Ok(ProviderStream::from_events(vec![
            StreamEvent::ResponseStarted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                metadata: None,
            },
            StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::ToolCall {
                    call: ToolCall::Shell {
                        id: Some("stream-shell-1".to_string()),
                        command: "echo streamed-yunxi-tool".to_string(),
                    },
                },
            },
            StreamEvent::ResponseCompleted {
                thread_id,
                turn_id,
                status: ResponseStatus::Completed,
            },
        ]))
    }
}

#[derive(Clone, Default)]
struct SlowStreamingProvider;

#[async_trait::async_trait]
impl AgentProvider for SlowStreamingProvider {
    async fn complete(
        &self,
        _request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        Ok(ProviderResponse::assistant("slow complete fallback"))
    }

    async fn stream(
        &self,
        _request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> yunxi_agent_core::AgentResult<ProviderStream> {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(ProviderStream::from_events(vec![
            StreamEvent::ResponseStarted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                metadata: None,
            },
            StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::Message {
                    role: ProtocolRole::Assistant,
                    content: "slow streamed response".to_string(),
                },
            },
            StreamEvent::ResponseCompleted {
                thread_id,
                turn_id,
                status: ResponseStatus::Completed,
            },
        ]))
    }
}

#[derive(Clone, Default)]
struct IncrementalStreamingProvider;

#[async_trait::async_trait]
impl AgentProvider for IncrementalStreamingProvider {
    async fn complete(
        &self,
        _request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        Ok(ProviderResponse::assistant("incremental complete fallback"))
    }

    async fn stream_with_sink(
        &self,
        _request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
        mut sink: Option<&mut dyn ProviderStreamEventSink>,
    ) -> yunxi_agent_core::AgentResult<ProviderStream> {
        let mut events = vec![StreamEvent::ResponseStarted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            metadata: None,
        }];
        let first = StreamEvent::ItemDelta {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            delta: yunxi_agent_protocol::response_text_delta("hel"),
            metadata: None,
        };
        if let Some(sink) = sink.as_mut() {
            sink.emit(first.clone()).await?;
        }
        events.push(first);
        tokio::time::sleep(Duration::from_millis(200)).await;
        events.push(StreamEvent::ItemDelta {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            delta: yunxi_agent_protocol::response_text_delta("lo"),
            metadata: None,
        });
        events.push(StreamEvent::ResponseCompleted {
            thread_id,
            turn_id,
            status: ResponseStatus::Completed,
        });
        if let Some(sink) = sink.as_mut() {
            for event in events.iter().skip(2) {
                sink.emit(event.clone()).await?;
            }
        }
        Ok(ProviderStream::from_events(events))
    }
}

#[derive(Clone, Default)]
struct RequestUserInputProvider;

#[async_trait::async_trait]
impl AgentProvider for RequestUserInputProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "user input was {}",
                tool_message.content.trim()
            )));
        }
        Ok(ProviderResponse::tool_call(
            ProviderToolCall::RequestUserInput {
                id: Some("input-1".to_string()),
                prompt: "enter value:".to_string(),
            },
        ))
    }
}

#[derive(Clone)]
struct ApprovalShellProvider {
    command: String,
}

impl ApprovalShellProvider {
    fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

#[async_trait::async_trait]
impl AgentProvider for ApprovalShellProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "approval tool result: {}",
                tool_message.content.trim()
            )));
        }
        Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
            id: Some("approval-shell-1".to_string()),
            command: self.command.clone(),
        }))
    }
}

#[derive(Clone, Default)]
struct FailingShellProvider;

#[async_trait::async_trait]
impl AgentProvider for FailingShellProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "tool said: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
            id: Some("shell-1".to_string()),
            command: "exit 7".to_string(),
        }))
    }
}

#[derive(Clone, Default)]
struct WriteShellProvider;

#[async_trait::async_trait]
impl AgentProvider for WriteShellProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "tool said: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
            id: Some("shell-write-1".to_string()),
            command: "echo denied > denied.txt".to_string(),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_shell_tool() {
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        ShellCallingProvider::default(),
        ShellToolRuntime,
        store.clone(),
    );
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use a tool"))
        .await
        .expect("yunxi runtime should complete tool loop");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("yunxi-tool")
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Reasoning {
            content
        } if content.contains("Tool dispatch routed shell")
            && content.contains("policy=approved")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandStarted {
            id: Some(id),
            command
        } if id == "shell-1" && command == "echo yunxi-tool"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            id: Some(id),
            status: CommandStatus::Completed,
            aggregated_output,
            ..
        } if id == "shell-1" && aggregated_output.contains("yunxi-tool")
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn run_stream_emits_events_before_slow_provider_completes() {
    let backend = YunXiRuntimeBackend::with_parts(
        SlowStreamingProvider,
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(&backend, AgentInput::text("slow stream"), run_control)
            .await
    });

    let first_event = stream.events.recv().await.expect("streamed event");
    assert!(matches!(
        first_event,
        AgentEvent::ThreadStarted { .. } | AgentEvent::TurnStarted | AgentEvent::Started { .. }
    ));
    assert!(!handle.is_finished());

    let result = handle.await.expect("join stream run").expect("stream run");
    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("slow streamed response")
    );
}

#[tokio::test]
async fn run_stream_emits_provider_delta_before_provider_completes() {
    let backend = YunXiRuntimeBackend::with_parts(
        IncrementalStreamingProvider,
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(
                &backend,
                AgentInput::text("incremental stream"),
                run_control,
            )
            .await
    });

    let mut saw_first_delta = false;
    while let Some(event) = stream.events.recv().await {
        if matches!(event, AgentEvent::Message { content, .. } if content == "hel") {
            saw_first_delta = true;
            break;
        }
    }

    assert!(saw_first_delta);
    assert!(!handle.is_finished());
    let result = handle.await.expect("join stream run").expect("stream run");
    assert_eq!(result.final_response.as_deref(), Some("hello"));
    let assistant_stream = result
        .events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::Message { content, stream } => {
                Some((content.as_str(), stream.as_ref().expect("message stream")))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        assistant_stream
            .iter()
            .map(|(content, _)| *content)
            .collect::<Vec<_>>(),
        vec!["hel", "lo", "hello"]
    );
    assert!(
        assistant_stream
            .windows(2)
            .all(|pair| pair[0].1.stream_id == pair[1].1.stream_id)
    );
    assert_eq!(assistant_stream[2].1.phase, AgentMessageStreamPhase::Final);
}

#[tokio::test]
async fn cancellation_drops_active_provider_stream_and_preserves_partial_text() {
    let backend = YunXiRuntimeBackend::with_parts(
        IncrementalStreamingProvider,
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(
                &backend,
                AgentInput::text("cancel incremental stream"),
                run_control,
            )
            .await
    });

    while let Some(event) = stream.events.recv().await {
        if matches!(event, AgentEvent::Message { content, .. } if content == "hel") {
            break;
        }
    }
    control.cancel();

    let result = tokio::time::timeout(Duration::from_millis(100), handle)
        .await
        .expect("provider future should be dropped immediately")
        .expect("join cancelled provider run")
        .expect("cancelled provider run");

    assert_eq!(result.status, AgentRunStatus::Cancelled);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Message { content, .. } if content == "hel"
    )));
    assert!(!result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Message { content, .. } if content == "lo" || content == "hello"
    )));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::Cancelled { .. }))
    );
}

#[tokio::test]
async fn run_stream_routes_interactive_approval_to_tool_execution() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::with_parts(
        ApprovalShellProvider::new("echo approval-ok"),
        ShellToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent =
        Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::OnRequest));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(&backend, AgentInput::text("approval run"), run_control)
            .await
    });

    let request = stream.approvals.recv().await.expect("approval request");
    assert_eq!(request.tool_name, "shell");
    let _ = request.respond_to.send(AgentRunApprovalDecision {
        approved: true,
        reason: Some("approved by test".to_string()),
    });

    let result = handle
        .await
        .expect("join approval run")
        .expect("approval run");
    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(matches!(
        result.final_response.as_deref(),
        Some(response) if response.contains("approval-ok")
    ));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ApprovalCompleted { approved: true, .. }))
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            status: CommandStatus::Completed,
            ..
        }
    )));
}

#[tokio::test]
async fn run_stream_routes_interactive_approval_denial_to_declined_tool_result() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::with_parts(
        ApprovalShellProvider::new("echo should-not-run"),
        ShellToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent =
        Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::OnRequest));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(&backend, AgentInput::text("approval deny"), run_control)
            .await
    });

    let request = stream.approvals.recv().await.expect("approval request");
    let _ = request.respond_to.send(AgentRunApprovalDecision {
        approved: false,
        reason: Some("declined by test".to_string()),
    });

    let result = handle.await.expect("join denial run").expect("denial run");
    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(matches!(
        result.final_response.as_deref(),
        Some(response) if response.contains("declined by test")
    ));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ApprovalCompleted {
            approved: false,
            reason: Some(reason),
            ..
        } if reason.contains("declined by test")
    )));
}

#[tokio::test]
async fn run_stream_routes_request_user_input_to_provider_tool_result() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::with_parts(
        RequestUserInputProvider,
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(&backend, AgentInput::text("need input"), run_control)
            .await
    });

    let request = stream.user_inputs.recv().await.expect("user input request");
    assert_eq!(request.prompt, "enter value:");
    let _ = request.respond_to.send(AgentRunUserInputResponse {
        value: Some("yunxi-user-value".to_string()),
    });

    let result = handle
        .await
        .expect("join user input run")
        .expect("user input run");
    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("user input was yunxi-user-value")
    );
}

#[tokio::test]
async fn run_stream_cancellation_reaches_running_shell_exec() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        ApprovalShellProvider::new(long_running_shell_command()),
        ShellToolRuntime,
        store.clone(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));
    let (control, mut stream) = AgentRunControl::streaming();
    let run_control = control.clone();
    let handle = tokio::spawn(async move {
        agent
            .run_with_backend_stream(&backend, AgentInput::text("cancel shell"), run_control)
            .await
    });

    loop {
        match stream.events.recv().await.expect("stream event") {
            AgentEvent::CommandStarted { .. } => break,
            _ => {}
        }
    }
    control.cancel();

    let result = handle
        .await
        .expect("join cancellation run")
        .expect("cancellation run");
    assert_eq!(result.status, AgentRunStatus::Cancelled);
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::Cancelled { .. }))
    );
    assert_eq!(store.list().await.expect("session list").len(), 0);
}

#[tokio::test]
async fn yunxi_runtime_executes_streamed_provider_tool_call() {
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        StreamingShellCallingProvider,
        ShellToolRuntime,
        store.clone(),
    );
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use a streamed tool"))
        .await
        .expect("yunxi runtime should complete streamed tool loop");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("streamed-yunxi-tool")
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            id: Some(id),
            status: CommandStatus::Completed,
            ..
        } if id == "stream-shell-1"
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn yunxi_runtime_reports_failed_shell_tool_with_warning() {
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(FailingShellProvider, ShellToolRuntime, store.clone());
    let agent =
        Agent::new(AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use a failing tool"))
        .await
        .expect("yunxi runtime should complete tool loop");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("exit code 7")
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::Warning { message } if message.contains("exit code 7")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            id: Some(id),
            status: CommandStatus::Failed,
            ..
        } if id == "shell-1"
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn yunxi_runtime_emits_escalation_events_when_sandbox_blocks_tool() {
    let workspace = TempDir::new().expect("workspace");
    let backend = YunXiRuntimeBackend::with_parts(
        WriteShellProvider,
        ShellToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(
        AgentConfig::new(workspace.path())
            .with_approval_mode(ApprovalMode::Never)
            .with_sandbox_mode(SandboxMode::ReadOnly),
    );

    let result = agent
        .run_with_backend(&backend, AgentInput::text("write through a guarded tool"))
        .await
        .expect("runtime should complete sandbox-blocked tool loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::EscalationRequested {
            id: Some(id),
            tool_name,
            reason,
            required_sandbox: Some(required_sandbox),
            required_network: None,
        } if id == "shell-write-1"
            && tool_name == "shell"
            && reason.contains("read-only")
            && required_sandbox == "workspace-write"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::EscalationCompleted {
            id: Some(id),
            approved: false,
            reason: Some(reason),
        } if id == "shell-write-1" && reason.contains("read-only")
    )));
    assert!(!workspace.path().join("denied.txt").exists());
}

#[tokio::test]
async fn yunxi_runtime_emits_approval_events_when_tool_requires_approval() {
    let backend = YunXiRuntimeBackend::with_parts(
        ShellCallingProvider::default(),
        CompositeToolRuntime::default(),
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(
        AgentConfig::new(PathBuf::from(".")).with_approval_mode(ApprovalMode::OnRequest),
    );

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use a guarded tool"))
        .await
        .expect("runtime should complete guarded tool loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ApprovalRequested {
            id: Some(id),
            tool_name,
            reason,
        } if id == "shell-1" && tool_name == "shell" && reason.contains("approval")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ApprovalCompleted {
            id: Some(id),
            approved: false,
            reason: Some(reason),
        } if id == "shell-1" && reason.contains("approval")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            id: Some(id),
            status: CommandStatus::Declined,
            ..
        } if id == "shell-1"
    )));
}

#[derive(Clone, Default)]
struct ToolSearchCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for ToolSearchCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "search result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::ToolSearch {
            id: Some("search-1".to_string()),
            query: "runtime".to_string(),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_dynamic_tool_search() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("runtime-notes.md"), "notes").expect("file");
    let backend = YunXiRuntimeBackend::with_parts(
        ToolSearchCallingProvider,
        ShellToolRuntime,
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("search tools"))
        .await
        .expect("runtime should complete dynamic tool loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolCallStarted {
            id: Some(id),
            name,
            ..
        } if id == "search-1" && name == "tool_search"
    )));
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("runtime-notes.md")
    );
}

#[derive(Clone, Default)]
struct SkillCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for SkillCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "skill result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Skill {
            id: Some("skill-1".to_string()),
            name: "writer".to_string(),
            arguments_json: Some(r#"{"topic":"runtime"}"#.to_string()),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_skill_tool() {
    let temp = TempDir::new().expect("temp dir");
    let skill_dir = temp.path().join(".codex/skills/writer");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: writer\ndescription: writes reports\n---\n# Writer\n",
    )
    .expect("skill file");
    let backend = YunXiRuntimeBackend::with_parts(
        SkillCallingProvider,
        CompositeToolRuntime::default(),
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use skill"))
        .await
        .expect("runtime should complete skill loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolCallCompleted {
            id: Some(id),
            name,
            status: CommandStatus::Completed,
            output,
        } if id == "skill-1" && name == "skill:writer" && output.contains("# Writer")
    )));
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("# Writer")
    );
}

#[derive(Clone, Default)]
struct MultiAgentCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for MultiAgentCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "multi-agent result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::MultiAgent {
            id: Some("agent-call-1".to_string()),
            action: "spawn".to_string(),
            arguments_json: Some(r#"{"task":"review runtime"}"#.to_string()),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_multi_agent_tool() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::with_parts(
        MultiAgentCallingProvider,
        CompositeToolRuntime::default(),
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("spawn reviewer"))
        .await
        .expect("runtime should complete multi-agent loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolCallCompleted {
            id: Some(id),
            name,
            status: CommandStatus::Completed,
            output,
        } if id == "agent-call-1" && name == "multi_agent:spawn" && output.contains("agent-1")
    )));
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("review runtime")
    );
}

#[derive(Clone, Default)]
struct MultiAgentSpawnRunProvider;

#[async_trait::async_trait]
impl AgentProvider for MultiAgentSpawnRunProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "child result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::MultiAgent {
            id: Some("agent-run-1".to_string()),
            action: "spawn_run".to_string(),
            arguments_json: Some(r#"{"task":"review runtime deeply"}"#.to_string()),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_emits_multi_agent_events_from_spawn_run() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        MultiAgentSpawnRunProvider,
        CompositeToolRuntime::default(),
        store.clone(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("spawn and run reviewer"))
        .await
        .expect("runtime should complete child run loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MultiAgentEvent {
            agent_id,
            status,
            message: Some(message),
            ..
        } if agent_id == "agent-1" && status == "completed" && message.contains("review runtime deeply")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            parent_session_id: Some(parent_session_id),
            status,
            message: Some(message),
        } if agent_id == "agent-1"
            && child_session_id == "agent-1-session"
            && !parent_session_id.is_empty()
            && status == "completed"
            && message.contains("YunXi child agent agent-1 completed task")
    )));

    let sessions = store.list().await.expect("sessions");
    assert_eq!(sessions.len(), 2);
    let parent = sessions
        .iter()
        .find(|session| session.prompt == "spawn and run reviewer")
        .expect("parent session");
    let child = sessions
        .iter()
        .find(|session| session.id.0 == "agent-1-session")
        .expect("child session");
    assert_eq!(child.parent_id.as_ref(), Some(&parent.id));
    assert!(parent.events.iter().any(|event| matches!(
        event,
        AgentEvent::StorageState {
            child_session_ids,
            ..
        } if child_session_ids == &vec!["agent-1-session".to_string()]
    )));
}

#[tokio::test]
async fn yunxi_runtime_returns_structured_child_failure_at_depth_limit() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::with_parts(
        MultiAgentSpawnRunProvider,
        CompositeToolRuntime::default(),
        InMemorySessionStore::default(),
    )
    .with_max_child_depth(0);
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("spawn and run reviewer"))
        .await
        .expect("parent runtime should keep running after child failure");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            status,
            message: Some(message),
            ..
        } if agent_id == "agent-1"
            && child_session_id == "agent-1-session"
            && status == "failed"
            && message.contains("recursion depth exceeded")
    )));
}

#[derive(Clone, Default)]
struct McpCallingProvider;

#[async_trait::async_trait]
impl AgentProvider for McpCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "mcp result: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Mcp {
            id: Some("mcp-1".to_string()),
            server: "local".to_string(),
            tool: "echo".to_string(),
            arguments_json: Some(r#"{"text":"ping"}"#.to_string()),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_mcp_tool_from_workspace_seed() {
    let temp = TempDir::new().expect("temp dir");
    let seed_dir = temp.path().join(".yunxi");
    std::fs::create_dir_all(&seed_dir).expect("seed dir");
    std::fs::write(
        seed_dir.join("mcp-runtime.json"),
        serde_json::json!({
            "snapshot": {
                "servers": {
                    "local": {
                        "config": {
                            "name": "local",
                            "transport": {"type": "stdio", "command": "fixture", "args": []},
                            "enabled": true
                        },
                        "resources": [],
                        "tools": [
                            {
                                "server": "local",
                                "name": "echo",
                                "title": "Echo",
                                "description": "fixture echo",
                                "input_schema": {"type": "object"},
                                "destructive_hint": false,
                                "open_world_hint": false,
                                "requires_approval": false
                            }
                        ],
                        "auth_status": "authenticated"
                    }
                },
                "plugins_available": false,
                "available_environment_ids": []
            },
            "tool_results": [
                {"server": "local", "tool": "echo", "content": "runtime-pong"}
            ]
        })
        .to_string(),
    )
    .expect("seed file");
    let backend = YunXiRuntimeBackend::with_parts(
        McpCallingProvider,
        CompositeToolRuntime::default(),
        InMemorySessionStore::default(),
    );
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use mcp"))
        .await
        .expect("runtime should complete mcp loop");

    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::McpToolCompleted {
            id: Some(id),
            server,
            tool,
            ..
        } if id == "mcp-1" && server == "local" && tool == "echo"
    )));
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("runtime-pong")
    );
}

#[tokio::test]
async fn stage_fixture_prompt_does_not_trigger_fixture_by_default() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(StaticProvider::default(), NoopToolRuntime, store.clone());
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(
            &backend,
            AgentInput::text("run stage 4m real parity fixture"),
        )
        .await
        .expect("default runtime should handle prompt normally");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("YunXi autonomous runtime accepted prompt: run stage 4m real parity fixture")
    );
    assert!(!temp.path().join("stage4m-runtime.txt").exists());
    assert!(
        !result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::DeepParityState { .. }))
    );
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn stage_4k_child_scoped_stream_fixture_emits_granular_child_events() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::for_workspace_with_runtime_fixtures(temp.path());
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(
            &backend,
            AgentInput::text("run stage 4k child scoped stream fixture"),
        )
        .await
        .expect("stage 4k fixture");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildScopedStream {
            event,
            message: Some(message),
            ..
        } if event == "child_provider_delta" && message.contains("Provider turn")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildScopedStream {
            event,
            message: Some(message),
            ..
        } if event == "child_tool_delta" && message.contains("YUNXI_CHILD_TOOL_DELTA")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildScopedStream { event, .. } if event == "child_storage_state"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildScopedStream { event, .. } if event == "child_session_finished"
    )));
}

#[tokio::test]
async fn stage_4k_cancellation_fixture_records_cancelled_boundaries() {
    let temp = TempDir::new().expect("temp dir");
    let backend = YunXiRuntimeBackend::for_workspace_with_runtime_fixtures(temp.path());
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(
            &backend,
            AgentInput::text("run stage 4k cancellation fixture"),
        )
        .await
        .expect("cancellation fixture");

    assert_eq!(result.status, AgentRunStatus::Cancelled);
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::Cancelled { .. }))
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ChildScopedStream { event, .. } if event == "child_cancelled"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::StorageState {
            session_id: Some(_),
            ..
        }
    )));
}

#[tokio::test]
async fn stage_4l_deep_parity_fixture_covers_all_closure_layers() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        StaticProvider::default().with_fixtures_enabled(),
        CompositeToolRuntime::default(),
        store.clone(),
    )
    .with_runtime_fixtures_enabled();
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(
            &backend,
            AgentInput::text("run stage 4l deep parity fixture"),
        )
        .await
        .expect("stage 4l fixture");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(matches!(
        result.final_response.as_deref(),
        Some(response) if response.contains("Stage 4L deep parity fixture completed")
    ));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ThreadState { .. }))
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::TurnMetadata { .. }))
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::TurnState { .. }))
    );
    let layers = result
        .events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::DeepParityState { layer, .. } => Some(layer.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(layers.len(), 12);
    assert!(layers.contains(&"02_provider_feature_matrix"));
    assert!(layers.contains(&"03_unified_exec"));
    assert!(layers.contains(&"06_mcp_lifecycle"));
    assert!(layers.contains(&"10_multi_agent_v2"));
    assert!(layers.contains(&"12_parity_harness"));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::CommandCompleted { .. }))
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ApprovalRequested { .. }))
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::EscalationRequested { .. }))
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::McpSession { status, .. } if status == "capability_negotiated"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolCallCompleted { name, .. } if name == "tool_search"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ContextStatus {
            compacted: true,
            ..
        }
    )));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ChildScopedStream { .. }))
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::StorageState {
            child_session_ids,
            ..
        } if child_session_ids.iter().any(|id| id.contains("stage-4l"))
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn stage_4m_real_parity_fixture_runs_real_runtime_chain() {
    let temp = TempDir::new().expect("temp dir");
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        StaticProvider::default().with_fixtures_enabled(),
        CompositeToolRuntime::default(),
        store.clone(),
    )
    .with_runtime_fixtures_enabled();
    let agent = Agent::new(AgentConfig::new(temp.path()).with_approval_mode(ApprovalMode::Never));

    let result = agent
        .run_with_backend(
            &backend,
            AgentInput::text("run stage 4m real parity fixture"),
        )
        .await
        .expect("stage 4m fixture");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(matches!(
        result.final_response.as_deref(),
        Some(response) if response.contains("Stage 4M real parity fixture completed")
    ));
    assert!(temp.path().join("stage4m-runtime.txt").is_file());
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::SandboxAttempt {
            id: Some(id),
            schema_version: 1,
            backend_id,
            backend_label,
            os_isolation: false,
            enforcement,
            enforcement_level,
            runner,
            unsupported_reason: Some(_),
            ..
        } if id == "stage-4m-shell-1"
            && !backend_id.is_empty()
            && !backend_label.is_empty()
            && enforcement == enforcement_level
            && runner == backend_id
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ApprovalCacheState {
            tool_name,
            reused: true,
            ..
        } if tool_name == "shell"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::McpSession {
            status,
            ..
        } if status == "reused" || status == "tool_started"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolCallCompleted {
            name,
            status: CommandStatus::Completed,
            ..
        } if name == "skill:stage4m"
    )));
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::ChildScopedStream { .. }))
    );
    let layers = result
        .events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::DeepParityState { layer, .. }
                if layer.starts_with("01_")
                    || layer.starts_with("02_")
                    || layer.starts_with("12_") =>
            {
                Some(layer.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(layers.contains(&"01_runtime_driver_state_machine"));
    assert!(layers.contains(&"02_provider_feature_matrix"));
    assert!(layers.contains(&"12_real_parity_harness"));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::StorageState {
            session_id: Some(_),
            child_session_ids,
            ..
        } if child_session_ids.iter().any(|id| id == "agent-1-session")
    )));
    assert_eq!(store.list().await.expect("session list").len(), 2);
}

#[test]
fn protocol_stream_events_map_to_agent_events() {
    let stream = vec![
        yunxi_agent_protocol::StreamEvent::ResponseStarted {
            thread_id: yunxi_agent_protocol::ThreadId("thread-1".to_string()),
            turn_id: yunxi_agent_protocol::TurnId("turn-1".to_string()),
            metadata: None,
        },
        yunxi_agent_protocol::StreamEvent::ItemDelta {
            thread_id: yunxi_agent_protocol::ThreadId("thread-1".to_string()),
            turn_id: yunxi_agent_protocol::TurnId("turn-1".to_string()),
            delta: yunxi_agent_protocol::ResponseItemDelta::ReasoningContent {
                item_id: None,
                delta: "reason".to_string(),
            },
            metadata: None,
        },
        yunxi_agent_protocol::StreamEvent::ItemDelta {
            thread_id: yunxi_agent_protocol::ThreadId("thread-1".to_string()),
            turn_id: yunxi_agent_protocol::TurnId("turn-1".to_string()),
            delta: yunxi_agent_protocol::ResponseItemDelta::MessageContent {
                item_id: Some("message-1".to_string()),
                delta: "hel".to_string(),
            },
            metadata: Some(yunxi_agent_protocol::StreamEventMetadata {
                event_id: "provider-message-17".to_string(),
                sequence: yunxi_agent_protocol::StreamEventSequence::ProviderReliable(17),
            }),
        },
        yunxi_agent_protocol::StreamEvent::ItemCompleted {
            thread_id: yunxi_agent_protocol::ThreadId("thread-1".to_string()),
            turn_id: yunxi_agent_protocol::TurnId("turn-1".to_string()),
            item: yunxi_agent_protocol::ResponseItem::AgentMessage {
                id: "message-1".to_string(),
                content: vec![yunxi_agent_protocol::ContentItem::OutputText {
                    text: "hello".to_string(),
                }],
                phase: yunxi_agent_protocol::MessagePhase::Completed,
            },
        },
    ];

    let events = protocol_stream_events_to_agent_events(&stream);

    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::Reasoning { content } if content == "reason"
    )));
    let messages = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::Message { content, stream } => {
                Some((content.as_str(), stream.as_ref().expect("stream identity")))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, "hel");
    assert_eq!(messages[0].1.turn_id, "turn-1");
    assert_eq!(messages[0].1.stream_id, "message-1");
    assert_eq!(messages[0].1.phase, AgentMessageStreamPhase::Delta);
    assert_eq!(messages[1].0, "hello");
    assert_eq!(messages[1].1.phase, AgentMessageStreamPhase::Final);
    assert_eq!(messages[0].1.event_id, "provider-message-17");
    assert_eq!(
        messages[0].1.source_sequence,
        AgentMessageSequence::ProviderReliable(17)
    );
    assert_eq!(
        messages[1].1.source_sequence,
        AgentMessageSequence::LocalFallback(1)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn provider_memory_extraction_writes_valid_json_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(
        r#"{"candidates":[{"kind":"preference","content":"用户偏好默认中文交流。","scope_hint":"global_user","sensitivity_hint":"low","confidence":0.9,"importance":0.8,"reason":"provider:language-preference"}]}"#,
    );
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::Provider);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("你好")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWrite {
            action,
            kind,
            status,
            revision: 1,
            merged_count: 1,
            ..
        } if action == "auto_saved" && kind == "preference" && status == "active"
    )));
}

#[tokio::test(flavor = "current_thread")]
async fn rule_generic_and_provider_rich_same_turn_keeps_rich_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(
        r#"{"candidates":[{"kind":"preference","content":"用户偏好使用中文回答，并且回答要简洁、保留关键细节。","scope_hint":"global_user","sensitivity_hint":"low","confidence":0.95,"importance":0.9,"reason":"provider:rich-language-preference"}]}"#,
    );
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::Auto);
    let agent = Agent::new(config);

    with_memory_enabled_home(home.path(), async {
        let result = agent
            .run_with_backend(&backend, AgentInput::text("以后请用中文回答"))
            .await
            .expect("runtime should complete");

        assert_eq!(result.status, AgentRunStatus::Completed);
        assert!(result.events.iter().any(|event| matches!(
            event,
            AgentEvent::MemoryWrite {
                action,
                kind,
                ..
            } if action == "auto_saved" && kind == "preference"
        )));
        let store = FilePersonaMemoryStore::for_workspace(workspace.path());
        let loaded = store.list(PersonaMemoryScope::Global);
        assert_eq!(loaded.records.len(), 1);
        assert!(loaded.records[0].content.contains("简洁"));
        assert!(loaded.records[0].content.contains("保留关键细节"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn provider_rich_memory_is_not_degraded_by_rule_generic_followup() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(
        r#"{"candidates":[{"kind":"preference","content":"用户偏好使用中文回答，并且回答要简洁、保留关键细节。","scope_hint":"global_user","sensitivity_hint":"low","confidence":0.95,"importance":0.9,"reason":"provider:rich-language-preference"}]}"#,
    );
    let provider_backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let provider_agent = Agent::new(
        AgentConfig::new(workspace.path())
            .with_provider("fixture")
            .with_model("fixture-model")
            .with_memory_extraction_mode(MemoryExtractionMode::Provider),
    );
    let rule_backend = YunXiRuntimeBackend::with_parts(
        StaticProvider::default(),
        NoopToolRuntime,
        InMemorySessionStore::default(),
    );
    let rule_agent = Agent::new(
        AgentConfig::new(workspace.path())
            .with_memory_extraction_mode(MemoryExtractionMode::RuleOnly),
    );

    with_memory_enabled_home(home.path(), async {
        provider_agent
            .run_with_backend(&provider_backend, AgentInput::text("你好"))
            .await
            .expect("provider runtime should complete");
        let result = rule_agent
            .run_with_backend(&rule_backend, AgentInput::text("以后请用中文回答"))
            .await
            .expect("rule runtime should complete");

        assert_eq!(result.status, AgentRunStatus::Completed);
        assert!(result.events.iter().any(|event| matches!(
            event,
            AgentEvent::MemoryWrite {
                action,
                merge_strategy: Some(strategy),
                revision: 2,
                ..
            } if action == "merged" && strategy == "preserve_existing"
        )));
        let store = FilePersonaMemoryStore::for_workspace(workspace.path());
        let loaded = store.list(PersonaMemoryScope::Global);
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].revision, 2);
        assert!(loaded.records[0].content.contains("简洁"));
        assert!(loaded.records[0].content.contains("保留关键细节"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn provider_memory_extraction_invalid_json_warns_and_auto_falls_back_to_rules() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new("not json");
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::Auto);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("以后请用中文回答")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWarning { warning, .. }
            if warning.contains("invalid JSON candidates payload")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWrite { action, kind, .. }
            if action == "auto_saved" && kind == "preference"
    )));
}

#[tokio::test(flavor = "current_thread")]
async fn provider_memory_extraction_empty_candidates_does_not_write_without_rule_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(r#"{"candidates":[]}"#);
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::Provider);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("请计算 2+2")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        !result
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::MemoryWrite { .. }))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn rule_only_memory_extraction_does_not_call_provider_extractor() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(
        r#"{"candidates":[{"kind":"preference","content":"用户偏好默认中文交流。"}]}"#,
    );
    let memory_calls = provider.memory_calls();
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::RuleOnly);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("以后请用中文回答")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(memory_calls.load(Ordering::SeqCst), 0);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWrite { action, kind, .. }
            if action == "auto_saved" && kind == "preference"
    )));
}

#[tokio::test(flavor = "current_thread")]
async fn provider_memory_extraction_mode_without_provider_or_model_warns_and_keeps_rule_candidate()
{
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let backend = YunXiRuntimeBackend::with_parts(
        MemoryExtractionFixtureProvider::new(r#"{"candidates":[]}"#),
        NoopToolRuntime,
        InMemorySessionStore::default(),
    )
    .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_memory_extraction_mode(MemoryExtractionMode::Provider);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("以后请用中文回答")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWarning { warning, .. }
            if warning.contains("provider and model are required")
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWrite { action, kind, .. }
            if action == "auto_saved" && kind == "preference"
    )));
}

#[tokio::test(flavor = "current_thread")]
async fn provider_memory_extraction_timeout_warns_and_does_not_block_main_response() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let provider = MemoryExtractionFixtureProvider::new(r#"{"candidates":[]}"#)
        .with_delay(Duration::from_secs(7));
    let backend =
        YunXiRuntimeBackend::with_parts(provider, NoopToolRuntime, InMemorySessionStore::default())
            .with_inherited_child_provider();
    let config = AgentConfig::new(workspace.path())
        .with_provider("fixture")
        .with_model("fixture-model")
        .with_memory_extraction_mode(MemoryExtractionMode::Auto);
    let agent = Agent::new(config);

    let result = with_memory_enabled_home(
        home.path(),
        agent.run_with_backend(&backend, AgentInput::text("请计算 2+2")),
    )
    .await
    .expect("runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("fixture main response")
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::MemoryWarning { warning, .. }
            if warning.contains("timed out")
    )));
}
