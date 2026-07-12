use crate::output_summary::truncate_chars;
use yunxi_agent_core::CommandStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolTimelineEntry {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) phase: ToolPhase,
    pub(crate) steps: Vec<String>,
    pub(crate) command: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) approval: Option<String>,
    pub(crate) output_summary: Option<String>,
    pub(crate) detail_id: Option<usize>,
}

impl ToolTimelineEntry {
    pub(crate) fn new(update: ToolTimelineUpdate) -> Self {
        let id = update.id.clone();
        let name = update.name.clone();
        let phase = update.phase;
        let mut entry = Self {
            id,
            name,
            phase,
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
        if self.name == "tool" || self.name == "approval" || self.name == "escalation" {
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

    pub(crate) fn is_same_tool(&self, update: &ToolTimelineUpdate) -> bool {
        if let (Some(left), Some(right)) = (&self.id, &update.id) {
            return left == right;
        }
        self.id.is_none() && update.id.is_none() && self.name == update.name && !self.is_terminal()
    }

    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.phase,
            ToolPhase::Completed | ToolPhase::Failed | ToolPhase::Declined | ToolPhase::Cancelled
        )
    }

    pub(crate) fn display_text(&self) -> String {
        let mut lines = Vec::new();
        let mut header = format!("{}: {}", self.name, self.steps.join(" -> "));
        if let Some(status) = &self.status {
            header.push_str(&format!(" ({status})"));
        }
        lines.push(header);
        if let Some(approval) = &self.approval {
            lines.push(format!("approval: {}", truncate_chars(approval, 180)));
        }
        if let Some(command) = &self.command {
            lines.push(format!("command: {}", truncate_chars(command, 180)));
        }
        if let Some(output_summary) = &self.output_summary {
            lines.push(match self.detail_id {
                Some(id) => format!("{output_summary}; details #{id}"),
                None => output_summary.clone(),
            });
        } else if let Some(id) = self.detail_id {
            lines.push(format!("details #{id}"));
        }
        lines.join("\n")
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
    pub(crate) id: Option<String>,
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

    pub(crate) fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub(crate) fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
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
    ApprovalRequired,
    Approved,
    Running,
    Completed,
    Failed,
    Declined,
    Cancelled,
}

impl ToolPhase {
    pub(crate) fn label(self) -> &'static str {
        match self {
            ToolPhase::ApprovalRequired => "approval required",
            ToolPhase::Approved => "approved",
            ToolPhase::Running => "running",
            ToolPhase::Completed => "completed",
            ToolPhase::Failed => "failed",
            ToolPhase::Declined => "declined",
            ToolPhase::Cancelled => "cancelled",
        }
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
