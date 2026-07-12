use crate::render::InteractiveBanner;
use yunxi_agent_core::{AgentEvent, AgentRunStatus, CommandStatus};

const MAX_EVENT_LINES: usize = 500;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TuiEventLine {
    pub label: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TuiApp {
    pub version: String,
    pub cwd: String,
    pub backend: String,
    pub provider_mode: String,
    pub provider_source: String,
    pub provider: String,
    pub model: String,
    pub input: String,
    pub events: Vec<TuiEventLine>,
    pub scroll: u16,
}

impl TuiApp {
    pub(crate) fn from_banner(banner: &InteractiveBanner) -> Self {
        Self {
            version: "v1.7.0".to_string(),
            cwd: banner.cwd.clone(),
            backend: banner.backend.clone(),
            provider_mode: if banner.provider_live {
                "live".to_string()
            } else {
                "offline".to_string()
            },
            provider_source: banner.provider_source.clone(),
            provider: banner.provider.clone(),
            model: banner.model.clone(),
            input: "yunxi> ".to_string(),
            events: Vec::new(),
            scroll: 0,
        }
    }

    pub(crate) fn push_line(
        &mut self,
        label: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.events.push(TuiEventLine {
            label: label.into(),
            message: message.into(),
        });
        if self.events.len() > MAX_EVENT_LINES {
            let overflow = self.events.len() - MAX_EVENT_LINES;
            self.events.drain(0..overflow);
        }
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.push_line("warning", message);
    }

    pub(crate) fn push_error(&mut self, message: &str) {
        self.push_line("error", message);
    }

    pub(crate) fn push_event(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::Message { content } => self.push_line("assistant", content),
            AgentEvent::Reasoning { content } => self.push_line("reasoning", content),
            AgentEvent::CommandStarted { command, .. } => {
                self.push_line("shell", format!("started: {command}"));
            }
            AgentEvent::CommandUpdated {
                aggregated_output, ..
            } if !aggregated_output.trim().is_empty() => {
                self.push_line("stdout", aggregated_output.trim());
            }
            AgentEvent::CommandCompleted {
                command,
                exit_code,
                status,
                ..
            } => self.push_line(
                "shell",
                format!(
                    "{command} -> {}{}",
                    command_status_label(*status),
                    exit_code
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::CommandFinished { command, exit_code } => {
                self.push_line("shell", format!("{command} exited with {exit_code}"));
            }
            AgentEvent::ToolCallStarted { name, .. } => {
                self.push_line("tool", format!("started: {name}"));
            }
            AgentEvent::ToolCallCompleted {
                name,
                output,
                status,
                ..
            } => {
                self.push_line("tool", format!("{name} -> {}", command_status_label(*status)));
                if !output.trim().is_empty() {
                    self.push_line("tool-output", output.trim());
                }
            }
            AgentEvent::McpToolStarted { server, tool, .. } => {
                self.push_line("mcp", format!("{server}/{tool} started"));
            }
            AgentEvent::McpToolCompleted {
                server,
                tool,
                status,
                ..
            } => self.push_line("mcp", format!("{server}/{tool} -> {status:?}")),
            AgentEvent::McpSession {
                server,
                status,
                message,
            } => self.push_line(
                "mcp-session",
                format!(
                    "{server}: {status}{}",
                    message
                        .as_ref()
                        .map(|value| format!(" - {value}"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::Warning { message } => self.push_warning(message),
            AgentEvent::Error { message } => self.push_error(message),
            AgentEvent::ProviderError {
                provider,
                classification,
                status,
                message,
            } => self.push_line(
                "provider",
                format!(
                    "{provider} {classification}{} - {message}",
                    status.map(|value| format!(" {value}")).unwrap_or_default()
                ),
            ),
            AgentEvent::Cancelled { reason } => self.push_line(
                "cancelled",
                reason.as_deref().unwrap_or("current turn cancelled"),
            ),
            AgentEvent::Completed { status, usage } => {
                self.push_line("turn", status_label(*status));
                if let Some(usage) = usage {
                    self.push_line(
                        "usage",
                        format!(
                            "input={} cached_input={} output={} reasoning_output={}",
                            usage.input_tokens,
                            usage.cached_input_tokens,
                            usage.output_tokens,
                            usage.reasoning_output_tokens
                        ),
                    );
                }
            }
            AgentEvent::FileChanged { path, kind } => {
                self.push_line("file", format!("{kind:?}: {path}"));
            }
            AgentEvent::PatchCompleted { status } => {
                self.push_line("patch", format!("{status:?}"));
            }
            AgentEvent::TodoUpdated { id, items } => {
                self.push_line(
                    "todo",
                    format!(
                        "{} item(s){}",
                        items.len(),
                        id.as_ref()
                            .map(|value| format!(" id={value}"))
                            .unwrap_or_default()
                    ),
                );
            }
            AgentEvent::ApprovalRequested {
                tool_name, reason, ..
            } => self.push_line("approval", format!("{tool_name}: {reason}")),
            AgentEvent::ApprovalCompleted {
                approved, reason, ..
            } => self.push_line(
                "approval",
                format!(
                    "{}{}",
                    if *approved { "approved" } else { "declined" },
                    reason
                        .as_ref()
                        .map(|value| format!(" - {value}"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::EscalationRequested {
                tool_name, reason, ..
            } => self.push_line("escalation", format!("{tool_name}: {reason}")),
            AgentEvent::EscalationCompleted {
                approved, reason, ..
            } => self.push_line(
                "escalation",
                format!(
                    "{}{}",
                    if *approved { "approved" } else { "declined" },
                    reason
                        .as_ref()
                        .map(|value| format!(" - {value}"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::ChildAgentEvent {
                agent_id,
                child_session_id,
                status,
                message,
                ..
            } => self.push_line(
                "child",
                format!(
                    "{agent_id} {status} session={child_session_id}{}",
                    message
                        .as_ref()
                        .map(|value| format!(" - {value}"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::ChildScopedStream {
                agent_id,
                event,
                seq,
                message,
                ..
            } => self.push_line(
                "child-stream",
                format!(
                    "{agent_id} #{seq} {event}{}",
                    message
                        .as_ref()
                        .map(|value| format!(" - {value}"))
                        .unwrap_or_default()
                ),
            ),
            AgentEvent::SandboxAttempt {
                platform,
                status,
                backend,
                command,
                ..
            } => self.push_line(
                "policy",
                format!(
                    "platform={platform} status={status} backend={backend} command={}",
                    command.as_deref().unwrap_or("none")
                ),
            ),
            AgentEvent::ContextStatus {
                active_context_tokens,
                token_limit_reached,
                compacted,
                dropped_messages,
            } => self.push_line(
                "context",
                format!(
                    "tokens={active_context_tokens} limit_reached={token_limit_reached} compacted={compacted} dropped={dropped_messages}"
                ),
            ),
            AgentEvent::StorageState {
                session_id,
                rollout_items,
                rollout_truncated,
                child_session_ids,
                ..
            } => self.push_line(
                "session",
                format!(
                    "{} rollout_items={rollout_items} truncated={rollout_truncated} children={}",
                    session_id.as_deref().unwrap_or("unknown"),
                    child_session_ids.len()
                ),
            ),
            AgentEvent::CommandUpdated { .. }
            | AgentEvent::ThreadStarted { .. }
            | AgentEvent::TurnStarted
            | AgentEvent::ThreadState { .. }
            | AgentEvent::TurnMetadata { .. }
            | AgentEvent::TurnState { .. }
            | AgentEvent::DeepParityState { .. }
            | AgentEvent::ApprovalCacheState { .. }
            | AgentEvent::MultiAgentEvent { .. }
            | AgentEvent::Started { .. } => {}
        }
    }

    pub(crate) fn header(&self) -> String {
        format!(
            "YunXi Agent {} | {} | provider={} mode={} model={}",
            self.version, self.cwd, self.provider, self.provider_mode, self.model
        )
    }

    pub(crate) fn subheader(&self) -> String {
        format!(
            "backend={} source={}",
            self.backend, self.provider_source
        )
    }
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

fn status_label(status: AgentRunStatus) -> &'static str {
    match status {
        AgentRunStatus::Completed => "completed",
        AgentRunStatus::Failed => "failed",
        AgentRunStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn banner() -> InteractiveBanner {
        InteractiveBanner {
            cwd: "D:/YunXi Agent".to_string(),
            backend: "yunxi".to_string(),
            provider_live: true,
            provider_source: "auto_live".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
        }
    }

    #[test]
    fn app_maps_events_to_compact_lines() {
        let mut app = TuiApp::from_banner(&banner());

        app.push_event(&AgentEvent::Reasoning {
            content: "Provider turn started".to_string(),
        });
        app.push_event(&AgentEvent::Message {
            content: "hello".to_string(),
        });

        assert!(app.header().contains("YunXi Agent v1.7.0"));
        assert_eq!(app.events[0].label, "reasoning");
        assert_eq!(app.events[1].label, "assistant");
    }
}
