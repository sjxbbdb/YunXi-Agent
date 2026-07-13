use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;
use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunControl, AgentRunResult,
    ApprovalMode, BackendKind, CommandStatus, SandboxMode,
};
use yunxi_agent_protocol::{
    FunctionCallOutput, ProtocolRole, ResponseItem, ResponseItemDelta, RuntimeEvent, ThreadId,
    ThreadState, ToolCall, ToolCallStatus, TurnId, TurnMetadata, TurnState, to_jsonl_line,
};
use yunxi_agent_storage::{
    FileSessionStore, HistoryLoadOptions, RolloutRecord, SessionGraphView, SessionHistory,
    SessionId, SessionRecord, SessionStore,
};

mod commands {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"));
}
mod input {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/input.rs"));
}
mod interactive {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/interactive.rs"));
}
mod provider_mode {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/provider_mode.rs"));
}
mod render {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/render.rs"));
}
mod terminal_mode {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/terminal_mode.rs"));
}
mod tui {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tui/mod.rs"));
}

const CODEX_CORE_PARITY_MAP: &str =
    include_str!("../../../docs/extraction-index/codex-core-agent-parity-map.md");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CliExitCode {
    Success,
    InvalidInput,
    ProviderError,
    ToolError,
    Cancelled,
    InternalError,
}

impl CliExitCode {
    fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::InvalidInput => 2,
            Self::ProviderError => 10,
            Self::ToolError => 20,
            Self::Cancelled => 130,
            Self::InternalError => 70,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "yunxi")]
#[command(version)]
#[command(about = "YunXi Agent v1.7.4 interactive terminal CLI")]
struct Cli {
    #[arg(
        long,
        value_name = "BACKEND",
        value_enum,
        default_value_t = CliBackend::Yunxi
    )]
    backend: CliBackend,

    #[arg(long, conflicts_with = "backend")]
    live: bool,

    #[arg(long, value_name = "PATH", default_value = ".", global = true)]
    cwd: PathBuf,

    #[arg(long, value_name = "MODEL")]
    model: Option<String>,

    #[arg(long, value_name = "PROVIDER")]
    provider: Option<String>,

    #[arg(long)]
    provider_live: bool,

    #[arg(long, conflicts_with = "provider_live")]
    offline: bool,

    #[arg(long = "codex-home", value_name = "PATH")]
    codex_home: Option<PathBuf>,

    #[arg(long, value_name = "TOKENS")]
    context_window_tokens: Option<i64>,

    #[arg(long, value_name = "TOKENS")]
    auto_compact_threshold_tokens: Option<i64>,

    #[arg(
        long,
        value_name = "MODE",
        value_enum,
        default_value_t = CliApprovalMode::OnRequest
    )]
    approval: CliApprovalMode,

    #[arg(
        long,
        value_name = "MODE",
        value_enum,
        default_value_t = CliSandboxMode::WorkspaceWrite
    )]
    sandbox: CliSandboxMode,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    jsonl: bool,

    #[arg(long, global = true, conflicts_with = "no_tui")]
    tui: bool,

    #[arg(long = "no-tui", global = true)]
    no_tui: bool,

    #[command(subcommand)]
    command: Option<CliCommand>,

    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    Sessions {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Parity {
        #[command(subcommand)]
        command: ParityCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    List,
    Show {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Rollout {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    History {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Graph,
    Resume {
        #[arg(value_name = "SESSION_ID")]
        id: String,
        #[arg(value_name = "PROMPT")]
        prompt: Vec<String>,
    },
    Fork {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Archive {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Unarchive {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Pin {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    Unpin {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum ParityCommand {
    Map,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliBackend {
    DryRun,
    Yunxi,
    Codex,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl From<CliBackend> for BackendKind {
    fn from(value: CliBackend) -> Self {
        match value {
            CliBackend::DryRun => BackendKind::DryRun,
            CliBackend::Yunxi => BackendKind::Yunxi,
            CliBackend::Codex => BackendKind::Codex,
        }
    }
}

impl From<CliApprovalMode> for ApprovalMode {
    fn from(value: CliApprovalMode) -> Self {
        match value {
            CliApprovalMode::Never => ApprovalMode::Never,
            CliApprovalMode::OnRequest => ApprovalMode::OnRequest,
            CliApprovalMode::OnFailure => ApprovalMode::OnFailure,
            CliApprovalMode::Untrusted => ApprovalMode::Untrusted,
        }
    }
}

impl From<CliSandboxMode> for SandboxMode {
    fn from(value: CliSandboxMode) -> Self {
        match value {
            CliSandboxMode::ReadOnly => SandboxMode::ReadOnly,
            CliSandboxMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
            CliSandboxMode::DangerFullAccess => SandboxMode::DangerFullAccess,
        }
    }
}

#[tokio::main]
async fn main() {
    let jsonl_requested = std::env::args().any(|arg| arg == "--jsonl");
    let code = match run_cli().await {
        Ok(()) => CliExitCode::Success,
        Err(error) => {
            if jsonl_requested {
                print_cli_error_jsonl(&error);
            } else {
                eprintln!("{}", redact_secret_fragments(&format!("{error:#}")));
            }
            classify_cli_error(&error)
        }
    };
    std::process::exit(code.code());
}

async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let provider_mode = provider_mode::ProviderMode::from_flags(cli.provider_live, cli.offline);

    let backend = if cli.live {
        BackendKind::Codex
    } else {
        cli.backend.into()
    };

    let mut config = AgentConfig::new(cli.cwd.clone())
        .with_approval_mode(cli.approval.into())
        .with_sandbox_mode(cli.sandbox.into());
    if let Some(model) = cli.model.clone() {
        config = config.with_model(model);
    }
    if let Some(provider) = cli.provider.clone() {
        config = config.with_provider(provider);
    }
    if let Some(codex_home) = cli.codex_home.clone() {
        config = config.with_codex_home(codex_home);
    }
    if let Some(context_window_tokens) = cli.context_window_tokens {
        config = config.with_context_window_tokens(context_window_tokens);
    }
    if let Some(auto_compact_threshold_tokens) = cli.auto_compact_threshold_tokens {
        config = config.with_auto_compact_threshold_tokens(auto_compact_threshold_tokens);
    }

    if let Some(command) = cli.command {
        return run_command(command, config, backend, provider_mode, cli.json, cli.jsonl).await;
    }

    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        if !cli.json && !cli.jsonl {
            let terminal_mode = terminal_mode::TerminalModeRequest {
                tui: cli.tui,
                no_tui: cli.no_tui,
                stdin_is_terminal: std::io::stdin().is_terminal(),
                stdout_is_terminal: std::io::stdout().is_terminal(),
            }
            .resolve();
            return interactive::run_interactive(interactive::InteractiveOptions {
                config,
                backend,
                provider_mode,
                terminal_mode,
            })
            .await;
        }
        bail!("a prompt is required");
    }

    let selection = provider_mode.resolve(backend, &config)?;
    let config = selection.apply_to_config(config);
    let offline_label = selection.is_offline_runtime();
    print_provider_selection_warning(&selection, cli.json, cli.jsonl)?;
    let result = run_agent_backend(backend, config, prompt, selection.live).await?;
    print_run_result(result, cli.json, cli.jsonl, offline_label)?;
    Ok(())
}

fn print_provider_selection_warning(
    selection: &provider_mode::ProviderSelection,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    let Some(warning) = selection.auto_fallback_warning() else {
        return Ok(());
    };
    let warning = redact_secret_fragments(warning);
    if jsonl {
        let message = warning.trim_start_matches("[warning] ").to_string();
        println!(
            "{}",
            to_jsonl_line(&RuntimeEvent::Error {
                thread_id: Some(ThreadId("cli-thread".to_string())),
                turn_id: Some(TurnId("cli-turn".to_string())),
                message,
            })?
        );
    } else if json {
        eprintln!("{warning}");
    } else {
        println!("{warning}");
    }
    Ok(())
}

fn classify_cli_error(error: &anyhow::Error) -> CliExitCode {
    if let Some(agent_error) = find_agent_error(error) {
        return match agent_error {
            AgentError::Provider { .. } => CliExitCode::ProviderError,
            AgentError::EmptyPrompt | AgentError::MissingWorkingDirectory { .. } => {
                CliExitCode::InvalidInput
            }
            AgentError::Execution { message }
                if message.to_ascii_lowercase().contains("cancelled") =>
            {
                CliExitCode::Cancelled
            }
            _ => CliExitCode::InternalError,
        };
    }
    let message = format!("{error:#}").to_ascii_lowercase();
    if message.contains("a prompt is required")
        || message.contains("session not found")
        || message.contains("invalid")
    {
        CliExitCode::InvalidInput
    } else if message.contains("cancelled") || message.contains("canceled") {
        CliExitCode::Cancelled
    } else if message.contains("provider") || message.contains("api key") {
        CliExitCode::ProviderError
    } else if message.contains("tool")
        || message.contains("patch")
        || message.contains("shell")
        || message.contains("sandbox")
    {
        CliExitCode::ToolError
    } else {
        CliExitCode::InternalError
    }
}

fn print_cli_error_jsonl(error: &anyhow::Error) {
    let event = if let Some(AgentError::Provider {
        provider,
        status,
        classification,
        message,
    }) = find_agent_error(error)
    {
        RuntimeEvent::ProviderError {
            thread_id: Some(ThreadId("cli-thread".to_string())),
            turn_id: Some(TurnId("cli-turn".to_string())),
            provider: provider.clone(),
            status: *status,
            classification: classification.clone(),
            message: redact_secret_fragments(message),
        }
    } else if format!("{error:#}")
        .to_ascii_lowercase()
        .contains("cancelled")
    {
        RuntimeEvent::Cancelled {
            thread_id: Some(ThreadId("cli-thread".to_string())),
            turn_id: Some(TurnId("cli-turn".to_string())),
            reason: Some(redact_secret_fragments(&format!("{error:#}"))),
        }
    } else {
        RuntimeEvent::Error {
            thread_id: Some(ThreadId("cli-thread".to_string())),
            turn_id: Some(TurnId("cli-turn".to_string())),
            message: redact_secret_fragments(&format!("{error:#}")),
        }
    };
    if let Ok(line) = to_jsonl_line(&event) {
        println!("{line}");
    }
}

fn find_agent_error(error: &anyhow::Error) -> Option<&AgentError> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<AgentError>())
}

pub(crate) fn redact_secret_fragments(message: &str) -> String {
    yunxi_agent_provider::redact_sensitive_text(message)
}

pub(crate) async fn run_agent_backend(
    backend: BackendKind,
    config: AgentConfig,
    prompt: String,
    provider_live: bool,
) -> Result<AgentRunResult> {
    let result = match backend {
        BackendKind::Yunxi => {
            let cwd = config.cwd.clone();
            let agent = Agent::new(config);
            let backend = if provider_live {
                yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace_with_live_provider(
                    cwd,
                    agent.config(),
                )
            } else {
                build_offline_yunxi_runtime(cwd)
            };
            agent
                .run_with_backend(&backend, AgentInput::text(prompt))
                .await
                .context("yunxi agent run failed")?
        }
        BackendKind::DryRun => {
            let agent = Agent::new(config);
            agent
                .run_dry(AgentInput::text(prompt))
                .await
                .context("agent run failed")?
        }
        BackendKind::Codex => run_codex_backend(config, prompt)
            .await
            .context("codex agent run failed")?,
    };

    Ok(result)
}

pub(crate) async fn run_agent_backend_stream(
    backend: BackendKind,
    config: AgentConfig,
    prompt: String,
    provider_live: bool,
    control: AgentRunControl,
) -> Result<AgentRunResult> {
    let result = match backend {
        BackendKind::Yunxi => {
            let cwd = config.cwd.clone();
            let agent = Agent::new(config);
            let backend = if provider_live {
                yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace_with_live_provider(
                    cwd,
                    agent.config(),
                )
            } else {
                build_offline_yunxi_runtime(cwd)
            };
            agent
                .run_with_backend_stream(&backend, AgentInput::text(prompt), control)
                .await
                .context("yunxi agent run failed")?
        }
        BackendKind::DryRun => {
            let agent = Agent::new(config);
            agent
                .run_with_backend_stream(
                    &yunxi_agent_core::DryRunBackend,
                    AgentInput::text(prompt),
                    control,
                )
                .await
                .context("agent run failed")?
        }
        BackendKind::Codex => run_codex_backend(config, prompt)
            .await
            .context("codex agent run failed")?,
    };

    Ok(result)
}

fn print_run_result(
    result: AgentRunResult,
    json: bool,
    jsonl: bool,
    offline_label: bool,
) -> Result<()> {
    if jsonl {
        for event in protocol_events_from_agent_events(&result.events) {
            println!("{}", to_jsonl_line(&event)?);
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        if offline_label {
            println!("[offline] {final_response}");
        } else {
            println!("{final_response}");
        }
    }

    Ok(())
}

fn build_offline_yunxi_runtime(cwd: PathBuf) -> yunxi_agent_runtime::YunXiRuntimeBackend {
    if runtime_fixtures_enabled() {
        yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace_with_runtime_fixtures(cwd)
    } else {
        yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace(cwd)
    }
}

fn runtime_fixtures_enabled() -> bool {
    std::env::var("YUNXI_RUNTIME_FIXTURES")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on" | "explicit"
            )
        })
        .unwrap_or(false)
}

fn protocol_events_from_agent_events(events: &[AgentEvent]) -> Vec<RuntimeEvent> {
    let mut thread_id = ThreadId("cli-thread".to_string());
    let turn_id = TurnId("cli-turn".to_string());
    let mut output = Vec::new();
    let mut emitted_thread = false;
    let mut emitted_turn = false;

    for event in events {
        match event {
            AgentEvent::ThreadStarted { thread_id: id } => {
                thread_id = ThreadId(id.clone());
                output.push(RuntimeEvent::ThreadStarted {
                    thread_id: thread_id.clone(),
                });
                emitted_thread = true;
            }
            AgentEvent::Started { prompt } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::Item {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: ResponseItem::Message {
                        role: ProtocolRole::User,
                        content: prompt.clone(),
                    },
                });
            }
            AgentEvent::TurnStarted => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
            }
            AgentEvent::ThreadState { state } => {
                let state_thread_id = ThreadId(state.thread_id.clone());
                thread_id = state_thread_id.clone();
                if !emitted_thread {
                    output.push(RuntimeEvent::ThreadStarted {
                        thread_id: state_thread_id.clone(),
                    });
                    emitted_thread = true;
                }
                output.push(RuntimeEvent::ThreadState {
                    thread_id: state_thread_id.clone(),
                    state: ThreadState {
                        thread_id: state_thread_id,
                        session_id: state.session_id.clone(),
                        parent_thread_id: state.parent_thread_id.clone().map(ThreadId),
                        status: state.status.clone(),
                        cwd: state.cwd.clone(),
                        resume_source: state.resume_source.clone(),
                        child_depth: state.child_depth,
                        data: state.data.clone(),
                    },
                });
            }
            AgentEvent::TurnMetadata { metadata } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                let mut protocol_metadata =
                    TurnMetadata::new(thread_id.clone(), turn_id.clone(), metadata.cwd.clone());
                protocol_metadata.session_id = metadata.session_id.clone();
                protocol_metadata.model = metadata.model.clone();
                protocol_metadata.provider = metadata.provider.clone();
                protocol_metadata.approval_mode = metadata.approval_mode.clone();
                protocol_metadata.sandbox_mode = metadata.sandbox_mode.clone();
                protocol_metadata.extra = metadata.data.clone();
                if let Some(context_phase) = &metadata.context_phase {
                    protocol_metadata
                        .extra
                        .insert("context_phase".to_string(), context_phase.clone());
                }
                if let Some(resume_source) = &metadata.resume_source {
                    protocol_metadata
                        .extra
                        .insert("resume_source".to_string(), resume_source.clone());
                }
                if let Some(cancellation_state) = &metadata.cancellation_state {
                    protocol_metadata
                        .extra
                        .insert("cancellation_state".to_string(), cancellation_state.clone());
                }
                protocol_metadata
                    .extra
                    .insert("child_depth".to_string(), metadata.child_depth.to_string());
                output.push(RuntimeEvent::TurnMetadata {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    metadata: protocol_metadata,
                });
            }
            AgentEvent::TurnState { state } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::TurnState {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    state: TurnState {
                        phase: state.phase.clone(),
                        status: state.status.clone(),
                        provider_status: state.provider_status.clone(),
                        tool_loop_status: state.tool_loop_status.clone(),
                        cancellation_state: state.cancellation_state.clone(),
                        data: state.data.clone(),
                    },
                });
            }
            AgentEvent::DeepParityState {
                layer,
                status,
                message,
                data,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::DeepParityState {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    layer: layer.clone(),
                    status: status.clone(),
                    message: message.clone(),
                    data: data.clone(),
                });
            }
            AgentEvent::SandboxAttempt {
                id,
                platform,
                status,
                backend,
                command,
                cwd,
                message,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::SandboxAttempt {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: id.clone(),
                    platform: platform.clone(),
                    status: status.clone(),
                    backend: backend.clone(),
                    command: command.clone(),
                    cwd: cwd.clone(),
                    message: message.clone(),
                });
            }
            AgentEvent::ApprovalCacheState {
                session_id,
                tool_name,
                key,
                decision,
                reused,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::ApprovalCacheState {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    session_id: session_id.clone(),
                    tool_name: tool_name.clone(),
                    key: key.clone(),
                    decision: decision.clone(),
                    reused: *reused,
                });
            }
            AgentEvent::Message { content } => output.push(RuntimeEvent::Item {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::Message {
                    role: ProtocolRole::Assistant,
                    content: content.clone(),
                },
            }),
            AgentEvent::Reasoning { content } => output.push(RuntimeEvent::Item {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::Reasoning {
                    content: content.clone(),
                },
            }),
            AgentEvent::CommandStarted { id, command } => output.push(RuntimeEvent::ToolStarted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call: ToolCall::Shell {
                    id: id.clone(),
                    command: command.clone(),
                },
            }),
            AgentEvent::CommandUpdated {
                id,
                aggregated_output,
                ..
            } => output.push(RuntimeEvent::ItemDelta {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                delta: ResponseItemDelta::ToolOutput {
                    call_id: id.clone(),
                    delta: aggregated_output.clone(),
                },
            }),
            AgentEvent::CommandCompleted {
                id,
                aggregated_output,
                status,
                ..
            } => output.push(RuntimeEvent::ToolCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                output: aggregated_output.clone(),
                success: *status == CommandStatus::Completed,
            }),
            AgentEvent::CommandFinished { command, exit_code } => {
                output.push(RuntimeEvent::ToolCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: None,
                    output: format!("{command} exited with {exit_code}"),
                    success: *exit_code == 0,
                });
            }
            AgentEvent::PatchCompleted { status } => output.push(RuntimeEvent::Item {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::FunctionCallOutput {
                    id: "patch".to_string(),
                    call_id: "patch".to_string(),
                    output: FunctionCallOutput::text(format!("patch status: {status:?}")),
                },
            }),
            AgentEvent::McpToolStarted { id, server, tool } => {
                output.push(RuntimeEvent::ToolStarted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    call: ToolCall::Mcp {
                        id: id.clone(),
                        server: server.clone(),
                        tool: tool.clone(),
                        arguments_json: None,
                    },
                });
            }
            AgentEvent::McpToolCompleted {
                id,
                server,
                tool,
                status,
            } => output.push(RuntimeEvent::Item {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::McpToolCall {
                    id: id.clone().unwrap_or_else(|| "mcp".to_string()),
                    call_id: id.clone().unwrap_or_else(|| "mcp".to_string()),
                    server: server.clone(),
                    tool: tool.clone(),
                    arguments: String::new(),
                    status: match status {
                        yunxi_agent_core::McpToolStatus::Completed => ToolCallStatus::Completed,
                        yunxi_agent_core::McpToolStatus::InProgress => ToolCallStatus::InProgress,
                        yunxi_agent_core::McpToolStatus::Failed => ToolCallStatus::Failed,
                    },
                },
            }),
            AgentEvent::ToolCallStarted {
                id,
                name,
                arguments_json,
            } => output.push(RuntimeEvent::ToolStarted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call: dynamic_tool_call(id.clone(), name, arguments_json.clone()),
            }),
            AgentEvent::ToolCallCompleted {
                id,
                name,
                output: tool_output,
                status,
            } => output.push(RuntimeEvent::ToolCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                output: format!("{name}: {tool_output}"),
                success: *status == CommandStatus::Completed,
            }),
            AgentEvent::ApprovalRequested {
                id,
                tool_name,
                reason,
            } => output.push(RuntimeEvent::ApprovalRequested {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                tool_name: tool_name.clone(),
                reason: reason.clone(),
            }),
            AgentEvent::ApprovalCompleted {
                id,
                approved,
                reason,
            } => output.push(RuntimeEvent::ApprovalCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                approved: *approved,
                reason: reason.clone(),
            }),
            AgentEvent::EscalationRequested {
                id,
                tool_name,
                reason,
                required_sandbox,
                required_network,
            } => output.push(RuntimeEvent::EscalationRequested {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                tool_name: tool_name.clone(),
                reason: reason.clone(),
                required_sandbox: required_sandbox.clone(),
                required_network: required_network.clone(),
            }),
            AgentEvent::EscalationCompleted {
                id,
                approved,
                reason,
            } => output.push(RuntimeEvent::EscalationCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                call_id: id.clone(),
                approved: *approved,
                reason: reason.clone(),
            }),
            AgentEvent::McpSession {
                server,
                status,
                message,
            } => output.push(RuntimeEvent::McpSession {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                server: server.clone(),
                status: status.clone(),
                message: message.clone(),
            }),
            AgentEvent::MultiAgentEvent {
                agent_id,
                parent_agent_id,
                status,
                message,
            } => output.push(RuntimeEvent::MultiAgent {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                agent_id: agent_id.clone(),
                parent_agent_id: parent_agent_id.clone(),
                status: status.clone(),
                message: message.clone(),
            }),
            AgentEvent::ChildAgentEvent {
                agent_id,
                child_session_id,
                parent_session_id,
                status,
                message,
            } => output.push(RuntimeEvent::ChildAgent {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                agent_id: agent_id.clone(),
                child_session_id: child_session_id.clone(),
                parent_session_id: parent_session_id.clone(),
                status: status.clone(),
                message: message.clone(),
            }),
            AgentEvent::ChildScopedStream {
                agent_id,
                child_session_id,
                parent_session_id,
                event,
                seq,
                message,
            } => output.push(RuntimeEvent::ChildScopedStream {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                agent_id: agent_id.clone(),
                child_session_id: child_session_id.clone(),
                parent_session_id: parent_session_id.clone(),
                event: event.clone(),
                seq: *seq,
                message: message.clone(),
            }),
            AgentEvent::ContextStatus {
                active_context_tokens,
                token_limit_reached,
                compacted,
                dropped_messages,
            } => output.push(RuntimeEvent::ContextStatus {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                active_context_tokens: *active_context_tokens,
                token_limit_reached: *token_limit_reached,
                compacted: *compacted,
                dropped_messages: *dropped_messages,
            }),
            AgentEvent::StorageState {
                session_id,
                parent_session_id,
                rollout_items,
                rollout_truncated,
                child_session_ids,
            } => output.push(RuntimeEvent::StorageState {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                session_id: session_id.clone(),
                parent_session_id: parent_session_id.clone(),
                rollout_items: *rollout_items,
                rollout_truncated: *rollout_truncated,
                child_session_ids: child_session_ids.clone(),
            }),
            AgentEvent::FileChanged { path, kind } => output.push(RuntimeEvent::FileChanged {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                path: path.clone(),
                kind: format!("{kind:?}"),
            }),
            AgentEvent::TodoUpdated { id, items } => output.push(RuntimeEvent::Item {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::Reasoning {
                    content: format!("todo updated {:?}: {} item(s)", id, items.len()),
                },
            }),
            AgentEvent::Warning { message } | AgentEvent::Error { message } => {
                output.push(RuntimeEvent::Error {
                    thread_id: Some(thread_id.clone()),
                    turn_id: Some(turn_id.clone()),
                    message: message.clone(),
                });
            }
            AgentEvent::Cancelled { reason } => output.push(RuntimeEvent::Cancelled {
                thread_id: Some(thread_id.clone()),
                turn_id: Some(turn_id.clone()),
                reason: reason.clone(),
            }),
            AgentEvent::ProviderError {
                provider,
                status,
                classification,
                message,
            } => output.push(RuntimeEvent::ProviderError {
                thread_id: Some(thread_id.clone()),
                turn_id: Some(turn_id.clone()),
                provider: provider.clone(),
                status: *status,
                classification: classification.clone(),
                message: message.clone(),
            }),
            AgentEvent::Completed { status, .. } => {
                output.push(RuntimeEvent::TurnCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                });
                if *status == yunxi_agent_core::AgentRunStatus::Failed {
                    output.push(RuntimeEvent::Error {
                        thread_id: Some(thread_id.clone()),
                        turn_id: Some(turn_id.clone()),
                        message: "agent run failed".to_string(),
                    });
                } else if *status == yunxi_agent_core::AgentRunStatus::Cancelled {
                    output.push(RuntimeEvent::Cancelled {
                        thread_id: Some(thread_id.clone()),
                        turn_id: Some(turn_id.clone()),
                        reason: Some("agent run cancelled".to_string()),
                    });
                }
            }
        }
    }

    output
}

fn dynamic_tool_call(id: Option<String>, name: &str, arguments_json: Option<String>) -> ToolCall {
    let value = arguments_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let arg = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
    };
    match name {
        "skill" => ToolCall::Skill {
            id,
            name: arg("name").unwrap_or_else(|| "skill".to_string()),
            arguments_json,
        },
        "multi_agent" => ToolCall::MultiAgent {
            id,
            action: arg("action").unwrap_or_else(|| "unknown".to_string()),
            arguments_json,
        },
        "tool_search" => ToolCall::ToolSearch {
            id,
            query: arg("query").unwrap_or_default(),
        },
        "request_user_input" => ToolCall::RequestUserInput {
            id,
            prompt: arg("prompt").unwrap_or_default(),
        },
        "view_image" => ToolCall::ViewImage {
            id,
            path: arg("path").unwrap_or_default(),
        },
        other => ToolCall::MultiAgent {
            id,
            action: other.to_string(),
            arguments_json,
        },
    }
}

fn ensure_protocol_turn_started(
    output: &mut Vec<RuntimeEvent>,
    thread_id: &ThreadId,
    turn_id: &TurnId,
    emitted_thread: &mut bool,
    emitted_turn: &mut bool,
) {
    if !*emitted_thread {
        output.push(RuntimeEvent::ThreadStarted {
            thread_id: thread_id.clone(),
        });
        *emitted_thread = true;
    }
    if !*emitted_turn {
        output.push(RuntimeEvent::TurnStarted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        });
        *emitted_turn = true;
    }
}

async fn run_command(
    command: CliCommand,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    match command {
        CliCommand::Sessions { command } => {
            run_session_command(command, config, backend, provider_mode, json, jsonl).await
        }
        CliCommand::Parity { command } => run_parity_command(command, json).await,
    }
}

async fn run_parity_command(command: ParityCommand, json: bool) -> Result<()> {
    match command {
        ParityCommand::Map => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "path": "docs/extraction-index/codex-core-agent-parity-map.md",
                        "content": CODEX_CORE_PARITY_MAP,
                    })
                );
            } else {
                print!("{CODEX_CORE_PARITY_MAP}");
            }
        }
    }
    Ok(())
}

async fn run_session_command(
    command: SessionCommand,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    let store = FileSessionStore::for_workspace(&config.cwd);
    match command {
        SessionCommand::List => {
            let sessions = store.list().await.context("failed to list sessions")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                for session in sessions {
                    println!(
                        "{}\t{:?}\t{}\t{}\tarchived={}\tpinned={}\t{}",
                        session.id.0,
                        session.status,
                        session.cwd.display(),
                        session.created_at_millis,
                        session.archived,
                        session.pinned,
                        preview(&session.prompt)
                    );
                }
            }
        }
        SessionCommand::Show { id } => {
            let session = store
                .load(&SessionId::new(id.clone()))
                .await
                .context("failed to load session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&session)?);
            } else {
                print_session(&session);
            }
        }
        SessionCommand::Rollout { id } => {
            let session = store
                .load(&SessionId::new(id.clone()))
                .await
                .context("failed to load session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            let rollout = RolloutRecord::from(session);
            print_rollout(&rollout, json)?;
        }
        SessionCommand::History { id } => {
            let history = store
                .history(&SessionId::new(id.clone()), HistoryLoadOptions::default())
                .await
                .context("failed to load session history")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_history(&history, json)?;
        }
        SessionCommand::Graph => {
            let graph = SessionGraphView::from_sessions(
                store
                    .list()
                    .await
                    .context("failed to list sessions for graph")?,
            )?;
            print_session_graph(&graph, json)?;
        }
        SessionCommand::Resume { id, prompt } => {
            let session_id = SessionId::new(id.clone());
            let session = store
                .load(&session_id)
                .await
                .context("failed to load session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            let mut resume_config = config
                .with_parent_session_id(id.clone())
                .with_session_title(format!("Resume {id}"));
            if resume_config.model.is_none() {
                if let Some(model) = session.model.clone() {
                    resume_config = resume_config.with_model(model);
                }
            }
            if resume_config.provider.is_none() {
                if let Some(provider) = session.provider.clone() {
                    resume_config = resume_config.with_provider(provider);
                }
            }
            let resume_prompt = build_resume_prompt(&prompt.join(" "));
            let selection = provider_mode.resolve(backend, &resume_config)?;
            let resume_config = selection.apply_to_config(resume_config);
            print_provider_selection_warning(&selection, json, jsonl)?;
            let result =
                run_agent_backend(backend, resume_config, resume_prompt, selection.live).await?;
            print_run_result(result, json, jsonl, selection.is_offline_runtime())?;
        }
        SessionCommand::Fork { id } => {
            let forked = store
                .fork(&SessionId::new(id.clone()))
                .await
                .context("failed to fork session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_record(&forked, json)?;
        }
        SessionCommand::Archive { id } => {
            let archived = store
                .archive(&SessionId::new(id.clone()), true)
                .await
                .context("failed to archive session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_record(&archived, json)?;
        }
        SessionCommand::Unarchive { id } => {
            let unarchived = store
                .archive(&SessionId::new(id.clone()), false)
                .await
                .context("failed to unarchive session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_record(&unarchived, json)?;
        }
        SessionCommand::Pin { id } => {
            let pinned = store
                .pin(&SessionId::new(id.clone()), true)
                .await
                .context("failed to pin session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_record(&pinned, json)?;
        }
        SessionCommand::Unpin { id } => {
            let unpinned = store
                .pin(&SessionId::new(id.clone()), false)
                .await
                .context("failed to unpin session")?
                .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;
            print_record(&unpinned, json)?;
        }
    }
    Ok(())
}

fn build_resume_prompt(prompt: &str) -> String {
    if prompt.trim().is_empty() {
        "Continue from the previous session."
    } else {
        prompt.trim()
    }
    .to_string()
}

fn print_record(record: &SessionRecord, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(record)?);
    } else {
        print_session(record);
    }
    Ok(())
}

fn print_rollout(rollout: &RolloutRecord, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(rollout)?);
    } else {
        println!("thread_id: {}", rollout.thread.id.0);
        if let Some(parent_id) = &rollout.thread.parent_id {
            println!("parent_id: {}", parent_id.0);
        }
        println!("status: {:?}", rollout.status);
        println!("items: {}", rollout.items.len());
        println!("prompt: {}", rollout.prompt);
        if let Some(final_response) = &rollout.final_response {
            println!("final_response: {final_response}");
        }
    }
    Ok(())
}

fn print_history(history: &SessionHistory, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(history)?);
    } else {
        println!("sessions: {}", history.sessions.len());
        println!("items: {}", history.items.len());
        for item in &history.items {
            println!(
                "{}\t{:?}\t{}",
                item.session_id.0,
                item.kind,
                preview(&item.content)
            );
        }
    }
    Ok(())
}

fn print_session_graph(graph: &SessionGraphView, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(graph)?);
    } else {
        println!("sessions: {}", graph.sessions.len());
        for (id, session) in &graph.sessions {
            let children = graph.children.get(id).map(Vec::len).unwrap_or_default();
            println!(
                "{}\tparent={}\tchildren={}\tarchived={}\tpinned={}\t{}",
                id.0,
                session
                    .parent_id
                    .as_ref()
                    .map(|parent| parent.0.as_str())
                    .unwrap_or("none"),
                children,
                session.archived,
                session.pinned,
                session
                    .task
                    .as_deref()
                    .or(session.title.as_deref())
                    .map(preview)
                    .unwrap_or_else(|| "untitled".to_string())
            );
        }
    }
    Ok(())
}

fn preview(prompt: &str) -> String {
    const MAX: usize = 80;
    if prompt.chars().count() <= MAX {
        return prompt.to_string();
    }
    let mut value = prompt
        .chars()
        .take(MAX.saturating_sub(3))
        .collect::<String>();
    value.push_str("...");
    value
}

fn print_session(session: &SessionRecord) {
    println!("id: {}", session.id.0);
    if let Some(parent_id) = &session.parent_id {
        println!("parent_id: {}", parent_id.0);
    }
    if let Some(title) = &session.title {
        println!("title: {title}");
    }
    println!("status: {:?}", session.status);
    println!("cwd: {}", session.cwd.display());
    println!("created_at_millis: {}", session.created_at_millis);
    println!("updated_at_millis: {}", session.updated_at_millis);
    println!("archived: {}", session.archived);
    println!("pinned: {}", session.pinned);
    if let Some(model) = &session.model {
        println!("model: {model}");
    }
    if let Some(provider) = &session.provider {
        println!("provider: {provider}");
    }
    println!("prompt: {}", session.prompt);
    if let Some(final_response) = &session.final_response {
        println!("final_response: {final_response}");
    }
    println!("events: {}", session.events.len());
}

async fn run_codex_backend(config: AgentConfig, prompt: String) -> Result<AgentRunResult> {
    let _ = (config, prompt);
    bail!(
        "codex compatibility backend is detached from the default CLI; use the yunxi-agent-codex compatibility crate explicitly"
    )
}
