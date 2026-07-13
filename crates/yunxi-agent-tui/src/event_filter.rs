use crate::output_summary::{OutputSummary, summarize_tool_output};
use crate::timeline::{ToolPhase, ToolTimelineUpdate, phase_from_command_status, status_label};
use yunxi_agent_core::{AgentEvent, AgentRunStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FilteredEvent {
    Assistant(String),
    Reasoning(String),
    Tool(ToolTimelineUpdate),
    ToolOutput {
        update: ToolTimelineUpdate,
        label: String,
        summary: OutputSummary,
    },
    Notice {
        kind: String,
        message: String,
    },
    NoticeWithDebug {
        kind: String,
        message: String,
        debug_label: String,
        debug_detail: String,
    },
    Warning(String),
    Error(String),
    DebugOnly {
        label: String,
        detail: String,
    },
    Suppress,
}

impl FilteredEvent {
    fn with_debug(self, label: impl Into<String>, detail: impl Into<String>) -> Self {
        match self {
            Self::Notice { kind, message } => Self::NoticeWithDebug {
                kind,
                message,
                debug_label: label.into(),
                debug_detail: detail.into(),
            },
            other => other,
        }
    }
}

pub(crate) fn classify(event: &AgentEvent) -> FilteredEvent {
    match event {
        AgentEvent::Message { content } => FilteredEvent::Assistant(content.clone()),
        AgentEvent::Reasoning { content } => classify_reasoning(content),
        AgentEvent::CommandStarted { id, command } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(id.clone(), "shell", ToolPhase::Running)
                .command(command.clone()),
        ),
        AgentEvent::CommandUpdated {
            command,
            aggregated_output,
            ..
        } => {
            if aggregated_output.trim().is_empty() {
                FilteredEvent::Suppress
            } else {
                FilteredEvent::DebugOnly {
                    label: format!("stdout {command}"),
                    detail: aggregated_output.clone(),
                }
            }
        }
        AgentEvent::CommandCompleted {
            id,
            command,
            aggregated_output,
            exit_code,
            status,
        } => {
            let update =
                ToolTimelineUpdate::new(id.clone(), "shell", phase_from_command_status(*status))
                    .command(command.clone())
                    .status(command_status_with_exit(*status, *exit_code));
            if let Some(summary) = summarize_tool_output("shell", aggregated_output) {
                FilteredEvent::ToolOutput {
                    update,
                    label: format!("shell output {command}"),
                    summary,
                }
            } else {
                FilteredEvent::Tool(update)
            }
        }
        AgentEvent::CommandFinished { command, exit_code } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(None, "shell", command_exit_phase(*exit_code))
                .command(command.clone())
                .status(format!("exit {exit_code}")),
        ),
        AgentEvent::ToolCallStarted {
            id,
            name,
            arguments_json,
        } => {
            if let Some(arguments_json) = arguments_json
                && !arguments_json.trim().is_empty()
            {
                return FilteredEvent::ToolOutput {
                    update: ToolTimelineUpdate::new(
                        id.clone(),
                        display_tool_name(name),
                        ToolPhase::Running,
                    ),
                    label: format!("tool arguments {}", display_tool_name(name)),
                    summary: OutputSummary {
                        visible: "arguments hidden".to_string(),
                        detail: arguments_json.clone(),
                        line_count: arguments_json.lines().count().max(1),
                        char_count: arguments_json.chars().count(),
                        hidden: true,
                    },
                };
            }
            FilteredEvent::Tool(ToolTimelineUpdate::new(
                id.clone(),
                display_tool_name(name),
                ToolPhase::Running,
            ))
        }
        AgentEvent::ToolCallCompleted {
            id,
            name,
            output,
            status,
        } => {
            let update = ToolTimelineUpdate::new(
                id.clone(),
                display_tool_name(name),
                phase_from_command_status(*status),
            )
            .status(status_label(*status));
            if let Some(summary) = summarize_tool_output(name, output) {
                FilteredEvent::ToolOutput {
                    update,
                    label: format!("tool output {}", display_tool_name(name)),
                    summary,
                }
            } else {
                FilteredEvent::Tool(update)
            }
        }
        AgentEvent::McpToolStarted { id, server, tool } => {
            FilteredEvent::Tool(ToolTimelineUpdate::new(
                id.clone(),
                format!("mcp {server}/{tool}"),
                ToolPhase::Running,
            ))
        }
        AgentEvent::McpToolCompleted {
            id,
            server,
            tool,
            status,
        } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(
                id.clone(),
                format!("mcp {server}/{tool}"),
                mcp_status_phase(*status),
            )
            .status(format!("{status:?}")),
        ),
        AgentEvent::ApprovalRequested {
            id,
            tool_name,
            reason,
        } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(
                id.clone(),
                display_tool_name(tool_name),
                ToolPhase::ApprovalRequired,
            )
            .approval(reason.clone()),
        ),
        AgentEvent::ApprovalCompleted {
            id,
            approved,
            reason,
        } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(
                id.clone(),
                "approval",
                if *approved {
                    ToolPhase::Approved
                } else {
                    ToolPhase::Declined
                },
            )
            .approval(
                reason
                    .clone()
                    .unwrap_or_else(|| approval_label(*approved).to_string()),
            ),
        ),
        AgentEvent::EscalationRequested {
            id,
            tool_name,
            reason,
            required_sandbox,
            required_network,
        } => {
            let mut approval = reason.clone();
            if let Some(sandbox) = required_sandbox {
                approval.push_str(&format!(" sandbox={sandbox}"));
            }
            if let Some(network) = required_network {
                approval.push_str(&format!(" network={network}"));
            }
            FilteredEvent::Tool(
                ToolTimelineUpdate::new(
                    id.clone(),
                    display_tool_name(tool_name),
                    ToolPhase::ApprovalRequired,
                )
                .approval(approval),
            )
        }
        AgentEvent::EscalationCompleted {
            id,
            approved,
            reason,
        } => FilteredEvent::Tool(
            ToolTimelineUpdate::new(
                id.clone(),
                "escalation",
                if *approved {
                    ToolPhase::Approved
                } else {
                    ToolPhase::Declined
                },
            )
            .approval(
                reason
                    .clone()
                    .unwrap_or_else(|| approval_label(*approved).to_string()),
            ),
        ),
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
            let detail = format!(
                "schema_version={schema_version} platform={platform} status={status} backend_id={backend_id} backend_label={backend_label} backend={backend} os_isolation={os_isolation} enforcement={enforcement} enforcement_level={enforcement_level} runner={runner} unsupported_reason={} cwd={cwd} command={} message={}",
                unsupported_reason.as_deref().unwrap_or("none"),
                command.as_deref().unwrap_or("none"),
                message.as_deref().unwrap_or("none")
            );
            if is_visible_sandbox_status(status) {
                FilteredEvent::Notice {
                    kind: "policy".to_string(),
                    message: detail,
                }
            } else {
                FilteredEvent::DebugOnly {
                    label: "sandbox attempt".to_string(),
                    detail,
                }
            }
        }
        AgentEvent::McpSession {
            server,
            status,
            message,
        } => {
            if status.eq_ignore_ascii_case("failed") || status.eq_ignore_ascii_case("error") {
                FilteredEvent::Notice {
                    kind: "mcp-session".to_string(),
                    message: format!(
                        "{server}: {status}{}",
                        message
                            .as_ref()
                            .map(|value| format!(" - {value}"))
                            .unwrap_or_default()
                    ),
                }
            } else {
                FilteredEvent::DebugOnly {
                    label: format!("mcp session {server}"),
                    detail: format!("{status} {}", message.as_deref().unwrap_or_default())
                        .trim()
                        .to_string(),
                }
            }
        }
        AgentEvent::PersonaLoaded {
            profile_id,
            enabled,
            ..
        } => FilteredEvent::DebugOnly {
            label: "persona".to_string(),
            detail: format!("profile={profile_id} enabled={enabled}"),
        },
        AgentEvent::PersonaContextInjected {
            profile_id,
            memory_count,
            budget_used_chars,
            budget_limit_chars,
            ..
        } => FilteredEvent::DebugOnly {
            label: "persona context".to_string(),
            detail: format!(
                "profile={profile_id} memories={memory_count} budget={budget_used_chars}/{budget_limit_chars}"
            ),
        },
        AgentEvent::MemoryRecall {
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
            ..
        } => FilteredEvent::DebugOnly {
            label: "memory recall".to_string(),
            detail: format!(
                "enabled={enabled} scope={scope} query={query} count={count} always_on={always_on_count} dropped_unrelated={dropped_unrelated} dropped_by_budget={dropped_by_budget} dropped_duplicates={dropped_duplicates} budget_used_chars={budget_used_chars} truncated={truncated}"
            ),
        },
        AgentEvent::MemoryCandidate {
            id,
            kind,
            sensitivity,
            status,
            write_policy,
            reason,
            ..
        } => FilteredEvent::DebugOnly {
            label: "memory candidate".to_string(),
            detail: format!(
                "id={id} kind={kind} sensitivity={sensitivity} status={status} policy={write_policy} reason={reason}"
            ),
        },
        AgentEvent::MemoryWrite {
            id,
            scope,
            kind,
            status,
            action,
            revision,
            merged_count,
            ..
        } => {
            let detail = format!(
                "id={id} scope={scope} kind={kind} status={status} action={action} revision={revision} merged_count={merged_count}"
            );
            match action.as_str() {
                "auto_saved" => FilteredEvent::Notice {
                    kind: "memory".to_string(),
                    message: format!("memory saved: {kind}"),
                }
                .with_debug("memory write", detail),
                "merged" => FilteredEvent::Notice {
                    kind: "memory".to_string(),
                    message: format!("memory updated: {kind}"),
                }
                .with_debug("memory write", detail),
                "pending_confirmation" => FilteredEvent::Notice {
                    kind: "memory".to_string(),
                    message: "memory pending review; run `yunxi memory pending` to review"
                        .to_string(),
                }
                .with_debug("memory write", detail),
                "discard" | "discarded" => FilteredEvent::Notice {
                    kind: "memory".to_string(),
                    message: "memory discarded by privacy policy".to_string(),
                }
                .with_debug("memory write", detail),
                "disabled" => FilteredEvent::Notice {
                    kind: "memory".to_string(),
                    message: "memory disabled".to_string(),
                }
                .with_debug("memory write", detail),
                _ => FilteredEvent::DebugOnly {
                    label: "memory write".to_string(),
                    detail,
                },
            }
        }
        AgentEvent::MemoryWarning { warning, .. } => FilteredEvent::Warning(warning.clone()),
        AgentEvent::Warning { message } => FilteredEvent::Warning(message.clone()),
        AgentEvent::Error { message } => FilteredEvent::Error(message.clone()),
        AgentEvent::ProviderError {
            provider,
            classification,
            status,
            message,
        } => FilteredEvent::Error(format!(
            "{provider} {classification}{} - {message}",
            status.map(|value| format!(" {value}")).unwrap_or_default()
        )),
        AgentEvent::Cancelled { reason } => FilteredEvent::Notice {
            kind: "cancelled".to_string(),
            message: reason
                .clone()
                .unwrap_or_else(|| "current turn cancelled".to_string()),
        },
        AgentEvent::Completed { status, usage } => {
            if *status != AgentRunStatus::Completed {
                FilteredEvent::Notice {
                    kind: "turn".to_string(),
                    message: format!("{status:?}"),
                }
            } else if let Some(usage) = usage {
                FilteredEvent::DebugOnly {
                    label: "usage".to_string(),
                    detail: format!(
                        "input={} cached_input={} output={} reasoning_output={}",
                        usage.input_tokens,
                        usage.cached_input_tokens,
                        usage.output_tokens,
                        usage.reasoning_output_tokens
                    ),
                }
            } else {
                FilteredEvent::Suppress
            }
        }
        AgentEvent::FileChanged { path, kind } => FilteredEvent::Notice {
            kind: "file".to_string(),
            message: format!("{kind:?}: {path}"),
        },
        AgentEvent::PatchCompleted { status } => FilteredEvent::Notice {
            kind: "patch".to_string(),
            message: format!("{status:?}"),
        },
        AgentEvent::TodoUpdated { id, items } => FilteredEvent::Notice {
            kind: "todo".to_string(),
            message: format!(
                "{} item(s){}",
                items.len(),
                id.as_ref()
                    .map(|value| format!(" id={value}"))
                    .unwrap_or_default()
            ),
        },
        AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            status,
            message,
            ..
        } => FilteredEvent::Notice {
            kind: "child".to_string(),
            message: format!(
                "{agent_id} {status} session={child_session_id}{}",
                message
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            ),
        },
        AgentEvent::ChildScopedStream {
            agent_id,
            event,
            seq,
            message,
            ..
        } => FilteredEvent::Notice {
            kind: "child-stream".to_string(),
            message: format!(
                "{agent_id} #{seq} {event}{}",
                message
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default()
            ),
        },
        AgentEvent::ContextStatus { .. }
        | AgentEvent::StorageState { .. }
        | AgentEvent::ThreadStarted { .. }
        | AgentEvent::TurnStarted
        | AgentEvent::ThreadState { .. }
        | AgentEvent::TurnMetadata { .. }
        | AgentEvent::TurnState { .. }
        | AgentEvent::DeepParityState { .. }
        | AgentEvent::ApprovalCacheState { .. }
        | AgentEvent::MultiAgentEvent { .. }
        | AgentEvent::Started { .. } => FilteredEvent::DebugOnly {
            label: debug_label(event).to_string(),
            detail: format!("{event:?}"),
        },
    }
}

fn classify_reasoning(content: &str) -> FilteredEvent {
    let mut value = content.trim_matches('\r').trim().to_string();
    for prefix in ["Provider turn started", "Provider turn completed"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            value = rest.trim().to_string();
        }
    }
    if value.is_empty() {
        return FilteredEvent::DebugOnly {
            label: "provider lifecycle".to_string(),
            detail: content.to_string(),
        };
    }
    let lower = value.to_ascii_lowercase();
    if [
        "tool dispatch",
        "sandbox decision",
        "sandbox runner",
        "restored history",
        "backend=policy guard",
        "policy guard",
        "enforcement_level=policy_only",
        "enforcement_level=process_lifecycle",
        "enforcement_level=policy_bypass",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return FilteredEvent::DebugOnly {
            label: "runtime reasoning".to_string(),
            detail: content.to_string(),
        };
    }
    FilteredEvent::Reasoning(value)
}

fn display_tool_name(name: &str) -> String {
    name.strip_prefix("skill:")
        .map(|value| format!("skill {}", value.trim()))
        .unwrap_or_else(|| name.to_string())
}

fn approval_label(approved: bool) -> &'static str {
    if approved { "approved" } else { "declined" }
}

fn command_status_with_exit(
    status: yunxi_agent_core::CommandStatus,
    exit_code: Option<i32>,
) -> String {
    match exit_code {
        Some(code) => format!("{} ({code})", status_label(status)),
        None => status_label(status).to_string(),
    }
}

fn command_exit_phase(exit_code: i32) -> ToolPhase {
    if exit_code == 0 {
        ToolPhase::Completed
    } else {
        ToolPhase::Failed
    }
}

fn mcp_status_phase(status: yunxi_agent_core::McpToolStatus) -> ToolPhase {
    match status {
        yunxi_agent_core::McpToolStatus::InProgress => ToolPhase::Running,
        yunxi_agent_core::McpToolStatus::Completed => ToolPhase::Completed,
        yunxi_agent_core::McpToolStatus::Failed => ToolPhase::Failed,
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
        AgentEvent::ContextStatus { .. } => "context",
        AgentEvent::StorageState { .. } => "session",
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
        AgentEvent::MemoryWarning { .. } => "memory warning",
        AgentEvent::MultiAgentEvent { .. } => "multi-agent",
        AgentEvent::Started { .. } => "started",
        _ => "event",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_write_auto_saved_is_visible_without_content() {
        let event = AgentEvent::MemoryWrite {
            schema_version: 2,
            id: "mem-1".to_string(),
            scope: "global_user".to_string(),
            kind: "preference".to_string(),
            status: "active".to_string(),
            action: "auto_saved".to_string(),
            revision: 1,
            merged_count: 1,
        };

        let filtered = classify(&event);

        assert!(matches!(
            filtered,
            FilteredEvent::NoticeWithDebug {
                kind,
                message,
                debug_detail,
                ..
            } if kind == "memory"
                && message == "memory saved: preference"
                && debug_detail.contains("id=mem-1")
                && debug_detail.contains("revision=1")
                && !message.contains("content")
        ));
    }

    #[test]
    fn memory_write_pending_points_to_cli_review_command() {
        let event = AgentEvent::MemoryWrite {
            schema_version: 2,
            id: "mem-2".to_string(),
            scope: "global_user".to_string(),
            kind: "personal_fact".to_string(),
            status: "pending".to_string(),
            action: "pending_confirmation".to_string(),
            revision: 1,
            merged_count: 1,
        };

        let filtered = classify(&event);

        assert!(matches!(
            filtered,
            FilteredEvent::NoticeWithDebug { kind, message, .. }
                if kind == "memory"
                    && message.contains("memory pending review")
                    && message.contains("yunxi memory pending")
                    && !message.contains("content")
        ));
    }

    #[test]
    fn memory_write_merged_uses_updated_notice() {
        let event = AgentEvent::MemoryWrite {
            schema_version: 2,
            id: "mem-3".to_string(),
            scope: "global_user".to_string(),
            kind: "preference".to_string(),
            status: "active".to_string(),
            action: "merged".to_string(),
            revision: 2,
            merged_count: 2,
        };

        let filtered = classify(&event);

        assert!(matches!(
            filtered,
            FilteredEvent::NoticeWithDebug {
                kind,
                message,
                debug_detail,
                ..
            } if kind == "memory"
                && message == "memory updated: preference"
                && debug_detail.contains("merged_count=2")
        ));
    }

    #[test]
    fn memory_write_discarded_hides_secret_like_details_from_notice() {
        let event = AgentEvent::MemoryWrite {
            schema_version: 2,
            id: "mem-secret".to_string(),
            scope: "global_user".to_string(),
            kind: "preference".to_string(),
            status: "rejected".to_string(),
            action: "discard".to_string(),
            revision: 1,
            merged_count: 1,
        };

        let filtered = classify(&event);

        assert!(matches!(
            filtered,
            FilteredEvent::NoticeWithDebug { kind, message, .. }
                if kind == "memory"
                    && message == "memory discarded by privacy policy"
                    && !message.contains("mem-secret")
        ));
    }
}
