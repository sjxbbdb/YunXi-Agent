use crate::output_summary::{redact_secrets, truncate_chars, truncate_graphemes_with_notice};
use yunxi_agent_core::CommandStatus;

const MAX_TOOL_ID_GRAPHEMES: usize = 512;
const MAX_TOOL_NAME_GRAPHEMES: usize = 256;
const MAX_TOOL_FIELD_GRAPHEMES: usize = 4 * 1024;
const MAX_TOOL_OUTPUT_SUMMARY_GRAPHEMES: usize = 8 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolTimelineEntry {
    pub(crate) id: ToolActivityId,
    pub(crate) name: String,
    pub(crate) phase: ToolActivityPhase,
    pub(crate) steps: Vec<String>,
    pub(crate) command: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) approval: Option<String>,
    pub(crate) output_summary: Option<String>,
    pub(crate) detail_id: Option<usize>,
}

/// Stable identity for one tool activity. The external tool id is retained as
/// an opaque value so it can be mapped to a single transcript cell.
pub(crate) type ToolActivityId = Option<String>;

pub(crate) type ToolActivity = ToolTimelineEntry;
pub(crate) type ToolActivityPhase = ToolPhase;

impl ToolTimelineEntry {
    pub(crate) fn new(update: ToolTimelineUpdate) -> Self {
        let id = update
            .id
            .as_deref()
            .map(|id| bounded_tool_field(id, MAX_TOOL_ID_GRAPHEMES, false));
        let name = bounded_tool_field(&update.name, MAX_TOOL_NAME_GRAPHEMES, false);
        let mut entry = Self {
            id,
            name,
            phase: ToolPhase::Requested,
            steps: Vec::new(),
            command: None,
            status: None,
            approval: None,
            output_summary: None,
            detail_id: None,
        };
        entry.apply(update);
        entry
    }

    pub(crate) fn apply(&mut self, update: ToolTimelineUpdate) {
        // A late structured approval event may refine an initial generic
        // decline into an explicit user cancellation. Other terminal states
        // remain immutable so retries and duplicate events cannot reopen them.
        let corrects_decline_to_cancel =
            self.phase == ToolPhase::Declined && update.phase == ToolPhase::Cancelled;
        if self.phase.is_terminal() && !corrects_decline_to_cancel {
            return;
        }
        if !corrects_decline_to_cancel && self.name != update.name && !update.name.is_empty() {
            self.name = bounded_tool_field(&update.name, MAX_TOOL_NAME_GRAPHEMES, false);
        }
        self.phase = update.phase;
        self.push_step(update.phase.label());
        if let Some(command) = update.command {
            self.command = Some(bounded_tool_field(&command, MAX_TOOL_FIELD_GRAPHEMES, true));
        }
        if let Some(status) = update.status {
            self.status = Some(bounded_tool_field(&status, MAX_TOOL_FIELD_GRAPHEMES, true));
        }
        if let Some(approval) = update.approval {
            self.approval = Some(bounded_tool_field(
                &approval,
                MAX_TOOL_FIELD_GRAPHEMES,
                true,
            ));
        }
        if let Some(output_summary) = update.output_summary {
            self.output_summary = Some(bounded_tool_field(
                &output_summary,
                MAX_TOOL_OUTPUT_SUMMARY_GRAPHEMES,
                true,
            ));
        }
        if let Some(detail_id) = update.detail_id {
            self.detail_id = Some(detail_id);
        }
    }

    pub(crate) fn display_text(&self) -> String {
        let mut header = format!("{}: {}", self.name, self.phase.label());
        if let Some(status) = &self.status {
            header.push_str(&format!(" ({status})"));
        }
        if self.steps.len() > 1 {
            header.push_str("; path=");
            header.push_str(&self.steps.join(" -> "));
        }
        if let Some(output_summary) = &self.output_summary {
            header.push_str("; ");
            header.push_str(&truncate_chars(output_summary, 180));
        } else if let Some(id) = self.detail_id {
            header.push_str(&format!("; details #{id}"));
        }
        header
    }

    fn push_step(&mut self, step: &str) {
        if self.steps.last().is_some_and(|last| last == step) {
            return;
        }
        self.steps.push(step.to_string());
    }
}

fn bounded_tool_field(value: &str, max_graphemes: usize, retain_tail: bool) -> String {
    let redacted = redact_secrets(value);
    truncate_graphemes_with_notice(&redacted, max_graphemes, retain_tail)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolTimelineUpdate {
    pub(crate) id: ToolActivityId,
    pub(crate) name: String,
    pub(crate) phase: ToolPhase,
    pub(crate) command: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) approval: Option<String>,
    pub(crate) output_summary: Option<String>,
    pub(crate) detail_id: Option<usize>,
}

impl ToolTimelineUpdate {
    pub(crate) fn new(id: Option<String>, name: impl Into<String>, phase: ToolPhase) -> Self {
        Self {
            id,
            name: name.into(),
            phase,
            command: None,
            status: None,
            approval: None,
            output_summary: None,
            detail_id: None,
        }
    }

    pub(crate) fn approval(mut self, approval: impl Into<String>) -> Self {
        self.approval = Some(approval.into());
        self
    }

    pub(crate) fn output_summary(mut self, output_summary: impl Into<String>) -> Self {
        self.output_summary = Some(output_summary.into());
        self
    }

    pub(crate) fn detail_id(mut self, detail_id: usize) -> Self {
        self.detail_id = Some(detail_id);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolPhase {
    Requested,
    ApprovalRequired,
    Approved,
    Running,
    Completed,
    Failed,
    Declined,
    Cancelled,
    PolicyDeclined,
}

impl ToolPhase {
    pub(crate) fn label(self) -> &'static str {
        match self {
            ToolPhase::Requested => "requested",
            ToolPhase::ApprovalRequired => "approval required",
            ToolPhase::Approved => "approved",
            ToolPhase::Running => "running",
            ToolPhase::Completed => "completed",
            ToolPhase::Failed => "failed",
            ToolPhase::Declined => "declined",
            ToolPhase::Cancelled => "cancelled",
            ToolPhase::PolicyDeclined => "policy declined",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Declined
                | Self::Cancelled
                | Self::PolicyDeclined
        )
    }
}

pub(crate) fn phase_from_command_status(status: CommandStatus) -> ToolPhase {
    match status {
        CommandStatus::InProgress => ToolPhase::Running,
        CommandStatus::Completed => ToolPhase::Completed,
        CommandStatus::Failed => ToolPhase::Failed,
        CommandStatus::Declined => ToolPhase::Declined,
        CommandStatus::Cancelled => ToolPhase::Cancelled,
    }
}

pub(crate) fn status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::InProgress => "in_progress",
        CommandStatus::Completed => "completed",
        CommandStatus::Failed => "failed",
        CommandStatus::Declined => "declined",
        CommandStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_activity_cell_reaches_terminal_state_and_ignores_late_events() {
        let mut activity = ToolActivity::new(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::Requested,
        ));
        activity.apply(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::ApprovalRequired,
        ));
        activity.apply(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::Running,
        ));
        activity.apply(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::Completed,
        ));
        activity.apply(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::Running,
        ));

        assert_eq!(activity.phase, ToolPhase::Completed);
        assert_eq!(activity.display_text().lines().count(), 1);
    }

    #[test]
    fn late_structured_cancellation_refines_an_initial_decline() {
        let mut activity = ToolActivity::new(ToolTimelineUpdate::new(
            Some("tool-1".to_string()),
            "shell",
            ToolPhase::ApprovalRequired,
        ));
        activity.apply(
            ToolTimelineUpdate::new(Some("tool-1".to_string()), "shell", ToolPhase::Declined)
                .output_summary("YX-APPROVAL-001 approval was not granted"),
        );
        activity.apply(
            ToolTimelineUpdate::new(Some("tool-1".to_string()), "approval", ToolPhase::Cancelled)
                .output_summary("YX-CANCEL-001 operation cancelled"),
        );

        assert_eq!(activity.name, "shell");
        assert_eq!(activity.phase, ToolPhase::Cancelled);
        assert_eq!(
            activity.steps,
            vec!["approval required", "declined", "cancelled"]
        );
        assert!(activity.display_text().contains("YX-CANCEL-001"));
        assert!(!activity.display_text().contains("YX-APPROVAL-001"));
    }

    #[test]
    fn tool_fields_are_redacted_and_bounded_before_storage() {
        let mut update = ToolTimelineUpdate::new(
            Some("id".repeat(MAX_TOOL_ID_GRAPHEMES)),
            "shell",
            ToolPhase::Running,
        );
        update.command = Some(format!(
            "sk-secret-value\n{}latest-command",
            "x".repeat(MAX_TOOL_FIELD_GRAPHEMES + 100)
        ));
        update.status = Some("s".repeat(MAX_TOOL_FIELD_GRAPHEMES + 100));
        update.approval = Some("a".repeat(MAX_TOOL_FIELD_GRAPHEMES + 100));
        update.output_summary = Some("o".repeat(MAX_TOOL_OUTPUT_SUMMARY_GRAPHEMES + 100));

        let activity = ToolActivity::new(update);

        assert!(
            activity
                .id
                .as_ref()
                .is_some_and(|id| id.len() <= MAX_TOOL_ID_GRAPHEMES)
        );
        let command = activity.command.as_deref().expect("command");
        assert!(!command.contains("secret-value"));
        assert!(command.contains("older content truncated"));
        assert!(command.ends_with("latest-command"));
        for value in [
            activity.status.as_deref().expect("status"),
            activity.approval.as_deref().expect("approval"),
        ] {
            assert!(
                unicode_segmentation::UnicodeSegmentation::graphemes(value, true).count()
                    <= MAX_TOOL_FIELD_GRAPHEMES
            );
        }
        assert!(
            unicode_segmentation::UnicodeSegmentation::graphemes(
                activity.output_summary.as_deref().expect("output summary"),
                true,
            )
            .count()
                <= MAX_TOOL_OUTPUT_SUMMARY_GRAPHEMES
        );
    }
}
