use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, BackendKind};
use yunxi_agent_weixin::{
    PRODUCTION_ILINK_ENDPOINT, WeixinAccountId, WeixinAccountMetadata, WeixinConnectionState,
};

use crate::provider_mode::ProviderMode;

#[derive(Debug, Subcommand)]
pub(crate) enum WeixinCommand {
    #[command(about = "Validate the login command skeleton; real QR login is not available yet")]
    Login {
        #[arg(long, default_value = "default")]
        account: String,
    },
    #[command(about = "Show the non-secret local Weixin capability status")]
    Status {
        #[arg(long, default_value = "default")]
        account: String,
    },
    #[command(about = "Run offline configuration diagnostics without contacting Weixin")]
    Doctor {
        #[arg(long, default_value = "default")]
        account: String,
    },
    #[command(about = "Validate future service configuration without starting long polling")]
    Serve {
        #[arg(long, default_value = "default")]
        account: String,
        #[arg(long, value_name = "PATH")]
        workspace: Option<PathBuf>,
    },
    #[command(about = "Inspect or validate future remote pairing operations")]
    Pair {
        #[command(subcommand)]
        command: WeixinPairCommand,
    },
    #[command(about = "Validate logout confirmation; credential removal is not available yet")]
    Logout {
        #[arg(long, default_value = "default")]
        account: String,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum WeixinPairCommand {
    List {
        #[arg(long, default_value = "default")]
        account: String,
    },
    Approve {
        #[arg(value_name = "PAIR_ID")]
        pair_id: String,
        #[arg(long, default_value = "default")]
        account: String,
    },
    Deny {
        #[arg(value_name = "PAIR_ID")]
        pair_id: String,
        #[arg(long, default_value = "default")]
        account: String,
    },
}

pub(crate) async fn run(
    command: WeixinCommand,
    mut config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json_output: bool,
) -> Result<()> {
    match command {
        WeixinCommand::Login { account } => {
            let account = safe_account(&account);
            bail!(
                "weixin login is not implemented in v2.1.2; real QR login is planned for v2.1.3 (account={account})"
            );
        }
        WeixinCommand::Status { account } => print_status(&account, json_output),
        WeixinCommand::Doctor { account } => print_doctor(&account, json_output),
        WeixinCommand::Serve { account, workspace } => {
            if let Some(workspace) = workspace {
                let metadata = std::fs::metadata(&workspace).with_context(|| {
                    format!(
                        "weixin serve workspace does not exist: {}",
                        workspace.display()
                    )
                })?;
                if !metadata.is_dir() {
                    bail!(
                        "weixin serve workspace must be a directory: {}",
                        workspace.display()
                    );
                }
                config.cwd = workspace
                    .canonicalize()
                    .context("failed to normalize weixin serve workspace")?;
            }
            let selection = provider_mode.resolve(backend, &config)?;
            let prepared_config = selection.apply_to_config(config);
            let account = safe_account(&account);
            bail!(
                "weixin serve is not implemented in v2.1.2; configuration was validated without starting long polling or the agent runtime (account={account}, workspace={}, provider_mode={})",
                prepared_config.cwd.display(),
                selection.source.as_str()
            );
        }
        WeixinCommand::Pair { command } => match command {
            WeixinPairCommand::List { account } => print_pair_list(&account, json_output),
            WeixinPairCommand::Approve { pair_id, account } => {
                let _ = pair_id;
                let account = safe_account(&account);
                bail!(
                    "weixin pair approve is not implemented in v2.1.2; remote approval is planned for a later version (account={account})"
                );
            }
            WeixinPairCommand::Deny { pair_id, account } => {
                let _ = pair_id;
                let account = safe_account(&account);
                bail!(
                    "weixin pair deny is not implemented in v2.1.2; remote approval is planned for a later version (account={account})"
                );
            }
        },
        WeixinCommand::Logout { account, confirm } => {
            let account = safe_account(&account);
            if !confirm {
                bail!("weixin logout requires --confirm (account={account})");
            }
            bail!(
                "weixin logout is not implemented in v2.1.2 because credential storage is planned for v2.1.4 (account={account})"
            );
        }
    }
}

fn print_status(account: &str, json_output: bool) -> Result<()> {
    let metadata = WeixinAccountMetadata {
        account_id: WeixinAccountId::new(account),
        connection_state: WeixinConnectionState::NotConfigured,
        private_chat_only: true,
        credentials_persisted: false,
    };
    let safe_account = metadata.account_id.to_string();
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "version": env!("CARGO_PKG_VERSION"),
                "account": safe_account,
                "state": metadata.connection_state,
                "endpoint": PRODUCTION_ILINK_ENDPOINT,
                "capabilities": {
                    "real_login": false,
                    "receive_messages": false,
                    "send_messages": false,
                    "persistent_service": false,
                    "group_chat": false,
                },
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin account: {safe_account}");
        println!("state: not_configured");
        println!("scope: private-chat design only; group chat disabled");
        println!("real login, messaging, and persistent service: unavailable in v2.1.2");
    }
    Ok(())
}

fn print_doctor(account: &str, json_output: bool) -> Result<()> {
    let safe_account = safe_account(account);
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": safe_account,
        "checks": {
            "rust_crate": "ready",
            "fixed_production_endpoint": true,
            "deterministic_mock_contract": true,
            "credentials_configured": false,
            "real_login_available": false,
            "group_chat_enabled": false,
        },
        "network_request_performed": false,
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin doctor: protocol and CLI skeleton ready");
        println!("account: {safe_account}");
        println!("network request performed: false");
        println!("real login and messaging: unavailable in v2.1.2");
    }
    Ok(())
}

fn print_pair_list(account: &str, json_output: bool) -> Result<()> {
    let account = safe_account(account);
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "account": account,
                "pairs": [],
                "pairing_available": false,
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin pairs: none");
        println!("account: {account}");
        println!("pairing is unavailable in v2.1.2");
    }
    Ok(())
}

fn safe_account(account: &str) -> String {
    WeixinAccountId::new(account).to_string()
}
