use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::PathBuf;
use yunxi_agent_core::{Agent, AgentConfig, AgentInput};

#[derive(Debug, Parser)]
#[command(name = "yunxi-agent-cli")]
#[command(about = "Run the extracted YunXi Agent core")]
struct Cli {
    #[arg(long, value_name = "PATH", default_value = ".")]
    cwd: PathBuf,

    #[arg(long, value_name = "MODEL")]
    model: Option<String>,

    #[arg(long, value_name = "PROVIDER")]
    provider: Option<String>,

    #[arg(long)]
    json: bool,

    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        bail!("a prompt is required");
    }

    let mut config = AgentConfig::new(cli.cwd);
    if let Some(model) = cli.model {
        config = config.with_model(model);
    }
    if let Some(provider) = cli.provider {
        config = config.with_provider(provider);
    }

    let agent = Agent::new(config);
    let result = agent
        .run_dry(AgentInput::text(prompt))
        .await
        .context("agent run failed")?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        println!("{final_response}");
    }

    Ok(())
}
