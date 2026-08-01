use crate::error_presentation::{ErrorCategory, ErrorPresentation, classify_error};
use crate::output_summary::{OutputSummary, redact_secrets, truncate_chars};
use crate::timeline::{ToolPhase, ToolTimelineUpdate, phase_from_command_status, status_label};
use std::fmt;
use yunxi_agent_core::{
    AgentEvent, AgentMessageSequence, AgentMessageStream, AgentMessageStreamPhase, AgentRunStatus,
    McpToolStatus,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TuiCellId(String);

impl TuiCellId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn with_suffix(&self, suffix: &str) -> Self {
        Self(format!("{}:{suffix}", self.0))
    }

    fn assistant_for_turn(turn_id: &str) -> Self {
        Self(format!("assistant:{:016x}", stable_hash(turn_id)))
    }

    #[cfg(test)]
    pub(crate) fn from_test(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for TuiCellId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiCellKind {
    UserMessage,
    AssistantMessage,
    ProgressSummary,
    ToolStatusSummary,
    ApprovalRequest,
    Notice,
    ErrorSummary,
    DebugDetail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationDetail {
    pub id: TuiCellId,
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiStreamState {
    pub stable_source: String,
    pub live_tail: String,
    pub committed: bool,
    pub identity: TuiStreamIdentity,
    pub offline_label: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiStreamIdentity {
    pub thread_id: String,
    pub turn_id: String,
    pub stream_id: String,
    pub event_id: String,
    pub source_sequence: TuiSourceSequence,
    pub phase: TuiStreamPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiSourceSequence {
    ProviderReliable(u64),
    LocalFallback(u64),
}

impl TuiSourceSequence {
    pub(crate) fn value(self) -> u64 {
        match self {
            Self::ProviderReliable(value) | Self::LocalFallback(value) => value,
        }
    }

    pub(crate) fn reliable_value(self) -> Option<u64> {
        match self {
            Self::ProviderReliable(value) => Some(value),
            Self::LocalFallback(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiStreamPhase {
    Started,
    Delta,
    Retry,
    Final,
    Finish,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PresentationVisibility {
    Transcript,
    DebugOnly,
    Hidden,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiEvent {
    pub id: TuiCellId,
    pub kind: TuiCellKind,
    pub visible_text: String,
    pub detail: Option<PresentationDetail>,
    pub stream: Option<TuiStreamState>,
    pub(crate) visibility: PresentationVisibility,
    pub(crate) tool_update: Option<ToolTimelineUpdate>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TuiPresentation {
    next_cell_sequence: u64,
    next_local_turn_sequence: u64,
    next_local_source_sequence: u64,
    current_local_turn_id: Option<String>,
    active_stream: Option<TuiStreamIdentity>,
    offline_label: bool,
}

impl TuiPresentation {
    pub(crate) fn set_offline_label(&mut self, enabled: bool) {
        self.offline_label = enabled;
    }

    pub(crate) fn present_agent_event(&mut self, event: &AgentEvent) -> TuiEvent {
        if matches!(event, AgentEvent::TurnStarted) {
            self.begin_local_turn();
        }

        let mut presented = match event {
            AgentEvent::Message { content, stream } => {
                return self.present_assistant_message(content, stream.as_ref());
            }
            AgentEvent::Reasoning { content } => {
                self.debug_only("reasoning", content.clone(), "reasoning")
            }
            AgentEvent::CommandStarted { id, command } => self.tool_event(
                id.as_deref(),
                "shell",
                ToolPhase::Running,
                TuiCellKind::ToolStatusSummary,
                Some(("shell command", command.clone())),
            ),
            AgentEvent::CommandUpdated {
                id,
                command,
                aggregated_output,
            } => {
                let mut event = self.tool_event(
                    id.as_deref(),
                    "shell",
                    ToolPhase::Running,
                    TuiCellKind::ToolStatusSummary,
                    Some((
                        "command update",
                        format!("command={command}\n{}", redact_secrets(aggregated_output)),
                    )),
                );
                if let Some(summary) = quiet_output_summary(aggregated_output)
                    && let Some(update) = event.tool_update.take()
                {
                    event.tool_update = Some(update.output_summary(summary.visible));
                }
                event.visibility = PresentationVisibility::DebugOnly;
                event.visible_text.clear();
                event
            }
            AgentEvent::CommandCompleted {
                id,
                command,
                aggregated_output,
                exit_code,
                status,
                execution_details,
            } => {
                let mut detail = format!(
                    "command={command}\nstatus={} exit_code={}",
                    status_label(*status),
                    exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string())
                );
                if let Some(execution_details) = execution_details {
                    detail.push_str("\nexecution_details=");
                    detail.push_str(&format_execution_details(execution_details));
                }
                if !aggregated_output.is_empty() {
                    detail.push_str("\noutput=\n");
                    detail.push_str(aggregated_output);
                }
                let mut event = self.tool_event(
                    id.as_deref(),
                    "shell",
                    phase_from_command_status(*status),
                    TuiCellKind::ToolStatusSummary,
                    Some(("shell result", detail)),
                );
                if let Some(summary) = quiet_output_summary(aggregated_output)
                    && let Some(update) = event.tool_update.take()
                {
                    event.tool_update = Some(update.output_summary(summary.visible));
                }
                match status {
                    yunxi_agent_core::CommandStatus::Failed => apply_tool_error(
                        &mut event,
                        ErrorCategory::Tool,
                        "command returned a non-zero status",
                    ),
                    yunxi_agent_core::CommandStatus::Declined => apply_tool_error(
                        &mut event,
                        ErrorCategory::Approval,
                        "command request was declined",
                    ),
                    yunxi_agent_core::CommandStatus::Cancelled => apply_tool_error(
                        &mut event,
                        ErrorCategory::Cancel,
                        "command request was cancelled",
                    ),
                    yunxi_agent_core::CommandStatus::InProgress
                    | yunxi_agent_core::CommandStatus::Completed => {}
                }
                event
            }
            AgentEvent::CommandFinished { command, exit_code } => {
                let mut event = self.tool_event(
                    None,
                    "shell",
                    command_exit_phase(*exit_code),
                    TuiCellKind::ToolStatusSummary,
                    Some((
                        "shell result",
                        format!("command={command}\nexit_code={exit_code}"),
                    )),
                );
                if *exit_code != 0 {
                    apply_tool_error(
                        &mut event,
                        ErrorCategory::Tool,
                        "command returned a non-zero status",
                    );
                }
                event
            }
            AgentEvent::ToolCallStarted {
                id,
                name,
                arguments_json,
            } => self.tool_event(
                id.as_deref(),
                &display_tool_name(name),
                ToolPhase::Running,
                TuiCellKind::ToolStatusSummary,
                arguments_json
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| ("tool arguments", value.clone())),
            ),
            AgentEvent::ToolCallCompleted {
                id,
                name,
                output,
                status,
            } => {
                let mut event = self.tool_event(
                    id.as_deref(),
                    &display_tool_name(name),
                    phase_from_command_status(*status),
                    TuiCellKind::ToolStatusSummary,
                    (!output.trim().is_empty()).then(|| ("tool output", output.clone())),
                );
                if let Some(summary) = quiet_output_summary(output)
                    && let Some(update) = event.tool_update.take()
                {
                    event.tool_update = Some(update.output_summary(summary.visible));
                }
                match status {
                    yunxi_agent_core::CommandStatus::Failed => {
                        apply_tool_error(&mut event, ErrorCategory::Tool, "tool returned a failure")
                    }
                    yunxi_agent_core::CommandStatus::Declined => apply_tool_error(
                        &mut event,
                        ErrorCategory::Approval,
                        "tool request was declined",
                    ),
                    yunxi_agent_core::CommandStatus::Cancelled => apply_tool_error(
                        &mut event,
                        ErrorCategory::Cancel,
                        "tool request was cancelled",
                    ),
                    yunxi_agent_core::CommandStatus::InProgress
                    | yunxi_agent_core::CommandStatus::Completed => {}
                }
                event
            }
            AgentEvent::McpToolStarted { id, server, tool } => self.tool_event(
                id.as_deref(),
                &format!(
                    "mcp {}/{}",
                    safe_identifier(server),
                    safe_identifier(tool)
                ),
                ToolPhase::Running,
                TuiCellKind::ToolStatusSummary,
                None,
            ),
            AgentEvent::McpToolCompleted {
                id,
                server,
                tool,
                status,
            } => {
                let mut event = self.tool_event(
                    id.as_deref(),
                    &format!(
                        "mcp {}/{}",
                        safe_identifier(server),
                        safe_identifier(tool)
                    ),
                    mcp_status_phase(*status),
                    TuiCellKind::ToolStatusSummary,
                    None,
                );
                if matches!(status, McpToolStatus::Failed) {
                    apply_tool_error(
                        &mut event,
                        ErrorCategory::Tool,
                        "MCP tool execution failed",
                    );
                }
                event
            }
            AgentEvent::ApprovalRequested {
                id,
                tool_name,
                reason,
            } => {
                let mut event = self.tool_event(
                    id.as_deref(),
                    &display_tool_name(tool_name),
                    ToolPhase::ApprovalRequired,
                    TuiCellKind::ApprovalRequest,
                    Some(("approval request", reason.clone())),
                );
                if let Some(update) = event.tool_update.take() {
                    event.tool_update = Some(update.approval("user decision required"));
                }
                event
            }
            AgentEvent::ApprovalCompleted {
                id,
                approved,
                reason,
            } => {
                let phase = approval_phase(*approved, reason.as_deref());
                let mut event = self.tool_event(
                    id.as_deref(),
                    "approval",
                    phase,
                    TuiCellKind::ToolStatusSummary,
                    reason
                        .as_ref()
                        .map(|value| ("approval decision", value.clone())),
                );
                if let Some(update) = event.tool_update.take() {
                    event.tool_update = Some(update.approval(approval_label(*approved)));
                }
                if !approved {
                    let category = if phase == ToolPhase::Cancelled {
                        ErrorCategory::Cancel
                    } else {
                        ErrorCategory::Approval
                    };
                    apply_tool_error(
                        &mut event,
                        category,
                        if phase == ToolPhase::PolicyDeclined {
                            "request was declined by policy"
                        } else if phase == ToolPhase::Cancelled {
                            "approval was cancelled"
                        } else {
                            "request was declined by the user"
                        },
                    );
                }
                event
            }
            AgentEvent::EscalationRequested {
                id,
                tool_name,
                reason,
                required_sandbox,
                required_network,
            } => {
                let detail = format!(
                    "reason={reason}\nrequired_sandbox={}\nrequired_network={}",
                    required_sandbox.as_deref().unwrap_or("none"),
                    required_network.as_deref().unwrap_or("none")
                );
                let mut event = self.tool_event(
                    id.as_deref(),
                    &display_tool_name(tool_name),
                    ToolPhase::ApprovalRequired,
                    TuiCellKind::ApprovalRequest,
                    Some(("escalation request", detail)),
                );
                if let Some(update) = event.tool_update.take() {
                    event.tool_update = Some(update.approval("additional permission required"));
                }
                event
            }
            AgentEvent::EscalationCompleted {
                id,
                approved,
                reason,
            } => {
                let phase = approval_phase(*approved, reason.as_deref());
                let mut event = self.tool_event(
                    id.as_deref(),
                    "escalation",
                    phase,
                    TuiCellKind::ToolStatusSummary,
                    reason
                        .as_ref()
                        .map(|value| ("escalation decision", value.clone())),
                );
                if let Some(update) = event.tool_update.take() {
                    event.tool_update = Some(update.approval(approval_label(*approved)));
                }
                if !approved {
                    let category = if phase == ToolPhase::Cancelled {
                        ErrorCategory::Cancel
                    } else {
                        ErrorCategory::Approval
                    };
                    apply_tool_error(
                        &mut event,
                        category,
                        "escalation request was not approved",
                    );
                }
                event
            }
            AgentEvent::SandboxAttempt { status, .. } => {
                if is_visible_sandbox_status(status) {
                    self.present_error_event(
                        ErrorCategory::Approval,
                        "policy action requires attention",
                        "sandbox attempt",
                        format!("{event:?}"),
                        "policy",
                    )
                } else {
                    self.debug_only("sandbox attempt", format!("{event:?}"), "sandbox")
                }
            }
            AgentEvent::McpSession {
                server,
                status,
                message,
            } => {
                let detail = format!(
                    "server={server} status={status} message={}",
                    message.as_deref().unwrap_or("none")
                );
                if status.eq_ignore_ascii_case("failed")
                    || status.eq_ignore_ascii_case("error")
                {
                    self.present_error_event(
                        ErrorCategory::Tool,
                        format!("MCP session {} failed", safe_identifier(server)),
                        "mcp session",
                        detail,
                        "mcp-session",
                    )
                } else {
                    self.debug_only("mcp session", detail, "mcp-session")
                }
            }
            AgentEvent::Warning { message } if is_tool_warning(message) => {
                self.debug_only("runtime warning", message.clone(), "runtime-warning")
            }
            AgentEvent::Warning { message } => self.visible_with_detail(
                TuiCellKind::Notice,
                "warning",
                safe_message_summary("runtime warning", message),
                "runtime warning",
                message.clone(),
            ),
            AgentEvent::MemoryWarning { warning, .. } => {
                self.debug_only("memory warning", warning.clone(), "memory-warning")
            }
            AgentEvent::Error { message } => {
                let category = classify_error(message);
                let context = if matches!(category, ErrorCategory::Tool) {
                    tool_error_context(message)
                } else {
                    "agent operation failed".to_string()
                };
                self.present_error_event(
                    category,
                    context,
                    "agent error",
                    message.clone(),
                    "error",
                )
            }
            AgentEvent::ProviderError {
                provider,
                classification,
                status,
                message,
            } => self.present_error_event(
                ErrorCategory::Provider,
                format!(
                    "provider {} failed ({classification}{})",
                    safe_identifier(provider),
                    status.map(|value| format!("/{value}")).unwrap_or_default()
                ),
                "provider error",
                message.clone(),
                "provider",
            ),
            AgentEvent::Cancelled { reason } => self.present_error_event(
                ErrorCategory::Cancel,
                "current turn cancelled",
                "cancellation",
                reason
                    .clone()
                    .unwrap_or_else(|| "current turn cancelled".to_string()),
                "cancelled",
            ),
            AgentEvent::Completed { status, usage } => {
                if *status != AgentRunStatus::Completed {
                    self.present_error_event(
                        if *status == AgentRunStatus::Cancelled {
                            ErrorCategory::Cancel
                        } else {
                            ErrorCategory::Unknown
                        },
                        format!("turn {}", format!("{status:?}").to_ascii_lowercase()),
                        "turn completion",
                        format!("status={status:?} usage={usage:?}"),
                        "turn",
                    )
                } else if usage.is_some() {
                    self.debug_only("usage", format!("{usage:?}"), "usage")
                } else {
                    self.hidden("turn-completed")
                }
            }
            AgentEvent::FileChanged { path, kind } => self.visible_with_detail(
                TuiCellKind::ProgressSummary,
                "file",
                format!("file change: {kind:?}"),
                "file change",
                format!("kind={kind:?} path={path}"),
            ),
            AgentEvent::PatchCompleted { status } => self.simple_visible(
                TuiCellKind::ProgressSummary,
                "patch",
                format!("patch {}", format!("{status:?}").to_ascii_lowercase()),
            ),
            AgentEvent::TodoUpdated { id, items } => self.visible_with_detail(
                TuiCellKind::ProgressSummary,
                "todo",
                format!("todo updated: {} item(s)", items.len()),
                "todo update",
                format!("id={id:?} items={items:?}"),
            ),
            AgentEvent::ChildAgentEvent {
                agent_id,
                child_session_id,
                status,
                message,
                ..
            } => self.visible_with_detail(
                TuiCellKind::ProgressSummary,
                "child-agent",
                format!(
                    "child agent {} {}",
                    safe_identifier(agent_id),
                    safe_identifier(status)
                ),
                "child agent event",
                format!(
                    "agent_id={agent_id} child_session_id={child_session_id} status={status} message={message:?}"
                ),
            ),
            AgentEvent::ChildScopedStream { .. } => {
                self.debug_only("child stream", format!("{event:?}"), "child-stream")
            }
            AgentEvent::PersonaLoaded { .. }
            | AgentEvent::PersonaContextInjected { .. }
            | AgentEvent::MemoryRecall { .. }
            | AgentEvent::MemoryCandidate { .. }
            | AgentEvent::MemoryWrite { .. }
            | AgentEvent::ContextStatus { .. }
            | AgentEvent::StorageState { .. }
            | AgentEvent::ThreadStarted { .. }
            | AgentEvent::TurnStarted
            | AgentEvent::ThreadState { .. }
            | AgentEvent::TurnMetadata { .. }
            | AgentEvent::TurnState { .. }
            | AgentEvent::DeepParityState { .. }
            | AgentEvent::ApprovalCacheState { .. }
            | AgentEvent::MultiAgentEvent { .. }
            | AgentEvent::Started { .. } => {
                self.debug_only(debug_label(event), format!("{event:?}"), debug_label(event))
            }
        };

        let control_phase = match event {
            AgentEvent::Cancelled { .. }
            | AgentEvent::ProviderError { .. }
            | AgentEvent::Error { .. } => Some(TuiStreamPhase::Cancel),
            AgentEvent::Completed { .. } => Some(TuiStreamPhase::Finish),
            _ => None,
        };
        if let Some(phase) = control_phase {
            presented.stream = self.stream_control(phase);
        }

        presented
    }

    pub(crate) fn present_user(&mut self, value: impl Into<String>) -> TuiEvent {
        self.begin_local_turn();
        self.simple_visible(TuiCellKind::UserMessage, "user", value.into())
    }

    pub(crate) fn present_notice(&mut self, kind: &str, message: &str) -> TuiEvent {
        self.simple_visible(TuiCellKind::Notice, kind, redact_secrets(message))
    }

    pub(crate) fn present_warning(&mut self, message: &str) -> TuiEvent {
        self.visible_with_detail(
            TuiCellKind::Notice,
            "warning",
            safe_message_summary("warning", message),
            "warning",
            message.to_string(),
        )
    }

    pub(crate) fn present_error(&mut self, message: &str) -> TuiEvent {
        let category = classify_error(message);
        self.present_error_event(
            category,
            safe_error_context(category),
            "error",
            message.to_string(),
            "error",
        )
    }

    fn present_error_event(
        &mut self,
        category: ErrorCategory,
        context: impl Into<String>,
        detail_label: &str,
        detail_content: String,
        prefix: &str,
    ) -> TuiEvent {
        let id = self.next_id(prefix);
        let detail = self.presentation_detail(&id, detail_label, detail_content);
        let error = ErrorPresentation::for_category(category, context)
            .with_detail_ref(detail.id.to_string());
        TuiEvent {
            id,
            kind: TuiCellKind::ErrorSummary,
            visible_text: error.display_summary(),
            detail: Some(detail),
            stream: None,
            visibility: PresentationVisibility::Transcript,
            tool_update: None,
        }
    }

    fn present_assistant_message(
        &mut self,
        content: &str,
        source: Option<&AgentMessageStream>,
    ) -> TuiEvent {
        let mut identity = source
            .map(stream_identity_from_core)
            .unwrap_or_else(|| self.next_legacy_stream_identity());
        if identity.phase == TuiStreamPhase::Started
            && self.active_stream.as_ref().is_some_and(|active| {
                active.turn_id == identity.turn_id && active.stream_id != identity.stream_id
            })
        {
            identity.phase = TuiStreamPhase::Retry;
        }
        let id = TuiCellId::assistant_for_turn(&identity.turn_id);
        self.next_local_source_sequence = self
            .next_local_source_sequence
            .max(identity.source_sequence.value());
        self.active_stream = Some(identity.clone());
        TuiEvent {
            id,
            kind: TuiCellKind::AssistantMessage,
            visible_text: content.to_string(),
            detail: None,
            stream: Some(TuiStreamState {
                stable_source: String::new(),
                live_tail: String::new(),
                committed: false,
                identity,
                offline_label: self.offline_label,
            }),
            visibility: if content.is_empty() {
                PresentationVisibility::Hidden
            } else {
                PresentationVisibility::Transcript
            },
            tool_update: None,
        }
    }

    fn tool_event(
        &mut self,
        external_id: Option<&str>,
        name: &str,
        phase: ToolPhase,
        kind: TuiCellKind,
        detail: Option<(&str, String)>,
    ) -> TuiEvent {
        let safe_name = safe_identifier(name);
        let id = external_id
            .map(|value| TuiCellId(format!("tool:{:016x}", stable_hash(value))))
            .unwrap_or_else(|| self.next_id("tool"));
        let detail = detail.map(|(label, content)| self.presentation_detail(&id, label, content));
        TuiEvent {
            id,
            kind,
            visible_text: format!("{safe_name}: {}", phase.label()),
            detail,
            stream: None,
            visibility: PresentationVisibility::Transcript,
            tool_update: Some(ToolTimelineUpdate::new(
                external_id.map(ToOwned::to_owned),
                safe_name,
                phase,
            )),
        }
    }

    fn visible_with_detail(
        &mut self,
        kind: TuiCellKind,
        prefix: &str,
        visible_text: String,
        detail_label: &str,
        detail_content: String,
    ) -> TuiEvent {
        let id = self.next_id(prefix);
        let detail = Some(self.presentation_detail(&id, detail_label, detail_content));
        TuiEvent {
            id,
            kind,
            visible_text,
            detail,
            stream: None,
            visibility: PresentationVisibility::Transcript,
            tool_update: None,
        }
    }

    fn simple_visible(
        &mut self,
        kind: TuiCellKind,
        prefix: &str,
        visible_text: String,
    ) -> TuiEvent {
        TuiEvent {
            id: self.next_id(prefix),
            kind,
            visible_text,
            detail: None,
            stream: None,
            visibility: PresentationVisibility::Transcript,
            tool_update: None,
        }
    }

    fn debug_only(&mut self, label: &str, content: String, prefix: &str) -> TuiEvent {
        self.debug_only_with_external_id(label, content, prefix, None)
    }

    fn debug_only_with_external_id(
        &mut self,
        label: &str,
        content: String,
        prefix: &str,
        external_id: Option<&str>,
    ) -> TuiEvent {
        let id = external_id
            .map(|value| TuiCellId(format!("{prefix}:{:016x}", stable_hash(value))))
            .unwrap_or_else(|| self.next_id(prefix));
        let detail = Some(self.presentation_detail(&id, label, content));
        TuiEvent {
            id,
            kind: TuiCellKind::DebugDetail,
            visible_text: String::new(),
            detail,
            stream: None,
            visibility: PresentationVisibility::DebugOnly,
            tool_update: None,
        }
    }

    fn hidden(&mut self, prefix: &str) -> TuiEvent {
        TuiEvent {
            id: self.next_id(prefix),
            kind: TuiCellKind::Notice,
            visible_text: String::new(),
            detail: None,
            stream: None,
            visibility: PresentationVisibility::Hidden,
            tool_update: None,
        }
    }

    fn presentation_detail(
        &self,
        cell_id: &TuiCellId,
        label: &str,
        content: String,
    ) -> PresentationDetail {
        PresentationDetail {
            id: TuiCellId(format!(
                "detail:{}:{:016x}",
                cell_id.as_str(),
                stable_hash(label)
            )),
            label: label.to_string(),
            content,
        }
    }

    fn next_id(&mut self, prefix: &str) -> TuiCellId {
        self.next_cell_sequence = self.next_cell_sequence.saturating_add(1);
        TuiCellId(format!("{prefix}:{:016x}", self.next_cell_sequence))
    }

    fn begin_local_turn(&mut self) {
        self.next_local_turn_sequence = self.next_local_turn_sequence.saturating_add(1);
        self.current_local_turn_id =
            Some(format!("local-turn-{:016x}", self.next_local_turn_sequence));
        self.active_stream = None;
    }

    fn next_legacy_stream_identity(&mut self) -> TuiStreamIdentity {
        let turn_id = self.current_local_turn_id.clone().unwrap_or_else(|| {
            self.begin_local_turn();
            self.current_local_turn_id
                .clone()
                .expect("local turn initialized")
        });
        self.next_local_source_sequence = self.next_local_source_sequence.saturating_add(1);
        let stream_id = self
            .active_stream
            .as_ref()
            .filter(|active| active.turn_id == turn_id)
            .map(|active| active.stream_id.clone())
            .unwrap_or_else(|| format!("{turn_id}:assistant"));
        TuiStreamIdentity {
            thread_id: "local".to_string(),
            event_id: format!(
                "fallback:local:{}:{}:{}",
                turn_id, stream_id, self.next_local_source_sequence
            ),
            turn_id,
            stream_id,
            source_sequence: TuiSourceSequence::LocalFallback(self.next_local_source_sequence),
            phase: TuiStreamPhase::Delta,
        }
    }

    fn stream_control(&mut self, phase: TuiStreamPhase) -> Option<TuiStreamState> {
        let mut identity = self.active_stream.clone()?;
        self.next_local_source_sequence = self
            .next_local_source_sequence
            .max(identity.source_sequence.value())
            .saturating_add(1);
        identity.event_id = format!(
            "fallback:local:{}:{}:{:?}:{}",
            identity.turn_id, identity.stream_id, phase, self.next_local_source_sequence
        );
        identity.source_sequence =
            TuiSourceSequence::LocalFallback(self.next_local_source_sequence);
        identity.phase = phase;
        if matches!(phase, TuiStreamPhase::Cancel | TuiStreamPhase::Finish) {
            self.active_stream = None;
        }
        Some(TuiStreamState {
            stable_source: String::new(),
            live_tail: String::new(),
            committed: false,
            identity,
            offline_label: self.offline_label,
        })
    }
}

fn stream_identity_from_core(stream: &AgentMessageStream) -> TuiStreamIdentity {
    TuiStreamIdentity {
        thread_id: stream.thread_id.clone(),
        turn_id: stream.turn_id.clone(),
        stream_id: stream.stream_id.clone(),
        event_id: stream.event_id.clone(),
        source_sequence: match stream.source_sequence {
            AgentMessageSequence::ProviderReliable(value) => {
                TuiSourceSequence::ProviderReliable(value)
            }
            AgentMessageSequence::LocalFallback(value) => TuiSourceSequence::LocalFallback(value),
        },
        phase: match stream.phase {
            AgentMessageStreamPhase::Started => TuiStreamPhase::Started,
            AgentMessageStreamPhase::Delta => TuiStreamPhase::Delta,
            AgentMessageStreamPhase::Final => TuiStreamPhase::Final,
        },
    }
}

fn quiet_output_summary(output: &str) -> Option<OutputSummary> {
    let detail = redact_secrets(output.trim());
    if detail.is_empty() {
        return None;
    }
    let line_count = detail.lines().count().max(1);
    let char_count = detail.chars().count();
    Some(OutputSummary {
        visible: format!("output captured: {line_count} line(s), {char_count} char(s)"),
        detail,
        line_count,
        char_count,
        hidden: true,
    })
}

fn format_execution_details(details: &yunxi_agent_core::CommandExecutionDetails) -> String {
    fn format_stream(stream: &yunxi_agent_core::DecodedExecOutput) -> String {
        format!(
            "original_bytes={} displayed_bytes={} replacement_count={} truncated={} integrity={:?} display={}",
            stream.original_bytes,
            stream.displayed_bytes,
            stream.replacement_count,
            stream.truncated,
            stream.integrity,
            redact_secrets(&stream.display_text),
        )
    }

    format!(
        "duration_millis={:?} timed_out={} stdout{{{}}} stderr{{{}}}",
        details.duration_millis,
        details.timed_out,
        format_stream(&details.stdout),
        format_stream(&details.stderr),
    )
}

fn is_tool_warning(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    lowered.contains("tool")
        || lowered.contains("command")
        || lowered.contains("exit code")
        || lowered.contains("approval")
        || lowered.contains("declined")
        || lowered.contains("cancel")
}

fn apply_tool_error(event: &mut TuiEvent, category: ErrorCategory, context: &str) {
    let mut error = ErrorPresentation::for_category(category, context);
    if let Some(detail) = event.detail.as_ref() {
        error = error.with_detail_ref(detail.id.to_string());
    }
    if let Some(update) = event.tool_update.take() {
        event.tool_update = Some(update.output_summary(error.display_summary()));
    }
}

fn approval_phase(approved: bool, reason: Option<&str>) -> ToolPhase {
    if approved {
        return ToolPhase::Approved;
    }
    let lowered = reason.unwrap_or_default().to_ascii_lowercase();
    if lowered.contains("policy") {
        ToolPhase::PolicyDeclined
    } else if lowered.contains("cancel") || lowered.contains("ctrl+c") {
        ToolPhase::Cancelled
    } else {
        ToolPhase::Declined
    }
}

fn safe_error_context(category: ErrorCategory) -> &'static str {
    match category {
        ErrorCategory::Provider => "provider request failed",
        ErrorCategory::Tool => "tool execution failed",
        ErrorCategory::Approval => "approval was not granted",
        ErrorCategory::Cancel => "operation cancelled",
        ErrorCategory::Terminal => "terminal interaction failed",
        ErrorCategory::Unknown => "operation failed",
    }
}

fn display_tool_name(name: &str) -> String {
    safe_identifier(
        &name
            .strip_prefix("skill:")
            .map(|value| format!("skill {}", value.trim()))
            .unwrap_or_else(|| name.to_string()),
    )
}

fn safe_identifier(value: &str) -> String {
    truncate_chars(&redact_secrets(value.trim()), 80)
}

fn safe_message_summary(label: &str, message: &str) -> String {
    let first_line = message.lines().next().unwrap_or_default().trim();
    if first_line.is_empty() {
        return label.to_string();
    }
    format!(
        "{label}: {}",
        truncate_chars(&redact_secrets(first_line), 160)
    )
}

fn tool_error_context(message: &str) -> String {
    let first_line = message.lines().next().unwrap_or_default().trim();
    let stripped = first_line
        .strip_prefix("agent execution failed: ")
        .or_else(|| first_line.strip_prefix("tool execution failed: "))
        .or_else(|| first_line.strip_prefix("execution failed: "))
        .unwrap_or(first_line)
        .trim();
    if stripped.is_empty() {
        "agent operation failed".to_string()
    } else {
        truncate_chars(&redact_secrets(stripped), 160)
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn approval_label(approved: bool) -> &'static str {
    if approved { "approved" } else { "declined" }
}

fn command_exit_phase(exit_code: i32) -> ToolPhase {
    if exit_code == 0 {
        ToolPhase::Completed
    } else {
        ToolPhase::Failed
    }
}

fn mcp_status_phase(status: McpToolStatus) -> ToolPhase {
    match status {
        McpToolStatus::InProgress => ToolPhase::Running,
        McpToolStatus::Completed => ToolPhase::Completed,
        McpToolStatus::Failed => ToolPhase::Failed,
    }
}

fn is_visible_sandbox_status(status: &str) -> bool {
    let status = status.to_ascii_lowercase();
    status.contains("denied")
        || status.contains("declined")
        || status.contains("blocked")
        || status.contains("failed")
        || status.contains("escalation")
        || status.contains("requires")
}

fn debug_label(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::Started { .. } => "started",
        AgentEvent::ThreadStarted { .. } => "thread started",
        AgentEvent::TurnStarted => "turn started",
        AgentEvent::ThreadState { .. } => "thread state",
        AgentEvent::TurnMetadata { .. } => "turn metadata",
        AgentEvent::TurnState { .. } => "turn state",
        AgentEvent::DeepParityState { .. } => "deep parity",
        AgentEvent::ApprovalCacheState { .. } => "approval cache",
        AgentEvent::PersonaLoaded { .. } => "persona",
        AgentEvent::PersonaContextInjected { .. } => "persona context",
        AgentEvent::MemoryRecall { .. } => "memory recall",
        AgentEvent::MemoryCandidate { .. } => "memory candidate",
        AgentEvent::MemoryWrite { .. } => "memory write",
        AgentEvent::ContextStatus { .. } => "context",
        AgentEvent::StorageState { .. } => "session",
        AgentEvent::MultiAgentEvent { .. } => "multi-agent",
        _ => "event",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yunxi_agent_core::CommandStatus;

    #[test]
    fn assistant_stream_has_stable_id_and_preserves_stable_live_boundary() {
        let mut presentation = TuiPresentation::default();
        let first = presentation.present_agent_event(&AgentEvent::Message {
            content: "hello".to_string(),
            stream: None,
        });
        let second = presentation.present_agent_event(&AgentEvent::Message {
            content: "\nworld".to_string(),
            stream: None,
        });

        assert_eq!(first.id, second.id);
        assert_eq!(first.visible_text, "hello");
        assert_eq!(second.visible_text, "\nworld");
        let first_stream = first.stream.expect("first stream");
        let second_stream = second.stream.expect("second stream");
        assert_eq!(
            first_stream.identity.turn_id,
            second_stream.identity.turn_id
        );
        assert_eq!(
            first_stream.identity.stream_id,
            second_stream.identity.stream_id
        );
        assert!(
            first_stream.identity.source_sequence.value()
                < second_stream.identity.source_sequence.value()
        );
    }

    #[test]
    fn tool_lifecycle_reuses_external_id_without_exposing_arguments_or_output() {
        let mut presentation = TuiPresentation::default();
        let started = presentation.present_agent_event(&AgentEvent::ToolCallStarted {
            id: Some("call-secret".to_string()),
            name: "shell".to_string(),
            arguments_json: Some("{\"password\":\"hidden\"}".to_string()),
        });
        let completed = presentation.present_agent_event(&AgentEvent::ToolCallCompleted {
            id: Some("call-secret".to_string()),
            name: "shell".to_string(),
            output: "complete stdout body".to_string(),
            status: CommandStatus::Completed,
        });

        assert_eq!(started.id, completed.id);
        assert!(!started.visible_text.contains("password"));
        assert!(!started.visible_text.contains("hidden"));
        assert!(!completed.visible_text.contains("stdout"));
        assert!(
            started
                .detail
                .expect("arguments detail")
                .content
                .contains("password")
        );
        assert!(
            completed
                .detail
                .expect("output detail")
                .content
                .contains("stdout")
        );
    }

    #[test]
    fn reasoning_memory_context_and_provider_wire_are_debug_only() {
        let mut presentation = TuiPresentation::default();
        for event in [
            AgentEvent::Reasoning {
                content: "raw thinking".to_string(),
            },
            AgentEvent::MemoryRecall {
                schema_version: 3,
                enabled: true,
                scope: "global".to_string(),
                query: "private query".to_string(),
                count: 1,
                budget_used_chars: 10,
                truncated: false,
                always_on_count: 1,
                dropped_unrelated: 0,
                dropped_by_budget: 0,
                dropped_duplicates: 0,
            },
            AgentEvent::Started {
                prompt: "hidden prompt".to_string(),
            },
            AgentEvent::CommandUpdated {
                id: Some("wire".to_string()),
                command: "provider_wire".to_string(),
                aggregated_output: "arguments_json full stdout".to_string(),
            },
        ] {
            let presented = presentation.present_agent_event(&event);
            assert_eq!(presented.visibility, PresentationVisibility::DebugOnly);
            assert!(presented.visible_text.is_empty());
            assert!(presented.detail.is_some());
        }
    }

    #[test]
    fn provider_error_visible_summary_excludes_wire_and_stack() {
        let mut presentation = TuiPresentation::default();
        let presented = presentation.present_agent_event(&AgentEvent::ProviderError {
            provider: "deepseek".to_string(),
            status: Some(500),
            classification: "server_error".to_string(),
            message: "provider wire body\nstack frame secret".to_string(),
        });

        assert_eq!(presented.kind, TuiCellKind::ErrorSummary);
        assert!(presented.visible_text.contains("deepseek"));
        assert!(!presented.visible_text.contains("wire"));
        assert!(!presented.visible_text.contains("stack"));
        assert!(
            presented
                .detail
                .expect("provider detail")
                .content
                .contains("stack")
        );
    }

    #[test]
    fn every_user_visible_error_event_uses_stable_code_and_next_step() {
        let mut presentation = TuiPresentation::default();

        let provider = presentation.present_agent_event(&AgentEvent::ProviderError {
            provider: "deepseek".to_string(),
            status: Some(500),
            classification: "server_error".to_string(),
            message: "wire body and stack".to_string(),
        });
        assert_error_summary(&provider, "YX-PROVIDER-001", true);

        let tool = presentation.present_agent_event(&AgentEvent::CommandCompleted {
            id: Some("tool-error".to_string()),
            command: "echo secret-command".to_string(),
            aggregated_output: "secret output".to_string(),
            exit_code: Some(7),
            status: CommandStatus::Failed,
            execution_details: None,
        });
        assert_tool_error_summary(&tool, "YX-TOOL-001", true);

        let declined = presentation.present_agent_event(&AgentEvent::ApprovalCompleted {
            id: Some("approval-error".to_string()),
            approved: false,
            reason: Some("declined by user".to_string()),
        });
        assert_tool_error_summary(&declined, "YX-APPROVAL-001", false);

        let policy_declined = presentation.present_agent_event(&AgentEvent::ApprovalCompleted {
            id: Some("policy-error".to_string()),
            approved: false,
            reason: Some("declined by policy".to_string()),
        });
        assert_tool_error_summary(&policy_declined, "YX-APPROVAL-001", false);

        let approval_cancelled = presentation.present_agent_event(&AgentEvent::ApprovalCompleted {
            id: Some("approval-cancel".to_string()),
            approved: false,
            reason: Some("cancelled by user (Ctrl+C)".to_string()),
        });
        assert_tool_error_summary(&approval_cancelled, "YX-CANCEL-001", true);
        assert_eq!(
            approval_cancelled
                .tool_update
                .as_ref()
                .expect("cancelled tool update")
                .phase,
            ToolPhase::Cancelled
        );

        let cancelled = presentation.present_agent_event(&AgentEvent::Cancelled {
            reason: Some("cancelled by user".to_string()),
        });
        assert_error_summary(&cancelled, "YX-CANCEL-001", true);

        let terminal = presentation.present_error("terminal resize failed\ninternal stack");
        assert_error_summary(&terminal, "YX-TERMINAL-001", true);

        let unknown = presentation.present_agent_event(&AgentEvent::Error {
            message: "opaque internal failure".to_string(),
        });
        assert_error_summary(&unknown, "YX-UNKNOWN-001", true);

        let execution_failed = presentation.present_agent_event(&AgentEvent::Error {
            message: "agent execution failed: failed to read tool_search directory C:\\Users\\24763\\AppData\\Local\\Temp\\WinSAT: 拒绝访问。 (os error 5)".to_string(),
        });
        assert_error_summary(&execution_failed, "YX-TOOL-001", true);
        assert!(
            execution_failed
                .visible_text
                .contains("failed to read tool_search directory")
        );

        assert!(!terminal.visible_text.contains("internal stack"));
        assert!(
            terminal
                .detail
                .expect("terminal detail")
                .content
                .contains("internal stack")
        );
    }

    fn assert_error_summary(event: &TuiEvent, code: &str, retryable: bool) {
        assert_eq!(event.kind, TuiCellKind::ErrorSummary);
        assert!(event.visible_text.contains(code));
        assert!(event.visible_text.contains("next:"));
        assert!(event.visible_text.contains(if retryable {
            "retryable=yes"
        } else {
            "retryable=no"
        }));
    }

    fn assert_tool_error_summary(event: &TuiEvent, code: &str, retryable: bool) {
        let summary = event
            .tool_update
            .as_ref()
            .and_then(|update| update.output_summary.as_deref())
            .expect("tool error summary");
        assert!(summary.contains(code));
        assert!(summary.contains("next:"));
        assert!(summary.contains(if retryable {
            "retryable=yes"
        } else {
            "retryable=no"
        }));
    }

    #[test]
    fn non_message_boundary_does_not_split_the_active_assistant_stream() {
        let mut presentation = TuiPresentation::default();
        let before_tool = presentation.present_agent_event(&AgentEvent::Message {
            content: "before".to_string(),
            stream: None,
        });
        presentation.present_agent_event(&AgentEvent::ToolCallStarted {
            id: Some("call-1".to_string()),
            name: "shell".to_string(),
            arguments_json: None,
        });
        let after_tool = presentation.present_agent_event(&AgentEvent::Message {
            content: "after".to_string(),
            stream: None,
        });

        assert_eq!(before_tool.id, after_tool.id);
        assert_eq!(after_tool.visible_text, "after");
    }
}
