use crate::approval_layout::approval_desired_height;
use crate::edit_buffer::{EditBuffer, EditBufferSnapshot};
use crate::input_map::{FocusTarget, TuiAction, resolve_key};
use crate::text_layout::{TextLayout, WrapPolicy};
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
    Composer,
    Approval {
        request: ApprovalRequestView,
        selected: usize,
    },
    UserInput {
        request: UserInputRequestView,
        buffer: EditBuffer,
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
    composer_prompt: String,
    composer: EditBuffer,
    suspended_composer: Option<EditBufferSnapshot>,
}

impl Default for BottomPane {
    fn default() -> Self {
        Self {
            mode: BottomPaneMode::Composer,
            composer_prompt: "yunxi> ".to_string(),
            composer: EditBuffer::default(),
            suspended_composer: None,
        }
    }
}

impl BottomPane {
    pub(crate) fn mode(&self) -> &BottomPaneMode {
        &self.mode
    }

    pub(crate) fn text_input_active(&self) -> bool {
        matches!(
            self.mode,
            BottomPaneMode::Composer | BottomPaneMode::UserInput { .. }
        )
    }

    pub(crate) fn composer_prompt(&self) -> &str {
        &self.composer_prompt
    }

    pub(crate) fn composer_buffer(&self) -> &EditBuffer {
        &self.composer
    }

    #[cfg(test)]
    pub(crate) fn composer_snapshot(&self) -> EditBufferSnapshot {
        self.composer.snapshot()
    }

    pub(crate) fn start_composer(&mut self, prompt: impl Into<String>) {
        self.composer_prompt = prompt.into();
        if let Some(snapshot) = self.suspended_composer.take() {
            self.composer.restore(snapshot);
        }
        self.mode = BottomPaneMode::Composer;
    }

    #[cfg(test)]
    pub(crate) fn reset_composer(&mut self, prompt: impl Into<String>) {
        self.suspended_composer = None;
        self.composer.clear();
        self.start_composer(prompt);
    }

    pub(crate) fn start_approval(&mut self, request: ApprovalRequestView) {
        self.suspend_composer();
        self.mode = BottomPaneMode::Approval {
            request,
            // Safe default: Enter confirms the explicitly selected decline.
            selected: 1,
        };
    }

    pub(crate) fn start_user_input(&mut self, request: UserInputRequestView) {
        self.suspend_composer();
        self.mode = BottomPaneMode::UserInput {
            request,
            buffer: EditBuffer::default(),
        };
    }

    pub(crate) fn paste(&mut self, value: &str) -> bool {
        match &mut self.mode {
            BottomPaneMode::Composer => {
                self.composer.insert_text(value);
                true
            }
            BottomPaneMode::UserInput { buffer, .. } => {
                buffer.insert_text(value);
                true
            }
            BottomPaneMode::Approval { .. } => false,
        }
    }

    pub(crate) fn handle_composer_key(&mut self, key: KeyEvent) -> ComposerAction {
        if !matches!(self.mode, BottomPaneMode::Composer) {
            return ComposerAction::None;
        }
        if key.kind != KeyEventKind::Press {
            return ComposerAction::None;
        }
        match resolve_key(FocusTarget::Composer, key) {
            TuiAction::InsertNewline => {
                self.composer.insert_newline();
                ComposerAction::None
            }
            TuiAction::Submit => ComposerAction::Submit(self.composer.submit_text()),
            TuiAction::Cancel => ComposerAction::Cancel,
            _ => match key.code {
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.composer.insert_newline();
                    ComposerAction::None
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.composer.insert_text(&ch.to_string());
                    ComposerAction::None
                }
                KeyCode::Backspace => {
                    self.composer.delete_previous_grapheme();
                    ComposerAction::None
                }
                KeyCode::Delete => {
                    self.composer.delete_next_grapheme();
                    ComposerAction::None
                }
                KeyCode::Left => {
                    self.composer.move_left();
                    ComposerAction::None
                }
                KeyCode::Right => {
                    self.composer.move_right();
                    ComposerAction::None
                }
                KeyCode::Home => {
                    self.composer.move_home();
                    ComposerAction::None
                }
                KeyCode::End => {
                    self.composer.move_end();
                    ComposerAction::None
                }
                _ => ComposerAction::None,
            },
        }
    }

    pub(crate) fn handle_composer_draft_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.mode, BottomPaneMode::Composer)
            || key.kind != KeyEventKind::Press
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return false;
        }
        let before = self.composer.snapshot();
        match key.code {
            KeyCode::Enter
                if matches!(
                    resolve_key(FocusTarget::Composer, key),
                    TuiAction::InsertNewline
                ) =>
            {
                self.composer.insert_newline();
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.composer.insert_newline();
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.composer.insert_text(&ch.to_string());
            }
            KeyCode::Backspace => self.composer.delete_previous_grapheme(),
            KeyCode::Delete => self.composer.delete_next_grapheme(),
            KeyCode::Left => self.composer.move_left(),
            KeyCode::Right => self.composer.move_right(),
            KeyCode::Home => self.composer.move_home(),
            KeyCode::End => self.composer.move_end(),
            // Submit and destructive clear are disabled while a turn is active.
            KeyCode::Enter | KeyCode::Esc => {}
            _ => {}
        }
        before != self.composer.snapshot()
    }

    pub(crate) fn handle_approval_key(&mut self, key: KeyEvent) -> ApprovalAction {
        let BottomPaneMode::Approval { selected, .. } = &mut self.mode else {
            return ApprovalAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return ApprovalAction::None;
        }
        let action = resolve_key(FocusTarget::Approval, key);
        match action {
            TuiAction::Cancel => {
                return ApprovalAction::Decide(ApprovalDecision {
                    approved: false,
                    reason: Some("cancelled by user (Ctrl+C)".to_string()),
                });
            }
            TuiAction::Decline => {
                return ApprovalAction::Decide(ApprovalDecision {
                    approved: false,
                    reason: Some("declined by YunXi TUI".to_string()),
                });
            }
            TuiAction::Approve => {
                return ApprovalAction::Decide(ApprovalDecision {
                    approved: true,
                    reason: Some("approved by YunXi TUI".to_string()),
                });
            }
            TuiAction::Submit => {
                return ApprovalAction::Decide(ApprovalDecision {
                    approved: *selected == 0,
                    reason: Some(if *selected == 0 {
                        "approved by YunXi TUI".to_string()
                    } else {
                        "declined by YunXi TUI".to_string()
                    }),
                });
            }
            TuiAction::InsertNewline
            | TuiAction::None
            | TuiAction::FocusNext
            | TuiAction::FocusPrevious => {}
            TuiAction::CloseDetails | TuiAction::PageUp | TuiAction::PageDown => {
                return ApprovalAction::None;
            }
            TuiAction::Paste
            | TuiAction::ScrollUp
            | TuiAction::ScrollDown
            | TuiAction::DetailScrollUp
            | TuiAction::DetailScrollDown => {
                return ApprovalAction::None;
            }
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('a') | KeyCode::Char('A') => {
                ApprovalAction::Decide(ApprovalDecision {
                    approved: true,
                    reason: Some("approved by YunXi TUI".to_string()),
                })
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('d') | KeyCode::Char('D') => {
                ApprovalAction::Decide(ApprovalDecision {
                    approved: false,
                    reason: Some("declined by YunXi TUI".to_string()),
                })
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ApprovalAction::Decide(ApprovalDecision {
                    approved: false,
                    reason: Some("cancelled by user (Ctrl+C)".to_string()),
                })
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Right => {
                *selected = (*selected + 1) % 2;
                ApprovalAction::None
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Left => {
                *selected = if *selected == 0 { 1 } else { 0 };
                ApprovalAction::None
            }
            KeyCode::Enter => ApprovalAction::None,
            _ => ApprovalAction::None,
        }
    }

    pub(crate) fn handle_user_input_key(&mut self, key: KeyEvent) -> UserInputAction {
        let BottomPaneMode::UserInput { buffer, .. } = &mut self.mode else {
            return UserInputAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return UserInputAction::None;
        }
        match resolve_key(FocusTarget::Composer, key) {
            TuiAction::InsertNewline => {
                buffer.insert_newline();
                UserInputAction::None
            }
            TuiAction::Submit => UserInputAction::Submit(UserInputResponse {
                value: Some(buffer.submit_text()),
            }),
            TuiAction::Cancel => UserInputAction::Cancel,
            _ => match key.code {
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buffer.insert_newline();
                    UserInputAction::None
                }
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buffer.insert_text(&ch.to_string());
                    UserInputAction::None
                }
                KeyCode::Backspace => {
                    buffer.delete_previous_grapheme();
                    UserInputAction::None
                }
                KeyCode::Delete => {
                    buffer.delete_next_grapheme();
                    UserInputAction::None
                }
                KeyCode::Left => {
                    buffer.move_left();
                    UserInputAction::None
                }
                KeyCode::Right => {
                    buffer.move_right();
                    UserInputAction::None
                }
                KeyCode::Home => {
                    buffer.move_home();
                    UserInputAction::None
                }
                KeyCode::End => {
                    buffer.move_end();
                    UserInputAction::None
                }
                _ => UserInputAction::None,
            },
        }
    }

    pub(crate) fn desired_height(&self) -> u16 {
        self.desired_height_for_width(usize::MAX)
    }

    pub(crate) fn desired_height_for_width(&self, width: usize) -> u16 {
        match &self.mode {
            BottomPaneMode::Composer => {
                composer_desired_height(&self.composer_prompt, self.composer.text(), width)
            }
            BottomPaneMode::Approval { request, .. } => approval_desired_height(request, width),
            BottomPaneMode::UserInput { request, buffer } => {
                composer_desired_height(&format!("{} ", request.prompt), buffer.text(), width)
            }
        }
    }

    fn suspend_composer(&mut self) {
        if matches!(self.mode, BottomPaneMode::Composer) {
            self.suspended_composer = Some(self.composer.snapshot());
        }
    }
}

pub(crate) fn composer_desired_height(prompt: &str, buffer: &str, width: usize) -> u16 {
    let inner_width = width.saturating_sub(2).max(1);
    let prompt_width = TextLayout::measure(prompt);
    let body_width = inner_width.saturating_sub(prompt_width).max(1);
    let display_rows = TextLayout::wrap(buffer, body_width, WrapPolicy::CodeBlock)
        .len()
        .max(1);
    3u16.saturating_add(display_rows.min(6) as u16)
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
    fn approval_defaults_to_decline_and_ctrl_c_is_explicit_cancel() {
        let mut pane = BottomPane::default();
        pane.start_approval(ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: None,
            reason: "needs approval".to_string(),
            risk_label: None,
        });

        assert!(matches!(
            pane.mode(),
            BottomPaneMode::Approval { selected: 1, .. }
        ));
        assert_eq!(
            pane.handle_approval_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL,)),
            ApprovalAction::Decide(ApprovalDecision {
                approved: false,
                reason: Some("cancelled by user (Ctrl+C)".to_string()),
            })
        );
    }

    #[test]
    fn empty_composer_uses_compact_height() {
        let pane = BottomPane::default();

        assert_eq!(pane.desired_height_for_width(58), 4);
    }

    #[test]
    fn narrow_approval_prioritizes_decision_over_command_height() {
        let mut pane = BottomPane::default();
        pane.start_approval(ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: "C:\\Users\\24763\\YunXi Agent\\very\\long\\workspace".to_string(),
            command: Some(
                "Remove-Item -Recurse -Force C:\\Users\\24763\\YunXi Agent\\generated".to_string(),
            ),
            reason: "destructive command requires explicit approval".to_string(),
            risk_label: Some("risk: destructive".to_string()),
        });

        assert!(matches!(
            pane.mode(),
            BottomPaneMode::Approval { selected: 1, .. }
        ));
        assert!(pane.desired_height_for_width(58) <= 10);
    }

    #[test]
    fn composer_height_accounts_for_long_cjk_display_width() {
        let mut pane = BottomPane::default();
        pane.paste("这是一段很长的中文输入用于验证窄终端自动换行高度不会覆盖底部提示");

        assert!(pane.desired_height_for_width(58) > 4);
        assert!(pane.desired_height_for_width(58) <= 9);
    }

    #[test]
    fn composer_height_accounts_for_long_unbroken_token() {
        let mut pane = BottomPane::default();
        pane.paste(
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );

        assert!(pane.desired_height_for_width(32) > 4);
    }

    #[test]
    fn composer_backspace_removes_complete_emoji_and_combining_graphemes() {
        let mut pane = BottomPane::default();
        pane.paste("中文👩‍💻e\u{301}");

        pane.handle_composer_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        pane.handle_composer_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(matches!(pane.mode(), BottomPaneMode::Composer));
        assert_eq!(pane.composer_buffer().text(), "中文");
        assert_eq!(pane.composer_buffer().cursor_grapheme(), 2);
    }

    #[test]
    fn composer_draft_survives_approval_and_user_input_views() {
        let mut pane = BottomPane::default();
        pane.paste("草稿👩‍💻");
        let snapshot = pane.composer_snapshot();
        pane.start_approval(ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: None,
            reason: "needs approval".to_string(),
            risk_label: None,
        });
        pane.start_composer("yunxi> ");
        assert_eq!(pane.composer_snapshot(), snapshot);

        pane.start_user_input(UserInputRequestView {
            id: None,
            prompt: "details".to_string(),
        });
        pane.paste("answer\r\n第二行");
        let response =
            pane.handle_user_input_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            response,
            UserInputAction::Submit(UserInputResponse {
                value: Some("answer\n第二行".to_string()),
            })
        );
        pane.start_composer("yunxi> ");
        assert_eq!(pane.composer_snapshot(), snapshot);
    }

    #[test]
    fn active_turn_draft_accepts_edits_but_not_submit_or_escape() {
        let mut pane = BottomPane::default();
        assert!(
            pane.handle_composer_draft_key(KeyEvent::new(KeyCode::Char('中'), KeyModifiers::NONE,))
        );
        assert!(
            !pane.handle_composer_draft_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        );
        assert!(!pane.handle_composer_draft_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,)));
        assert_eq!(pane.composer_buffer().text(), "中");
    }

    #[test]
    fn reset_composer_is_the_only_explicit_draft_reset_path() {
        let mut pane = BottomPane::default();
        pane.paste("keep me");
        pane.start_composer("next> ");
        assert_eq!(pane.composer_buffer().text(), "keep me");

        pane.reset_composer("fresh> ");
        assert!(pane.composer_buffer().is_empty());
        assert_eq!(pane.composer_prompt(), "fresh> ");
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
