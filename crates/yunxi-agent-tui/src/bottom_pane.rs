use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequestView {
    pub id: Option<String>,
    pub tool_name: String,
    pub cwd: String,
    pub command: Option<String>,
    pub reason: String,
    pub risk_label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalDecision {
    pub approved: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserInputRequestView {
    pub id: Option<String>,
    pub prompt: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserInputResponse {
    pub value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BottomPaneMode {
    Composer {
        prompt: String,
        buffer: String,
        cursor: usize,
    },
    Approval {
        request: ApprovalRequestView,
        selected: usize,
    },
    UserInput {
        request: UserInputRequestView,
        buffer: String,
        cursor: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ComposerAction {
    None,
    Submit(String),
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalAction {
    None,
    Decide(ApprovalDecision),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UserInputAction {
    None,
    Submit(UserInputResponse),
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BottomPane {
    mode: BottomPaneMode,
}

impl Default for BottomPane {
    fn default() -> Self {
        Self {
            mode: BottomPaneMode::Composer {
                prompt: "yunxi> ".to_string(),
                buffer: String::new(),
                cursor: 0,
            },
        }
    }
}

impl BottomPane {
    pub(crate) fn mode(&self) -> &BottomPaneMode {
        &self.mode
    }

    pub(crate) fn start_composer(&mut self, prompt: impl Into<String>) {
        self.mode = BottomPaneMode::Composer {
            prompt: prompt.into(),
            buffer: String::new(),
            cursor: 0,
        };
    }

    pub(crate) fn start_approval(&mut self, request: ApprovalRequestView) {
        self.mode = BottomPaneMode::Approval {
            request,
            selected: 0,
        };
    }

    pub(crate) fn start_user_input(&mut self, request: UserInputRequestView) {
        self.mode = BottomPaneMode::UserInput {
            request,
            buffer: String::new(),
            cursor: 0,
        };
    }

    pub(crate) fn paste(&mut self, value: &str) {
        match &mut self.mode {
            BottomPaneMode::Composer { buffer, cursor, .. }
            | BottomPaneMode::UserInput { buffer, cursor, .. } => {
                insert_at_cursor(buffer, cursor, value);
            }
            BottomPaneMode::Approval { .. } => {}
        }
    }

    pub(crate) fn handle_composer_key(&mut self, key: KeyEvent) -> ComposerAction {
        let BottomPaneMode::Composer { buffer, cursor, .. } = &mut self.mode else {
            return ComposerAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return ComposerAction::None;
        }
        match key.code {
            KeyCode::Enter
                if key
                    .modifiers
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT) =>
            {
                insert_at_cursor(buffer, cursor, "\n");
                ComposerAction::None
            }
            KeyCode::Enter => {
                let submitted = buffer.trim_end_matches('\n').to_string();
                buffer.clear();
                *cursor = 0;
                ComposerAction::Submit(submitted)
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ComposerAction::Cancel
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                insert_at_cursor(buffer, cursor, "\n");
                ComposerAction::None
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                insert_at_cursor(buffer, cursor, &ch.to_string());
                ComposerAction::None
            }
            KeyCode::Backspace => {
                remove_before_cursor(buffer, cursor);
                ComposerAction::None
            }
            KeyCode::Delete => {
                remove_at_cursor(buffer, *cursor);
                ComposerAction::None
            }
            KeyCode::Left => {
                *cursor = previous_boundary(buffer, *cursor);
                ComposerAction::None
            }
            KeyCode::Right => {
                *cursor = next_boundary(buffer, *cursor);
                ComposerAction::None
            }
            KeyCode::Home => {
                *cursor = 0;
                ComposerAction::None
            }
            KeyCode::End => {
                *cursor = buffer.len();
                ComposerAction::None
            }
            KeyCode::Esc => {
                buffer.clear();
                *cursor = 0;
                ComposerAction::None
            }
            _ => ComposerAction::None,
        }
    }

    pub(crate) fn handle_approval_key(&mut self, key: KeyEvent) -> ApprovalAction {
        let BottomPaneMode::Approval { selected, .. } = &mut self.mode else {
            return ApprovalAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return ApprovalAction::None;
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('a') | KeyCode::Char('A') => {
                ApprovalAction::Decide(ApprovalDecision {
                    approved: true,
                    reason: Some("approved by YunXi TUI".to_string()),
                })
            }
            KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Char('d')
            | KeyCode::Char('D')
            | KeyCode::Esc => ApprovalAction::Decide(ApprovalDecision {
                approved: false,
                reason: Some("declined by YunXi TUI".to_string()),
            }),
            KeyCode::Tab | KeyCode::Down | KeyCode::Right => {
                *selected = (*selected + 1) % 2;
                ApprovalAction::None
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Left => {
                *selected = if *selected == 0 { 1 } else { 0 };
                ApprovalAction::None
            }
            KeyCode::Enter => ApprovalAction::Decide(ApprovalDecision {
                approved: *selected == 0,
                reason: Some(if *selected == 0 {
                    "approved by YunXi TUI".to_string()
                } else {
                    "declined by YunXi TUI".to_string()
                }),
            }),
            _ => ApprovalAction::None,
        }
    }

    pub(crate) fn handle_user_input_key(&mut self, key: KeyEvent) -> UserInputAction {
        let BottomPaneMode::UserInput { buffer, cursor, .. } = &mut self.mode else {
            return UserInputAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return UserInputAction::None;
        }
        match key.code {
            KeyCode::Enter => {
                let value = buffer.trim_end_matches('\n').to_string();
                UserInputAction::Submit(UserInputResponse { value: Some(value) })
            }
            KeyCode::Esc => UserInputAction::Cancel,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                UserInputAction::Cancel
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                insert_at_cursor(buffer, cursor, &ch.to_string());
                UserInputAction::None
            }
            KeyCode::Backspace => {
                remove_before_cursor(buffer, cursor);
                UserInputAction::None
            }
            KeyCode::Delete => {
                remove_at_cursor(buffer, *cursor);
                UserInputAction::None
            }
            KeyCode::Left => {
                *cursor = previous_boundary(buffer, *cursor);
                UserInputAction::None
            }
            KeyCode::Right => {
                *cursor = next_boundary(buffer, *cursor);
                UserInputAction::None
            }
            KeyCode::Home => {
                *cursor = 0;
                UserInputAction::None
            }
            KeyCode::End => {
                *cursor = buffer.len();
                UserInputAction::None
            }
            _ => UserInputAction::None,
        }
    }

    pub(crate) fn desired_height(&self) -> u16 {
        self.desired_height_for_width(usize::MAX)
    }

    pub(crate) fn desired_height_for_width(&self, _width: usize) -> u16 {
        match &self.mode {
            BottomPaneMode::Composer { buffer, .. } => {
                let lines = buffer.lines().count().max(1) as u16;
                3u16.saturating_add(lines.min(4))
            }
            BottomPaneMode::Approval { request, .. } => {
                let command_lines = request.command.as_ref().map(|_| 1).unwrap_or(0);
                9 + command_lines
            }
            BottomPaneMode::UserInput { .. } => 5,
        }
    }
}

impl ApprovalRequestView {
    pub(crate) fn risk_label(&self) -> String {
        if let Some(label) = self
            .risk_label
            .as_ref()
            .filter(|label| !label.trim().is_empty())
        {
            return label.clone();
        }
        let command = self
            .command
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if contains_any(
            &command,
            &[
                "remove-item",
                "rm -rf",
                "del ",
                "rmdir",
                "rd /s",
                "format ",
                "shutdown",
            ],
        ) {
            "risk: destructive".to_string()
        } else if contains_any(
            &command,
            &[
                "curl ",
                "wget ",
                "invoke-webrequest",
                "invoke-restmethod",
                "irm ",
            ],
        ) {
            "risk: network".to_string()
        } else if contains_any(
            &command,
            &[
                ">",
                "set-content",
                "add-content",
                "out-file",
                "new-item",
                "copy ",
                "move ",
                "apply_patch",
            ],
        ) {
            "risk: writes workspace".to_string()
        } else if contains_any(
            &command,
            &["get-content", "type ", "cat ", "rg ", "findstr "],
        ) {
            "risk: reads workspace".to_string()
        } else {
            "risk: low".to_string()
        }
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn insert_at_cursor(buffer: &mut String, cursor: &mut usize, value: &str) {
    let cursor_at = clamp_to_boundary(buffer, *cursor);
    buffer.insert_str(cursor_at, value);
    *cursor = cursor_at + value.len();
}

fn remove_before_cursor(buffer: &mut String, cursor: &mut usize) {
    if *cursor == 0 || buffer.is_empty() {
        return;
    }
    let start = previous_boundary(buffer, *cursor);
    buffer.drain(start..*cursor);
    *cursor = start;
}

fn remove_at_cursor(buffer: &mut String, cursor: usize) {
    if cursor >= buffer.len() {
        return;
    }
    let end = next_boundary(buffer, cursor);
    buffer.drain(cursor..end);
}

fn previous_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_to_boundary(value, cursor);
    value[..cursor]
        .char_indices()
        .next_back()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_to_boundary(value, cursor);
    value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| cursor + idx)
        .unwrap_or(value.len())
}

fn clamp_to_boundary(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    while cursor > 0 && !value.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_shortcuts_emit_decisions() {
        let mut pane = BottomPane::default();
        pane.start_approval(ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo hi".to_string()),
            reason: "needs approval".to_string(),
            risk_label: None,
        });

        let action =
            pane.handle_approval_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(
            action,
            ApprovalAction::Decide(ApprovalDecision {
                approved: true,
                reason: Some("approved by YunXi TUI".to_string())
            })
        );
    }

    #[test]
    fn empty_composer_uses_compact_height() {
        let pane = BottomPane::default();

        assert_eq!(pane.desired_height_for_width(58), 4);
    }

    #[test]
    fn approval_risk_label_classifies_common_commands() {
        let destructive = ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("Remove-Item -Recurse -Force target".to_string()),
            reason: "needs approval".to_string(),
            risk_label: None,
        };
        let network = ApprovalRequestView {
            command: Some("curl https://example.test".to_string()),
            ..destructive.clone()
        };
        let explicit = ApprovalRequestView {
            command: Some("echo hi".to_string()),
            risk_label: Some("risk: custom".to_string()),
            ..destructive.clone()
        };

        assert_eq!(destructive.risk_label(), "risk: destructive");
        assert_eq!(network.risk_label(), "risk: network");
        assert_eq!(explicit.risk_label(), "risk: custom");
    }
}
