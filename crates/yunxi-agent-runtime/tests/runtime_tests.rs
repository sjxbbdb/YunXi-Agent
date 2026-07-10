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
    AgentProvider, ProviderRequest, ProviderResponse, ProviderRole, ProviderToolCall,
    StaticProvider,
};
use yunxi_agent_runtime::YunXiRuntimeBackend;
use yunxi_agent_storage::{InMemorySessionStore, SessionStore};
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
    assert_eq!(
        result.events.first(),
        Some(&AgentEvent::Started {
            prompt: "explain this project".to_string()
        })
    );
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
    messages: Arc<Mutex<Vec<ProviderRole>>>,
}

#[async_trait::async_trait]
impl AgentProvider for CapturingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        *self.messages.lock().expect("messages lock") = request
            .messages
            .iter()
            .map(|message| message.role)
            .collect::<Vec<_>>();
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
        *messages.lock().expect("messages lock"),
        vec![ProviderRole::System, ProviderRole::User]
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
