use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentInput, AgentRunResult, ApprovalMode, BackendKind, SandboxMode,
};
use yunxi_agent_storage::{
    FileSessionStore, HistoryLoadOptions, RolloutRecord, SessionHistory, SessionId, SessionRecord,
    SessionStore,
};

const CODEX_CORE_PARITY_MAP: &str =
    include_str!("../../../docs/extraction-index/codex-core-agent-parity-map.md");

#[derive(Debug, Parser)]
#[command(name = "yunxi-agent-cli")]
#[command(about = "Run the extracted YunXi Agent core")]
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
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
        return run_command(command, config, backend, cli.json, cli.jsonl).await;
    }

    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        bail!("a prompt is required");
    }

    let result = run_agent_backend(backend, config, prompt).await?;
    print_run_result(result, cli.json, cli.jsonl)?;
    Ok(())
}

async fn run_agent_backend(
    backend: BackendKind,
    config: AgentConfig,
    prompt: String,
) -> Result<AgentRunResult> {
    let result = match backend {
        BackendKind::Yunxi => {
            let cwd = config.cwd.clone();
            let agent = Agent::new(config);
            let backend = yunxi_agent_runtime::YunXiRuntimeBackend::for_workspace(cwd);
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

fn print_run_result(result: AgentRunResult, json: bool, jsonl: bool) -> Result<()> {
    if jsonl {
        for event in result.events {
            println!("{}", serde_json::to_string(&event)?);
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        println!("{final_response}");
    }

    Ok(())
}

async fn run_command(
    command: CliCommand,
    config: AgentConfig,
    backend: BackendKind,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    match command {
        CliCommand::Sessions { command } => {
            run_session_command(command, config, backend, json, jsonl).await
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
            let result = run_agent_backend(backend, resume_config, resume_prompt).await?;
            print_run_result(result, json, jsonl)?;
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
