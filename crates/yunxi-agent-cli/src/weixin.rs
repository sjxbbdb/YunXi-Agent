use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use yunxi_agent_core::{AgentConfig, BackendKind};
use yunxi_agent_weixin::{
    IlinkHttpClient, LoginPollState, PRODUCTION_ILINK_ENDPOINT, SystemWeixinSecretStore,
    WeixinAccountId, WeixinAccountMetadata, WeixinAccountRecord, WeixinAccountStore,
    WeixinConnectionState, WeixinCredentialReference, WeixinLoginCancellation, WeixinLoginEvent,
    WeixinLoginOptions, WeixinLoginOutcome, WeixinLoginStateMachine, WeixinSecretStore,
    WeixinSecretStoreError, generate_data_key,
};

use crate::provider_mode::ProviderMode;

#[derive(Debug, Subcommand)]
pub(crate) enum WeixinCommand {
    #[command(about = "Log in with a QR code and save a system credential reference")]
    Login {
        #[arg(long, default_value = "default")]
        account: String,
    },
    #[command(about = "Show non-secret local Weixin login status")]
    Status {
        #[arg(long, default_value = "default")]
        account: String,
    },
    #[command(about = "Run offline Weixin login and credential diagnostics")]
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
    #[command(about = "Inspect future remote pairing operations")]
    Pair {
        #[command(subcommand)]
        command: WeixinPairCommand,
    },
    #[command(about = "Delete the selected Weixin credential and metadata")]
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
        WeixinCommand::Login { account } => run_login(&account, &config, json_output).await,
        WeixinCommand::Status { account } => print_status(&account, &config.cwd, json_output),
        WeixinCommand::Doctor { account } => print_doctor(&account, &config.cwd, json_output),
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
                "weixin serve is not implemented in v2.1.3; configuration was validated without starting long polling or the agent runtime (account={account}, workspace={}, provider_mode={})",
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
                    "weixin pair approve is not implemented in v2.1.3; remote approval is planned for a later version (account={account})"
                );
            }
            WeixinPairCommand::Deny { pair_id, account } => {
                let _ = pair_id;
                let account = safe_account(&account);
                bail!(
                    "weixin pair deny is not implemented in v2.1.3; remote approval is planned for a later version (account={account})"
                );
            }
        },
        WeixinCommand::Logout { account, confirm } => {
            if !confirm {
                let account = safe_account(&account);
                bail!("weixin logout requires --confirm (account={account})");
            }
            run_logout(&account, &config.cwd, json_output)
        }
    }
}

async fn run_login(account: &str, config: &AgentConfig, json_output: bool) -> Result<()> {
    if json_output {
        bail!(
            "weixin login requires interactive output; omit --json so the QR is only displayed in the terminal"
        );
    }
    let account_id = WeixinAccountId::new(account);
    let mut client = IlinkHttpClient::new(account, None)
        .context("weixin login could not initialize the fixed iLink client")?;
    let cancellation = WeixinLoginCancellation::default();
    let signal_cancellation = cancellation.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_cancellation.cancel();
        }
    });
    let machine = WeixinLoginStateMachine::new(WeixinLoginOptions::default());
    let result = machine
        .run(&mut client, &cancellation, |event| {
            print_login_event(&account_id, event);
        })
        .await;
    signal_task.abort();
    let outcome = result.context("weixin QR login failed")?;
    let store = SystemWeixinSecretStore::new();
    let credential = persist_login(&store, &account_id, outcome, &config.cwd)?;
    println!("weixin login succeeded for {}", safe_account(account));
    println!("credential backend: {}", credential.backend);
    println!(
        "metadata: {}",
        WeixinAccountStore::new(&config.cwd)
            .path_for(&account_id)
            .display()
    );
    println!("messages and long polling remain disabled in v2.1.3");
    Ok(())
}

fn persist_login<S: WeixinSecretStore>(
    store: &S,
    account_id: &WeixinAccountId,
    outcome: WeixinLoginOutcome,
    workspace: &Path,
) -> Result<WeixinCredentialReference> {
    let data_key = generate_data_key().context("weixin data key generation failed")?;
    let reference = store.credential_reference(account_id);
    store
        .put_data_key(account_id, &data_key)
        .context("weixin secure data-key storage is unavailable")?;
    let credential =
        match store.put_token(account_id, &outcome.bot_token, &reference.data_key_target) {
            Ok(credential) => credential,
            Err(error) => {
                let _ = store.delete_data_key(account_id);
                return Err(anyhow::anyhow!(
                    "weixin secure token storage failed: {error}"
                ));
            }
        };
    let account_store = WeixinAccountStore::new(workspace);
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let record =
        match WeixinAccountRecord::new(account_id, credential.clone(), workspace, now_millis) {
            Ok(record) => record,
            Err(error) => {
                let _ = store.delete_token(account_id);
                let _ = store.delete_data_key(account_id);
                return Err(anyhow::anyhow!("weixin account metadata failed: {error}"));
            }
        };
    if let Err(error) = account_store.save(account_id, &record) {
        let token_cleanup = store.delete_token(account_id);
        let key_cleanup = store.delete_data_key(account_id);
        if token_cleanup.is_err() || key_cleanup.is_err() {
            return Err(anyhow::anyhow!(
                "weixin account metadata failed and secure credential cleanup failed"
            ));
        }
        return Err(anyhow::anyhow!("weixin account metadata failed: {error}"));
    }
    Ok(credential)
}

fn print_login_event(account_id: &WeixinAccountId, event: WeixinLoginEvent) {
    match event {
        WeixinLoginEvent::QrReady { display } => {
            println!("weixin login account: {account_id}");
            println!("endpoint: {PRODUCTION_ILINK_ENDPOINT}");
            println!("scan this QR text in WeChat (displayed only in the terminal):");
            println!("{}", display.terminal_text());
        }
        WeixinLoginEvent::PollState { state } => {
            println!("weixin login state: {}", login_state_label(state));
        }
        WeixinLoginEvent::Cancelled => println!("weixin login cancelled; no credential was saved"),
        WeixinLoginEvent::TimedOut => println!("weixin login timed out; run login again"),
        WeixinLoginEvent::Failed { failure } => println!("weixin login failed: {failure}"),
    }
}

fn login_state_label(state: LoginPollState) -> &'static str {
    match state {
        LoginPollState::Wait => "waiting for scan",
        LoginPollState::Scanned => "scanned; waiting for confirmation",
        LoginPollState::Confirmed => "confirmed",
        LoginPollState::ScannedButRedirect => "scanned but redirect is unsupported",
        LoginPollState::BoundRedirect => "redirect binding is unsupported",
        LoginPollState::Expired => "expired",
        LoginPollState::NeedVerifyCode => "verification code required",
        LoginPollState::VerifyCodeBlocked => "verification code blocked",
    }
}

fn print_status(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account_id = WeixinAccountId::new(account);
    let account_store = WeixinAccountStore::new(workspace);
    let Some(record) = account_store.load(&account_id)? else {
        let metadata = WeixinAccountMetadata {
            account_id,
            connection_state: WeixinConnectionState::NotConfigured,
            private_chat_only: true,
            credentials_persisted: false,
        };
        return print_unconfigured_status(metadata, json_output);
    };
    let store = SystemWeixinSecretStore::new();
    let credential_state = credential_state(&store, &account_id);
    let account = record.account_id.clone();
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": account,
        "state": record.connection_state,
        "endpoint": record.endpoint,
        "schema_version": record.schema_version,
        "credential_backend": record.credential.backend,
        "credential_reference_present": true,
        "credential_state": credential_state,
        "metadata_path": account_store.path_for(&account_id),
        "workspace_id": record.workspace_id,
        "created_at_millis": record.created_at_millis,
        "updated_at_millis": record.updated_at_millis,
        "capabilities": {
            "real_login": true,
            "receive_messages": false,
            "send_messages": false,
            "persistent_service": false,
            "group_chat": false,
        },
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin account: {account}");
        println!("state: ready");
        println!("credential state: {credential_state}");
        println!(
            "metadata: {}",
            account_store.path_for(&account_id).display()
        );
        println!("QR login is available; messages and long polling remain disabled");
    }
    Ok(())
}

fn print_unconfigured_status(metadata: WeixinAccountMetadata, json_output: bool) -> Result<()> {
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
                "credential_state": "not_configured",
                "capabilities": {
                    "real_login": true,
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
        println!("QR login is available; messages and long polling remain disabled");
    }
    Ok(())
}

fn credential_state<S: WeixinSecretStore>(store: &S, account_id: &WeixinAccountId) -> &'static str {
    match (store.get_token(account_id), store.get_data_key(account_id)) {
        (Ok(_), Ok(_)) => "present",
        (Err(WeixinSecretStoreError::Unavailable), _)
        | (_, Err(WeixinSecretStoreError::Unavailable)) => "unavailable",
        (Err(WeixinSecretStoreError::PermissionDenied), _)
        | (_, Err(WeixinSecretStoreError::PermissionDenied)) => "permission_denied",
        _ => "missing",
    }
}

fn print_doctor(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account_id = WeixinAccountId::new(account);
    let account_store = WeixinAccountStore::new(workspace);
    let record = account_store.load(&account_id)?;
    let store = SystemWeixinSecretStore::new();
    let credential_state = credential_state(&store, &account_id);
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": safe_account(account),
        "checks": {
            "rust_crate": "ready",
            "fixed_production_endpoint": true,
            "qr_login_state_machine": true,
            "account_metadata": record.is_some(),
            "credential_store": credential_state,
            "credentials_configured": record.is_some() && credential_state == "present",
            "real_login_available": true,
            "message_receive_enabled": false,
            "message_send_enabled": false,
            "group_chat_enabled": false,
        },
        "metadata_path": record.as_ref().map(|_| account_store.path_for(&account_id)),
        "network_request_performed": false,
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin doctor: QR login and secure credential checks ready");
        println!("account: {}", safe_account(account));
        println!("credential state: {credential_state}");
        println!("network request performed: false");
        println!("message receive, send, long polling, and group chat: unavailable in v2.1.3");
    }
    Ok(())
}

fn run_logout(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account_id = WeixinAccountId::new(account);
    let account_store = WeixinAccountStore::new(workspace);
    if account_store.load(&account_id)?.is_none() {
        return print_logout_result(&account_id, false, json_output);
    }
    let store = SystemWeixinSecretStore::new();
    delete_secret_if_present(&store, &account_id, true)
        .context("weixin secure credential deletion failed")?;
    delete_secret_if_present(&store, &account_id, false)
        .context("weixin secure data-key deletion failed")?;
    account_store
        .delete(&account_id)
        .context("weixin account metadata deletion failed")?;
    print_logout_result(&account_id, true, json_output)
}

fn delete_secret_if_present<S: WeixinSecretStore>(
    store: &S,
    account_id: &WeixinAccountId,
    token: bool,
) -> Result<()> {
    let result = if token {
        store.delete_token(account_id)
    } else {
        store.delete_data_key(account_id)
    };
    match result {
        Ok(()) | Err(WeixinSecretStoreError::NotFound) => Ok(()),
        Err(error) => Err(anyhow::anyhow!(error)),
    }
}

fn print_logout_result(
    account_id: &WeixinAccountId,
    removed: bool,
    json_output: bool,
) -> Result<()> {
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": account_id.to_string(),
        "removed": removed,
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if removed {
        println!("weixin credential and metadata removed for {account_id}");
    } else {
        println!("weixin account {account_id} was not configured");
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
        println!("pairing is unavailable in v2.1.3");
    }
    Ok(())
}

fn safe_account(account: &str) -> String {
    WeixinAccountId::new(account).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use yunxi_agent_weixin::{FakeWeixinSecretStore, SecretString, WeixinSecretStore};

    fn outcome() -> WeixinLoginOutcome {
        WeixinLoginOutcome {
            bot_token: SecretString::new("bot-token-secret"),
            ilink_bot_id: Some(SecretString::new("bot-id-secret")),
            ilink_user_id: Some(SecretString::new("user-id-secret")),
            base_url: Some(PRODUCTION_ILINK_ENDPOINT.to_string()),
        }
    }

    #[test]
    fn login_persistence_writes_only_safe_metadata_after_secure_secret_store() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new("private-account-name");
        let store = FakeWeixinSecretStore::new();
        let credential = persist_login(&store, &account, outcome(), workspace.path())
            .expect("fake secure login persistence");
        let record = WeixinAccountStore::new(workspace.path())
            .load(&account)
            .expect("metadata read")
            .expect("metadata record");
        let json = serde_json::to_string(&record).expect("metadata JSON");
        assert_eq!(record.credential, credential);
        assert!(!json.contains("bot-token-secret"));
        assert!(!json.contains("user-id-secret"));
        assert!(store.get_token(&account).is_ok());
        assert!(store.get_data_key(&account).is_ok());
    }

    #[test]
    fn login_persistence_rejects_unavailable_secure_store_without_metadata() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new("private-account-name");
        let store = FakeWeixinSecretStore::unavailable();
        let error = persist_login(&store, &account, outcome(), workspace.path())
            .expect_err("secure store failure must stop login");
        let text = error.to_string();
        assert!(text.contains("secure data-key storage"));
        assert!(!text.contains("bot-token-secret"));
        assert!(
            WeixinAccountStore::new(workspace.path())
                .load(&account)
                .expect("metadata read")
                .is_none()
        );
    }
}
