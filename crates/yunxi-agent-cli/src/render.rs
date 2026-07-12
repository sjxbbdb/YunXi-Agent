use anyhow::Result;
use std::io::{self, Write};
use yunxi_agent_core::{AgentEvent, AgentRunResult, AgentRunStatus, CommandStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractiveBanner {
    pub cwd: String,
    pub backend: String,
    pub provider_live: bool,
    pub provider_source: String,
    pub model: String,
    pub provider: String,
}

pub(crate) fn print_banner(banner: &InteractiveBanner) {
    println!("YunXi Agent v1.5.0 interactive CLI");
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
}

pub(crate) fn render_agent_event(event: &AgentEvent, state: &mut RenderState) -> Result<()> {
    match event {
        AgentEvent::Message { content } => {
            state.saw_assistant_message = true;
            let rendered = state.render_assistant_content(content);
            if state.last_assistant_message.as_deref() != Some(rendered.as_str()) {
                println!("{rendered}");
                state.last_assistant_message = Some(rendered);
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
            platform,
            status,
            backend,
            command,
            cwd,
            message,
            ..
        } => {
            println!(
                "[policy-guard] platform={platform} status={status} backend={backend} cwd={cwd} command={} message={} (advisory only, no OS isolation)",
                command.as_deref().unwrap_or("none"),
                message.as_deref().unwrap_or("none")
            );
        }
    }
    io::stdout().flush()?;
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
