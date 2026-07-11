use crate::commands::{InteractiveCommand, help_text, parse_interactive_command};
use crate::render::{InteractiveBanner, print_banner, render_agent_result};
use crate::run_agent_backend;
use anyhow::{Context, Result, bail};
use std::io::{self, BufRead, IsTerminal, Write};
use yunxi_agent_core::{AgentConfig, AgentRunResult, BackendKind};
use yunxi_agent_storage::{FileSessionStore, SessionId, SessionStore};

#[derive(Clone, Debug)]
pub(crate) struct InteractiveOptions {
    pub config: AgentConfig,
    pub backend: BackendKind,
    pub provider_live: bool,
}

#[derive(Clone, Debug)]
struct InteractiveSession {
    config: AgentConfig,
    backend: BackendKind,
    provider_live: bool,
    active_session_id: Option<String>,
    turn_count: usize,
}

pub(crate) async fn run_interactive(options: InteractiveOptions) -> Result<()> {
    if options.provider_live && !live_provider_credentials_configured() {
        bail!(
            "live provider requires YUNXI_PROVIDER_API_KEY or the environment variable named by YUNXI_PROVIDER_API_KEY_ENV"
        );
    }

    let stdin_is_terminal = io::stdin().is_terminal();
    let mut session = InteractiveSession::new(options);
    session.print_banner();
    session.read_eval_loop(stdin_is_terminal).await
}

impl InteractiveSession {
    fn new(options: InteractiveOptions) -> Self {
        Self {
            config: options.config,
            backend: options.backend,
            provider_live: options.provider_live,
            active_session_id: None,
            turn_count: 0,
        }
    }

    fn print_banner(&self) {
        print_banner(&InteractiveBanner {
            cwd: self.config.cwd.display().to_string(),
            backend: format!("{:?}", self.backend).to_ascii_lowercase(),
            provider_live: self.provider_live,
            model: self.config.model.clone(),
            provider: self.config.provider.clone(),
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
            InteractiveCommand::Model(model) => self.handle_model_command(model),
            InteractiveCommand::Provider(provider) => self.handle_provider_command(provider),
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
            if self.provider_live { "live" } else { "offline" }
        );
        println!(
            "provider: {}",
            self.config.provider.as_deref().unwrap_or("default")
        );
        println!("model: {}", self.config.model.as_deref().unwrap_or("default"));
    }

    fn handle_model_command(&mut self, model: Option<String>) {
        if let Some(model) = model {
            self.config.model = Some(model);
        }
        println!("model: {}", self.config.model.as_deref().unwrap_or("default"));
    }

    fn handle_provider_command(&mut self, provider: Option<String>) {
        if let Some(provider) = provider {
            self.config.provider = Some(provider);
        }
        println!(
            "provider: {}",
            self.config.provider.as_deref().unwrap_or("default")
        );
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

        let provider_live = self.provider_live;
        let backend = self.backend;
        let turn = run_agent_backend(backend, turn_config, prompt, provider_live);

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

fn live_provider_credentials_configured() -> bool {
    if std::env::var("YUNXI_PROVIDER_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    let env_name = std::env::var("YUNXI_PROVIDER_API_KEY_ENV")
        .unwrap_or_else(|_| "OPENAI_API_KEY".to_string());
    std::env::var(env_name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}
