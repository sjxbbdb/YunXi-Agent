use crate::commands::{InteractiveCommand, help_text, parse_interactive_command};
use crate::input::{InteractiveInput, PlainInput};
use crate::provider_mode::{ProviderMode, ProviderSelection};
use crate::render::{
    InteractiveBanner, InteractiveRenderer, PlainInteractiveRenderer, RenderState,
};
use crate::terminal_mode::ResolvedTerminalMode;
use crate::tui::{TuiHandle, TuiInput, TuiInteractiveRenderer};
use crate::{redact_secret_fragments, run_agent_backend_stream};
use anyhow::{Context, Result};
use std::io::{self, IsTerminal};
use yunxi_agent_core::{
    AgentConfig, AgentEvent, AgentRunControl, AgentRunResult, AgentRunStatus, BackendKind,
    TokenUsage,
};
use yunxi_agent_mcp::{McpTransport, load_workspace_mcp_configs};
use yunxi_agent_storage::{FileSessionStore, SessionId, SessionStore};
use yunxi_agent_tools::workspace_tool_registry;

#[derive(Clone, Debug)]
pub(crate) struct InteractiveOptions {
    pub config: AgentConfig,
    pub backend: BackendKind,
    pub provider_mode: ProviderMode,
    pub terminal_mode: ResolvedTerminalMode,
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
    let terminal_mode = options.terminal_mode;
    let mut session = InteractiveSession::new(options)?;

    match terminal_mode {
        ResolvedTerminalMode::Plain => {
            let mut renderer = PlainInteractiveRenderer;
            session.print_banner(&mut renderer)?;
            let prompt_enabled = io::stdin().is_terminal() && io::stdout().is_terminal();
            let stdin = io::stdin();
            let reader = io::BufReader::new(stdin.lock());
            let stdout = io::stdout();
            let mut input = PlainInput::new(reader, stdout, prompt_enabled);
            session.read_eval_loop(&mut input, &mut renderer).await
        }
        ResolvedTerminalMode::Tui => {
            let handle = TuiHandle::enter()?;
            let mut renderer = TuiInteractiveRenderer::new(handle.clone());
            session.print_banner(&mut renderer)?;
            let mut input = TuiInput::new(handle);
            session.read_eval_loop(&mut input, &mut renderer).await
        }
    }
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

    fn print_banner(&self, renderer: &mut dyn InteractiveRenderer) -> Result<()> {
        renderer.banner(&InteractiveBanner {
            cwd: self.config.cwd.display().to_string(),
            backend: format!("{:?}", self.backend).to_ascii_lowercase(),
            provider_live: self.provider_selection.live,
            provider_source: self.provider_selection.source.as_str().to_string(),
            model: self.provider_selection.model.clone(),
            provider: self.provider_selection.provider.clone(),
        })?;
        if let Some(warning) = self.provider_selection.auto_fallback_warning() {
            renderer.warning(warning)?;
        }
        Ok(())
    }

    async fn read_eval_loop(
        &mut self,
        input: &mut dyn InteractiveInput,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<()> {
        loop {
            let Some(input_line) = input.read_prompt("yunxi> ")? else {
                if input.print_eof_message() {
                    println!("YunXi interactive session ended.");
                }
                return Ok(());
            };

            let input_line = input_line.trim();
            if input_line.is_empty() {
                continue;
            }

            if let Some(command) = parse_interactive_command(input_line) {
                if !self.handle_command(command, renderer).await? {
                    renderer.notice("session", "YunXi interactive session ended.")?;
                    return Ok(());
                }
                continue;
            }

            if let Err(error) = self
                .run_turn(input_line.to_string(), input, renderer)
                .await
            {
                renderer.error(&redact_secret_fragments(&format!("{error:#}")))?;
            }
        }
    }

    async fn handle_command(
        &mut self,
        command: InteractiveCommand,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<bool> {
        match command {
            InteractiveCommand::Exit => return Ok(false),
            InteractiveCommand::Help => renderer.notice("help", help_text())?,
            InteractiveCommand::Clear => renderer.clear()?,
            InteractiveCommand::Cwd => {
                renderer.notice("cwd", &self.config.cwd.display().to_string())?;
            }
            InteractiveCommand::Session => {
                renderer.notice("session", &self.session_summary_text())?;
            }
            InteractiveCommand::Status => renderer.notice("status", &self.status_text()?)?,
            InteractiveCommand::Tools => renderer.notice("tools", &self.tools_text()?)?,
            InteractiveCommand::Mcp => renderer.notice("mcp", &self.mcp_text()?)?,
            InteractiveCommand::Cost => renderer.notice("cost", &self.cost_text())?,
            InteractiveCommand::Model(model) => self.handle_model_command(model, renderer)?,
            InteractiveCommand::Provider(provider) => {
                self.handle_provider_command(provider, renderer)?;
            }
            InteractiveCommand::Resume(session_id) => {
                self.resume_session(session_id, renderer).await?;
            }
            InteractiveCommand::Unknown(message) => renderer.notice("command", &message)?,
        }
        Ok(true)
    }

    fn session_summary_text(&self) -> String {
        [
            format!(
                "session: {}",
                self.active_session_id.as_deref().unwrap_or("new")
            ),
            format!("turns: {}", self.turn_count),
            format!("cwd: {}", self.config.cwd.display()),
            format!(
                "provider_mode: {}",
                if self.provider_selection.live {
                    "live"
                } else {
                    "offline"
                }
            ),
            format!(
                "provider_source: {}",
                self.provider_selection.source.as_str()
            ),
            format!("provider: {}", self.provider_selection.provider),
            format!("model: {}", self.provider_selection.model),
        ]
        .join("\n")
    }

    fn status_text(&self) -> Result<String> {
        let mut lines = self
            .session_summary_text()
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        lines.push(format!("observed_events: {}", self.stats.observed_events));
        match &self.stats.last_turn {
            Some(summary) => {
                lines.push(format!("last_turn_status: {}", status_label(summary.status)));
                lines.push(format!("last_turn_events: {}", summary.events));
                lines.push(format!(
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
                ));
                lines.push(format!(
                    "last_turn_final_response: {}",
                    summary.final_response
                ));
            }
            None => lines.push("last_turn_status: none".to_string()),
        }
        let registry = workspace_tool_registry(&self.config.cwd)?;
        lines.push(format!(
            "tools: fixed={} dynamic={}",
            registry.specs().count(),
            registry.dynamic_specs().count()
        ));
        let mcp_servers = load_workspace_mcp_configs(&self.config.cwd)?;
        lines.push(format!("mcp_servers: {}", mcp_servers.len()));
        lines.extend(self.cost_text().lines().map(ToOwned::to_owned));
        Ok(lines.join("\n"))
    }

    fn tools_text(&self) -> Result<String> {
        let registry = workspace_tool_registry(&self.config.cwd)?;
        let fixed = registry.specs().collect::<Vec<_>>();
        let dynamic = registry.dynamic_specs().collect::<Vec<_>>();
        let mut lines = vec![format!(
            "tools: fixed={} dynamic={}",
            fixed.len(),
            dynamic.len()
        )];
        for spec in fixed {
            lines.push(format!(
                "[tool] {} visible={} - {}",
                spec.name, spec.model_visible, spec.description
            ));
        }
        for spec in dynamic {
            lines.push(format!(
                "[dynamic-tool] {} kind={:?} source={} - {}",
                spec.name,
                spec.kind,
                spec.source.as_deref().unwrap_or("workspace"),
                spec.description
            ));
        }
        Ok(lines.join("\n"))
    }

    fn mcp_text(&self) -> Result<String> {
        let configs = load_workspace_mcp_configs(&self.config.cwd)?;
        let seed_path = self.config.cwd.join(".yunxi").join("mcp-runtime.json");
        let mut lines = vec![
            format!("mcp_servers: {}", configs.len()),
            format!(
            "mcp_runtime_seed: {}",
            if seed_path.is_file() {
                seed_path.display().to_string()
            } else {
                "none".to_string()
            }
        )];
        if configs.is_empty() {
            lines.push("mcp_status: no workspace MCP configured".to_string());
        }
        for config in configs {
            lines.push(format!(
                "[mcp] {} enabled={} transport={}",
                config.name,
                config.enabled,
                describe_mcp_transport(&config.transport)
            ));
        }
        Ok(lines.join("\n"))
    }

    fn cost_text(&self) -> String {
        if self.provider_selection.is_offline_runtime() {
            return [
                "last_turn_usage: n/a - offline, no model call".to_string(),
                "session_usage: n/a - offline, no model call".to_string(),
            ]
            .join("\n");
        }
        let mut lines = Vec::new();
        lines.push(match &self.stats.last_turn {
            Some(summary) if !summary.usage.is_zero() => {
                format!("last_turn_usage: {}", summary.usage)
            }
            Some(_) => "last_turn_usage: unavailable".to_string(),
            None => "last_turn_usage: none".to_string(),
        });
        if self.stats.total_usage.is_zero() {
            lines.push("session_usage: unavailable".to_string());
        } else {
            lines.push(format!("session_usage: {}", self.stats.total_usage));
        }
        lines.join("\n")
    }

    fn handle_model_command(
        &mut self,
        model: Option<String>,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<()> {
        if let Some(model) = model {
            self.config.model = Some(model);
        }
        self.refresh_provider_selection()?;
        renderer.notice("model", &format!("model: {}", self.provider_selection.model))?;
        Ok(())
    }

    fn handle_provider_command(
        &mut self,
        provider: Option<String>,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<()> {
        if let Some(provider) = provider {
            self.config.provider = Some(provider);
        }
        self.refresh_provider_selection()?;
        renderer.notice(
            "provider",
            &format!("provider: {}", self.provider_selection.provider),
        )?;
        Ok(())
    }

    async fn resume_session(
        &mut self,
        session_id: String,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<()> {
        let store = FileSessionStore::for_workspace(&self.config.cwd);
        let Some(record) = store
            .load(&SessionId::new(session_id.clone()))
            .await
            .context("failed to load session for interactive resume")?
        else {
            renderer.notice("session", &format!("session not found: {session_id}"))?;
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
        renderer.notice("session", &format!("resumed session: {session_id}"))?;
        Ok(())
    }

    async fn run_turn(
        &mut self,
        prompt: String,
        input: &mut dyn InteractiveInput,
        renderer: &mut dyn InteractiveRenderer,
    ) -> Result<()> {
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
        let mut render_state =
            RenderState::with_offline_label(self.provider_selection.is_offline_runtime());
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
                        Some(event) => renderer.event(&event, &mut render_state)?,
                        None => events_open = false,
                    }
                }
                request = stream.approvals.recv(), if approvals_open => {
                    match request {
                        Some(request) => renderer.approval_request(request, input)?,
                        None => approvals_open = false,
                    }
                }
                request = stream.user_inputs.recv(), if user_inputs_open => {
                    match request {
                        Some(request) => renderer.user_input_request(request, input)?,
                        None => user_inputs_open = false,
                    }
                }
                signal = tokio::signal::ctrl_c(), if control_slot.is_some() => {
                    match signal {
                        Ok(()) => {
                            if let Some(control) = &control_slot {
                                control.cancel();
                            }
                            renderer.warning("[cancelled] cancellation requested")?;
                        }
                        Err(error) => renderer.warning(&format!("[cancelled] cancellation requested; signal error: {error}"))?,
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
                renderer.event(
                    &AgentEvent::Message {
                        content: final_response.clone(),
                    },
                    &mut render_state,
                )?;
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
