use crate::streaming::append_fragment;
use yunxi_agent_core::{AgentEvent, AgentRunStatus, CommandStatus};

const MAX_HISTORY_CELLS: usize = 800;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HistoryCell {
    User(String),
    Assistant { content: String, active: bool },
    Reasoning { content: String, active: bool },
    Event { kind: String, message: String },
    Warning(String),
    Error(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Transcript {
    cells: Vec<HistoryCell>,
    last_assistant_content: Option<String>,
}

impl Transcript {
    pub(crate) fn clear(&mut self) {
        self.cells.clear();
        self.last_assistant_content = None;
    }

    pub(crate) fn cells(&self) -> &[HistoryCell] {
        &self.cells
    }

    pub(crate) fn push_user(&mut self, value: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::User(value.into()));
    }

    pub(crate) fn push_assistant(&mut self, value: &str) {
        let value = value.trim_matches('\r');
        if value.trim().is_empty() {
            return;
        }
        if self.last_assistant_content.as_deref() == Some(value) {
            return;
        }
        if let Some(HistoryCell::Assistant { content, active }) = self.cells.last_mut()
            && *active
        {
            if value.starts_with(content.as_str()) {
                *content = value.to_string();
            } else {
                append_fragment(content, value);
            }
            self.last_assistant_content = Some(content.clone());
            return;
        }
        self.finalize_reasoning();
        self.last_assistant_content = Some(value.to_string());
        self.push_cell(HistoryCell::Assistant {
            content: value.to_string(),
            active: true,
        });
    }

    pub(crate) fn push_reasoning(&mut self, value: &str) {
        let value = value.trim_matches('\r');
        if value.trim().is_empty() {
            return;
        }
        if let Some(HistoryCell::Reasoning { content, active }) = self.cells.last_mut()
            && *active
        {
            append_fragment(content, value);
            return;
        }
        self.finalize_assistant();
        self.push_cell(HistoryCell::Reasoning {
            content: value.to_string(),
            active: true,
        });
    }

    pub(crate) fn push_notice(&mut self, kind: impl Into<String>, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Event {
            kind: kind.into(),
            message: message.into(),
        });
    }

    pub(crate) fn push_warning(&mut self, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Warning(message.into()));
    }

    pub(crate) fn push_error(&mut self, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Error(message.into()));
    }

    pub(crate) fn push_agent_event(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::Message { content } => self.push_assistant(content),
            AgentEvent::Reasoning { content } => self.push_reasoning(content),
            AgentEvent::CommandStarted { command, .. } => {
                self.push_notice("shell", format!("started: {command}"));
            }
            AgentEvent::CommandUpdated {
                aggregated_output, ..
            } if !aggregated_output.trim().is_empty() => {
                self.push_notice("stdout", aggregated_output.trim());
            }
            AgentEvent::CommandCompleted {
                command,
                exit_code,
                status,
                ..
            } => self.push_notice(
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
                self.push_notice("shell", format!("{command} exited with {exit_code}"));
            }
            AgentEvent::ToolCallStarted { name, .. } => {
                self.push_notice("tool", format!("started: {name}"));
            }
            AgentEvent::ToolCallCompleted {
                name,
                output,
                status,
                ..
            } => {
                self.push_notice("tool", format!("{name} -> {}", command_status_label(*status)));
                if !output.trim().is_empty() {
                    self.push_notice("tool-output", output.trim());
                }
            }
            AgentEvent::McpToolStarted { server, tool, .. } => {
                self.push_notice("mcp", format!("{server}/{tool} started"));
            }
            AgentEvent::McpToolCompleted {
                server,
                tool,
                status,
                ..
            } => self.push_notice("mcp", format!("{server}/{tool} -> {status:?}")),
            AgentEvent::McpSession {
                server,
                status,
                message,
            } => self.push_notice(
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
            } => self.push_error(format!(
                "{provider} {classification}{} - {message}",
                status.map(|value| format!(" {value}")).unwrap_or_default()
            )),
            AgentEvent::Cancelled { reason } => self.push_notice(
                "cancelled",
                reason.as_deref().unwrap_or("current turn cancelled"),
            ),
            AgentEvent::Completed { status, usage } => {
                self.finalize_streams();
                if *status != AgentRunStatus::Completed {
                    self.push_notice("turn", status_label(*status));
                }
                if let Some(usage) = usage {
                    self.push_notice(
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
                self.push_notice("file", format!("{kind:?}: {path}"));
            }
            AgentEvent::PatchCompleted { status } => {
                self.push_notice("patch", format!("{status:?}"));
            }
            AgentEvent::TodoUpdated { id, items } => {
                self.push_notice(
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
            } => self.push_notice("approval", format!("{tool_name}: {reason}")),
            AgentEvent::ApprovalCompleted {
                approved, reason, ..
            } => self.push_notice(
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
            } => self.push_notice("escalation", format!("{tool_name}: {reason}")),
            AgentEvent::EscalationCompleted {
                approved, reason, ..
            } => self.push_notice(
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
            } => self.push_notice(
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
            } => self.push_notice(
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
            } => self.push_notice(
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
            } => self.push_notice(
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
            } => self.push_notice(
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

    fn finalize_streams(&mut self) {
        self.finalize_reasoning();
        self.finalize_assistant();
    }

    fn finalize_reasoning(&mut self) {
        if let Some(HistoryCell::Reasoning { active, .. }) = self.cells.last_mut() {
            *active = false;
        }
    }

    fn finalize_assistant(&mut self) {
        if let Some(HistoryCell::Assistant { active, .. }) = self.cells.last_mut() {
            *active = false;
        }
    }

    fn push_cell(&mut self, cell: HistoryCell) {
        self.cells.push(cell);
        if self.cells.len() > MAX_HISTORY_CELLS {
            let overflow = self.cells.len() - MAX_HISTORY_CELLS;
            self.cells.drain(0..overflow);
        }
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

    #[test]
    fn reasoning_deltas_are_merged_into_one_cell() {
        let mut transcript = Transcript::default();
        for delta in ["用户", "输入", "了", "\"", "测试", "\""] {
            transcript.push_reasoning(delta);
        }
        assert_eq!(transcript.cells().len(), 1);
        assert_eq!(
            transcript.cells()[0],
            HistoryCell::Reasoning {
                content: "用户输入了\"测试\"".to_string(),
                active: true
            }
        );
    }

    #[test]
    fn duplicate_assistant_messages_are_suppressed() {
        let mut transcript = Transcript::default();
        transcript.push_assistant("hello");
        transcript.push_assistant("hello");
        assert_eq!(transcript.cells().len(), 1);
    }
}
