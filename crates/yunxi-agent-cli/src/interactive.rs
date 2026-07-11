use crate::commands::{InteractiveCommand, help_text, parse_interactive_command};
use crate::provider_mode::{ProviderMode, ProviderSelection};
use crate::render::{InteractiveBanner, print_banner, render_agent_result};
use crate::run_agent_backend;
use anyhow::{Context, Result};
use std::io::{self, BufRead, IsTerminal, Write};
use yunxi_agent_core::{AgentConfig, AgentRunResult, BackendKind};
use yunxi_agent_storage::{FileSessionStore, SessionId, SessionStore};

#[derive(Clone, Debug)]
pub(crate) struct InteractiveOptions {
    pub config: AgentConfig,
    pub backend: BackendKind,
    pub provider_mode: ProviderMode,
}

#[derive(Clone, Debug)]
struct InteractiveSession {
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    provider_selection: ProviderSelection,
    active_session_id: Option<String>,
    turn_count: usize,
}

pub(crate) async fn run_interactive(options: InteractiveOptions) -> Result<()> {
    let stdin_is_terminal = io::stdin().is_terminal();
    let mut session = InteractiveSession::new(options)?;
    session.print_banner();
    session.read_eval_loop(stdin_is_terminal).await
}

impl InteractiveSession {
    fn new(options: InteractiveOptions) -> Result<Self> {
        let provider_selection = options
            .provider_mode
            .resolve(options.backend, &options.config)?;
        Ok(Self {
            config: options.config,
            backend: options.backend,
            provider_mode: options.provider_mode,
            provider_selection,
            active_session_id: None,
            turn_count: 0,
        })
    }

    fn print_banner(&self) {
        print_banner(&InteractiveBanner {
            cwd: self.config.cwd.display().to_string(),
            backend: format!("{:?}", self.backend).to_ascii_lowercase(),
            provider_live: self.provider_selection.live,
            provider_source: self.provider_selection.source.as_str().to_string(),
            model: self.provider_selection.model.clone(),
            provider: self.provider_selection.provider.clone(),
        });
    }

    async fn read_eval_loop(&mut self, stdin_is_terminal: bool) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        let mut stdout = io::stdout();

        loop {
            if stdin_is_terminal {
                print!("yunxi> ");
                stdout.flush()?;
            }

            let mut input = String::new();
            let bytes = reader
                .read_line(&mut input)
                .context("failed to read interactive input")?;
            if bytes == 0 {
                if !stdin_is_terminal {
                    println!("YunXi interactive session ended.");
                }
                return Ok(());
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            if let Some(command) = parse_interactive_command(input) {
                if !self.handle_command(command).await? {
                    println!("YunXi interactive session ended.");
                    return Ok(());
                }
                continue;
            }

            self.run_turn(input.to_string()).await?;
        }
    }

    async fn handle_command(&mut self, command: InteractiveCommand) -> Result<bool> {
        match command {
            InteractiveCommand::Exit => return Ok(false),
            InteractiveCommand::Help => println!("{}", help_text()),
            InteractiveCommand::Clear => println!("---"),
            InteractiveCommand::Cwd => println!("{}", self.config.cwd.display()),
            InteractiveCommand::Session => self.print_session_summary(),
            InteractiveCommand::Model(model) => self.handle_model_command(model)?,
            InteractiveCommand::Provider(provider) => self.handle_provider_command(provider)?,
            InteractiveCommand::Resume(session_id) => self.resume_session(session_id).await?,
            InteractiveCommand::Unknown(message) => println!("{message}"),
        }
        Ok(true)
    }

    fn print_session_summary(&self) {
        println!(
            "session: {}",
            self.active_session_id.as_deref().unwrap_or("new")
        );
        println!("turns: {}", self.turn_count);
        println!("cwd: {}", self.config.cwd.display());
        println!(
            "provider_mode: {}",
            if self.provider_selection.live {
                "live"
            } else {
                "offline"
            }
        );
        println!(
            "provider_source: {}",
            self.provider_selection.source.as_str()
        );
        println!("provider: {}", self.provider_selection.provider);
        println!("model: {}", self.provider_selection.model);
    }

    fn handle_model_command(&mut self, model: Option<String>) -> Result<()> {
        if let Some(model) = model {
            self.config.model = Some(model);
        }
        self.refresh_provider_selection()?;
        println!("model: {}", self.provider_selection.model);
        Ok(())
    }

    fn handle_provider_command(&mut self, provider: Option<String>) -> Result<()> {
        if let Some(provider) = provider {
            self.config.provider = Some(provider);
        }
        self.refresh_provider_selection()?;
        println!("provider: {}", self.provider_selection.provider);
        Ok(())
    }

    async fn resume_session(&mut self, session_id: String) -> Result<()> {
        let store = FileSessionStore::for_workspace(&self.config.cwd);
        let Some(record) = store
            .load(&SessionId::new(session_id.clone()))
            .await
            .context("failed to load session for interactive resume")?
        else {
            println!("session not found: {session_id}");
            return Ok(());
        };

        if self.config.model.is_none() {
            self.config.model = record.model;
        }
        if self.config.provider.is_none() {
            self.config.provider = record.provider;
        }
        self.active_session_id = Some(session_id.clone());
        self.turn_count = 0;
        self.refresh_provider_selection()?;
        println!("resumed session: {session_id}");
        Ok(())
    }

    async fn run_turn(&mut self, prompt: String) -> Result<()> {
        let mut turn_config = self.config.clone();
        if let Some(parent_session_id) = &self.active_session_id {
            turn_config = turn_config
                .with_parent_session_id(parent_session_id.clone())
                .with_session_title(format!("Interactive turn {}", self.turn_count + 1));
        } else {
            turn_config = turn_config.with_session_title("YunXi interactive session");
        }

        let backend = self.backend;
        self.provider_selection = self.provider_mode.resolve(backend, &turn_config)?;
        let turn_config = self.provider_selection.apply_to_config(turn_config);
        let turn = run_agent_backend(backend, turn_config, prompt, self.provider_selection.live);

        tokio::select! {
            result = turn => {
                let result = result?;
                render_agent_result(&result)?;
                self.update_session_state(&result);
            }
            signal = tokio::signal::ctrl_c() => {
                match signal {
                    Ok(()) => println!("[cancelled] current turn cancelled"),
                    Err(error) => println!("[cancelled] current turn cancelled; signal error: {error}"),
                }
            }
        }
        Ok(())
    }

    fn update_session_state(&mut self, result: &AgentRunResult) {
        if let Some(session_id) = session_id_from_result(result) {
            self.active_session_id = Some(session_id);
        }
        self.turn_count = self.turn_count.saturating_add(1);
    }

    fn refresh_provider_selection(&mut self) -> Result<()> {
        self.provider_selection = self.provider_mode.resolve(self.backend, &self.config)?;
        Ok(())
    }
}

fn session_id_from_result(result: &AgentRunResult) -> Option<String> {
    result.events.iter().rev().find_map(|event| match event {
        yunxi_agent_core::AgentEvent::StorageState {
            session_id: Some(session_id),
            ..
        } => Some(session_id.clone()),
        yunxi_agent_core::AgentEvent::ThreadState { state } => state.session_id.clone(),
        _ => None,
    })
}
