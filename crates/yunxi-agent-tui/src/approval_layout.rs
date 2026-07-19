use crate::bottom_pane::ApprovalRequestView;
use crate::text_layout::{TextLayout, WrapPolicy};

const HEADER_LINES: usize = 1;
const REASON_LINES: usize = 1;
const RISK_LINES: usize = 1;
const COMMAND_LINES: usize = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApprovalLayout {
    pub(crate) lines: Vec<ApprovalLayoutLine>,
}

impl ApprovalLayout {
    pub(crate) fn desired_height(&self) -> u16 {
        (self.lines.len() as u16).saturating_add(2)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalLayoutLine {
    Label {
        kind: ApprovalLineKind,
        label: String,
        text: String,
    },
    Blank,
    Action {
        label: &'static str,
        selected: bool,
        shortcut: &'static str,
    },
    Hint(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalLineKind {
    Header,
    Reason,
    Risk,
    Command,
}

pub(crate) fn approval_desired_height(request: &ApprovalRequestView, width: usize) -> u16 {
    approval_layout_for_width(request, 0, width).desired_height()
}

pub(crate) fn approval_layout_for_width(
    request: &ApprovalRequestView,
    selected: usize,
    width: usize,
) -> ApprovalLayout {
    let inner_width = width.saturating_sub(2).max(20);
    let mut lines = Vec::new();

    push_labeled(
        &mut lines,
        ApprovalLineKind::Header,
        "approval ",
        &format!("{} in {}", request.tool_name, request.cwd),
        inner_width,
        HEADER_LINES,
        WrapPolicy::WindowsPathAware,
    );
    push_labeled(
        &mut lines,
        ApprovalLineKind::Reason,
        "reason   ",
        &request.reason,
        inner_width,
        REASON_LINES,
        WrapPolicy::NaturalText,
    );
    push_labeled(
        &mut lines,
        ApprovalLineKind::Risk,
        "risk     ",
        &request.risk_label(),
        inner_width,
        RISK_LINES,
        WrapPolicy::NaturalText,
    );
    if let Some(command) = &request.command {
        push_labeled(
            &mut lines,
            ApprovalLineKind::Command,
            "command  ",
            command,
            inner_width,
            COMMAND_LINES,
            command_policy(command),
        );
    }

    lines.push(ApprovalLayoutLine::Blank);
    lines.push(ApprovalLayoutLine::Action {
        label: "Approve",
        selected: selected == 0,
        shortcut: "Enter/Y",
    });
    lines.push(ApprovalLayoutLine::Action {
        label: "Decline",
        selected: selected == 1,
        shortcut: "N/Esc",
    });
    lines.push(ApprovalLayoutLine::Hint("Tab changes selection"));

    ApprovalLayout { lines }
}

fn push_labeled(
    lines: &mut Vec<ApprovalLayoutLine>,
    kind: ApprovalLineKind,
    label: &'static str,
    text: &str,
    width: usize,
    max_lines: usize,
    policy: WrapPolicy,
) {
    let label_width = TextLayout::measure(label);
    let text_width = width.saturating_sub(label_width).max(8);
    let wrapped = wrap_text(text, text_width, max_lines.max(1), policy);
    let continuation = " ".repeat(label_width);
    for (index, text) in wrapped.into_iter().enumerate() {
        lines.push(ApprovalLayoutLine::Label {
            kind,
            label: if index == 0 {
                label.to_string()
            } else {
                continuation.clone()
            },
            text,
        });
    }
}

fn wrap_text(value: &str, width: usize, max_lines: usize, policy: WrapPolicy) -> Vec<String> {
    let mut rows = TextLayout::wrap(value.trim(), width, policy)
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>();
    if rows.len() <= max_lines {
        return rows;
    }

    rows.truncate(max_lines);
    if let Some(last) = rows.last_mut() {
        *last = append_ellipsis(last, width);
    }
    rows
}

fn append_ellipsis(value: &str, width: usize) -> String {
    if width <= 3 {
        return String::new();
    }
    let candidate = format!("{value}...");
    if TextLayout::measure(&candidate) <= width {
        candidate
    } else {
        truncate_end(&candidate, width)
    }
}

fn truncate_end(value: &str, width: usize) -> String {
    TextLayout::truncate(value, width)
}

fn command_policy(command: &str) -> WrapPolicy {
    if command.contains("://") {
        WrapPolicy::UrlAware
    } else if command.contains('\\') || command.contains(":\\") {
        WrapPolicy::WindowsPathAware
    } else {
        WrapPolicy::NaturalText
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ApprovalRequestView {
        ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: "D:/YunXi Agent/workspace/with/a/very/long/path".to_string(),
            command: Some(
                "Remove-Item -Recurse -Force D:/YunXi Agent/workspace/generated/very-long-output"
                    .to_string(),
            ),
            reason:
                "tool execution requires approval before running a potentially destructive command"
                    .to_string(),
            risk_label: Some("risk: destructive".to_string()),
        }
    }

    #[test]
    fn approval_layout_reserves_decline_and_shortcut_lines_on_narrow_width() {
        let layout = approval_layout_for_width(&request(), 0, 58);
        let labels = layout
            .lines
            .iter()
            .filter_map(|line| match line {
                ApprovalLayoutLine::Action { label, .. } => Some(*label),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["Approve", "Decline"]);
        assert!(
            layout
                .lines
                .contains(&ApprovalLayoutLine::Hint("Tab changes selection"))
        );
        assert!(layout.desired_height() <= 12);
    }

    #[test]
    fn narrow_approval_preserves_risk_path_and_destructive_command_identity() {
        let request = ApprovalRequestView {
            cwd: "C:\\Users\\24763\\YunXi Agent\\包含空格\\危险输出目录".to_string(),
            command: Some(
                "Remove-Item -Recurse -Force C:\\Users\\24763\\YunXi Agent\\包含空格\\危险输出目录"
                    .to_string(),
            ),
            reason: "削除前に利用者の明示的な承認が必要です".to_string(),
            ..request()
        };
        let layout = approval_layout_for_width(&request, 1, 58);
        let rendered = layout
            .lines
            .iter()
            .filter_map(|line| match line {
                ApprovalLayoutLine::Label { label, text, .. } => Some(format!("{label}{text}")),
                ApprovalLayoutLine::Action { label, .. } => Some((*label).to_string()),
                ApprovalLayoutLine::Hint(value) => Some((*value).to_string()),
                ApprovalLayoutLine::Blank => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("risk: destructive"));
        assert!(rendered.contains("Remove-Item"));
        assert!(rendered.contains("C:\\"));
        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab changes selection"));
    }
}
