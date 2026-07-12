use crate::commands::{InteractiveCommand, help_text, parse_interactive_command};
use crate::provider_mode::{ProviderMode, ProviderSelection};
use crate::render::{InteractiveBanner, RenderState, print_banner, render_agent_event};
use crate::{redact_secret_fragments, run_agent_backend_stream};
use anyhow::{Context, Result};
use std::io::{self, BufRead, IsTerminal, Write};
use yunxi_agent_core::{
    AgentConfig, AgentEvent, AgentRunApprovalDecision, AgentRunApprovalRequest, AgentRunControl,
    AgentRunResult, AgentRunStatus, AgentRunUserInputRequest, AgentRunUserInputResponse,
    BackendKind, TokenUsage,
};
use yunxi_agent_mcp::{McpTransport, load_workspace_mcp_configs};
use yunxi_agent_storage::{FileSessionStore, SessionId, SessionStore};
use yunxi_agent_tools::workspace_tool_registry;

#[derive(Clone, Debug)]
pub(crate) struct InteractiveOptions {
    pub config: AgentConfig,
    pub backend: BackendKind,
    pub provider_mode: ProviderMode,
}

#[derive(Clone, Debug)]
struct InteractiveSession {
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    provider_selection: ProviderSelection,
    active_session_id: Option<String>,
    turn_count: usize,
    stats: InteractiveStats,
}

pub(crate) async fn run_interactive(options: InteractiveOptions) -> Result<()> {
    let stdin_is_terminal = io::stdin().is_terminal();
    let mut session = InteractiveSession::new(options)?;
    session.print_banner();
    session.read_eval_loop(stdin_is_terminal).await
}

impl InteractiveSession {
    fn new(options: InteractiveOptions) -> Result<Self> {
        let provider_selection = options
            .provider_mode
            .resolve(options.backend, &options.config)?;
        Ok(Self {
            config: options.config,
            backend: options.backend,
            provider_mode: options.provider_mode,
            provider_selection,
            active_session_id: None,
            turn_count: 0,
            stats: InteractiveStats::default(),
        })
    }

    fn print_banner(&self) {
        print_banner(&InteractiveBanner {
            cwd: self.config.cwd.display().to_string(),
            backend: format!("{:?}", self.backend).to_ascii_lowercase(),
            provider_live: self.provider_selection.live,
            provider_source: self.provider_selection.source.as_str().to_string(),
            model: self.provider_selection.model.clone(),
            provider: self.provider_selection.provider.clone(),
        });
    }

    async fn read_eval_loop(&mut self, stdin_is_terminal: bool) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        let mut stdout = io::stdout();

        loop {
            if stdin_is_terminal {
                print!("yunxi> ");
                stdout.flush()?;
            }

            let mut input = String::new();
            let bytes = reader
                .read_line(&mut input)
                .context("failed to read interactive input")?;
            if bytes == 0 {
                if !stdin_is_terminal {
                    println!("YunXi interactive session ended.");
                }
                return Ok(());
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            if let Some(command) = parse_interactive_command(input) {
                if !self.handle_command(command).await? {
                    println!("YunXi interactive session ended.");
                    return Ok(());
                }
                continue;
            }

            if let Err(error) = self
                .run_turn(input.to_string(), &mut reader, &mut stdout)
                .await
            {
                eprintln!(
                    "[error] {}",
                    redact_secret_fragments(&format!("{error:#}"))
                );
            }
        }
    }

    async fn handle_command(&mut self, command: InteractiveCommand) -> Result<bool> {
        match command {
            InteractiveCommand::Exit => return Ok(false),
            InteractiveCommand::Help => println!("{}", help_text()),
            InteractiveCommand::Clear => {
                print!("\x1b[2J\x1b[H");
                io::stdout().flush()?;
            }
            InteractiveCommand::Cwd => println!("{}", self.config.cwd.display()),
            InteractiveCommand::Session => self.print_session_summary(),
            InteractiveCommand::Status => self.print_status()?,
            InteractiveCommand::Tools => self.print_tools()?,
            InteractiveCommand::Mcp => self.print_mcp()?,
            InteractiveCommand::Cost => self.print_cost(),
            InteractiveCommand::Model(model) => self.handle_model_command(model)?,
            InteractiveCommand::Provider(provider) => self.handle_provider_command(provider)?,
            InteractiveCommand::Resume(session_id) => self.resume_session(session_id).await?,
            InteractiveCommand::Unknown(message) => println!("{message}"),
        }
        Ok(true)
    }

    fn print_session_summary(&self) {
        println!(
            "session: {}",
            self.active_session_id.as_deref().unwrap_or("new")
        );
        println!("turns: {}", self.turn_count);
        println!("cwd: {}", self.config.cwd.display());
        println!(
            "provider_mode: {}",
            if self.provider_selection.live {
                "live"
            } else {
                "offline"
            }
        );
        println!(
            "provider_source: {}",
            self.provider_selection.source.as_str()
        );
        println!("provider: {}", self.provider_selection.provider);
        println!("model: {}", self.provider_selection.model);
    }

    fn print_status(&self) -> Result<()> {
        self.print_session_summary();
        println!("observed_events: {}", self.stats.observed_events);
        match &self.stats.last_turn {
            Some(summary) => {
                println!("last_turn_status: {}", status_label(summary.status));
                println!("last_turn_events: {}", summary.events);
                println!(
                    "last_turn_activity: messages={} reasoning={} tools={}/{} commands={}/{}/{} mcp={}/{} mcp_sessions={} files={} patches={} todos={} approvals={} escalations={} children={} warnings={} errors={} cancelled={}",
                    summary.messages,
                    summary.reasoning,
                    summary.tool_started,
                    summary.tool_completed,
                    summary.command_started,
                    summary.command_updated,
                    summary.command_completed,
                    summary.mcp_started,
                    summary.mcp_completed,
                    summary.mcp_sessions,
                    summary.file_changes,
                    summary.patch_completed,
                    summary.todo_updates,
                    summary.approvals,
                    summary.escalations,
                    summary.child_events,
                    summary.warnings,
                    summary.errors,
                    summary.cancelled
                );
                println!("last_turn_final_response: {}", summary.final_response);
            }
            None => println!("last_turn_status: none"),
        }
        let registry = workspace_tool_registry(&self.config.cwd)?;
        println!(
            "tools: fixed={} dynamic={}",
            registry.specs().count(),
            registry.dynamic_specs().count()
        );
        let mcp_servers = load_workspace_mcp_configs(&self.config.cwd)?;
        println!("mcp_servers: {}", mcp_servers.len());
        self.print_cost();
        Ok(())
    }

    fn print_tools(&self) -> Result<()> {
        let registry = workspace_tool_registry(&self.config.cwd)?;
        let fixed = registry.specs().collect::<Vec<_>>();
        let dynamic = registry.dynamic_specs().collect::<Vec<_>>();
        println!("tools: fixed={} dynamic={}", fixed.len(), dynamic.len());
        for spec in fixed {
            println!(
                "[tool] {} visible={} - {}",
                spec.name, spec.model_visible, spec.description
            );
        }
        for spec in dynamic {
            println!(
                "[dynamic-tool] {} kind={:?} source={} - {}",
                spec.name,
                spec.kind,
                spec.source.as_deref().unwrap_or("workspace"),
                spec.description
            );
        }
        Ok(())
    }

    fn print_mcp(&self) -> Result<()> {
        let configs = load_workspace_mcp_configs(&self.config.cwd)?;
        let seed_path = self.config.cwd.join(".yunxi").join("mcp-runtime.json");
        println!("mcp_servers: {}", configs.len());
        println!(
            "mcp_runtime_seed: {}",
            if seed_path.is_file() {
                seed_path.display().to_string()
            } else {
                "none".to_string()
            }
        );
        if configs.is_empty() {
            println!("mcp_status: no workspace MCP configured");
        }
        for config in configs {
            println!(
                "[mcp] {} enabled={} transport={}",
                config.name,
                config.enabled,
                describe_mcp_transport(&config.transport)
            );
        }
        Ok(())
    }

    fn print_cost(&self) {
        match &self.stats.last_turn {
            Some(summary) if !summary.usage.is_zero() => {
                println!("last_turn_usage: {}", summary.usage);
            }
            Some(_) => println!("last_turn_usage: unavailable"),
            None => println!("last_turn_usage: none"),
        }
        if self.stats.total_usage.is_zero() {
            println!("session_usage: unavailable");
        } else {
            println!("session_usage: {}", self.stats.total_usage);
        }
    }

    fn handle_model_command(&mut self, model: Option<String>) -> Result<()> {
        if let Some(model) = model {
            self.config.model = Some(model);
        }
        self.refresh_provider_selection()?;
        println!("model: {}", self.provider_selection.model);
        Ok(())
    }

    fn handle_provider_command(&mut self, provider: Option<String>) -> Result<()> {
        if let Some(provider) = provider {
            self.config.provider = Some(provider);
        }
        self.refresh_provider_selection()?;
        println!("provider: {}", self.provider_selection.provider);
        Ok(())
    }

    async fn resume_session(&mut self, session_id: String) -> Result<()> {
        let store = FileSessionStore::for_workspace(&self.config.cwd);
        let Some(record) = store
            .load(&SessionId::new(session_id.clone()))
            .await
            .context("failed to load session for interactive resume")?
        else {
            println!("session not found: {session_id}");
            return Ok(());
        };

        if self.config.model.is_none() {
            self.config.model = record.model;
        }
        if self.config.provider.is_none() {
            self.config.provider = record.provider;
        }
        self.active_session_id = Some(session_id.clone());
        self.turn_count = 0;
        self.stats = InteractiveStats::default();
        self.refresh_provider_selection()?;
        println!("resumed session: {session_id}");
        Ok(())
    }

    async fn run_turn<R>(
        &mut self,
        prompt: String,
        reader: &mut R,
        stdout: &mut io::Stdout,
    ) -> Result<()>
    where
        R: BufRead,
    {
        let mut turn_config = self.config.clone();
        if let Some(parent_session_id) = &self.active_session_id {
            turn_config = turn_config
                .with_parent_session_id(parent_session_id.clone())
                .with_session_title(format!("Interactive turn {}", self.turn_count + 1));
        } else {
            turn_config = turn_config.with_session_title("YunXi interactive session");
        }

        let backend = self.backend;
        self.provider_selection = self.provider_mode.resolve(backend, &turn_config)?;
        let turn_config = self.provider_selection.apply_to_config(turn_config);
        let (control, mut stream) = AgentRunControl::streaming();
        let run_control = control.clone();
        let mut control_slot = Some(control);
        let mut turn = Box::pin(run_agent_backend_stream(
            backend,
            turn_config,
            prompt,
            self.provider_selection.live,
            run_control,
        ));
        let mut render_state = RenderState::default();
        let mut result: Option<AgentRunResult> = None;
        let mut events_open = true;
        let mut approvals_open = true;
        let mut user_inputs_open = true;

        loop {
            if result.is_some() && !events_open && !approvals_open && !user_inputs_open {
                break;
            }

            tokio::select! {
                event = stream.events.recv(), if events_open => {
                    match event {
                        Some(event) => render_agent_event(&event, &mut render_state)?,
                        None => events_open = false,
                    }
                }
                request = stream.approvals.recv(), if approvals_open => {
                    match request {
                        Some(request) => respond_to_approval_request(request, reader, stdout)?,
                        None => approvals_open = false,
                    }
                }
                request = stream.user_inputs.recv(), if user_inputs_open => {
                    match request {
                        Some(request) => respond_to_user_input_request(request, reader, stdout)?,
                        None => user_inputs_open = false,
                    }
                }
                signal = tokio::signal::ctrl_c(), if control_slot.is_some() => {
                    match signal {
                        Ok(()) => {
                            if let Some(control) = &control_slot {
                                control.cancel();
                            }
                            println!("[cancelled] cancellation requested");
                        }
                        Err(error) => println!("[cancelled] cancellation requested; signal error: {error}"),
                    }
                }
                turn_result = &mut turn, if result.is_none() => {
                    let completed = turn_result.context("interactive turn failed")?;
                    result = Some(completed);
                    control_slot = None;
                }
            }
        }
        if let Some(result) = result {
            if !render_state.saw_assistant_message()
                && let Some(final_response) = &result.final_response
            {
                println!("{final_response}");
            }
            self.record_turn_result(&result);
        }
        Ok(())
    }

    fn record_turn_result(&mut self, result: &AgentRunResult) {
        if let Some(session_id) = session_id_from_result(result) {
            self.active_session_id = Some(session_id);
        }
        self.turn_count = self.turn_count.saturating_add(1);
        self.stats.record(TurnSummary::from_result(result));
    }

    fn refresh_provider_selection(&mut self) -> Result<()> {
        self.provider_selection = self.provider_mode.resolve(self.backend, &self.config)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct InteractiveStats {
    last_turn: Option<TurnSummary>,
    total_usage: UsageTotals,
    observed_events: usize,
}

impl InteractiveStats {
    fn record(&mut self, summary: TurnSummary) {
        self.observed_events = self.observed_events.saturating_add(summary.events);
        self.total_usage.add_totals(summary.usage);
        self.last_turn = Some(summary);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TurnSummary {
    status: Option<AgentRunStatus>,
    final_response: bool,
    events: usize,
    messages: usize,
    reasoning: usize,
    command_started: usize,
    command_updated: usize,
    command_completed: usize,
    tool_started: usize,
    tool_completed: usize,
    mcp_started: usize,
    mcp_completed: usize,
    mcp_sessions: usize,
    file_changes: usize,
    patch_completed: usize,
    todo_updates: usize,
    approvals: usize,
    escalations: usize,
    child_events: usize,
    warnings: usize,
    errors: usize,
    cancelled: usize,
    usage: UsageTotals,
}

impl TurnSummary {
    fn from_result(result: &AgentRunResult) -> Self {
        let mut summary = Self {
            status: Some(result.status),
            final_response: result.final_response.is_some(),
            events: result.events.len(),
            ..Self::default()
        };
        for event in &result.events {
            match event {
                AgentEvent::Message { .. } => summary.messages += 1,
                AgentEvent::Reasoning { .. } => summary.reasoning += 1,
                AgentEvent::CommandStarted { .. } => summary.command_started += 1,
                AgentEvent::CommandUpdated { .. } => summary.command_updated += 1,
                AgentEvent::CommandCompleted { .. } | AgentEvent::CommandFinished { .. } => {
                    summary.command_completed += 1;
                }
                AgentEvent::ToolCallStarted { .. } => summary.tool_started += 1,
                AgentEvent::ToolCallCompleted { .. } => summary.tool_completed += 1,
                AgentEvent::McpToolStarted { .. } => summary.mcp_started += 1,
                AgentEvent::McpToolCompleted { .. } => summary.mcp_completed += 1,
                AgentEvent::McpSession { .. } => summary.mcp_sessions += 1,
                AgentEvent::FileChanged { .. } => summary.file_changes += 1,
                AgentEvent::PatchCompleted { .. } => summary.patch_completed += 1,
                AgentEvent::TodoUpdated { .. } => summary.todo_updates += 1,
                AgentEvent::ApprovalRequested { .. } | AgentEvent::ApprovalCompleted { .. } => {
                    summary.approvals += 1;
                }
                AgentEvent::EscalationRequested { .. } | AgentEvent::EscalationCompleted { .. } => {
                    summary.escalations += 1;
                }
                AgentEvent::ChildAgentEvent { .. }
                | AgentEvent::ChildScopedStream { .. }
                | AgentEvent::MultiAgentEvent { .. } => summary.child_events += 1,
                AgentEvent::Warning { .. } => summary.warnings += 1,
                AgentEvent::Error { .. } | AgentEvent::ProviderError { .. } => {
                    summary.errors += 1;
                }
                AgentEvent::Cancelled { .. } => summary.cancelled += 1,
                AgentEvent::Completed { status, usage } => {
                    summary.status = Some(*status);
                    if let Some(usage) = usage {
                        summary.usage.add_usage(*usage);
                    }
                }
                AgentEvent::Started { .. }
                | AgentEvent::ThreadStarted { .. }
                | AgentEvent::TurnStarted
                | AgentEvent::ThreadState { .. }
                | AgentEvent::TurnMetadata { .. }
                | AgentEvent::TurnState { .. }
                | AgentEvent::DeepParityState { .. }
                | AgentEvent::SandboxAttempt { .. }
                | AgentEvent::ApprovalCacheState { .. }
                | AgentEvent::ContextStatus { .. }
                | AgentEvent::StorageState { .. } => {}
            }
        }
        summary
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct UsageTotals {
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    reasoning_output_tokens: i64,
}

impl UsageTotals {
    fn add_usage(&mut self, usage: TokenUsage) {
        self.input_tokens = self.input_tokens.saturating_add(usage.input_tokens);
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(usage.cached_input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(usage.output_tokens);
        self.reasoning_output_tokens = self
            .reasoning_output_tokens
            .saturating_add(usage.reasoning_output_tokens);
    }

    fn add_totals(&mut self, other: Self) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(other.cached_input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.reasoning_output_tokens = self
            .reasoning_output_tokens
            .saturating_add(other.reasoning_output_tokens);
    }

    fn is_zero(self) -> bool {
        self.input_tokens == 0
            && self.cached_input_tokens == 0
            && self.output_tokens == 0
            && self.reasoning_output_tokens == 0
    }
}

impl std::fmt::Display for UsageTotals {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "input={} cached_input={} output={} reasoning_output={}",
            self.input_tokens,
            self.cached_input_tokens,
            self.output_tokens,
            self.reasoning_output_tokens
        )
    }
}

fn status_label(status: Option<AgentRunStatus>) -> &'static str {
    match status {
        Some(AgentRunStatus::Completed) => "completed",
        Some(AgentRunStatus::Failed) => "failed",
        Some(AgentRunStatus::Cancelled) => "cancelled",
        None => "unknown",
    }
}

fn describe_mcp_transport(transport: &McpTransport) -> String {
    match transport {
        McpTransport::Stdio { command, args } => {
            if args.is_empty() {
                format!("stdio:{command}")
            } else {
                format!("stdio:{} {}", command, args.join(" "))
            }
        }
        McpTransport::Http { url } => format!("http:{url}"),
    }
}

fn respond_to_approval_request<R>(
    request: AgentRunApprovalRequest,
    reader: &mut R,
    stdout: &mut io::Stdout,
) -> Result<()>
where
    R: BufRead,
{
    println!(
        "[approval] {} requires approval in {}",
        request.tool_name, request.cwd
    );
    if let Some(command) = &request.command {
        println!("[approval] command: {command}");
    }
    println!("[approval] reason: {}", request.reason);
    print!("approve? y/N: ");
    stdout.flush()?;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .context("failed to read approval response")?;
    let approved = matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes" | "approve" | "approved"
    );
    let reason = if approved {
        Some("approved by YunXi interactive CLI".to_string())
    } else {
        Some("declined by YunXi interactive CLI".to_string())
    };
    let _ = request
        .respond_to
        .send(AgentRunApprovalDecision { approved, reason });
    Ok(())
}

fn respond_to_user_input_request<R>(
    request: AgentRunUserInputRequest,
    reader: &mut R,
    stdout: &mut io::Stdout,
) -> Result<()>
where
    R: BufRead,
{
    print!("{} ", request.prompt);
    stdout.flush()?;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .context("failed to read requested user input")?;
    let value = line.trim_end_matches(['\r', '\n']).to_string();
    let _ = request.respond_to.send(AgentRunUserInputResponse {
        value: Some(value),
    });
    Ok(())
}

fn session_id_from_result(result: &AgentRunResult) -> Option<String> {
    result.events.iter().rev().find_map(|event| match event {
        yunxi_agent_core::AgentEvent::StorageState {
            session_id: Some(session_id),
            ..
        } => Some(session_id.clone()),
        yunxi_agent_core::AgentEvent::ThreadState { state } => state.session_id.clone(),
        _ => None,
    })
}
