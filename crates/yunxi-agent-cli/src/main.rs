use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs::{self, OpenOptions};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunControl, AgentRunResult,
    ApprovalMode, BackendKind, CommandStatus, ControlAuditRecord, ControlRequest, ControlScope,
    ControlSnapshot, ControlVerb, MemoryExtractionMode, SandboxMode,
};
use yunxi_agent_persona::{
    MemoryKind, MemoryRecord, MemorySensitivity, MemoryStatus, PersonaProfileStore,
    PersonaSettings, now_millis as memory_now_millis,
};
use yunxi_agent_protocol::{
    FunctionCallOutput, ProtocolRole, ResponseItem, ResponseItemDelta, RuntimeEvent, ThreadId,
    ThreadState, ToolCall, ToolCallStatus, TurnId, TurnMetadata, TurnState, to_jsonl_line,
};
use yunxi_agent_runtime::control_snapshot;
use yunxi_agent_storage::{
    FileControlStore, FilePersonaMemoryStore, FileSessionStore, HistoryLoadOptions,
    PersonaMemoryScope, RolloutRecord, SessionGraphView, SessionHistory, SessionId, SessionRecord,
    SessionStore, SessionSummary, WeixinAccountLockState,
};
use yunxi_agent_weixin::{WeixinAccountId, WeixinAccountStore};

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
mod web;
mod weixin;
mod workspace;
mod tui {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tui/mod.rs"));
}

const CODEX_CORE_PARITY_MAP: &str =
    include_str!("../../../docs/extraction-index/codex-core-agent-parity-map.md");
const DEFAULT_WEIXIN_AUTOSTART_ACCOUNT: &str = "default";
const WEIXIN_AUTOSTART_ENV: &str = "YUNXI_WEIXIN_AUTOSTART";
const WEIXIN_AUTOSTART_ACCOUNT_ENV: &str = "YUNXI_WEIXIN_ACCOUNT";
const WEIXIN_AUTOSTART_WORKSPACE_ENV: &str = "YUNXI_WEIXIN_WORKSPACE";
const WEIXIN_AUTOSTART_READY_TIMEOUT_MS_ENV: &str = "YUNXI_WEIXIN_AUTOSTART_READY_TIMEOUT_MS";
const DEFAULT_WEIXIN_AUTOSTART_READY_TIMEOUT_MS: u64 = 4_000;
const MAX_WEIXIN_AUTOSTART_READY_TIMEOUT_MS: u64 = 30_000;
const WEIXIN_AUTOSTART_READY_POLL_MS: u64 = 100;
const WEIXIN_AUTOSTART_READY_SETTLE_MS: u64 = 150;

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
#[command(about = "YunXi Agent interactive terminal CLI")]
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

    #[arg(
        long = "no-weixin-autostart",
        global = true,
        help = "Do not automatically start the local Weixin gateway when entering interactive TUI or plain CLI"
    )]
    no_weixin_autostart: bool,

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
    Weixin {
        #[command(subcommand)]
        command: weixin::WeixinCommand,
    },
    Web {
        #[arg(
            long,
            default_value = "127.0.0.1",
            help = "Host/IP address for the local YunXi Web console"
        )]
        bind: std::net::IpAddr,
        #[arg(
            long,
            default_value_t = web::DEFAULT_WEB_PORT,
            help = "Port for the local YunXi Web console"
        )]
        port: u16,
    },
    Bot {
        #[command(subcommand)]
        command: BotCommand,
    },
}

#[derive(Debug, Subcommand)]
enum BotCommand {
    #[command(about = "Start the local companion gateway")]
    Start {
        #[arg(
            long,
            default_value = "weixin",
            help = "Comma-separated channels to enable; currently only weixin is supported"
        )]
        channels: String,
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
        #[arg(long, default_value = "default")]
        account: String,
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
    Weixin,
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
    List,
    Import {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
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
    let stdin_is_terminal = std::io::stdin().is_terminal();
    let stdout_is_terminal = std::io::stdout().is_terminal();
    let invocation = if cli.jsonl {
        terminal_mode::InvocationKind::Jsonl
    } else if cli.json {
        terminal_mode::InvocationKind::Json
    } else if cli.command.is_some() {
        terminal_mode::InvocationKind::Command
    } else if cli.prompt.iter().all(|part| part.trim().is_empty()) {
        terminal_mode::InvocationKind::Interactive
    } else {
        terminal_mode::InvocationKind::OneShot
    };
    let terminal_resolution = terminal_mode::TerminalModeRequest {
        tui: cli.tui,
        no_tui: cli.no_tui,
        stdin_is_terminal,
        stdout_is_terminal,
        ci: continuous_integration_environment(),
        invocation,
    }
    .resolve();
    let provider_mode =
        provider_mode::ProviderMode::from_flags(cli.provider_live || cli.live, cli.offline);

    let backend = cli.backend.into();

    let config = build_agent_config(&cli)?;

    reject_detached_codex_backend(backend)?;

    if let Some(command) = cli.command {
        return run_command(
            command,
            config,
            backend,
            provider_mode,
            cli.json,
            cli.jsonl,
            cli.no_weixin_autostart,
            cli.companion,
        )
        .await;
    }

    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        if !cli.json && !cli.jsonl {
            if let Some(notice) = terminal_resolution.fallback_notice {
                eprintln!("{notice}");
            }
            if should_attempt_interactive_weixin_autostart(&cli, terminal_resolution) {
                report_weixin_autostart_result(maybe_autostart_weixin_gateway(
                    &config,
                    backend,
                    provider_mode,
                    cli.companion,
                ));
            }
            return interactive::run_interactive(interactive::InteractiveOptions {
                config,
                backend,
                provider_mode,
                terminal_mode: terminal_resolution.mode,
            })
            .await;
        }
        bail!("a prompt is required");
    }

    run_prompt(prompt, config, backend, provider_mode, cli.json, cli.jsonl).await
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WeixinAutostartReport {
    pid: u32,
    stdout_log: PathBuf,
    stderr_log: PathBuf,
    readiness: WeixinAutostartReadiness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum WeixinAutostartReadiness {
    Ready,
    TimedOut { timeout_ms: u64 },
    Exited { status: ExitStatus },
}

fn report_weixin_autostart_result(result: Result<Option<WeixinAutostartReport>>) {
    match result {
        Ok(Some(report)) => match report.readiness {
            WeixinAutostartReadiness::Ready => eprintln!(
                "[weixin] gateway ready: pid={}, stdout={}, stderr={}",
                report.pid,
                report.stdout_log.display(),
                report.stderr_log.display()
            ),
            WeixinAutostartReadiness::TimedOut { timeout_ms } => eprintln!(
                "[weixin] gateway startup timed out after {timeout_ms}ms: pid={}, stdout={}, stderr={}",
                report.pid,
                report.stdout_log.display(),
                report.stderr_log.display()
            ),
            WeixinAutostartReadiness::Exited { status } => eprintln!(
                "[weixin] gateway exited before ready (status={status}): stdout={}, stderr={}",
                report.stdout_log.display(),
                report.stderr_log.display()
            ),
        },
        Ok(None) => {}
        Err(error) => {
            eprintln!(
                "[weixin] gateway autostart skipped: {}",
                redact_secret_fragments(&format!("{error:#}"))
            );
        }
    }
}

fn should_attempt_interactive_weixin_autostart(
    cli: &Cli,
    terminal_resolution: terminal_mode::TerminalModeResolution,
) -> bool {
    !cli.no_weixin_autostart
        && weixin_autostart_enabled()
        && cli.command.is_none()
        && cli.prompt.iter().all(|part| part.trim().is_empty())
        && !cli.json
        && !cli.jsonl
        && terminal_mode_allows_weixin_autostart(terminal_resolution)
}

fn terminal_mode_allows_weixin_autostart(
    terminal_resolution: terminal_mode::TerminalModeResolution,
) -> bool {
    match terminal_resolution.mode {
        terminal_mode::ResolvedTerminalMode::Tui => true,
        terminal_mode::ResolvedTerminalMode::Plain => {
            terminal_resolution.reason == terminal_mode::TerminalModeReason::NoTuiRequested
        }
    }
}

fn should_attempt_command_weixin_autostart(no_weixin_autostart: bool) -> bool {
    !no_weixin_autostart && weixin_autostart_enabled()
}

fn weixin_autostart_enabled() -> bool {
    weixin_autostart_env_value_enabled(std::env::var(WEIXIN_AUTOSTART_ENV).ok().as_deref())
}

fn weixin_autostart_env_value_enabled(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let value = value.trim();
    if value.is_empty() {
        return true;
    }
    !matches!(
        value.to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no"
    )
}

fn resolve_weixin_autostart_workspace(
    config_cwd: &Path,
    account_id: &WeixinAccountId,
) -> Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    push_weixin_workspace_candidate(&mut candidates, config_cwd.to_path_buf());
    if let Ok(value) = std::env::var(WEIXIN_AUTOSTART_WORKSPACE_ENV) {
        let value = value.trim();
        if !value.is_empty() {
            push_weixin_workspace_candidate(&mut candidates, PathBuf::from(value));
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        push_weixin_workspace_candidate(&mut candidates, current_dir);
    }
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(install_root) = current_exe.parent().and_then(Path::parent)
    {
        push_weixin_workspace_candidate(&mut candidates, install_root.to_path_buf());
    }
    if let Some(compiled_root) = compiled_workspace_root() {
        push_weixin_workspace_candidate(&mut candidates, compiled_root);
    }

    for workspace in candidates {
        if !workspace.is_dir() {
            continue;
        }
        let account_store = WeixinAccountStore::new(&workspace);
        let record = account_store.load(account_id).with_context(|| {
            format!(
                "weixin autostart metadata load failed in workspace {}",
                workspace.display()
            )
        })?;
        if record.is_some() {
            return Ok(Some(workspace));
        }
    }
    Ok(None)
}

fn push_weixin_workspace_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }
    let path = fs::canonicalize(&path).unwrap_or(path);
    if candidates
        .iter()
        .any(|candidate| same_path(candidate, &path))
    {
        return;
    }
    candidates.push(path);
}

fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn compiled_workspace_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn maybe_autostart_weixin_gateway(
    config: &AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    explicit_companion: bool,
) -> Result<Option<WeixinAutostartReport>> {
    if backend == BackendKind::Codex {
        return Ok(None);
    }
    let account = std::env::var(WEIXIN_AUTOSTART_ACCOUNT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WEIXIN_AUTOSTART_ACCOUNT.to_string());
    let account_id = WeixinAccountId::new(&account);
    let Some(workspace) = resolve_weixin_autostart_workspace(&config.cwd, &account_id)? else {
        return Ok(None);
    };
    let mut runtime_config = config.clone();
    runtime_config.cwd = workspace.clone();
    let account_store = WeixinAccountStore::new(&workspace);
    let Some(record) = account_store
        .load(&account_id)
        .context("weixin autostart metadata load failed")?
    else {
        return Ok(None);
    };
    let state_store = yunxi_agent_storage::FileWeixinStateStore::for_workspace(&workspace);
    let lock_state = state_store
        .lock_state(&record.account_id)
        .context("weixin autostart account lock check failed")?;
    if lock_state.state == WeixinAccountLockState::Active {
        return Ok(None);
    }

    let log_dir = workspace.join(".yunxi").join("weixin").join("logs");
    fs::create_dir_all(&log_dir).context("weixin autostart log directory creation failed")?;
    let stamp = now_millis_u64();
    let stdout_log = log_dir.join(format!("autostart-{stamp}.stdout.log"));
    let stderr_log = log_dir.join(format!("autostart-{stamp}.stderr.log"));
    let stdout_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stdout_log)
        .context("weixin autostart stdout log open failed")?;
    let stderr_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stderr_log)
        .context("weixin autostart stderr log open failed")?;

    let exe = std::env::current_exe().context("weixin autostart current executable failed")?;
    let mut command = Command::new(exe);
    append_runtime_cli_args(
        &mut command,
        &runtime_config,
        backend,
        provider_mode,
        explicit_companion,
    );
    command
        .args([
            "bot",
            "start",
            "--channels",
            "weixin",
            "--account",
            account.as_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    configure_background_process(&mut command);
    let mut child = command
        .spawn()
        .context("weixin autostart process spawn failed")?;
    let pid = child.id();
    let readiness =
        wait_for_weixin_autostart_readiness(&mut child, &state_store, &record.account_id, pid)?;
    Ok(Some(WeixinAutostartReport {
        pid,
        stdout_log,
        stderr_log,
        readiness,
    }))
}

fn weixin_autostart_ready_timeout_ms() -> u64 {
    parse_weixin_autostart_ready_timeout(
        std::env::var(WEIXIN_AUTOSTART_READY_TIMEOUT_MS_ENV)
            .ok()
            .as_deref(),
    )
}

fn parse_weixin_autostart_ready_timeout(value: Option<&str>) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_WEIXIN_AUTOSTART_READY_TIMEOUT_MS)
        .clamp(100, MAX_WEIXIN_AUTOSTART_READY_TIMEOUT_MS)
}

fn wait_for_weixin_autostart_readiness(
    child: &mut Child,
    state_store: &yunxi_agent_storage::FileWeixinStateStore,
    account_id: &str,
    pid: u32,
) -> Result<WeixinAutostartReadiness> {
    let timeout_ms = weixin_autostart_ready_timeout_ms();
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Some(status) = child
            .try_wait()
            .context("weixin autostart readiness process probe failed")?
        {
            return Ok(WeixinAutostartReadiness::Exited { status });
        }
        let lock_state = state_store
            .lock_state(account_id)
            .context("weixin autostart readiness lock check failed")?;
        if lock_state.state == WeixinAccountLockState::Active && lock_state.pid == Some(pid) {
            std::thread::sleep(Duration::from_millis(WEIXIN_AUTOSTART_READY_SETTLE_MS));
            if child
                .try_wait()
                .context("weixin autostart readiness settle probe failed")?
                .is_none()
            {
                return Ok(WeixinAutostartReadiness::Ready);
            }
        }
        if Instant::now() >= deadline {
            return Ok(WeixinAutostartReadiness::TimedOut { timeout_ms });
        }
        std::thread::sleep(Duration::from_millis(WEIXIN_AUTOSTART_READY_POLL_MS));
    }
}

fn append_runtime_cli_args(
    command: &mut Command,
    config: &AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    explicit_companion: bool,
) {
    command.arg("--cwd").arg(&config.cwd);
    match backend {
        BackendKind::Yunxi => {}
        BackendKind::DryRun => {
            command.args(["--backend", "dry-run"]);
        }
        BackendKind::Codex => {
            command.args(["--backend", "codex"]);
        }
    }
    match provider_mode {
        provider_mode::ProviderMode::Auto => {}
        provider_mode::ProviderMode::ForcedLive => {
            command.arg("--provider-live");
        }
        provider_mode::ProviderMode::ForcedOffline => {
            command.arg("--offline");
        }
    }
    if let Some(model) = &config.model {
        command.args(["--model", model]);
    }
    if let Some(provider) = &config.provider {
        command.args(["--provider", provider]);
    }
    if let Some(codex_home) = &config.codex_home {
        command.arg("--codex-home").arg(codex_home);
    }
    if let Some(context_window_tokens) = config.context_window_tokens {
        command.args([
            "--context-window-tokens",
            &context_window_tokens.to_string(),
        ]);
    }
    if let Some(auto_compact_threshold_tokens) = config.auto_compact_threshold_tokens {
        command.args([
            "--auto-compact-threshold-tokens",
            &auto_compact_threshold_tokens.to_string(),
        ]);
    }
    if explicit_companion {
        command.arg("--companion");
    }
    command.args(["--approval", approval_mode_arg(config.approval_mode)]);
    command.args(["--sandbox", sandbox_mode_arg(config.sandbox_mode)]);
    command.args([
        "--memory-extraction",
        memory_extraction_mode_arg(config.memory_extraction_mode),
    ]);
}

fn approval_mode_arg(mode: ApprovalMode) -> &'static str {
    match mode {
        ApprovalMode::Never => "never",
        ApprovalMode::OnRequest => "on-request",
        ApprovalMode::OnFailure => "on-failure",
        ApprovalMode::Untrusted => "untrusted",
    }
}

fn sandbox_mode_arg(mode: SandboxMode) -> &'static str {
    match mode {
        SandboxMode::ReadOnly => "read-only",
        SandboxMode::WorkspaceWrite => "workspace-write",
        SandboxMode::DangerFullAccess => "danger-full-access",
    }
}

fn memory_extraction_mode_arg(mode: MemoryExtractionMode) -> &'static str {
    match mode {
        MemoryExtractionMode::Auto => "auto",
        MemoryExtractionMode::RuleOnly => "rule-only",
        MemoryExtractionMode::Provider => "provider",
    }
}

fn now_millis_u64() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn configure_background_process(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cli() -> Cli {
        Cli {
            backend: CliBackend::Yunxi,
            live: false,
            cwd: None,
            model: None,
            provider: None,
            provider_live: false,
            offline: false,
            codex_home: None,
            context_window_tokens: None,
            auto_compact_threshold_tokens: None,
            memory_extraction: CliMemoryExtractionMode::Auto,
            approval: CliApprovalMode::OnRequest,
            sandbox: CliSandboxMode::WorkspaceWrite,
            json: false,
            jsonl: false,
            tui: false,
            no_tui: false,
            companion: false,
            no_weixin_autostart: false,
            command: None,
            prompt: Vec::new(),
        }
    }

    fn terminal_resolution(
        mode: terminal_mode::ResolvedTerminalMode,
        reason: terminal_mode::TerminalModeReason,
    ) -> terminal_mode::TerminalModeResolution {
        terminal_mode::TerminalModeResolution {
            mode,
            reason,
            fallback_notice: None,
        }
    }

    fn tui_terminal_resolution() -> terminal_mode::TerminalModeResolution {
        terminal_resolution(
            terminal_mode::ResolvedTerminalMode::Tui,
            terminal_mode::TerminalModeReason::InteractiveTerminal,
        )
    }

    fn no_tui_plain_terminal_resolution() -> terminal_mode::TerminalModeResolution {
        terminal_resolution(
            terminal_mode::ResolvedTerminalMode::Plain,
            terminal_mode::TerminalModeReason::NoTuiRequested,
        )
    }

    #[test]
    fn autostart_env_parser_handles_common_truthy_and_falsey_values() {
        assert!(weixin_autostart_env_value_enabled(None));
        assert!(weixin_autostart_env_value_enabled(Some("1")));
        assert!(weixin_autostart_env_value_enabled(Some(" yes ")));
        assert!(!weixin_autostart_env_value_enabled(Some("0")));
        assert!(!weixin_autostart_env_value_enabled(Some("false")));
        assert!(!weixin_autostart_env_value_enabled(Some("off")));
        assert!(!weixin_autostart_env_value_enabled(Some("no")));
    }

    #[test]
    fn autostart_decision_allows_tui_and_explicit_plain_interactive_cli() {
        let cli = base_cli();
        assert!(should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            terminal_resolution(
                terminal_mode::ResolvedTerminalMode::Plain,
                terminal_mode::TerminalModeReason::StdinNotTerminal,
            )
        ));
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            terminal_resolution(
                terminal_mode::ResolvedTerminalMode::Plain,
                terminal_mode::TerminalModeReason::StdoutNotTerminal,
            )
        ));
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            terminal_resolution(
                terminal_mode::ResolvedTerminalMode::Plain,
                terminal_mode::TerminalModeReason::ContinuousIntegration,
            )
        ));

        let mut cli = base_cli();
        cli.no_tui = true;
        assert!(should_attempt_interactive_weixin_autostart(
            &cli,
            no_tui_plain_terminal_resolution()
        ));

        let mut cli = base_cli();
        cli.no_weixin_autostart = true;
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));

        let mut cli = base_cli();
        cli.command = Some(CliCommand::Eval {
            command: EvalCommand::Weixin,
        });
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));

        let mut cli = base_cli();
        cli.prompt = vec![String::from("hello")];
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));

        let mut cli = base_cli();
        cli.json = true;
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));

        let mut cli = base_cli();
        cli.jsonl = true;
        assert!(!should_attempt_interactive_weixin_autostart(
            &cli,
            tui_terminal_resolution()
        ));
    }

    #[test]
    fn autostart_runtime_args_preserve_explicit_companion() {
        let config = AgentConfig::new("D:\\workspace");
        let mut command = Command::new("yunxi");
        append_runtime_cli_args(
            &mut command,
            &config,
            BackendKind::Yunxi,
            provider_mode::ProviderMode::Auto,
            true,
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.windows(1).any(|part| part == ["--companion"]));
        assert!(
            args.windows(2)
                .any(|part| part == ["--approval", "on-request"])
        );
        assert!(
            args.windows(2)
                .any(|part| part == ["--sandbox", "workspace-write"])
        );
    }

    #[test]
    fn autostart_ready_timeout_is_bounded() {
        assert_eq!(
            parse_weixin_autostart_ready_timeout(None),
            DEFAULT_WEIXIN_AUTOSTART_READY_TIMEOUT_MS
        );
        assert_eq!(parse_weixin_autostart_ready_timeout(Some("0")), 100);
        assert_eq!(
            parse_weixin_autostart_ready_timeout(Some("999999")),
            MAX_WEIXIN_AUTOSTART_READY_TIMEOUT_MS
        );
        assert_eq!(
            parse_weixin_autostart_ready_timeout(Some("not-a-number")),
            DEFAULT_WEIXIN_AUTOSTART_READY_TIMEOUT_MS
        );
    }
}

fn build_agent_config(cli: &Cli) -> Result<AgentConfig> {
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
    config.companion.love_letters.enabled = persisted_settings.love_letters_enabled;
    config.companion.cloud_control_enabled = persisted_settings.cloud_control_enabled;
    if cli.companion
        || matches!(
            cli.command.as_ref(),
            Some(CliCommand::Companion {
                command: CompanionCommand::Check { .. }
            })
        )
    {
        config.companion.enabled = true;
        config.companion.allow_tool_requests = true;
    }
    Ok(config)
}

fn continuous_integration_environment() -> bool {
    std::env::var("CI")
        .map(|value| {
            let value = value.trim();
            !value.is_empty()
                && !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "off")
        })
        .unwrap_or(false)
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
    let invocation = prepare_runtime_invocation(config, backend, provider_mode)?;
    let offline_label = invocation.selection.is_offline_runtime();
    print_provider_selection_warning(&invocation.selection, json, jsonl)?;
    let result = run_agent_backend(
        backend,
        invocation.config,
        prompt,
        invocation.selection.live,
    )
    .await?;
    print_run_result(result, json, jsonl, offline_label)?;
    Ok(())
}

pub(crate) struct RuntimeInvocation {
    pub config: AgentConfig,
    pub selection: provider_mode::ProviderSelection,
}

pub(crate) fn prepare_runtime_invocation(
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
) -> Result<RuntimeInvocation> {
    let selection = provider_mode.resolve(backend, &config)?;
    let config = selection.apply_to_config(config);
    Ok(RuntimeInvocation { config, selection })
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
            let agent = Agent::new(config);
            let backend = build_yunxi_runtime_backend(agent.config(), provider_live);
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
            let agent = Agent::new(config);
            let backend = build_yunxi_runtime_backend(agent.config(), provider_live);
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

pub(crate) fn build_yunxi_runtime_backend(
    config: &AgentConfig,
    provider_live: bool,
) -> yunxi_agent_runtime::YunXiRuntimeBackend {
    let cwd = config.cwd.clone();
    if provider_live {
        yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace_with_live_provider(cwd, config)
    } else {
        build_offline_yunxi_runtime(cwd)
    }
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
    no_weixin_autostart: bool,
    explicit_companion: bool,
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
        CliCommand::Weixin { command } => {
            weixin::run(command, config, backend, provider_mode, json).await
        }
        CliCommand::Web { bind, port } => {
            if should_attempt_command_weixin_autostart(no_weixin_autostart) {
                report_weixin_autostart_result(maybe_autostart_weixin_gateway(
                    &config,
                    backend,
                    provider_mode,
                    explicit_companion,
                ));
            }
            web::run(web::WebOptions {
                bind,
                port,
                config,
                backend,
                provider_mode,
            })
            .await
        }
        CliCommand::Bot { command } => {
            run_bot_command(command, config, backend, provider_mode, json).await
        }
    }
}

async fn run_bot_command(
    command: BotCommand,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
    json: bool,
) -> Result<()> {
    match command {
        BotCommand::Start {
            channels,
            dir,
            account,
        } => {
            let channels = channels
                .split(',')
                .map(str::trim)
                .filter(|channel| !channel.is_empty())
                .collect::<Vec<_>>();
            if channels.is_empty() {
                bail!("bot start requires at least one channel");
            }
            let unsupported = channels
                .iter()
                .copied()
                .filter(|channel| !channel.eq_ignore_ascii_case("weixin"))
                .collect::<Vec<_>>();
            if !unsupported.is_empty() {
                bail!(
                    "bot start currently supports only the weixin channel; unsupported channels: {}",
                    unsupported.join(", ")
                );
            }
            weixin::run(
                weixin::WeixinCommand::Serve {
                    account,
                    workspace: dir,
                },
                config,
                backend,
                provider_mode,
                json,
            )
            .await
        }
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
            "--jsonl is only supported for agent execution commands in v2.1.5; use --json for sessions metadata commands"
        ),
        CliCommand::Parity { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.1.5; use --json for parity commands"
        ),
        CliCommand::Persona { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.1.5; use --json for persona management commands"
        ),
        CliCommand::Memory { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.1.5; use --json for memory management commands"
        ),
        CliCommand::Companion { .. } => bail!(
            "--jsonl is only supported for agent execution commands in v2.1.5; use --json for companion management commands"
        ),
        CliCommand::Controls { .. } => bail!(
            "--jsonl is only supported for agent execution commands; use --json for control commands"
        ),
        CliCommand::Eval { .. } => Ok(()),
        CliCommand::Weixin { .. } => bail!(
            "--jsonl is only supported for agent execution commands; use --json for weixin metadata commands"
        ),
        CliCommand::Web { .. } => {
            bail!("--jsonl is not supported for the long-running web console command")
        }
        CliCommand::Bot { .. } => bail!(
            "--jsonl is not supported for bot gateway commands; use the normal terminal output or --json"
        ),
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
        EvalCommand::Weixin => {
            let report = yunxi_agent_eval::run_default_weixin_suite()
                .map_err(|error| anyhow::anyhow!("weixin evaluation failed to load: {error}"))?;
            if jsonl {
                println!("{}", serde_json::to_string(&report)?);
            } else if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", yunxi_agent_eval::render_weixin_text(&report));
            }
            if report.metrics.failed_scenarios > 0 || !report.golden_passed {
                bail!(
                    "weixin evaluation failed: {} scenarios failed, golden_failures={}",
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
            print_persona_profile(&settings, json)?;
        }
        PersonaCommand::List => {
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Show),
                "completed",
                "persona profile list",
            )?;
            print_persona_list(&settings, json)?;
        }
        PersonaCommand::Import { file } => {
            let profile = PersonaProfileStore::import_file(&file)
                .with_context(|| format!("failed to import persona profile {}", file.display()))?;
            settings.active_profile = profile.id.clone();
            settings.save().context("failed to save persona settings")?;
            append_control_audit(
                &store,
                &ControlRequest::new(ControlScope::Persona, ControlVerb::Update),
                "completed",
                format!("imported and activated profile {}", profile.id),
            )?;
            print_persona_status(&settings, json)?;
        }
        PersonaCommand::Set { profile } => {
            let selected = PersonaProfileStore::load(&profile)
                .with_context(|| format!("failed to load persona profile {profile}"))?;
            settings.active_profile = selected.id;
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
    let profile = PersonaProfileStore::load_active_checked(settings)
        .context("failed to load active persona profile")?;
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

fn print_persona_profile(settings: &PersonaSettings, json: bool) -> Result<()> {
    let profile = PersonaProfileStore::load_active_checked(settings)
        .context("failed to load active persona profile")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        println!("id: {}", profile.id);
        println!("display_name: {}", profile.display_name);
        println!("version: {}", profile.version);
        println!("identity: {}", profile.layers.identity);
        println!("soul: {}", profile.layers.soul);
        println!("values: {}", profile.layers.values);
        println!("voice: {}", profile.layers.voice);
        println!("companion_style: {}", profile.layers.companion_style);
        println!("work_style: {}", profile.layers.work_style);
        println!("boundaries: {}", profile.layers.boundaries);
        println!("addressing: {}", profile.layers.addressing);
        println!(
            "companion_rules.soul_signature: {}",
            profile
                .companion_rules
                .soul_signature
                .as_deref()
                .unwrap_or("")
        );
        println!(
            "companion_rules.warmth: {:?}",
            profile.companion_rules.warmth
        );
        println!(
            "companion_rules.directness: {:?}",
            profile.companion_rules.directness
        );
        println!(
            "companion_rules.initiative: {:?}",
            profile.companion_rules.initiative
        );
        println!("companion_rules.humor: {:?}", profile.companion_rules.humor);
        println!(
            "companion_rules.emotional_attunement: {:?}",
            profile.companion_rules.emotional_attunement
        );
        for (index, rule) in profile.companion_rules.reply_rules.iter().enumerate() {
            println!("companion_rules.reply_rules[{index}]: {rule}");
        }
        for (index, rule) in profile.companion_rules.memory_use_rules.iter().enumerate() {
            println!("companion_rules.memory_use_rules[{index}]: {rule}");
        }
        for (index, rule) in profile
            .companion_rules
            .relationship_rules
            .iter()
            .enumerate()
        {
            println!("companion_rules.relationship_rules[{index}]: {rule}");
        }
        for (index, rule) in profile.companion_rules.forbidden_styles.iter().enumerate() {
            println!("companion_rules.forbidden_styles[{index}]: {rule}");
        }
        for constraint in profile.constraints {
            println!("constraint.{}: {}", constraint.id, constraint.content);
        }
    }
    Ok(())
}

fn print_persona_list(settings: &PersonaSettings, json: bool) -> Result<()> {
    let profiles = PersonaProfileStore::list().context("failed to list persona profiles")?;
    if json {
        let profiles = profiles
            .into_iter()
            .map(|profile| {
                serde_json::json!({
                    "id": profile.id,
                    "display_name": profile.display_name,
                    "version": profile.version,
                    "active": profile.id == settings.active_profile,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&profiles)?);
    } else {
        for profile in profiles {
            let marker = if profile.id == settings.active_profile {
                "*"
            } else {
                " "
            };
            println!(
                "{marker} {} — {} (v{})",
                profile.id, profile.display_name, profile.version
            );
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
    let now = memory_now_millis();
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
    let recallable = records
        .iter()
        .filter(|record| record.is_recallable_at(now))
        .count();
    let non_recallable_active = active.saturating_sub(recallable);
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
                "runtime": {
                    "recallable": recallable,
                    "non_recallable_active": non_recallable_active,
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
        println!("runtime_recallable: {recallable}");
        println!("runtime_non_recallable_active: {non_recallable_active}");
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
        let now = memory_now_millis();
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
        let recallable = records
            .iter()
            .filter(|record| record.is_recallable_at(now))
            .count();
        let non_recallable_active = active.saturating_sub(recallable);
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
                "runtime": {
                    "recallable": recallable,
                    "non_recallable_active": non_recallable_active,
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
    let now = memory_now_millis();
    if json {
        let records = records
            .iter()
            .map(|record| memory_record_json(record, now))
            .collect::<Vec<_>>();
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
            let blockers = memory_runtime_blockers(record, now);
            let blocked_by = if blockers.is_empty() {
                "-".to_string()
            } else {
                blockers.join(",")
            };
            println!(
                "{}\tstorage={}\truntime={}\tblocked_by={}\t{}\t{}\trev={}\tmerged={}\t{}\t{}",
                record.id,
                memory_status_label(record.status),
                memory_runtime_status_label(record, now),
                blocked_by,
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
    let now = memory_now_millis();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "record": memory_record_json(record, now),
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
        println!(
            "runtime_status: {}",
            memory_runtime_status_label(record, now)
        );
        println!("runtime_recallable: {}", record.is_recallable_at(now));
        let blockers = memory_runtime_blockers(record, now);
        if !blockers.is_empty() {
            println!("runtime_blockers: {}", blockers.join(","));
        }
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

fn memory_record_json(record: &MemoryRecord, now_millis: u128) -> serde_json::Value {
    let mut value = serde_json::to_value(record).unwrap_or_else(|_| serde_json::json!({}));
    if let serde_json::Value::Object(map) = &mut value {
        let blockers = memory_runtime_blockers(record, now_millis);
        map.insert(
            "runtime_status".to_string(),
            serde_json::json!(memory_runtime_status_label(record, now_millis)),
        );
        map.insert(
            "runtime_recallable".to_string(),
            serde_json::json!(blockers.is_empty()),
        );
        map.insert("runtime_blockers".to_string(), serde_json::json!(blockers));
    }
    value
}

fn memory_runtime_status_label(record: &MemoryRecord, now_millis: u128) -> &'static str {
    if memory_runtime_blockers(record, now_millis).is_empty() {
        "recallable"
    } else {
        "not_recallable"
    }
}

fn memory_runtime_blockers(record: &MemoryRecord, now_millis: u128) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    match record.status {
        MemoryStatus::Active => {}
        MemoryStatus::Pending => blockers.push("pending"),
        MemoryStatus::Rejected => blockers.push("rejected"),
        MemoryStatus::Archived => blockers.push("archived"),
    }
    if record
        .temporal
        .valid_from_millis
        .is_some_and(|valid_from| valid_from > now_millis)
    {
        blockers.push("future_valid_from");
    }
    if record
        .temporal
        .expires_at_millis
        .is_some_and(|expires_at| expires_at <= now_millis)
    {
        blockers.push("expired");
    }
    if record.invalidation.invalidated_at_millis.is_some() {
        blockers.push("invalidated");
    }
    if record.invalidation.superseded_by.is_some() {
        blockers.push("superseded");
    }
    blockers
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
