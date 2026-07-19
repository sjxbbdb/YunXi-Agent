use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;
use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunControl, AgentRunResult,
    ApprovalMode, BackendKind, CommandStatus, ControlAuditRecord, ControlRequest, ControlScope,
    ControlSnapshot, ControlVerb, MemoryExtractionMode, SandboxMode,
};
use yunxi_agent_persona::{
    MemoryKind, MemoryRecord, MemorySensitivity, MemoryStatus, PersonaSettings,
    yunxi_companion_strong,
};
use yunxi_agent_protocol::{
    FunctionCallOutput, ProtocolRole, ResponseItem, ResponseItemDelta, RuntimeEvent, ThreadId,
    ThreadState, ToolCall, ToolCallStatus, TurnId, TurnMetadata, TurnState, to_jsonl_line,
};
use yunxi_agent_runtime::control_snapshot;
use yunxi_agent_storage::{
    FileControlStore, FilePersonaMemoryStore, FileSessionStore, HistoryLoadOptions,
    PersonaMemoryScope, RolloutRecord, SessionGraphView, SessionHistory, SessionId, SessionRecord,
    SessionStore, SessionSummary,
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
mod jsonl_redaction {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/jsonl_redaction.rs"
    ));
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
mod workspace;
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
#[command(about = "YunXi Agent v2.0.4 interactive terminal CLI")]
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

    #[arg(long, value_name = "PATH", global = true)]
    cwd: Option<PathBuf>,

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
        long = "memory-extraction",
        value_name = "MODE",
        value_enum,
        default_value_t = CliMemoryExtractionMode::Auto,
        help = "Memory extraction mode: auto, rule-only, or provider"
    )]
    memory_extraction: CliMemoryExtractionMode,

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

    #[arg(long, global = true, conflicts_with = "jsonl")]
    json: bool,

    #[arg(
        long,
        global = true,
        conflicts_with = "json",
        help = "Emit agent execution events as JSON Lines; metadata commands reject this flag"
    )]
    jsonl: bool,

    #[arg(long, global = true, conflicts_with = "no_tui")]
    tui: bool,

    #[arg(long = "no-tui", global = true)]
    no_tui: bool,

    #[arg(
        long,
        global = true,
        help = "Enable conservative proactive companion planning"
    )]
    companion: bool,

    #[command(subcommand)]
    command: Option<CliCommand>,

    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    Run {
        #[arg(value_name = "PROMPT", num_args = 1..)]
        prompt: Vec<String>,
    },
    Sessions {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Parity {
        #[command(subcommand)]
        command: ParityCommand,
    },
    Persona {
        #[command(subcommand)]
        command: PersonaCommand,
    },
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    Companion {
        #[command(subcommand)]
        command: CompanionCommand,
    },
    Controls {
        #[command(subcommand)]
        command: ControlCommand,
    },
    Eval {
        #[command(subcommand)]
        command: EvalCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CompanionCommand {
    Status,
    On,
    Off,
    History,
    Clear {
        #[arg(long)]
        confirm: bool,
    },
    Check {
        #[arg(value_name = "CONTEXT")]
        context: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ControlCommand {
    Status,
    Show {
        #[arg(value_enum)]
        scope: CliControlScope,
    },
    Enable {
        #[arg(value_enum)]
        scope: CliControlScope,
    },
    Disable {
        #[arg(value_enum)]
        scope: CliControlScope,
    },
    Clear {
        #[arg(value_enum)]
        scope: CliControlScope,
        #[arg(long)]
        confirm: bool,
    },
    Refresh,
    Audit,
}

#[derive(Debug, Subcommand)]
enum EvalCommand {
    Companion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliControlScope {
    Companion,
    Memory,
    Persona,
    Relationship,
}

impl From<CliControlScope> for ControlScope {
    fn from(scope: CliControlScope) -> Self {
        match scope {
            CliControlScope::Companion => Self::Companion,
            CliControlScope::Memory => Self::Memory,
            CliControlScope::Persona => Self::Persona,
            CliControlScope::Relationship => Self::Relationship,
        }
    }
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

#[derive(Debug, Subcommand)]
enum PersonaCommand {
    Status,
    Profile,
    Set {
        #[arg(value_name = "PROFILE")]
        profile: String,
    },
    On,
    Off,
}

#[derive(Debug, Subcommand)]
enum MemoryCommand {
    Status,
    List {
        #[arg(long, conflicts_with = "workspace")]
        global: bool,
        #[arg(long, conflicts_with = "global")]
        workspace: bool,
    },
    Show {
        #[arg(value_name = "ID")]
        id: String,
    },
    Search {
        #[arg(value_name = "QUERY", num_args = 1..)]
        query: Vec<String>,
        #[arg(long, conflicts_with = "workspace")]
        global: bool,
        #[arg(long, conflicts_with = "global")]
        workspace: bool,
    },
    Pending,
    Approve {
        #[arg(value_name = "ID")]
        id: String,
    },
    Reject {
        #[arg(value_name = "ID")]
        id: String,
    },
    Delete {
        #[arg(value_name = "ID")]
        id: String,
    },
    Clear {
        #[arg(long)]
        workspace: bool,
        #[arg(long)]
        confirm: bool,
    },
    On,
    Off,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliMemoryExtractionMode {
    Auto,
    RuleOnly,
    Provider,
}

impl From<CliMemoryExtractionMode> for MemoryExtractionMode {
    fn from(mode: CliMemoryExtractionMode) -> Self {
        match mode {
            CliMemoryExtractionMode::Auto => Self::Auto,
            CliMemoryExtractionMode::RuleOnly => Self::RuleOnly,
            CliMemoryExtractionMode::Provider => Self::Provider,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliBackend {
    DryRun,
    Yunxi,
    #[allow(dead_code)]
    #[value(skip)]
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
    let Some(cli) = parse_cli()? else {
        return Ok(());
    };
    let provider_mode =
        provider_mode::ProviderMode::from_flags(cli.provider_live || cli.live, cli.offline);

    let backend = cli.backend.into();

    let cwd = workspace::resolve_cli_cwd(cli.cwd.clone())?;
    let mut config = AgentConfig::new(cwd)
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
    config = config.with_memory_extraction_mode(cli.memory_extraction.into());
    let persisted_settings = PersonaSettings::load();
    config.companion.enabled = persisted_settings.companion_enabled;
    config.companion.cloud_control_enabled = persisted_settings.cloud_control_enabled;
    if cli.companion
        || matches!(
            cli.command,
            Some(CliCommand::Companion {
                command: CompanionCommand::Check { .. }
            })
        )
    {
        config.companion.enabled = true;
        config.companion.allow_tool_requests = true;
    }

    reject_detached_codex_backend(backend)?;

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

    run_prompt(prompt, config, backend, provider_mode, cli.json, cli.jsonl).await
}

fn parse_cli() -> Result<Option<Cli>> {
    match Cli::try_parse() {
        Ok(cli) => Ok(Some(cli)),
        Err(error) if !error.use_stderr() => {
            print!("{error}");
            Ok(None)
        }
        Err(error) => bail!("{}", friendly_clap_error(&error)),
    }
}

fn friendly_clap_error(error: &clap::Error) -> String {
    let mut message = error.to_string();
    if message.contains("yunxi sessions")
        || message.contains("yunxi.exe sessions")
        || message.contains("yunxi-agent-cli.exe sessions")
        || message.contains("sessions [OPTIONS]")
    {
        message.push_str(
            "\nIf you meant to ask about `sessions` as a prompt, run `yunxi -- sessions` or `yunxi run sessions`.",
        );
    }
    if message.contains("yunxi parity")
        || message.contains("yunxi.exe parity")
        || message.contains("yunxi-agent-cli.exe parity")
        || message.contains("parity [OPTIONS]")
    {
        message.push_str(
            "\nIf you meant to ask about `parity` as a prompt, run `yunxi -- parity` or `yunxi run parity`.",
        );
    }
    message
}

fn reject_detached_codex_backend(backend: BackendKind) -> Result<()> {
    if backend == BackendKind::Codex {
        bail!(
            "codex compatibility backend is detached from the default CLI; use the yunxi-agent-codex compatibility crate explicitly"
        );
    }
    Ok(())
}

async fn run_prompt(
    prompt: String,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    let selection = provider_mode.resolve(backend, &config)?;
    let config = selection.apply_to_config(config);
    let offline_label = selection.is_offline_runtime();
    print_provider_selection_warning(&selection, json, jsonl)?;
    let result = run_agent_backend(backend, config, prompt, selection.live).await?;
    print_run_result(result, json, jsonl, offline_label)?;
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
        || message.contains("cannot be used with")
        || message.contains("required")
        || message.contains("unrecognized")
        || message.contains("usage:")
        || message.contains("only supported")
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
            let event = jsonl_redaction::redact_runtime_event_for_jsonl(event);
            println!("{}", to_jsonl_line(&event)?);
        }
    } else if json {
        let result = jsonl_redaction::redact_agent_run_result_for_json(result);
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
                schema_version,
                platform,
                status,
                backend,
                backend_id,
                backend_label,
                os_isolation,
                enforcement,
                enforcement_level,
                runner,
                unsupported_reason,
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
                    schema_version: *schema_version,
                    platform: platform.clone(),
                    status: status.clone(),
                    backend: backend.clone(),
                    backend_id: backend_id.clone(),
                    backend_label: backend_label.clone(),
                    os_isolation: *os_isolation,
                    enforcement: enforcement.clone(),
                    enforcement_level: enforcement_level.clone(),
                    runner: runner.clone(),
                    unsupported_reason: unsupported_reason.clone(),
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
            AgentEvent::PersonaLoaded {
                schema_version,
                profile_id,
                display_name,
                enabled,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::PersonaLoaded {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    profile_id: profile_id.clone(),
                    display_name: display_name.clone(),
                    enabled: *enabled,
                });
            }
            AgentEvent::PersonaContextInjected {
                schema_version,
                profile_id,
                memory_count,
                budget_used_chars,
                budget_limit_chars,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::PersonaContextInjected {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    profile_id: profile_id.clone(),
                    memory_count: *memory_count,
                    budget_used_chars: *budget_used_chars,
                    budget_limit_chars: *budget_limit_chars,
                });
            }
            AgentEvent::MemoryRecall {
                schema_version,
                enabled,
                scope,
                query,
                count,
                budget_used_chars,
                truncated,
                always_on_count,
                dropped_unrelated,
                dropped_by_budget,
                dropped_duplicates,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::MemoryRecall {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    enabled: *enabled,
                    scope: scope.clone(),
                    query: query.clone(),
                    count: *count,
                    budget_used_chars: *budget_used_chars,
                    truncated: *truncated,
                    always_on_count: *always_on_count,
                    dropped_unrelated: *dropped_unrelated,
                    dropped_by_budget: *dropped_by_budget,
                    dropped_duplicates: *dropped_duplicates,
                });
            }
            AgentEvent::MemoryCandidate {
                schema_version,
                id,
                kind,
                sensitivity,
                status,
                write_policy,
                reason,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::MemoryCandidate {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    id: id.clone(),
                    kind: kind.clone(),
                    sensitivity: sensitivity.clone(),
                    status: status.clone(),
                    write_policy: write_policy.clone(),
                    reason: reason.clone(),
                });
            }
            AgentEvent::MemoryWrite {
                schema_version,
                id,
                scope,
                kind,
                status,
                action,
                revision,
                merged_count,
                merge_strategy,
                conflict_family,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::MemoryWrite {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    id: id.clone(),
                    scope: scope.clone(),
                    kind: kind.clone(),
                    status: status.clone(),
                    action: action.clone(),
                    revision: *revision,
                    merged_count: *merged_count,
                    merge_strategy: merge_strategy.clone(),
                    conflict_family: conflict_family.clone(),
                });
            }
            AgentEvent::MemoryWarning {
                schema_version,
                warning,
            } => {
                ensure_protocol_turn_started(
                    &mut output,
                    &thread_id,
                    &turn_id,
                    &mut emitted_thread,
                    &mut emitted_turn,
                );
                output.push(RuntimeEvent::MemoryWarning {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    schema_version: *schema_version,
                    warning: warning.clone(),
                });
            }
            AgentEvent::Message { content, .. } => output.push(RuntimeEvent::Item {
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
            } => {
                let call_id = id.clone().unwrap_or_else(|| "mcp".to_string());
                let protocol_status = match status {
                    yunxi_agent_core::McpToolStatus::Completed => ToolCallStatus::Completed,
                    yunxi_agent_core::McpToolStatus::InProgress => ToolCallStatus::InProgress,
                    yunxi_agent_core::McpToolStatus::Failed => ToolCallStatus::Failed,
                };
                output.push(RuntimeEvent::ToolCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: Some(call_id.clone()),
                    output: format!("{server}.{tool} status: {status:?}"),
                    success: *status == yunxi_agent_core::McpToolStatus::Completed,
                });
                output.push(RuntimeEvent::Item {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: ResponseItem::McpToolCall {
                        id: call_id.clone(),
                        call_id,
                        server: server.clone(),
                        tool: tool.clone(),
                        arguments: String::new(),
                        status: protocol_status,
                    },
                });
            }
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
    ensure_command_jsonl_supported(&command, jsonl)?;
    match command {
        CliCommand::Run { prompt } => {
            run_prompt(
                prompt.join(" "),
                config,
                backend,
                provider_mode,
                json,
                jsonl,
            )
            .await
        }
        CliCommand::Sessions { command } => {
            run_session_command(command, config, backend, provider_mode, json, jsonl).await
        }
        CliCommand::Parity { command } => run_parity_command(command, json).await,
        CliCommand::Persona { command } => run_persona_command(command, &config, json).await,
        CliCommand::Memory { command } => run_memory_command(command, config, json).await,
        CliCommand::Companion { command } => {
            run_companion_command(command, config, backend, provider_mode, json, jsonl).await
        }
        CliCommand::Controls { command } => run_control_command(command, config, json).await,
        CliCommand::Eval { command } => run_eval_command(command, json, jsonl).await,
    }
}

fn ensure_command_jsonl_supported(command: &CliCommand, jsonl: bool) -> Result<()> {
    if !jsonl {
        return Ok(());
    }
    match command {
        CliCommand::Run { .. }
        | CliCommand::Sessions {
            command: SessionCommand::Resume { .. },
        } => Ok(()),
        CliCommand::Sessions { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for sessions metadata commands"
        ),
        CliCommand::Parity { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for parity commands"
        ),
        CliCommand::Persona { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for persona management commands"
        ),
        CliCommand::Memory { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for memory management commands"
        ),
        CliCommand::Companion { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for companion management commands"
        ),
        CliCommand::Controls { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.0.4; use --json for control commands"
        ),
        CliCommand::Eval { .. } => Ok(()),
    }
}

async fn run_eval_command(command: EvalCommand, json: bool, jsonl: bool) -> Result<()> {
    match command {
        EvalCommand::Companion => {
            let report = yunxi_agent_eval::run_default_suite()
                .map_err(|error| anyhow::anyhow!("companion evaluation failed to load: {error}"))?;
            if jsonl {
                println!("{}", serde_json::to_string(&report)?);
            } else if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", yunxi_agent_eval::render_text(&report));
            }
            if report.metrics.failed_scenarios > 0 || !report.golden_passed {
                bail!(
                    "companion evaluation failed: {} scenarios failed, golden_failures={}",
                    report.metrics.failed_scenarios,
                    report.golden_failures.len()
                );
            }
        }
    }
    Ok(())
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

async fn run_companion_command(
    command: CompanionCommand,
    mut config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    match command {
        CompanionCommand::Check { context } => {
            config.companion.enabled = true;
            config.companion.allow_tool_requests = true;
            let context = context.join(" ");
            let prompt = if context.trim().is_empty() {
                "long idle companion check".to_string()
            } else {
                format!("companion check: {context}")
            };
            run_prompt(prompt, config, backend, provider_mode, json, jsonl).await
        }
        CompanionCommand::Status => {
            run_control_command(
                ControlCommand::Show {
                    scope: CliControlScope::Companion,
                },
                config,
                json,
            )
            .await
        }
        CompanionCommand::On => {
            run_control_command(
                ControlCommand::Enable {
                    scope: CliControlScope::Companion,
                },
                config,
                json,
            )
            .await
        }
        CompanionCommand::Off => {
            run_control_command(
                ControlCommand::Disable {
                    scope: CliControlScope::Companion,
                },
                config,
                json,
            )
            .await
        }
        CompanionCommand::Clear { confirm } => {
            run_control_command(
                ControlCommand::Clear {
                    scope: CliControlScope::Companion,
                    confirm,
                },
                config,
                json,
            )
            .await
        }
        CompanionCommand::History => {
            let store = FileControlStore::for_workspace(&config.cwd);
            let request = ControlRequest::new(ControlScope::Companion, ControlVerb::Show);
            let records = store.companion_history()?;
            append_control_audit(
                &store,
                &request,
                "completed",
                format!("records={}", records.len()),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else if records.is_empty() {
                println!("companion history: empty");
            } else {
                for record in records {
                    println!(
                        "{} trigger={} confirm={} reason={} message={}",
                        record.timestamp_millis,
                        record.trigger,
                        record.requires_user_confirmation,
                        record.reason,
                        record.message
                    );
                }
            }
            Ok(())
        }
    }
}

async fn run_control_command(
    command: ControlCommand,
    mut config: AgentConfig,
    json: bool,
) -> Result<()> {
    let store = FileControlStore::for_workspace(&config.cwd);
    let mut settings = PersonaSettings::load();
    match command {
        ControlCommand::Status => {
            let request = ControlRequest::new(ControlScope::Companion, ControlVerb::Show);
            let snapshot = control_snapshot(&config)?;
            append_control_audit(&store, &request, "completed", "unified control snapshot")?;
            print_control_snapshot(&snapshot, None, json)?;
        }
        ControlCommand::Refresh => {
            let request = ControlRequest::new(ControlScope::Companion, ControlVerb::Refresh);
            let snapshot = control_snapshot(&config)?;
            append_control_audit(&store, &request, "completed", "refreshed unified snapshot")?;
            print_control_snapshot(&snapshot, None, json)?;
        }
        ControlCommand::Show { scope } => {
            let scope = scope.into();
            let request = ControlRequest::new(scope, ControlVerb::Show);
            let snapshot = control_snapshot(&config)?;
            append_control_audit(&store, &request, "completed", "scope snapshot")?;
            print_control_snapshot(&snapshot, Some(scope), json)?;
        }
        ControlCommand::Enable { scope } => {
            let scope = scope.into();
            let request = ControlRequest::new(scope, ControlVerb::Enable);
            match scope {
                ControlScope::Companion => {
                    settings.companion_enabled = true;
                    config.companion.enabled = true;
                }
                ControlScope::Memory => settings.memory_enabled = true,
                ControlScope::Persona => settings.persona_enabled = true,
                ControlScope::Relationship => {
                    append_control_audit(
                        &store,
                        &request,
                        "rejected",
                        "relationship is a read-only derived view",
                    )?;
                    bail!("relationship controls are read-only");
                }
            }
            settings
                .save()
                .context("failed to persist control settings")?;
            append_control_audit(&store, &request, "completed", "persisted enabled state")?;
            print_control_snapshot(&control_snapshot(&config)?, Some(scope), json)?;
        }
        ControlCommand::Disable { scope } => {
            let scope = scope.into();
            let request = ControlRequest::new(scope, ControlVerb::Disable);
            match scope {
                ControlScope::Companion => {
                    settings.companion_enabled = false;
                    config.companion.enabled = false;
                }
                ControlScope::Memory => settings.memory_enabled = false,
                ControlScope::Persona => settings.persona_enabled = false,
                ControlScope::Relationship => {
                    append_control_audit(
                        &store,
                        &request,
                        "rejected",
                        "relationship is a read-only derived view",
                    )?;
                    bail!("relationship controls are read-only");
                }
            }
            settings
                .save()
                .context("failed to persist control settings")?;
            append_control_audit(&store, &request, "completed", "persisted disabled state")?;
            print_control_snapshot(&control_snapshot(&config)?, Some(scope), json)?;
        }
        ControlCommand::Clear { scope, confirm } => {
            let scope = scope.into();
            let request = if confirm {
                ControlRequest::new(scope, ControlVerb::Clear).confirmed()
            } else {
                ControlRequest::new(scope, ControlVerb::Clear)
            };
            if !request.confirmation_satisfied() {
                append_control_audit(
                    &store,
                    &request,
                    "rejected",
                    "explicit confirmation missing",
                )?;
                bail!(
                    "{} clear requires --confirm; scope impact must be acknowledged",
                    scope.as_str()
                );
            }
            let detail = match scope {
                ControlScope::Companion => {
                    let cleared = store.clear_companion_history()?;
                    format!("cleared local companion history records={cleared}")
                }
                ControlScope::Memory => {
                    let summary = FilePersonaMemoryStore::for_workspace(&config.cwd)
                        .clear_workspace()
                        .context("failed to clear workspace memory")?;
                    format!(
                        "archived workspace memory active={} pending={} remaining_pending={}",
                        summary.archived_active_records,
                        summary.archived_pending_records,
                        summary.remaining_pending_records
                    )
                }
                ControlScope::Persona | ControlScope::Relationship => {
                    append_control_audit(
                        &store,
                        &request,
                        "rejected",
                        "scope is read-only and has no clear operation",
                    )?;
                    bail!("{} is read-only and cannot be cleared", scope.as_str());
                }
            };
            append_control_audit(&store, &request, "completed", &detail)?;
            if json {
                println!("{}", serde_json::json!({"scope": scope, "result": detail}));
            } else {
                println!("scope: {}", scope.as_str());
                println!("result: {detail}");
            }
        }
        ControlCommand::Audit => {
            let records = store.audit_records()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else if records.is_empty() {
                println!("control audit: empty");
            } else {
                for record in records {
                    println!(
                        "{} scope={} verb={} outcome={} source={} detail={}",
                        record.timestamp_millis,
                        record.scope.as_str(),
                        record.verb.as_str(),
                        record.outcome,
                        record.source,
                        record.detail
                    );
                }
            }
        }
    }
    Ok(())
}

fn append_control_audit(
    store: &FileControlStore,
    request: &ControlRequest,
    outcome: impl Into<String>,
    detail: impl Into<String>,
) -> Result<()> {
    store
        .append_audit(&ControlAuditRecord::new(
            request,
            outcome,
            detail,
            "yunxi-cli",
        ))
        .context("failed to append control audit")
}

fn print_control_snapshot(
    snapshot: &ControlSnapshot,
    scope: Option<ControlScope>,
    json: bool,
) -> Result<()> {
    if json {
        if let Some(scope) = scope {
            println!("{}", serde_json::to_string_pretty(&snapshot.scope(scope))?);
        } else {
            println!("{}", serde_json::to_string_pretty(snapshot)?);
        }
        return Ok(());
    }
    println!("companion_enabled: {}", snapshot.companion_enabled);
    println!("cloud_control_enabled: {}", snapshot.cloud_control_enabled);
    println!(
        "quiet_hours: {}",
        snapshot.quiet_hours.as_deref().unwrap_or("none")
    );
    for state in snapshot
        .scopes
        .iter()
        .filter(|state| scope.is_none_or(|scope| state.scope == scope))
    {
        println!("scope: {}", state.scope.as_str());
        if let Some(enabled) = state.enabled {
            println!("enabled: {enabled}");
        }
        println!("source: {}", state.source.as_str());
        println!("summary: {}", state.summary);
        if let Some(effect) = &state.clear_effect {
            println!("clear_effect: {effect}");
        }
    }
    if let Some(change) = &snapshot.recent_change {
        println!("recent_change: {change}");
    }
    Ok(())
}

async fn run_persona_command(
    command: PersonaCommand,
    config: &AgentConfig,
    json: bool,
) -> Result<()> {
    let mut settings = PersonaSettings::load();
    let store = FileControlStore::for_workspace(&config.cwd);
    match command {
        PersonaCommand::Status => {
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Show),
                "completed",
                "persona status",
            )?;
            print_persona_status(&settings, json)?;
        }
        PersonaCommand::Profile => {
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Show),
                "completed",
                "persona profile",
            )?;
            print_persona_profile(json)?;
        }
        PersonaCommand::Set { profile } => {
            if profile != "yunxi_companion_strong" {
                bail!("unknown persona profile: {profile}");
            }
            settings.active_profile = profile;
            settings.save().context("failed to save persona settings")?;
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Update),
                "completed",
                "active profile changed",
            )?;
            print_persona_status(&settings, json)?;
        }
        PersonaCommand::On => {
            settings.persona_enabled = true;
            settings.save().context("failed to save persona settings")?;
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Enable),
                "completed",
                "persona enabled",
            )?;
            print_persona_status(&settings, json)?;
        }
        PersonaCommand::Off => {
            settings.persona_enabled = false;
            settings.save().context("failed to save persona settings")?;
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Disable),
                "completed",
                "persona disabled",
            )?;
            print_persona_status(&settings, json)?;
        }
    }
    Ok(())
}

async fn run_memory_command(command: MemoryCommand, config: AgentConfig, json: bool) -> Result<()> {
    let store = FilePersonaMemoryStore::for_workspace(&config.cwd);
    let control_store = FileControlStore::for_workspace(&config.cwd);
    let mut settings = PersonaSettings::load();
    match command {
        MemoryCommand::Status => {
            let load = store.list(PersonaMemoryScope::All);
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Show),
                "completed",
                "memory status",
            )?;
            print_memory_status(&settings, &load.records, &load.warnings, json)?;
        }
        MemoryCommand::List { global, workspace } => {
            let load = store.list(memory_scope_from_flags(global, workspace));
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Show),
                "completed",
                format!("memory list records={}", load.records.len()),
            )?;
            print_memory_records(&load.records, &load.warnings, json)?;
        }
        MemoryCommand::Show { id } => {
            let load = store.show(&id);
            let record = load
                .records
                .first()
                .ok_or_else(|| anyhow::anyhow!("memory not found: {id}"))?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Show),
                "completed",
                format!("memory show id={id}"),
            )?;
            print_memory_record(record, &load.warnings, json)?;
        }
        MemoryCommand::Search {
            query,
            global,
            workspace,
        } => {
            let load = store.search(&query.join(" "), memory_scope_from_flags(global, workspace));
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Show),
                "completed",
                format!("memory search records={}", load.records.len()),
            )?;
            print_memory_records(&load.records, &load.warnings, json)?;
        }
        MemoryCommand::Pending => {
            let load = store.pending_records();
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Show),
                "completed",
                format!("memory pending records={}", load.records.len()),
            )?;
            print_memory_records(&load.records, &load.warnings, json)?;
        }
        MemoryCommand::Approve { id } => {
            let record = store
                .update_status(&id, MemoryStatus::Active)
                .context("failed to approve memory")?
                .ok_or_else(|| anyhow::anyhow!("memory not found: {id}"))?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Update),
                "completed",
                format!("approved memory id={id}"),
            )?;
            print_memory_record(&record, &[], json)?;
        }
        MemoryCommand::Reject { id } => {
            let record = store
                .update_status(&id, MemoryStatus::Rejected)
                .context("failed to reject memory")?
                .ok_or_else(|| anyhow::anyhow!("memory not found: {id}"))?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Update),
                "completed",
                format!("rejected memory id={id}"),
            )?;
            print_memory_record(&record, &[], json)?;
        }
        MemoryCommand::Delete { id } => {
            let record = store
                .update_status(&id, MemoryStatus::Archived)
                .context("failed to archive memory")?
                .ok_or_else(|| anyhow::anyhow!("memory not found: {id}"))?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Update),
                "completed",
                format!("archived memory id={id}"),
            )?;
            print_memory_record(&record, &[], json)?;
        }
        MemoryCommand::Clear { workspace, confirm } => {
            let request = if workspace && confirm {
                ControlRequest::new(ControlScope::Memory, ControlVerb::Clear).confirmed()
            } else {
                ControlRequest::new(ControlScope::Memory, ControlVerb::Clear)
            };
            if !workspace || !confirm {
                append_control_audit(
                    &control_store,
                    &request,
                    "rejected",
                    "memory clear requires workspace scope and confirmation",
                )?;
                bail!("memory clear requires --workspace --confirm");
            }
            let summary = store
                .clear_workspace()
                .context("failed to clear workspace memory")?;
            append_control_audit(
                &control_store,
                &request,
                "completed",
                format!(
                    "archived active={} pending={} remaining_pending={}",
                    summary.archived_active_records,
                    summary.archived_pending_records,
                    summary.remaining_pending_records
                ),
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "scope": "workspace",
                        "archived_active_records": summary.archived_active_records,
                        "archived_pending_records": summary.archived_pending_records,
                        "remaining_pending_records": summary.remaining_pending_records,
                    }))?
                );
            } else {
                println!(
                    "workspace memory archived_active_records: {}",
                    summary.archived_active_records
                );
                println!(
                    "workspace memory archived_pending_records: {}",
                    summary.archived_pending_records
                );
                println!(
                    "workspace memory remaining_pending_records: {}",
                    summary.remaining_pending_records
                );
            }
        }
        MemoryCommand::On => {
            let first_enable_notice_shown = !settings.memory_enabled;
            settings.memory_enabled = true;
            settings.save().context("failed to save memory settings")?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Enable),
                "completed",
                "memory enabled",
            )?;
            let load = store.list(PersonaMemoryScope::All);
            print_memory_on_status(
                &settings,
                &load.records,
                &load.warnings,
                &store,
                first_enable_notice_shown,
                json,
            )?;
        }
        MemoryCommand::Off => {
            settings.memory_enabled = false;
            settings.save().context("failed to save memory settings")?;
            append_control_audit(
                &control_store,
                &ControlRequest::new(ControlScope::Memory, ControlVerb::Disable),
                "completed",
                "memory disabled",
            )?;
            let load = store.list(PersonaMemoryScope::All);
            print_memory_status(&settings, &load.records, &load.warnings, json)?;
        }
    }
    Ok(())
}

fn print_persona_status(settings: &PersonaSettings, json: bool) -> Result<()> {
    let profile = yunxi_companion_strong();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "persona_enabled": settings.persona_enabled,
                "memory_enabled": settings.memory_enabled,
                "companion_enabled": settings.companion_enabled,
                "cloud_control_enabled": settings.cloud_control_enabled,
                "active_profile": settings.active_profile,
                "profile": {
                    "id": profile.id,
                    "display_name": profile.display_name,
                    "version": profile.version,
                }
            }))?
        );
    } else {
        println!("persona_enabled: {}", settings.persona_enabled);
        println!("memory_enabled: {}", settings.memory_enabled);
        println!("companion_enabled: {}", settings.companion_enabled);
        println!("cloud_control_enabled: {}", settings.cloud_control_enabled);
        println!("active_profile: {}", settings.active_profile);
        println!("display_name: {}", profile.display_name);
        println!("profile_version: {}", profile.version);
    }
    Ok(())
}

fn print_persona_profile(json: bool) -> Result<()> {
    let profile = yunxi_companion_strong();
    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        println!("id: {}", profile.id);
        println!("display_name: {}", profile.display_name);
        println!("version: {}", profile.version);
        println!("identity: {}", profile.layers.identity);
        println!("voice: {}", profile.layers.voice);
        println!("companion_style: {}", profile.layers.companion_style);
        println!("work_style: {}", profile.layers.work_style);
        println!("boundaries: {}", profile.layers.boundaries);
        for constraint in profile.constraints {
            println!("constraint.{}: {}", constraint.id, constraint.content);
        }
    }
    Ok(())
}

fn print_memory_status(
    settings: &PersonaSettings,
    records: &[MemoryRecord],
    warnings: &[String],
    json: bool,
) -> Result<()> {
    let active = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .count();
    let pending = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Pending)
        .count();
    let rejected = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Rejected)
        .count();
    let archived = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Archived)
        .count();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "memory_enabled": settings.memory_enabled,
                "persona_enabled": settings.persona_enabled,
                "active_profile": settings.active_profile,
                "counts": {
                    "total": records.len(),
                    "active": active,
                    "pending": pending,
                    "rejected": rejected,
                    "archived": archived,
                },
                "warnings": warnings,
            }))?
        );
    } else {
        println!("memory_enabled: {}", settings.memory_enabled);
        println!("records: {}", records.len());
        println!("active: {active}");
        println!("pending: {pending}");
        println!("rejected: {rejected}");
        println!("archived: {archived}");
        for warning in warnings {
            println!("[memory-warning] {warning}");
        }
    }
    Ok(())
}

fn print_memory_on_status(
    settings: &PersonaSettings,
    records: &[MemoryRecord],
    warnings: &[String],
    store: &FilePersonaMemoryStore,
    first_enable_notice_shown: bool,
    json: bool,
) -> Result<()> {
    if json {
        let active = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Active)
            .count();
        let pending = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Pending)
            .count();
        let rejected = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Rejected)
            .count();
        let archived = records
            .iter()
            .filter(|record| record.status == MemoryStatus::Archived)
            .count();
        let roots = store.storage_roots();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "memory_enabled": settings.memory_enabled,
                "persona_enabled": settings.persona_enabled,
                "active_profile": settings.active_profile,
                "first_enable_notice_shown": first_enable_notice_shown,
                "storage_roots": {
                    "global": roots.global_root,
                    "workspace": roots.workspace_root,
                },
                "pending_policy_summary": {
                    "auto_saved": "low-risk preferences, corrections, and project context",
                    "pending": "personal facts, relationship notes, emotional state, goals, events, medium sensitivity content",
                    "discarded": "secret-like content such as API keys, bearer tokens, passwords, and sk-* markers",
                },
                "provider_extraction": {
                    "default_mode": "auto",
                    "auto_mode_note": "when a live provider and model are configured, YunXi may run one additional structured extraction call after the main response; failures fall back to local rules",
                    "override_flag": "--memory-extraction <auto|rule-only|provider>",
                },
                "disable_command": "yunxi memory off",
                "pending_command": "yunxi memory pending",
                "counts": {
                    "total": records.len(),
                    "active": active,
                    "pending": pending,
                    "rejected": rejected,
                    "archived": archived,
                },
                "warnings": warnings,
            }))?
        );
    } else {
        if first_enable_notice_shown {
            let roots = store.storage_roots();
            println!("YunXi memory is now enabled.");
            println!("storage.global: {}", roots.global_root.display());
            println!("storage.workspace: {}", roots.workspace_root.display());
            println!("low-risk preferences and project context may be auto-saved.");
            println!("personal or sensitive candidates require `yunxi memory pending` approval.");
            println!("secret-like content is discarded instead of stored.");
            println!(
                "auto extraction may run one extra structured provider call when live provider and model are configured; use `--memory-extraction rule-only` to force local rules."
            );
            println!(
                "disable with `yunxi memory off`; archive workspace memory with `yunxi memory clear --workspace --confirm`."
            );
        }
        print_memory_status(settings, records, warnings, false)?;
    }
    Ok(())
}

fn print_memory_records(records: &[MemoryRecord], warnings: &[String], json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "records": records,
                "warnings": warnings,
            }))?
        );
    } else {
        println!("records: {}", records.len());
        for record in records {
            println!(
                "{}\t{}\t{}\t{}\trev={}\tmerged={}\t{}\t{}",
                record.id,
                memory_status_label(record.status),
                memory_kind_label(record.kind),
                memory_sensitivity_label(record.sensitivity),
                record.revision,
                record.merged_count,
                record.scope.label(),
                preview(&record.content)
            );
        }
        for warning in warnings {
            println!("[memory-warning] {warning}");
        }
    }
    Ok(())
}

fn print_memory_record(record: &MemoryRecord, warnings: &[String], json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "record": record,
                "warnings": warnings,
            }))?
        );
    } else {
        println!("id: {}", record.id);
        println!("schema_version: {}", record.schema_version);
        println!("scope: {}", record.scope.label());
        println!("kind: {}", memory_kind_label(record.kind));
        println!(
            "sensitivity: {}",
            memory_sensitivity_label(record.sensitivity)
        );
        println!("status: {}", memory_status_label(record.status));
        println!("confidence: {}", record.confidence);
        println!("importance: {}", record.importance);
        if let Some(source_session_id) = &record.source_session_id {
            println!("source_session_id: {source_session_id}");
        }
        println!("created_at_millis: {}", record.created_at_millis);
        println!("updated_at_millis: {}", record.updated_at_millis);
        println!("dedup_key: {}", record.dedup_key);
        println!("revision: {}", record.revision);
        println!("merged_count: {}", record.merged_count);
        println!("content: {}", record.content);
        for warning in warnings {
            println!("[memory-warning] {warning}");
        }
    }
    Ok(())
}

fn memory_scope_from_flags(global: bool, workspace: bool) -> PersonaMemoryScope {
    if global {
        PersonaMemoryScope::Global
    } else if workspace {
        PersonaMemoryScope::Workspace
    } else {
        PersonaMemoryScope::All
    }
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
                let summaries = session_summaries(&sessions);
                println!("{}", serde_json::to_string_pretty(&summaries)?);
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

fn session_summaries(sessions: &[SessionRecord]) -> Vec<SessionSummary> {
    sessions
        .iter()
        .map(|session| {
            let child_count = sessions
                .iter()
                .filter(|candidate| candidate.parent_id.as_ref() == Some(&session.id))
                .count();
            session.summary_with_child_count(child_count)
        })
        .collect()
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

fn memory_kind_label(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Preference => "preference",
        MemoryKind::PersonalFact => "personal_fact",
        MemoryKind::RelationshipNote => "relationship_note",
        MemoryKind::EmotionalState => "emotional_state",
        MemoryKind::Goal => "goal",
        MemoryKind::ProjectContext => "project_context",
        MemoryKind::Correction => "correction",
        MemoryKind::Event => "event",
        MemoryKind::ToolTraceSummary => "tool_trace_summary",
    }
}

fn memory_sensitivity_label(sensitivity: MemorySensitivity) -> &'static str {
    match sensitivity {
        MemorySensitivity::Low => "low",
        MemorySensitivity::Medium => "medium",
        MemorySensitivity::High => "high",
    }
}

fn memory_status_label(status: MemoryStatus) -> &'static str {
    match status {
        MemoryStatus::Active => "active",
        MemoryStatus::Pending => "pending",
        MemoryStatus::Rejected => "rejected",
        MemoryStatus::Archived => "archived",
    }
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
