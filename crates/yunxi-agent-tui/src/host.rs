use crate::app::{YunxiTuiApp, YunxiTuiBanner};
use crate::bottom_pane::{
    ApprovalAction, ApprovalDecision, ApprovalRequestView, ComposerAction, UserInputAction,
    UserInputRequestView, UserInputResponse,
};
use crate::render::render_tui_frame;
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange, Event,
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, read,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

pub struct YunxiTui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: YunxiTuiApp,
    _guard: TerminalGuard,
}

impl YunxiTui {
    pub fn enter() -> Result<Self> {
        let guard = TerminalGuard::enter()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self {
            terminal,
            app: YunxiTuiApp::default(),
            _guard: guard,
        })
    }

    pub fn set_banner(&mut self, banner: YunxiTuiBanner) -> Result<()> {
        self.app.set_banner(banner);
        self.draw()
    }

    pub fn clear_transcript(&mut self) -> Result<()> {
        self.app.clear_transcript();
        self.draw()
    }

    pub fn push_agent_event(&mut self, event: &yunxi_agent_core::AgentEvent) -> Result<()> {
        self.app.push_agent_event(event);
        self.draw()
    }

    pub fn push_assistant(&mut self, content: &str) -> Result<()> {
        self.app.push_assistant(content);
        self.draw()
    }

    pub fn push_notice(&mut self, kind: &str, message: &str) -> Result<()> {
        self.app.push_notice(kind, message);
        self.draw()
    }

    pub fn push_warning(&mut self, message: &str) -> Result<()> {
        self.app.push_warning(message);
        self.draw()
    }

    pub fn push_error(&mut self, message: &str) -> Result<()> {
        self.app.push_error(message);
        self.draw()
    }

    pub fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>> {
        self.app.start_prompt(prompt);
        loop {
            self.draw()?;
            match read()? {
                Event::Key(key) if is_ctrl_d(key) => return Ok(None),
                Event::Key(key) => match self.app.bottom_pane_mut().handle_composer_key(key) {
                    ComposerAction::None => {}
                    ComposerAction::Cancel => return Ok(None),
                    ComposerAction::Submit(value) => {
                        if !value.trim().is_empty() {
                            self.app.push_user(value.clone());
                        }
                        self.draw()?;
                        return Ok(Some(value));
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    pub fn request_approval(&mut self, request: ApprovalRequestView) -> Result<ApprovalDecision> {
        self.app.start_approval(request);
        loop {
            self.draw()?;
            match read()? {
                Event::Key(key) => match self.app.bottom_pane_mut().handle_approval_key(key) {
                    ApprovalAction::None => {}
                    ApprovalAction::Decide(decision) => {
                        self.app.push_notice(
                            "approval",
                            if decision.approved {
                                "approved"
                            } else {
                                "declined"
                            },
                        );
                        self.app.start_prompt("yunxi> ");
                        self.draw()?;
                        return Ok(decision);
                    }
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    pub fn request_user_input(
        &mut self,
        request: UserInputRequestView,
    ) -> Result<UserInputResponse> {
        self.app.start_user_input(request);
        loop {
            self.draw()?;
            match read()? {
                Event::Key(key) => match self.app.bottom_pane_mut().handle_user_input_key(key) {
                    UserInputAction::None => {}
                    UserInputAction::Cancel => {
                        self.app.start_prompt("yunxi> ");
                        self.draw()?;
                        return Ok(UserInputResponse { value: None });
                    }
                    UserInputAction::Submit(response) => {
                        self.app.start_prompt("yunxi> ");
                        self.draw()?;
                        return Ok(response);
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    fn draw(&mut self) -> Result<()> {
        self.terminal
            .draw(|frame| render_tui_frame(frame, &self.app))
            .map(|_| ())?;
        Ok(())
    }
}

fn is_ctrl_d(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('d')
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange,
            Hide
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            Show,
            DisableFocusChange,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}
