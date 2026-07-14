use crate::input::InteractiveInput;
use anyhow::Result;
use std::io::{self, Write};
use yunxi_agent_core::{
    AgentEvent, AgentRunApprovalDecision, AgentRunApprovalRequest, AgentRunResult, AgentRunStatus,
    AgentRunUserInputRequest, AgentRunUserInputResponse, CommandStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractiveBanner {
    pub cwd: String,
    pub backend: String,
    pub provider_live: bool,
    pub provider_source: String,
    pub model: String,
    pub provider: String,
}

pub(crate) trait InteractiveRenderer {
    fn banner(&mut self, banner: &InteractiveBanner) -> Result<()>;
    fn warning(&mut self, message: &str) -> Result<()>;
    fn notice(&mut self, label: &str, message: &str) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
    fn event(&mut self, event: &AgentEvent, state: &mut RenderState) -> Result<()>;
    fn approval_request(
        &mut self,
        request: AgentRunApprovalRequest,
        input: &mut dyn InteractiveInput,
    ) -> Result<()>;
    fn user_input_request(
        &mut self,
        request: AgentRunUserInputRequest,
        input: &mut dyn InteractiveInput,
    ) -> Result<()>;
    fn tick(&mut self) -> Result<()> {
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn set_debug_events(&mut self, enabled: bool) -> Result<()>;
    fn show_details(&mut self, id: Option<usize>) -> Result<()>;
    fn error(&mut self, message: &str) -> Result<()>;
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PlainInteractiveRenderer;

impl InteractiveRenderer for PlainInteractiveRenderer {
    fn banner(&mut self, banner: &InteractiveBanner) -> Result<()> {
        print_banner(banner);
        Ok(())
    }

    fn warning(&mut self, message: &str) -> Result<()> {
        println!("{message}");
        Ok(())
    }

    fn notice(&mut self, _label: &str, message: &str) -> Result<()> {
        println!("{message}");
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        print!("\x1b[2J\x1b[H");
        io::stdout().flush()?;
        Ok(())
    }

    fn event(&mut self, event: &AgentEvent, state: &mut RenderState) -> Result<()> {
        render_agent_event(event, state)
    }

    fn approval_request(
        &mut self,
        request: AgentRunApprovalRequest,
        input: &mut dyn InteractiveInput,
    ) -> Result<()> {
        respond_to_approval_request(request, input)
    }

    fn user_input_request(
        &mut self,
        request: AgentRunUserInputRequest,
        input: &mut dyn InteractiveInput,
    ) -> Result<()> {
        respond_to_user_input_request(request, input)
    }

    fn error(&mut self, message: &str) -> Result<()> {
        eprintln!("[error] {message}");
        Ok(())
    }

    fn set_debug_events(&mut self, enabled: bool) -> Result<()> {
        println!(
            "[debug] event debug {} (plain renderer already prints raw event stream)",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    fn show_details(&mut self, id: Option<usize>) -> Result<()> {
        match id {
            Some(id) => println!("[details] TUI detail #{id} is not available in plain mode"),
            None => println!("[details] TUI details are not available in plain mode"),
        }
        Ok(())
    }
}

pub(crate) fn print_banner(banner: &InteractiveBanner) {
    println!("YunXi Agent v1.8.3 interactive CLI");
    println!("cwd: {}", banner.cwd);
    println!("backend: {}", banner.backend);
    println!(
        "provider_mode: {}",
        if banner.provider_live {
            "live"
        } else {
            "offline"
        }
    );
    println!("provider_source: {}", banner.provider_source);
    println!("provider: {}", banner.provider);
    println!("model: {}", banner.model);
    if !banner.provider_live {
        println!("offline_runtime: static_provider (stage fixtures disabled by default)");
    }
    println!("Type /help for commands, /exit to quit.");
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RenderState {
    saw_assistant_message: bool,
    last_assistant_message: Option<String>,
    last_command_output: Option<String>,
    offline_label: bool,
}

impl RenderState {
    pub(crate) fn with_offline_label(offline_label: bool) -> Self {
        Self {
            offline_label,
            ..Self::default()
        }
    }

    pub(crate) fn saw_assistant_message(&self) -> bool {
        self.saw_assistant_message
    }

    pub(crate) fn render_assistant_content(&self, content: &str) -> String {
        if self.offline_label {
            format!("[offline] {content}")
        } else {
            content.to_string()
        }
    }

    pub(crate) fn observe_assistant_content(&mut self, content: &str) -> Option<String> {
        self.saw_assistant_message = true;
        let rendered = self.render_assistant_content(content);
        if self.last_assistant_message.as_deref() == Some(rendered.as_str()) {
            return None;
        }
        self.last_assistant_message = Some(rendered.clone());
        Some(rendered)
    }
}

pub(crate) fn render_agent_event(event: &AgentEvent, state: &mut RenderState) -> Result<()> {
    match event {
        AgentEvent::Message { content } => {
            if let Some(rendered) = state.observe_assistant_content(content) {
                println!("{rendered}");
            }
        }
        AgentEvent::Reasoning { content } => {
            println!("[reasoning] {content}");
        }
        AgentEvent::CommandStarted { command, .. } => {
            println!("[shell] started: {command}");
        }
        AgentEvent::CommandUpdated {
            aggregated_output, ..
        } => {
            if !aggregated_output.trim().is_empty()
                && state.last_command_output.as_deref() != Some(aggregated_output)
            {
                println!("{aggregated_output}");
                state.last_command_output = Some(aggregated_output.clone());
            }
        }
        AgentEvent::CommandCompleted {
            command,
            exit_code,
            status,
            ..
        } => {
            println!(
                "[shell] {command} -> {}{}",
                command_status_label(*status),
                exit_code
                    .map(|code| format!(" ({code})"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::CommandFinished { command, exit_code } => {
            println!("[shell] {command} exited with {exit_code}");
        }
        AgentEvent::FileChanged { path, kind } => {
            println!("[file] {kind:?}: {path}");
        }
        AgentEvent::PatchCompleted { status } => {
            println!("[patch] {status:?}");
        }
        AgentEvent::TodoUpdated { id, items } => {
            println!(
                "[todo] {} item(s){}",
                items.len(),
                id.as_ref()
                    .map(|value| format!(" id={value}"))
                    .unwrap_or_default()
            );
            for item in items {
                println!(
                    "[todo] [{}] {}",
                    if item.completed { "x" } else { " " },
                    item.text
                );
            }
        }
        AgentEvent::ToolCallStarted { name, .. } => {
            println!("[tool] started: {name}");
        }
        AgentEvent::ToolCallCompleted {
            name,
            output,
            status,
            ..
        } => {
            println!("[tool] {name} -> {}", command_status_label(*status));
            if !output.trim().is_empty() {
                println!("{output}");
            }
        }
        AgentEvent::McpToolStarted { server, tool, .. } => {
            println!("[mcp] {server}/{tool} started");
        }
        AgentEvent::McpToolCompleted {
            server,
            tool,
            status,
            ..
        } => {
            println!("[mcp] {server}/{tool} -> {status:?}");
        }
        AgentEvent::McpSession {
            server,
            status,
            message,
        } => {
            println!(
                "[mcp-session] {server}: {status}{}",
                message
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::ApprovalRequested {
            tool_name, reason, ..
        } => {
            println!("[approval] requested for {tool_name}: {reason}");
        }
        AgentEvent::ApprovalCompleted {
            approved, reason, ..
        } => {
            println!(
                "[approval] {}{}",
                if *approved { "approved" } else { "declined" },
                reason
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::EscalationRequested {
            tool_name,
            reason,
            required_sandbox,
            required_network,
            ..
        } => {
            println!(
                "[escalation] {tool_name}: {reason}{}{}",
                required_sandbox
                    .as_ref()
                    .map(|value| format!(" sandbox={value}"))
                    .unwrap_or_default(),
                required_network
                    .as_ref()
                    .map(|value| format!(" network={value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::EscalationCompleted {
            approved, reason, ..
        } => {
            println!(
                "[escalation] {}{}",
                if *approved { "approved" } else { "declined" },
                reason
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            status,
            message,
            ..
        } => {
            println!(
                "[child:{agent_id}] {status} session={child_session_id}{}",
                message
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::ChildScopedStream {
            agent_id,
            event,
            seq,
            message,
            ..
        } => {
            println!(
                "  [child:{agent_id} #{seq}] {event}{}",
                message
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            );
        }
        AgentEvent::ContextStatus {
            active_context_tokens,
            token_limit_reached,
            compacted,
            dropped_messages,
        } => {
            println!(
                "[context] tokens={active_context_tokens} limit_reached={token_limit_reached} compacted={compacted} dropped={dropped_messages}"
            );
        }
        AgentEvent::StorageState {
            session_id,
            rollout_items,
            rollout_truncated,
            child_session_ids,
            ..
        } => {
            println!(
                "[session] {} rollout_items={rollout_items} truncated={rollout_truncated} children={}",
                session_id.as_deref().unwrap_or("unknown"),
                child_session_ids.len()
            );
        }
        AgentEvent::PersonaLoaded {
            profile_id,
            display_name,
            enabled,
            ..
        } => {
            println!("[persona] loaded profile={profile_id} display_name={display_name} enabled={enabled}");
        }
        AgentEvent::PersonaContextInjected {
            profile_id,
            memory_count,
            budget_used_chars,
            budget_limit_chars,
            ..
        } => {
            println!(
                "[persona] context profile={profile_id} memories={memory_count} budget={budget_used_chars}/{budget_limit_chars}"
            );
        }
        AgentEvent::MemoryRecall {
            count,
            always_on_count,
            dropped_unrelated,
            dropped_by_budget,
            dropped_duplicates,
            budget_used_chars,
            truncated,
            ..
        } => {
            let dropped = dropped_unrelated + dropped_by_budget + dropped_duplicates;
            println!(
                "[memory] recalled={count} always_on={always_on_count} dropped={dropped} unrelated={dropped_unrelated} duplicates={dropped_duplicates} budget_dropped={dropped_by_budget} budget_used={budget_used_chars} truncated={truncated}"
            );
        }
        AgentEvent::MemoryCandidate {
            id,
            kind,
            sensitivity,
            status,
            write_policy,
            reason,
            ..
        } => {
            println!(
                "[memory] candidate id={id} kind={kind} sensitivity={sensitivity} status={status} policy={write_policy} reason={reason}"
            );
        }
        AgentEvent::MemoryWrite {
            kind,
            status,
            action,
            revision,
            merged_count,
            merge_strategy,
            conflict_family,
            ..
        } => {
            let strategy = merge_strategy
                .as_ref()
                .map(|value| format!(" merge_strategy={value}"))
                .unwrap_or_default();
            let conflict = conflict_family
                .as_ref()
                .map(|value| format!(" conflict_family={value}"))
                .unwrap_or_default();
            println!(
                "[memory] write action={action} kind={kind} status={status} revision={revision} merged_count={merged_count}{strategy}{conflict}"
            );
        }
        AgentEvent::MemoryWarning { warning, .. } => {
            println!("[memory-warning] {warning}");
        }
        AgentEvent::Warning { message } => {
            println!("[warning] {message}");
        }
        AgentEvent::Error { message } => {
            println!("[error] {message}");
        }
        AgentEvent::ProviderError {
            provider,
            classification,
            status,
            message,
        } => {
            println!(
                "[provider:{provider}] {classification}{} - {message}",
                status.map(|value| format!(" {value}")).unwrap_or_default()
            );
        }
        AgentEvent::Cancelled { reason } => {
            println!(
                "[cancelled] {}",
                reason.as_deref().unwrap_or("current turn cancelled")
            );
        }
        AgentEvent::Completed { status, usage } => {
            if *status != AgentRunStatus::Completed {
                println!("[turn] {status:?}");
            }
            if let Some(usage) = usage {
                println!(
                    "[usage] input={} cached_input={} output={} reasoning_output={}",
                    usage.input_tokens,
                    usage.cached_input_tokens,
                    usage.output_tokens,
                    usage.reasoning_output_tokens
                );
            }
        }
        AgentEvent::ThreadStarted { .. }
        | AgentEvent::TurnStarted
        | AgentEvent::ThreadState { .. }
        | AgentEvent::TurnMetadata { .. }
        | AgentEvent::TurnState { .. }
        | AgentEvent::DeepParityState { .. }
        | AgentEvent::ApprovalCacheState { .. }
        | AgentEvent::MultiAgentEvent { .. }
        | AgentEvent::Started { .. } => {}
        AgentEvent::SandboxAttempt {
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
            ..
        } => {
            println!(
                "[sandbox] schema_version={schema_version} platform={platform} status={status} backend_id={backend_id} backend_label={backend_label} backend={backend} os_isolation={os_isolation} enforcement={enforcement} enforcement_level={enforcement_level} runner={runner} unsupported_reason={} cwd={cwd} command={} message={} (policy guard/process lifecycle unless enforcement=os_restricted)",
                unsupported_reason.as_deref().unwrap_or("none"),
                command.as_deref().unwrap_or("none"),
                message.as_deref().unwrap_or("none")
            );
        }
    }
    io::stdout().flush()?;
    Ok(())
}

fn respond_to_approval_request(
    request: AgentRunApprovalRequest,
    input: &mut dyn InteractiveInput,
) -> Result<()> {
    println!(
        "[approval] {} requires approval in {}",
        request.tool_name, request.cwd
    );
    if let Some(command) = &request.command {
        println!("[approval] command: {command}");
    }
    println!("[approval] reason: {}", request.reason);
    let line = input
        .read_response("approve? y/N: ")?
        .unwrap_or_default();
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

fn respond_to_user_input_request(
    request: AgentRunUserInputRequest,
    input: &mut dyn InteractiveInput,
) -> Result<()> {
    let value = input
        .read_response(&format!("{} ", request.prompt))?
        .unwrap_or_default();
    let _ = request.respond_to.send(AgentRunUserInputResponse {
        value: Some(value),
    });
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn render_agent_result(result: &AgentRunResult) -> Result<()> {
    let mut state = RenderState::default();

    for event in &result.events {
        render_agent_event(event, &mut state)?;
    }

    if !state.saw_assistant_message() {
        if let Some(final_response) = &result.final_response {
            println!("{}", state.render_assistant_content(final_response));
        }
    }
    io::stdout().flush()?;
    Ok(())
}

fn command_status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::InProgress => "in_progress",
        CommandStatus::Completed => "completed",
        CommandStatus::Failed => "failed",
        CommandStatus::Declined => "declined",
        CommandStatus::Cancelled => "cancelled",
    }
}
