use crate::map_config_to_codex_options;
#[cfg(feature = "codex-native")]
use crate::map_exec_jsonl;
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentInput, AgentResult, AgentRunResult,
};
#[cfg(feature = "codex-native")]
use yunxi_agent_core::{AgentEvent, AgentRunStatus};

#[derive(Clone, Debug, Default)]
pub struct CodexNativeBackend;

impl CodexNativeBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AgentBackend for CodexNativeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        let prompt = input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let _options = map_config_to_codex_options(&config)?;

        #[cfg(feature = "codex-native")]
        {
            run_native_codex(_options, prompt.to_string()).await
        }

        #[cfg(not(feature = "codex-native"))]
        {
            Err(AgentError::Execution {
                message: "codex-native feature is not enabled".to_string(),
            })
        }
    }
}

#[cfg(feature = "codex-native")]
async fn run_native_codex(
    options: crate::CodexRunOptions,
    prompt: String,
) -> AgentResult<AgentRunResult> {
    let output = live_runner::run_codex_exec_in_process(options, prompt).await?;
    let events = map_exec_jsonl(&output.jsonl)?;
    let final_response = output.final_message.or_else(|| {
        events.iter().rev().find_map(|event| match event {
            AgentEvent::Message { content } => Some(content.clone()),
            _ => None,
        })
    });
    let status = if output.failed {
        AgentRunStatus::Failed
    } else {
        AgentRunStatus::Completed
    };

    Ok(AgentRunResult {
        status,
        final_response,
        events,
    })
}

#[cfg(feature = "codex-native")]
mod live_runner {
    use std::sync::Arc;

    use codex_app_server_client::{
        DEFAULT_IN_PROCESS_CHANNEL_CAPACITY, EnvironmentManager, ExecServerRuntimePaths,
        InProcessAppServerClient, InProcessClientStartArgs, InProcessServerEvent,
    };
    use codex_app_server_protocol::{
        ClientRequest, ConfigWarningNotification, JSONRPCErrorError, McpServerElicitationAction,
        McpServerElicitationRequestResponse, RequestId, ServerNotification, ServerRequest,
        ThreadSource, ThreadStartParams, ThreadStartResponse, ThreadUnsubscribeParams,
        ThreadUnsubscribeResponse, TurnStartParams, TurnStartResponse, TurnStatus,
    };
    use codex_arg0::Arg0DispatchPaths;
    use codex_config::{CloudConfigBundleLoader, LoaderOverrides};
    use codex_core::config::{Config, ConfigBuilder, ConfigOverrides};
    use codex_exec::{
        CodexStatus, EventProcessorWithJsonOutput, ThreadErrorEvent, ThreadEvent,
        ThreadStartedEvent,
    };
    use codex_feedback::CodexFeedback;
    use codex_protocol::config_types::SandboxMode as CodexSandboxMode;
    use codex_protocol::protocol::{AskForApproval, SessionSource};
    use codex_protocol::user_input::UserInput;
    use yunxi_agent_core::{AgentError, AgentResult};

    use crate::{CodexApproval, CodexRunOptions, CodexSandbox};

    pub struct NativeRunOutput {
        pub jsonl: String,
        pub final_message: Option<String>,
        pub failed: bool,
    }

    struct RequestIdSequencer {
        next: i64,
    }

    impl RequestIdSequencer {
        fn new() -> Self {
            Self { next: 1 }
        }

        fn next(&mut self) -> RequestId {
            let id = self.next;
            self.next += 1;
            RequestId::Integer(id)
        }
    }

    pub async fn run_codex_exec_in_process(
        options: CodexRunOptions,
        prompt: String,
    ) -> AgentResult<NativeRunOutput> {
        let arg0_paths = arg0_dispatch_paths();
        let config = build_codex_config(&options, &arg0_paths).await?;
        let runtime_paths = ExecServerRuntimePaths::from_optional_paths(
            arg0_paths.codex_self_exe.clone(),
            arg0_paths.codex_linux_sandbox_exe.clone(),
        )
        .map_err(execution_error)?;
        let environment_manager =
            EnvironmentManager::from_codex_home(config.codex_home.clone(), Some(runtime_paths))
                .await
                .map_err(execution_error)?;
        let state_db = codex_core::init_state_db(&config).await;
        let config_warnings = config
            .startup_warnings
            .iter()
            .map(|warning| ConfigWarningNotification {
                summary: warning.clone(),
                details: None,
                path: None,
                range: None,
            })
            .collect();

        let mut client = InProcessAppServerClient::start(InProcessClientStartArgs {
            arg0_paths,
            config: Arc::new(config.clone()),
            cli_overrides: Vec::new(),
            loader_overrides: LoaderOverrides::default(),
            strict_config: false,
            cloud_config_bundle: CloudConfigBundleLoader::default(),
            feedback: CodexFeedback::new(),
            log_db: None,
            state_db,
            environment_manager: Arc::new(environment_manager),
            config_warnings,
            session_source: SessionSource::Exec,
            enable_codex_api_key_env: true,
            client_name: "yunxi-agent-codex".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            experimental_api: true,
            mcp_server_openai_form_elicitation: false,
            opt_out_notification_methods: Vec::new(),
            channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
        })
        .await
        .map_err(execution_error)?;

        let mut request_ids = RequestIdSequencer::new();
        let response: ThreadStartResponse = send_request_with_response(
            &client,
            ClientRequest::ThreadStart {
                request_id: request_ids.next(),
                params: thread_start_params_from_config(&config, &options),
            },
            "thread/start",
        )
        .await
        .map_err(execution_message)?;

        let thread_id = response.thread.id.clone();
        let mut processor = EventProcessorWithJsonOutput::new(None);
        let mut jsonl = String::new();
        push_event(
            &mut jsonl,
            ThreadEvent::ThreadStarted(ThreadStartedEvent {
                thread_id: thread_id.clone(),
            }),
        )?;

        let response: TurnStartResponse = send_request_with_response(
            &client,
            ClientRequest::TurnStart {
                request_id: request_ids.next(),
                params: TurnStartParams {
                    thread_id: thread_id.clone(),
                    client_user_message_id: None,
                    input: vec![
                        UserInput::Text {
                            text: prompt,
                            text_elements: Vec::new(),
                        }
                        .into(),
                    ],
                    responsesapi_client_metadata: None,
                    additional_context: None,
                    environments: None,
                    cwd: Some(config.cwd.to_path_buf()),
                    runtime_workspace_roots: None,
                    approval_policy: Some(config.permissions.approval_policy.value().into()),
                    approvals_reviewer: Some(config.approvals_reviewer.into()),
                    sandbox_policy: None,
                    permissions: None,
                    model: None,
                    service_tier: None,
                    effort: config.model_reasoning_effort.clone(),
                    summary: None,
                    personality: None,
                    output_schema: None,
                    collaboration_mode: None,
                    multi_agent_mode: None,
                },
            },
            "turn/start",
        )
        .await
        .map_err(execution_message)?;
        let turn_id = response.turn.id;
        let mut failed = false;

        loop {
            let Some(server_event) = client.next_event().await else {
                break;
            };

            match server_event {
                InProcessServerEvent::ServerRequest(request) => {
                    handle_server_request(&client, request, &mut failed).await;
                }
                InProcessServerEvent::ServerNotification(notification) => {
                    if notification_marks_failure(&notification, &thread_id, &turn_id) {
                        failed = true;
                    }

                    if should_process_notification(&notification, &thread_id, &turn_id) {
                        let collected = processor.collect_thread_events(notification);
                        for event in collected.events {
                            push_event(&mut jsonl, event)?;
                        }
                        if collected.status == CodexStatus::InitiateShutdown {
                            request_shutdown(&client, &mut request_ids, &thread_id)
                                .await
                                .map_err(execution_message)?;
                            break;
                        }
                    }
                }
                InProcessServerEvent::Lagged { skipped } => {
                    push_event(
                        &mut jsonl,
                        ThreadEvent::Error(ThreadErrorEvent {
                            message: format!(
                                "in-process app-server event stream lagged; dropped {skipped} events"
                            ),
                        }),
                    )?;
                }
            }
        }

        let final_message = processor.final_message().map(ToOwned::to_owned);
        client.shutdown().await.map_err(execution_error)?;

        Ok(NativeRunOutput {
            jsonl,
            final_message,
            failed,
        })
    }

    fn arg0_dispatch_paths() -> Arg0DispatchPaths {
        Arg0DispatchPaths {
            codex_self_exe: std::env::current_exe().ok(),
            codex_linux_sandbox_exe: None,
            main_execve_wrapper_exe: None,
        }
    }

    async fn build_codex_config(
        options: &CodexRunOptions,
        arg0_paths: &Arg0DispatchPaths,
    ) -> AgentResult<Config> {
        let overrides = ConfigOverrides {
            model: options.model.clone(),
            cwd: Some(options.cwd.clone()),
            approval_policy: Some(map_approval(options.approval)),
            sandbox_mode: Some(map_sandbox(options.sandbox)),
            model_provider: options.provider.clone(),
            codex_self_exe: arg0_paths.codex_self_exe.clone(),
            codex_linux_sandbox_exe: arg0_paths.codex_linux_sandbox_exe.clone(),
            main_execve_wrapper_exe: arg0_paths.main_execve_wrapper_exe.clone(),
            ephemeral: Some(options.ephemeral),
            ..ConfigOverrides::default()
        };

        let mut builder = ConfigBuilder::default().harness_overrides(overrides);
        if let Some(codex_home) = options.codex_home.clone() {
            builder = builder.codex_home(codex_home);
        }

        builder.build().await.map_err(execution_error)
    }

    fn map_approval(approval: CodexApproval) -> AskForApproval {
        match approval {
            CodexApproval::Never => AskForApproval::Never,
            CodexApproval::OnRequest | CodexApproval::OnFailure => AskForApproval::OnRequest,
            CodexApproval::Untrusted => AskForApproval::UnlessTrusted,
        }
    }

    fn map_sandbox(sandbox: CodexSandbox) -> CodexSandboxMode {
        match sandbox {
            CodexSandbox::ReadOnly => CodexSandboxMode::ReadOnly,
            CodexSandbox::WorkspaceWrite => CodexSandboxMode::WorkspaceWrite,
            CodexSandbox::DangerFullAccess => CodexSandboxMode::DangerFullAccess,
        }
    }

    fn thread_start_params_from_config(
        config: &Config,
        options: &CodexRunOptions,
    ) -> ThreadStartParams {
        let permissions = config
            .permissions
            .active_permission_profile()
            .map(|active| active.id);
        let sandbox = permissions
            .is_none()
            .then(|| codex_app_server_protocol::SandboxMode::from(map_sandbox(options.sandbox)));

        ThreadStartParams {
            model: config.model.clone(),
            model_provider: Some(config.model_provider_id.clone()),
            cwd: Some(config.cwd.to_string_lossy().to_string()),
            runtime_workspace_roots: Some(config.workspace_roots.clone()),
            approval_policy: Some(config.permissions.approval_policy.value().into()),
            approvals_reviewer: Some(config.approvals_reviewer.into()),
            sandbox,
            permissions,
            config: thread_config_overrides_from_config(config),
            ephemeral: Some(options.ephemeral),
            thread_source: Some(ThreadSource::User),
            ..ThreadStartParams::default()
        }
    }

    fn thread_config_overrides_from_config(
        config: &Config,
    ) -> Option<std::collections::HashMap<String, serde_json::Value>> {
        config.bypass_hook_trust.then(|| {
            std::collections::HashMap::from([(
                "bypass_hook_trust".to_string(),
                serde_json::Value::Bool(true),
            )])
        })
    }

    async fn send_request_with_response<T>(
        client: &InProcessAppServerClient,
        request: ClientRequest,
        method: &str,
    ) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        client.request_typed(request).await.map_err(|err| {
            if method.is_empty() {
                err.to_string()
            } else {
                format!("{method}: {err}")
            }
        })
    }

    async fn request_shutdown(
        client: &InProcessAppServerClient,
        request_ids: &mut RequestIdSequencer,
        thread_id: &str,
    ) -> Result<(), String> {
        send_request_with_response::<ThreadUnsubscribeResponse>(
            client,
            ClientRequest::ThreadUnsubscribe {
                request_id: request_ids.next(),
                params: ThreadUnsubscribeParams {
                    thread_id: thread_id.to_string(),
                },
            },
            "thread/unsubscribe",
        )
        .await
        .map(|_| ())
    }

    fn notification_marks_failure(
        notification: &ServerNotification,
        thread_id: &str,
        turn_id: &str,
    ) -> bool {
        match notification {
            ServerNotification::Error(payload) => {
                payload.thread_id == thread_id && payload.turn_id == turn_id && !payload.will_retry
            }
            ServerNotification::TurnCompleted(payload) => {
                payload.thread_id == thread_id
                    && payload.turn.id == turn_id
                    && matches!(
                        payload.turn.status,
                        TurnStatus::Failed | TurnStatus::Interrupted
                    )
            }
            _ => false,
        }
    }

    fn should_process_notification(
        notification: &ServerNotification,
        thread_id: &str,
        turn_id: &str,
    ) -> bool {
        match notification {
            ServerNotification::ConfigWarning(_) | ServerNotification::DeprecationNotice(_) => true,
            ServerNotification::Warning(notification) => notification
                .thread_id
                .as_deref()
                .is_none_or(|candidate| candidate == thread_id),
            ServerNotification::Error(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::HookCompleted(notification) => {
                notification.thread_id == thread_id
                    && notification
                        .turn_id
                        .as_deref()
                        .is_none_or(|candidate| candidate == turn_id)
            }
            ServerNotification::HookStarted(notification) => {
                notification.thread_id == thread_id
                    && notification
                        .turn_id
                        .as_deref()
                        .is_none_or(|candidate| candidate == turn_id)
            }
            ServerNotification::ItemCompleted(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::ItemStarted(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::ModelRerouted(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::ModelVerification(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::ThreadTokenUsageUpdated(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::TurnCompleted(notification) => {
                notification.thread_id == thread_id && notification.turn.id == turn_id
            }
            ServerNotification::TurnDiffUpdated(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::TurnPlanUpdated(notification) => {
                notification.thread_id == thread_id && notification.turn_id == turn_id
            }
            ServerNotification::TurnStarted(notification) => {
                notification.thread_id == thread_id && notification.turn.id == turn_id
            }
            _ => false,
        }
    }

    async fn handle_server_request(
        client: &InProcessAppServerClient,
        request: ServerRequest,
        failed: &mut bool,
    ) {
        let method = server_request_method_name(&request);
        let request_id = request.id().clone();
        let result = match request {
            ServerRequest::McpServerElicitationRequest { .. } => {
                match serde_json::to_value(McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Cancel,
                    content: None,
                    meta: None,
                }) {
                    Ok(value) => client
                        .resolve_server_request(request_id, value)
                        .await
                        .map_err(|err| {
                            format!(
                                "failed to resolve `mcpServer/elicitation/request` server request: {err}"
                            )
                        }),
                    Err(err) => Err(format!("failed to encode MCP elicitation response: {err}")),
                }
            }
            _ => client
                .reject_server_request(
                    request_id,
                    JSONRPCErrorError {
                        code: -32000,
                        message: format!("{method} is not supported in YunXi headless mode"),
                        data: None,
                    },
                )
                .await
                .map_err(|err| format!("failed to reject `{method}` server request: {err}")),
        };

        if result.is_err() {
            *failed = true;
        }
    }

    fn server_request_method_name(request: &ServerRequest) -> String {
        serde_json::to_value(request)
            .ok()
            .and_then(|value| {
                value
                    .get("method")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn push_event(jsonl: &mut String, event: ThreadEvent) -> AgentResult<()> {
        let line = serde_json::to_string(&event).map_err(|err| AgentError::Execution {
            message: format!("failed to serialize Codex event: {err}"),
        })?;
        jsonl.push_str(&line);
        jsonl.push('\n');
        Ok(())
    }

    fn execution_error(error: impl std::fmt::Display) -> AgentError {
        AgentError::Execution {
            message: error.to_string(),
        }
    }

    fn execution_message(message: String) -> AgentError {
        AgentError::Execution { message }
    }
}
