use crate::output_summary::truncate_chars;
use yunxi_agent_core::CommandStatus;

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
        let id = update.id.clone();
        let name = update.name.clone();
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
            self.name = update.name;
        }
        self.phase = update.phase;
        self.push_step(update.phase.label());
        if let Some(command) = update.command {
            self.command = Some(command);
        }
        if let Some(status) = update.status {
            self.status = Some(status);
        }
        if let Some(approval) = update.approval {
            self.approval = Some(approval);
        }
        if let Some(output_summary) = update.output_summary {
            self.output_summary = Some(output_summary);
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
}
