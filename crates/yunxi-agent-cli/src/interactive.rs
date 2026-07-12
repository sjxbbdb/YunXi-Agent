use crate::commands::{InteractiveCommand, help_text, parse_interactive_command};
use crate::provider_mode::{ProviderMode, ProviderSelection};
use crate::render::{InteractiveBanner, RenderState, print_banner, render_agent_event};
use crate::{redact_secret_fragments, run_agent_backend_stream};
use anyhow::{Context, Result};
use std::io::{self, BufRead, IsTerminal, Write};
use yunxi_agent_core::{
    AgentConfig, AgentRunApprovalDecision, AgentRunApprovalRequest, AgentRunControl,
    AgentRunResult, AgentRunUserInputRequest, AgentRunUserInputResponse, BackendKind,
};
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

            if let Err(error) = self
                .run_turn(input.to_string(), &mut reader, &mut stdout)
                .await
            {
                eprintln!(
                    "[error] {}",
                    redact_secret_fragments(&format!("{error:#}"))
                );
            }
        }
    }

    async fn handle_command(&mut self, command: InteractiveCommand) -> Result<bool> {
        match command {
            InteractiveCommand::Exit => return Ok(false),
            InteractiveCommand::Help => println!("{}", help_text()),
            InteractiveCommand::Clear => {
                print!("\x1b[2J\x1b[H");
                io::stdout().flush()?;
            }
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

    async fn run_turn<R>(
        &mut self,
        prompt: String,
        reader: &mut R,
        stdout: &mut io::Stdout,
    ) -> Result<()>
    where
        R: BufRead,
    {
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
        let (control, mut stream) = AgentRunControl::streaming();
        let run_control = control.clone();
        let mut control_slot = Some(control);
        let mut turn = Box::pin(run_agent_backend_stream(
            backend,
            turn_config,
            prompt,
            self.provider_selection.live,
            run_control,
        ));
        let mut render_state = RenderState::default();
        let mut result: Option<AgentRunResult> = None;
        let mut events_open = true;
        let mut approvals_open = true;
        let mut user_inputs_open = true;

        loop {
            if result.is_some() && !events_open && !approvals_open && !user_inputs_open {
                break;
            }

            tokio::select! {
                event = stream.events.recv(), if events_open => {
                    match event {
                        Some(event) => render_agent_event(&event, &mut render_state)?,
                        None => events_open = false,
                    }
                }
                request = stream.approvals.recv(), if approvals_open => {
                    match request {
                        Some(request) => respond_to_approval_request(request, reader, stdout)?,
                        None => approvals_open = false,
                    }
                }
                request = stream.user_inputs.recv(), if user_inputs_open => {
                    match request {
                        Some(request) => respond_to_user_input_request(request, reader, stdout)?,
                        None => user_inputs_open = false,
                    }
                }
                signal = tokio::signal::ctrl_c(), if control_slot.is_some() => {
                    match signal {
                        Ok(()) => {
                            if let Some(control) = &control_slot {
                                control.cancel();
                            }
                            println!("[cancelled] cancellation requested");
                        }
                        Err(error) => println!("[cancelled] cancellation requested; signal error: {error}"),
                    }
                }
                turn_result = &mut turn, if result.is_none() => {
                    let completed = turn_result.context("interactive turn failed")?;
                    result = Some(completed);
                    control_slot = None;
                }
            }
        }
        if let Some(result) = result {
            if !render_state.saw_assistant_message()
                && let Some(final_response) = &result.final_response
            {
                println!("{final_response}");
            }
            self.update_session_state(&result);
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

fn respond_to_approval_request<R>(
    request: AgentRunApprovalRequest,
    reader: &mut R,
    stdout: &mut io::Stdout,
) -> Result<()>
where
    R: BufRead,
{
    println!(
        "[approval] {} requires approval in {}",
        request.tool_name, request.cwd
    );
    if let Some(command) = &request.command {
        println!("[approval] command: {command}");
    }
    println!("[approval] reason: {}", request.reason);
    print!("approve? y/N: ");
    stdout.flush()?;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .context("failed to read approval response")?;
    let approved = matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes" | "approve" | "approved"
    );
    let reason = if approved {
        Some("approved by YunXi interactive CLI".to_string())
    } else {
        Some("declined by YunXi interactive CLI".to_string())
    };
    let _ = request
        .respond_to
        .send(AgentRunApprovalDecision { approved, reason });
    Ok(())
}

fn respond_to_user_input_request<R>(
    request: AgentRunUserInputRequest,
    reader: &mut R,
    stdout: &mut io::Stdout,
) -> Result<()>
where
    R: BufRead,
{
    print!("{} ", request.prompt);
    stdout.flush()?;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .context("failed to read requested user input")?;
    let value = line.trim_end_matches(['\r', '\n']).to_string();
    let _ = request.respond_to.send(AgentRunUserInputResponse {
        value: Some(value),
    });
    Ok(())
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
