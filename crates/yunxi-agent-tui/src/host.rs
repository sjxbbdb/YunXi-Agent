use crate::app::{YunxiTuiApp, YunxiTuiBanner};
use crate::bottom_pane::{
    ApprovalAction, ApprovalDecision, ApprovalRequestView, ComposerAction, UserInputAction,
    UserInputRequestView, UserInputResponse,
};
use crate::frame::FrameScheduler;
use crate::render::render_tui_frame;
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseEventKind, poll, read,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Size;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

pub struct YunxiTui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: YunxiTuiApp,
    frame: FrameScheduler,
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
            frame: FrameScheduler::default(),
            _guard: guard,
        })
    }

    pub fn set_banner(&mut self, banner: YunxiTuiBanner) -> Result<()> {
        self.app.set_banner(banner);
        self.request_draw_now()
    }

    pub fn clear_transcript(&mut self) -> Result<()> {
        self.app.clear_transcript();
        self.request_draw_now()
    }

    pub fn push_agent_event(&mut self, event: &yunxi_agent_core::AgentEvent) -> Result<()> {
        self.app.push_agent_event(event);
        self.request_draw()
    }

    pub fn push_assistant(&mut self, content: &str) -> Result<()> {
        self.app.push_assistant(content);
        self.request_draw()
    }

    pub fn push_notice(&mut self, kind: &str, message: &str) -> Result<()> {
        self.app.push_notice(kind, message);
        self.request_draw_now()
    }

    pub fn push_warning(&mut self, message: &str) -> Result<()> {
        self.app.push_warning(message);
        self.request_draw_now()
    }

    pub fn push_error(&mut self, message: &str) -> Result<()> {
        self.app.push_error(message);
        self.request_draw_now()
    }

    pub fn tick(&mut self) -> Result<()> {
        self.drain_navigation_events()?;
        self.flush_frame(Instant::now())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.request_draw_now()
    }

    pub fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>> {
        self.app.start_prompt(prompt);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
                Event::Key(key) if is_ctrl_d(key) => return Ok(None),
                Event::Key(key) => match self.app.bottom_pane_mut().handle_composer_key(key) {
                    ComposerAction::None => {}
                    ComposerAction::Cancel => return Ok(None),
                    ComposerAction::Submit(value) => {
                        if !value.trim().is_empty() {
                            self.app.push_user(value.clone());
                        }
                        self.request_draw_now()?;
                        return Ok(Some(value));
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    pub fn request_approval(&mut self, request: ApprovalRequestView) -> Result<ApprovalDecision> {
        self.app.start_approval(request);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
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
                        self.request_draw_now()?;
                        return Ok(decision);
                    }
                },
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    pub fn request_user_input(
        &mut self,
        request: UserInputRequestView,
    ) -> Result<UserInputResponse> {
        self.app.start_user_input(request);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
                Event::Key(key) => match self.app.bottom_pane_mut().handle_user_input_key(key) {
                    UserInputAction::None => {}
                    UserInputAction::Cancel => {
                        self.app.start_prompt("yunxi> ");
                        self.request_draw_now()?;
                        return Ok(UserInputResponse { value: None });
                    }
                    UserInputAction::Submit(response) => {
                        self.app.start_prompt("yunxi> ");
                        self.request_draw_now()?;
                        return Ok(response);
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    fn request_draw(&mut self) -> Result<()> {
        self.frame.mark_dirty();
        self.flush_frame(Instant::now())
    }

    fn request_draw_now(&mut self) -> Result<()> {
        self.frame.force();
        self.flush_frame(Instant::now())
    }

    fn flush_frame(&mut self, now: Instant) -> Result<()> {
        if self.frame.should_draw(now) {
            self.draw(now)?;
        }
        Ok(())
    }

    fn draw(&mut self, now: Instant) -> Result<()> {
        self.terminal
            .draw(|frame| render_tui_frame(frame, &self.app))
            .map(|_| ())?;
        self.frame.record_draw(now);
        Ok(())
    }

    fn drain_navigation_events(&mut self) -> Result<()> {
        let mut changed = false;
        while poll(Duration::ZERO)? {
            let event = read()?;
            changed |= self.handle_navigation_event(&event)?;
        }
        if changed {
            self.frame.force();
        }
        Ok(())
    }

    fn handle_navigation_event(&mut self, event: &Event) -> Result<bool> {
        let visible_height = self.transcript_visible_height()?;
        match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.app.scroll_up(3, visible_height);
                    Ok(true)
                }
                MouseEventKind::ScrollDown => {
                    self.app.scroll_down(3, visible_height);
                    Ok(true)
                }
                _ => Ok(false),
            },
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::PageUp => {
                    self.app.page_up(visible_height);
                    Ok(true)
                }
                KeyCode::PageDown => {
                    self.app.page_down(visible_height);
                    Ok(true)
                }
                KeyCode::Home => {
                    self.app.jump_top(visible_height);
                    Ok(true)
                }
                KeyCode::End => {
                    self.app.follow_tail();
                    Ok(true)
                }
                _ => Ok(false),
            },
            Event::Resize(_, _) => Ok(true),
            _ => Ok(false),
        }
    }

    fn transcript_visible_height(&self) -> Result<usize> {
        let size = self.terminal.size()?;
        Ok(transcript_visible_height(
            size,
            self.app.bottom_pane().desired_height(),
        ))
    }
}

fn transcript_visible_height(size: Size, bottom_pane_height: u16) -> usize {
    let bottom_height = bottom_pane_height.min(size.height.saturating_sub(4)).max(3);
    let transcript_height = size.height.saturating_sub(3).saturating_sub(bottom_height);
    transcript_height.saturating_sub(2).max(1) as usize
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
            EnableMouseCapture,
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
            DisableMouseCapture,
            DisableFocusChange,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_visible_height_matches_tui_layout() {
        assert_eq!(
            transcript_visible_height(
                Size {
                    width: 100,
                    height: 18,
                },
                3,
            ),
            10
        );
        assert_eq!(
            transcript_visible_height(
                Size {
                    width: 100,
                    height: 6,
                },
                3,
            ),
            1
        );
    }
}
