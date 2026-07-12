use crate::render::{InteractiveBanner, InteractiveRenderer, RenderState};
use crate::tui::app::TuiApp;
use crate::tui::render::render_tui_frame;
use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};
use yunxi_agent_core::AgentEvent;

pub(crate) struct TuiInteractiveRenderer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: Option<TuiApp>,
    _guard: TerminalGuard,
}

impl TuiInteractiveRenderer {
    pub(crate) fn new() -> Result<Self> {
        let guard = TerminalGuard::enter()?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            app: None,
            _guard: guard,
        })
    }

    fn draw(&mut self) -> Result<()> {
        if let Some(app) = &self.app {
            self.terminal
                .draw(|frame| render_tui_frame(frame, app))
                .map(|_| ())?;
        }
        Ok(())
    }
}

impl InteractiveRenderer for TuiInteractiveRenderer {
    fn banner(&mut self, banner: &InteractiveBanner) -> Result<()> {
        self.app = Some(TuiApp::from_banner(banner));
        self.draw()
    }

    fn warning(&mut self, message: &str) -> Result<()> {
        if let Some(app) = &mut self.app {
            app.push_warning(message);
        }
        self.draw()
    }

    fn event(&mut self, event: &AgentEvent, _state: &mut RenderState) -> Result<()> {
        if let Some(app) = &mut self.app {
            app.push_event(event);
        }
        self.draw()
    }

    fn error(&mut self, message: &str) -> Result<()> {
        if let Some(app) = &mut self.app {
            app.push_error(message);
        }
        self.draw()
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
