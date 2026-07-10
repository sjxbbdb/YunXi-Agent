use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunStatus, ApprovalMode,
    CommandStatus, FileChangeKind, PatchStatus,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderResponse, ProviderRole,
    ProviderToolCall, StaticProvider,
};
use yunxi_agent_runtime::{YunXiRuntimeBackend, protocol_stream_events_to_agent_events};
use yunxi_agent_storage::{InMemorySessionStore, SessionId, SessionRecord, SessionStore};
use yunxi_agent_tools::{NoopToolRuntime, ShellToolRuntime};

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

    assert_eq!(
        messages
            .lock()
            .expect("messages lock")
            .iter()
            .map(|message| message.role)
            .collect::<Vec<_>>(),
        vec![ProviderRole::System, ProviderRole::User]
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
    assert_eq!(
        captured
            .iter()
            .map(|message| message.role)
            .collect::<Vec<_>>(),
        vec![
            ProviderRole::User,
            ProviderRole::Assistant,
            ProviderRole::User,
            ProviderRole::Assistant,
            ProviderRole::User,
        ]
    );
    assert_eq!(captured[0].content, "root prompt");
    assert_eq!(captured[1].content, "root answer");
    assert_eq!(captured[2].content, "child prompt");
    assert_eq!(captured[3].content, "child answer");
    assert_eq!(captured[4].content, "continue work");
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
    assert_eq!(
        captured.first().map(|message| message.role),
        Some(ProviderRole::System)
    );
    assert!(
        captured
            .first()
            .expect("first message")
            .content
            .contains("Compacted")
    );
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
        },
    ];

    let events = protocol_stream_events_to_agent_events(&stream);

    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::Reasoning { content } if content == "reason"
    )));
}
