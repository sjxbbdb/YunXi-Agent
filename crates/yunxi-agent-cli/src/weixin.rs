use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use yunxi_agent_core::{AgentConfig, BackendKind, DryRunBackend};
use yunxi_agent_storage::{
    FileSessionStore, FileWeixinStateStore, SessionId, SessionStore, WeixinAccountLockState,
    WeixinCredentialReferenceRecord, WeixinLatencyTraceRecord, WeixinPairRequest, WeixinStateError,
    WeixinStateSnapshot, WeixinStateStore,
};
use yunxi_agent_voice::{VoiceClientConfig, VoiceRuntimeClient};
use yunxi_agent_weixin::{
    IlinkHttpClient, LoginPollState, PRODUCTION_ILINK_ENDPOINT, SystemWeixinSecretStore,
    WeixinAccountId, WeixinAccountMetadata, WeixinAccountRecord, WeixinAccountStore,
    WeixinAccountStoreError, WeixinConnectionState, WeixinCredentialReference,
    WeixinLoginCancellation, WeixinLoginEvent, WeixinLoginOptions, WeixinLoginOutcome,
    WeixinLoginStateMachine, WeixinLoginTransport, WeixinRuntimeDispatcherAdapter,
    WeixinSecretStore, WeixinSecretStoreError, WeixinServeCancellation, WeixinServeOptions,
    WeixinServeReport, WeixinServeStoppedReason, WeixinTurnSupervisor, WeixinTurnSupervisorOptions,
    WeixinVoiceBridge, WeixinVoiceTranscriber, generate_data_key, run_weixin_serve_loop,
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
    #[command(about = "Run the foreground Weixin private-chat long polling service")]
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
    #[command(about = "Manage local Weixin conversation sessions")]
    Session {
        #[command(subcommand)]
        command: WeixinSessionCommand,
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

#[derive(Debug, Subcommand)]
pub(crate) enum WeixinSessionCommand {
    Reset {
        #[arg(long, default_value = "default")]
        account: String,
        #[arg(long, value_name = "PEER_HASH")]
        peer: String,
        #[arg(long)]
        confirm: bool,
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
            run_serve(&account, config, backend, provider_mode, json_output).await
        }
        WeixinCommand::Pair { command } => match command {
            WeixinPairCommand::List { account } => {
                print_pair_list(&account, &config.cwd, json_output)
            }
            WeixinPairCommand::Approve { pair_id, account } => {
                run_pair_transition(&account, &config.cwd, &pair_id, true, json_output)
            }
            WeixinPairCommand::Deny { pair_id, account } => {
                run_pair_transition(&account, &config.cwd, &pair_id, false, json_output)
            }
        },
        WeixinCommand::Session { command } => match command {
            WeixinSessionCommand::Reset {
                account,
                peer,
                confirm,
            } => {
                if !confirm {
                    bail!("weixin session reset requires --confirm");
                }
                run_session_reset(&account, &config.cwd, &peer, json_output).await
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
    let mut client = IlinkHttpClient::new(account, None)
        .context("weixin login could not initialize the fixed iLink client")?;
    let cancellation = WeixinLoginCancellation::default();
    let signal_cancellation = cancellation.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_cancellation.cancel();
        }
    });
    let store = SystemWeixinSecretStore::new();
    let account_store = WeixinAccountStore::new(&config.cwd);
    let mut stdout = std::io::stdout();
    let result = run_login_with_dependencies(
        account,
        &config.cwd,
        json_output,
        &mut client,
        &store,
        &account_store,
        &cancellation,
        WeixinLoginStateMachine::new(WeixinLoginOptions::default()),
        &mut stdout,
    )
    .await;
    signal_task.abort();
    result.map(|_| ())
}

async fn run_serve(
    account: &str,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json_output: bool,
) -> Result<()> {
    let invocation = crate::prepare_runtime_invocation(config, backend, provider_mode)?;
    let selection = invocation.selection;
    let prepared_config = invocation.config;
    let account_id = WeixinAccountId::new(account);
    let account_hash = account_id.to_string();
    let account_store = WeixinAccountStore::new(&prepared_config.cwd);
    let state_store = FileWeixinStateStore::for_workspace(&prepared_config.cwd);
    let record = account_store
        .load(&account_id)
        .context("weixin serve account metadata load failed")?
        .ok_or_else(|| {
            anyhow::anyhow!("weixin serve requires login first (account={account_hash})")
        })?;
    let (state, _migration) =
        ensure_weixin_state_initialized_from_metadata(&record, &state_store, now_millis_u64())
            .context("weixin serve state initialization failed")?;
    let store = SystemWeixinSecretStore::new();
    let token = store.get_token(&account_id).with_context(|| {
        format!("weixin serve token credential is unavailable (account={account_hash})")
    })?;
    let data_key = store.get_data_key(&account_id).with_context(|| {
        format!("weixin serve encrypted pending queue key is unavailable (account={account_hash})")
    })?;
    let mut lock = state_store
        .try_acquire_account_lock(&record.account_id, &record.workspace_id)
        .with_context(|| {
            format!("weixin serve refused to acquire account lock (account={account_hash})")
        })?;
    let mut client = IlinkHttpClient::new(account, Some(token))
        .context("weixin serve could not initialize the fixed iLink client")?;
    let voice_runtime = VoiceRuntimeClient::new(VoiceClientConfig::from_env())
        .context("weixin serve voice runtime configuration is invalid")?;
    let voice_bridge = Arc::new(
        WeixinVoiceBridge::new(voice_runtime)
            .context("weixin serve could not initialize voice transcription")?,
    );
    let mut options = WeixinServeOptions::new(record.account_id.clone());
    options.max_polls = serve_max_polls_from_env()?;
    options.background_runtime_dispatch = true;
    options.background_delivery_dispatch = true;
    let remote_control_hub =
        yunxi_agent_weixin::WeixinRemoteControlHub::with_state_store(state_store.clone());
    let voice_transcriber: Arc<dyn WeixinVoiceTranscriber> = voice_bridge.clone();
    let supervisor_options =
        WeixinTurnSupervisorOptions::new(prepared_config.clone(), record.workspace_id.clone())
            .with_remote_control_hub(remote_control_hub.clone())
            .with_voice_transcriber(voice_transcriber);
    let delivery_sink: Arc<dyn yunxi_agent_weixin::WeixinRuntimeSink> = Arc::new(
        yunxi_agent_weixin::WeixinDeliverySpoolSink::new(state_store.clone(), data_key.clone()),
    );
    let supervisor = WeixinTurnSupervisor::new(
        state_store.clone(),
        data_key.clone(),
        supervisor_options,
        Arc::clone(&delivery_sink),
    );
    let runtime_dispatcher: Arc<dyn yunxi_agent_weixin::WeixinRuntimeDispatcher> = match backend {
        BackendKind::Yunxi => Arc::new(WeixinRuntimeDispatcherAdapter::new(
            supervisor,
            crate::build_yunxi_runtime_backend(&prepared_config, selection.live),
        )),
        BackendKind::DryRun => Arc::new(WeixinRuntimeDispatcherAdapter::new(
            supervisor,
            DryRunBackend,
        )),
        BackendKind::Codex => {
            bail!("weixin serve runtime dispatch does not support the codex backend")
        }
    };
    options.runtime_dispatcher = Some(runtime_dispatcher);
    let delivery_transport: Arc<dyn yunxi_agent_weixin::WeixinMessageTransport> =
        Arc::new(client.clone());
    options.delivery_dispatcher =
        Some(Arc::new(yunxi_agent_weixin::WeixinDeliveryDispatcher::new(
            state_store.clone(),
            data_key.clone(),
            delivery_transport,
        )));
    options.outbound_sink = Some(delivery_sink);
    options.remote_control_hub = Some(remote_control_hub);
    let cancellation = WeixinServeCancellation::default();
    if !json_output {
        println!("weixin serve started");
        println!("account: {account_hash}");
        println!("workspace: {}", record.workspace_id);
        println!("provider mode: {}", selection.source.as_str());
        println!("state store schema: {}", state.schema_version);
        println!(
            "private chat long polling, text/voice runtime dispatch, background delivery drain, encrypted sendmessage spool, and slash-command control enabled; group chat remains disabled"
        );
    }
    let serve_result = tokio::select! {
        result = run_weixin_serve_loop(&mut client, &state_store, options, &data_key, &cancellation) => result,
        signal = tokio::signal::ctrl_c() => {
            let _ = signal;
            cancellation.cancel();
            let now = now_millis_u64();
            let _ = state_store.record_poll_health(
                &record.account_id,
                yunxi_agent_storage::WeixinConnectionStateRecord::Ready,
                Some("serve_stopped".to_string()),
                now,
            );
            Ok(WeixinServeReport {
                stopped_reason: Some(WeixinServeStoppedReason::Cancelled),
                ..WeixinServeReport::default()
            })
        }
    };
    let release_result = lock.release();
    let report = serve_result.context("weixin serve loop failed")?;
    release_result.context("weixin serve account lock release failed")?;
    print_serve_report(&record.account_id, &report, json_output)
}

#[allow(clippy::too_many_arguments)]
async fn run_login_with_dependencies<T, S, W>(
    account: &str,
    workspace: &Path,
    json_output: bool,
    transport: &mut T,
    store: &S,
    account_store: &WeixinAccountStore,
    cancellation: &WeixinLoginCancellation,
    machine: WeixinLoginStateMachine,
    output: &mut W,
) -> Result<WeixinCredentialReference>
where
    T: WeixinLoginTransport,
    S: WeixinSecretStore,
    W: Write,
{
    if json_output {
        bail!(
            "weixin login requires interactive output; omit --json so the QR is only displayed in the terminal"
        );
    }

    let account_id = WeixinAccountId::new(account);
    let mut output_error = None;
    let result = machine
        .run(transport, cancellation, |event| {
            if output_error.is_none()
                && let Err(error) = write_login_event(output, &account_id, event)
            {
                output_error = Some(error);
            }
        })
        .await;
    if let Some(error) = output_error {
        return Err(error).context("weixin login output failed");
    }

    let outcome = result.context("weixin QR login failed")?;
    let credential = persist_login(store, &account_id, outcome, account_store, workspace)?;
    writeln!(
        output,
        "weixin login succeeded for {}",
        safe_account(account)
    )
    .context("weixin login output failed")?;
    writeln!(output, "credential backend: {}", credential.backend)
        .context("weixin login output failed")?;
    writeln!(
        output,
        "metadata: {}",
        account_store.path_for(&account_id).display()
    )
    .context("weixin login output failed")?;
    writeln!(
        output,
        "foreground private-chat polling, runtime dispatch, background delivery drain, and final-text sendmessage spool are available after starting weixin serve"
    )
    .context("weixin login output failed")?;
    Ok(credential)
}

fn persist_login<S: WeixinSecretStore>(
    store: &S,
    account_id: &WeixinAccountId,
    outcome: WeixinLoginOutcome,
    account_store: &WeixinAccountStore,
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
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    if let Err(error) = state_store.upsert_account_state(
        &record.account_id,
        &record.workspace_id,
        &record.endpoint,
        Some(credential_record(&credential)),
        now_millis,
    ) {
        let _ = account_store.delete(account_id);
        let _ = store.delete_token(account_id);
        let _ = store.delete_data_key(account_id);
        return Err(anyhow::anyhow!("weixin state store failed: {error}"));
    }
    Ok(credential)
}

fn write_login_event<W: Write>(
    output: &mut W,
    account_id: &WeixinAccountId,
    event: WeixinLoginEvent,
) -> std::io::Result<()> {
    match event {
        WeixinLoginEvent::QrReady { display } => {
            writeln!(output, "weixin login account: {account_id}")?;
            writeln!(output, "endpoint: {PRODUCTION_ILINK_ENDPOINT}")?;
            writeln!(
                output,
                "scan this QR text in WeChat (displayed only in the terminal):"
            )?;
            writeln!(output, "{}", display.terminal_text())?;
        }
        WeixinLoginEvent::PollState { state } => {
            writeln!(output, "weixin login state: {}", login_state_label(state))?;
        }
        WeixinLoginEvent::Cancelled => {
            writeln!(output, "weixin login cancelled; no credential was saved")?;
        }
        WeixinLoginEvent::TimedOut => {
            writeln!(output, "weixin login timed out; run login again")?;
        }
        WeixinLoginEvent::Failed { failure } => {
            writeln!(output, "weixin login failed: {failure}")?;
        }
    }
    Ok(())
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
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    let account_hash = safe_account(account);
    let Some(record) = account_store.load(&account_id)? else {
        let metadata = WeixinAccountMetadata {
            account_id,
            connection_state: WeixinConnectionState::NotConfigured,
            private_chat_only: true,
            credentials_persisted: false,
        };
        let state = state_store.load(&account_hash)?;
        let lock_state = state_store.lock_state(&account_hash)?;
        return print_unconfigured_status(metadata, state.as_ref(), &lock_state, json_output);
    };
    let store = SystemWeixinSecretStore::new();
    let credential_state = credential_state(&store, &account_id);
    let account = record.account_id.clone();
    let now = now_millis_u64();
    let (state, state_store_migration) =
        ensure_weixin_state_initialized_from_metadata(&record, &state_store, now)
            .context("weixin legacy metadata state initialization failed")?;
    let lock_state = state_store.lock_state(&account)?;
    let state_connection = format!("{:?}", state.connection_state).to_ascii_lowercase();
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": account,
        "state": state_connection,
        "endpoint": record.endpoint,
        "account_schema_version": record.schema_version,
        "state_store_schema_version": state.schema_version,
        "state_store_configured": true,
        "state_store_migration": state_store_migration.as_str(),
        "credential_backend": record.credential.backend,
        "credential_reference_present": true,
        "credential_state": credential_state,
        "workspace_id": record.workspace_id,
        "account_lock_state": lock_state.state.as_str(),
        "pending_inbound_count": state.pending_inbound_count(),
        "pending_delivery_count": state.pending_delivery_count(),
        "pending_remote_control_count": state.pending_remote_control_count_at(now),
        "pair_request_count": state.pair_request_count(),
        "latency_trace_count": state.latency_traces.len(),
        "recent_latency_traces": latency_trace_reports(&state, 5),
        "last_redacted_error": state.last_redacted_error.clone(),
        "created_at_millis": record.created_at_millis,
        "updated_at_millis": record.updated_at_millis,
        "capabilities": {
            "real_login": true,
            "receive_messages": true,
            "send_messages": true,
            "send_message_mode": "final_text_spool",
            "voice_input": true,
            "voice_reply_mode": "text_only",
            "voice_runtime_health_checked": false,
            "background_delivery_dispatch": true,
            "foreground_long_polling": true,
            "runtime_dispatch": true,
            "persistent_service": false,
            "remote_approval": true,
            "remote_user_input": true,
            "remote_cancellation": true,
            "remote_control_mode": "slash_command_agent_run_control",
            "group_chat": false,
        },
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin account: {account}");
        println!("state: {state_connection}");
        println!("credential state: {credential_state}");
        println!("state store configured: true");
        if state_store_migration == WeixinStateInitialization::InitializedFromLegacyMetadata {
            println!("state store initialized from existing metadata");
        }
        println!("account lock state: {}", lock_state.state.as_str());
        print_latency_summary(&state);
        println!(
            "QR login, foreground private-chat polling, runtime dispatch, background delivery drain, final-text sendmessage spool, and slash-command AgentRunControl are available"
        );
    }
    Ok(())
}

fn print_unconfigured_status(
    metadata: WeixinAccountMetadata,
    state: Option<&WeixinStateSnapshot>,
    lock_state: &yunxi_agent_storage::WeixinAccountLockInfo,
    json_output: bool,
) -> Result<()> {
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
                "state_store_schema_version": state.map(|state| state.schema_version),
                "state_store_configured": state.is_some(),
                "state_store_migration": "not_applicable",
                "account_lock_state": lock_state.state.as_str(),
                "pending_inbound_count": state.map(|state| state.pending_inbound_count()).unwrap_or(0),
                "pending_delivery_count": state.map(|state| state.pending_delivery_count()).unwrap_or(0),
                "pending_remote_control_count": state.map(|state| state.pending_remote_control_count_at(now_millis_u64())).unwrap_or(0),
                "pair_request_count": state.map(|state| state.pair_request_count()).unwrap_or(0),
                "latency_trace_count": state.map(|state| state.latency_traces.len()).unwrap_or(0),
                "recent_latency_traces": state.map(|state| latency_trace_reports(state, 5)).unwrap_or_default(),
                "last_redacted_error": state.and_then(|state| state.last_redacted_error.clone()),
                "capabilities": {
                    "real_login": true,
                    "receive_messages": false,
                    "send_messages": false,
                    "send_message_mode": "requires_configured_account",
                    "background_delivery_dispatch": false,
                    "foreground_long_polling": true,
                    "runtime_dispatch": false,
                    "persistent_service": false,
                    "remote_approval": false,
                    "remote_user_input": false,
                    "remote_cancellation": false,
                    "remote_control_mode": "requires_configured_account",
                    "group_chat": false,
                },
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin account: {safe_account}");
        println!("state: not_configured");
        println!("state store configured: {}", state.is_some());
        println!("account lock state: {}", lock_state.state.as_str());
        println!(
            "QR login is available; foreground private-chat polling requires a configured account"
        );
    }
    Ok(())
}

fn latency_trace_reports(state: &WeixinStateSnapshot, limit: usize) -> Vec<serde_json::Value> {
    state
        .recent_latency_traces(limit)
        .iter()
        .map(latency_trace_report)
        .collect()
}

fn latency_trace_report(trace: &WeixinLatencyTraceRecord) -> serde_json::Value {
    json!({
        "item_id": trace.item_id.clone(),
        "message_id_hash": trace.message_id_hash.clone(),
        "peer_id_hash": trace.peer_id_hash.clone(),
        "session_id": trace.session_id.clone(),
        "terminal_status": trace.terminal_status.clone(),
        "terminal_reason": trace.terminal_reason.clone(),
        "delivery_status": trace.delivery_status.clone(),
        "delivery_segment_count": trace.delivery_segment_count,
        "delivery_attempt_count": trace.delivery_attempt_count,
        "delivery_success_count": trace.delivery_success_count,
        "error_label": trace.error_label.clone(),
        "timestamps": {
            "poll_started_at_millis": trace.poll_started_at_millis,
            "poll_completed_at_millis": trace.poll_completed_at_millis,
            "inbound_committed_at_millis": trace.inbound_committed_at_millis,
            "dispatch_claimed_at_millis": trace.dispatch_claimed_at_millis,
            "runtime_started_at_millis": trace.runtime_started_at_millis,
            "runtime_completed_at_millis": trace.runtime_completed_at_millis,
            "spool_written_at_millis": trace.spool_written_at_millis,
            "turn_completed_at_millis": trace.turn_completed_at_millis,
            "delivery_started_at_millis": trace.delivery_started_at_millis,
            "delivery_completed_at_millis": trace.delivery_completed_at_millis,
        },
        "durations_millis": {
            "total": trace.total_latency_millis(),
            "poll": trace.poll_latency_millis(),
            "queue": trace.queue_latency_millis(),
            "runtime": trace.runtime_latency_millis(),
            "spool": trace.spool_latency_millis(),
            "delivery_wait": trace.delivery_wait_millis(),
            "delivery": trace.delivery_latency_millis(),
        },
    })
}

fn print_latency_summary(state: &WeixinStateSnapshot) {
    let Some(trace) = state.recent_latency_traces(1).into_iter().next() else {
        return;
    };
    let status = trace.terminal_status.as_deref().unwrap_or("in_progress");
    let delivery_status = trace.delivery_status.as_deref().unwrap_or("not_started");
    println!(
        "last latency trace: item={} total={} poll={} queue={} runtime={} spool={} delivery_wait={} delivery={} status={} delivery_status={}",
        trace.item_id,
        duration_label(trace.total_latency_millis()),
        duration_label(trace.poll_latency_millis()),
        duration_label(trace.queue_latency_millis()),
        duration_label(trace.runtime_latency_millis()),
        duration_label(trace.spool_latency_millis()),
        duration_label(trace.delivery_wait_millis()),
        duration_label(trace.delivery_latency_millis()),
        status,
        delivery_status,
    );
    if let Some(error_label) = trace.error_label.as_deref() {
        println!("last latency error: {error_label}");
    }
}

fn duration_label(duration_millis: Option<u64>) -> String {
    duration_millis
        .map(|duration| format!("{duration}ms"))
        .unwrap_or_else(|| "n/a".to_string())
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WeixinStateInitialization {
    AlreadyCurrent,
    InitializedFromLegacyMetadata,
}

impl WeixinStateInitialization {
    fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyCurrent => "already_current",
            Self::InitializedFromLegacyMetadata => "initialized_from_legacy_metadata",
        }
    }
}

fn ensure_weixin_state_initialized_from_metadata(
    record: &WeixinAccountRecord,
    state_store: &FileWeixinStateStore,
    now_millis: u64,
) -> Result<(WeixinStateSnapshot, WeixinStateInitialization), WeixinStateError> {
    if let Some(state) = state_store.load(&record.account_id)? {
        return Ok((state, WeixinStateInitialization::AlreadyCurrent));
    }
    let state = state_store.upsert_account_state(
        &record.account_id,
        &record.workspace_id,
        &record.endpoint,
        Some(credential_record(&record.credential)),
        now_millis,
    )?;
    Ok((
        state,
        WeixinStateInitialization::InitializedFromLegacyMetadata,
    ))
}

fn print_doctor(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account_id = WeixinAccountId::new(account);
    let account_store = WeixinAccountStore::new(workspace);
    let account_hash = safe_account(account);
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    let record = match account_store.load(&account_id) {
        Ok(record) => record,
        Err(error) => return print_doctor_metadata_error(account, error, json_output),
    };
    let mut state_store_migration = "not_applicable";
    let state_result = match record.as_ref() {
        Some(record) => {
            match ensure_weixin_state_initialized_from_metadata(
                record,
                &state_store,
                now_millis_u64(),
            ) {
                Ok((state, migration)) => {
                    state_store_migration = migration.as_str();
                    Ok(Some(state))
                }
                Err(error) => Err(error),
            }
        }
        None => state_store.load(&account_hash),
    };
    let state = match &state_result {
        Ok(state) => state.as_ref(),
        Err(_) => None,
    };
    let lock_state = state_store.lock_state(&account_hash)?;
    let store = SystemWeixinSecretStore::new();
    let credential_state = credential_state(&store, &account_id);
    let state_store_schema = match &state_result {
        Ok(Some(_)) => "current",
        Ok(None) => "missing",
        Err(WeixinStateError::FutureSchema { .. }) => "future_schema",
        Err(WeixinStateError::InvalidJson { .. }) => "invalid_json",
        Err(_) => "unavailable",
    };
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": safe_account(account),
        "checks": {
            "rust_crate": "ready",
            "fixed_production_endpoint": true,
            "qr_login_state_machine": true,
            "account_metadata": record.is_some(),
            "state_store": state_store_schema,
            "state_store_schema_current": matches!(state_result, Ok(Some(_))),
            "state_store_migration": state_store_migration,
            "credential_store": credential_state,
            "credentials_configured": record.is_some() && credential_state == "present",
            "real_login_available": true,
            "account_lock_state": lock_state.state.as_str(),
            "private_chat_enabled": true,
            "encrypted_pending_queue": record.is_some() && credential_state == "present",
            "message_receive_enabled": record.is_some() && credential_state == "present",
            "runtime_dispatch_enabled": record.is_some() && credential_state == "present",
            "message_send_enabled": record.is_some() && credential_state == "present",
            "message_send_mode": "final_text_spool",
            "voice_input_enabled": record.is_some() && credential_state == "present",
            "voice_reply_mode": "text_only",
            "voice_runtime_health_checked": false,
            "background_delivery_dispatch": record.is_some() && credential_state == "present",
            "remote_approval_enabled": record.is_some() && credential_state == "present",
            "remote_user_input_enabled": record.is_some() && credential_state == "present",
            "remote_cancellation_enabled": record.is_some() && credential_state == "present",
            "remote_control_mode": "slash_command_agent_run_control",
            "group_chat_enabled": false,
        },
        "state_store_schema_version": state.map(|state| state.schema_version),
        "pending_inbound_count": state.map(|state| state.pending_inbound_count()).unwrap_or(0),
        "pending_delivery_count": state.map(|state| state.pending_delivery_count()).unwrap_or(0),
        "pending_remote_control_count": state.map(|state| state.pending_remote_control_count_at(now_millis_u64())).unwrap_or(0),
        "pair_request_count": state.map(|state| state.pair_request_count()).unwrap_or(0),
        "latency_trace_count": state.map(|state| state.latency_traces.len()).unwrap_or(0),
        "recent_latency_traces": state.map(|state| latency_trace_reports(state, 5)).unwrap_or_default(),
        "last_redacted_error": state.and_then(|state| state.last_redacted_error.clone()),
        "network_request_performed": false,
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin doctor: QR login and secure credential checks ready");
        println!("account: {}", safe_account(account));
        println!("credential state: {credential_state}");
        println!("state store: {state_store_schema}");
        if state_store_migration
            == WeixinStateInitialization::InitializedFromLegacyMetadata.as_str()
        {
            println!("state store initialized from existing metadata");
        }
        println!("account lock state: {}", lock_state.state.as_str());
        println!("network request performed: false");
        println!(
            "private-chat foreground receive/runtime dispatch/sendmessage spool/slash control: {}; group chat: unavailable",
            if record.is_some() && credential_state == "present" {
                "available"
            } else {
                "requires configured credentials"
            }
        );
    }
    Ok(())
}

fn print_doctor_metadata_error(
    account: &str,
    error: WeixinAccountStoreError,
    json_output: bool,
) -> Result<()> {
    let safe_error = metadata_error_label(&error);
    let report = json!({
        "channel": "weixin",
        "version": env!("CARGO_PKG_VERSION"),
        "account": safe_account(account),
        "checks": {
            "rust_crate": "ready",
            "fixed_production_endpoint": true,
            "qr_login_state_machine": true,
            "account_metadata": false,
            "account_metadata_error": safe_error,
            "state_store": "not_checked",
            "state_store_schema_current": false,
            "state_store_migration": "not_attempted_metadata_error",
            "credential_store": "not_checked",
            "credentials_configured": false,
            "real_login_available": true,
            "account_lock_state": "not_checked",
            "private_chat_enabled": true,
            "encrypted_pending_queue": false,
            "message_receive_enabled": false,
            "runtime_dispatch_enabled": false,
            "message_send_enabled": false,
            "message_send_mode": "requires_valid_metadata",
            "background_delivery_dispatch": false,
            "remote_approval_enabled": false,
            "remote_user_input_enabled": false,
            "remote_cancellation_enabled": false,
            "remote_control_mode": "requires_valid_metadata",
            "group_chat_enabled": false,
        },
        "state_store_schema_version": serde_json::Value::Null,
        "pending_inbound_count": 0,
        "pending_delivery_count": 0,
        "pending_remote_control_count": 0,
        "pair_request_count": 0,
        "latency_trace_count": 0,
        "recent_latency_traces": [],
        "last_redacted_error": safe_error,
        "network_request_performed": false,
        "secrets_included": false,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("weixin doctor: account metadata is invalid");
        println!("account: {}", safe_account(account));
        println!("metadata error: {safe_error}");
        println!("network request performed: false");
    }
    Ok(())
}

fn metadata_error_label(error: &WeixinAccountStoreError) -> &'static str {
    match error {
        WeixinAccountStoreError::Io { .. } => "metadata_io_error",
        WeixinAccountStoreError::InvalidJson => "metadata_invalid_json",
        WeixinAccountStoreError::InvalidRecord => "metadata_invalid_record",
        WeixinAccountStoreError::WorkspaceUnavailable => "metadata_workspace_unavailable",
    }
}

fn run_logout(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account_id = WeixinAccountId::new(account);
    let account_store = WeixinAccountStore::new(workspace);
    let account_hash = safe_account(account);
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    if state_store.lock_state(&account_hash)?.state == WeixinAccountLockState::Active {
        bail!(
            "weixin logout refused because a local service holds the account lock (account={account_hash})"
        );
    }
    let metadata_exists = account_store.load(&account_id)?.is_some();
    let state_exists = state_store.load(&account_hash)?.is_some();
    if !metadata_exists && !state_exists {
        return print_logout_result(&account_id, false, json_output);
    }
    state_store
        .delete(&account_hash)
        .context("weixin state deletion failed")?;
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

fn print_pair_list(account: &str, workspace: &Path, json_output: bool) -> Result<()> {
    let account = safe_account(account);
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    let state = state_store.load(&account)?;
    let pairs = state
        .as_ref()
        .map(|state| state.pair_requests.clone())
        .unwrap_or_default();
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "account": account,
                "pairs": pairs,
                "pairing_available": true,
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin pairs: {}", pairs.len());
        println!("account: {account}");
        println!(
            "pairing lifecycle is local-state only; slash-command control is available only for approved private chats during foreground serve"
        );
    }
    Ok(())
}

fn run_pair_transition(
    account: &str,
    workspace: &Path,
    pair_id: &str,
    approve: bool,
    json_output: bool,
) -> Result<()> {
    let account = safe_account(account);
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    let now = now_millis_u64();
    let request = if approve {
        state_store.approve_pair_request(&account, pair_id, now)
    } else {
        state_store.deny_pair_request(&account, pair_id, now)
    }
    .with_context(|| {
        format!(
            "weixin pair {} failed for account={account}",
            if approve { "approve" } else { "deny" }
        )
    })?;
    print_pair_transition_result(&account, &request, approve, json_output)
}

fn print_pair_transition_result(
    account: &str,
    request: &WeixinPairRequest,
    approve: bool,
    json_output: bool,
) -> Result<()> {
    let action = if approve { "approve" } else { "deny" };
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "version": env!("CARGO_PKG_VERSION"),
                "account": account,
                "action": action,
                "request_id": request.request_id,
                "state": request.state,
                "expires_at_millis": request.expires_at_millis,
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin pair {action} recorded for {}", request.request_id);
        println!("account: {account}");
        println!("state: {:?}", request.state);
    }
    Ok(())
}

async fn run_session_reset(
    account: &str,
    workspace: &Path,
    peer_id_hash: &str,
    json_output: bool,
) -> Result<()> {
    if !peer_id_hash.starts_with("peer#") {
        bail!("weixin session reset requires a redacted peer hash like peer#12345678");
    }
    let account_hash = safe_account(account);
    let state_store = FileWeixinStateStore::for_workspace(workspace);
    let mut state = state_store
        .load(&account_hash)?
        .ok_or_else(|| anyhow::anyhow!("weixin state does not exist for account={account_hash}"))?;
    let mut session_ids = state
        .conversation_bindings
        .iter()
        .filter(|binding| {
            binding.account_id == account_hash && binding.peer_id_hash == peer_id_hash
        })
        .flat_map(|binding| {
            [
                Some(binding.session_id.clone()),
                binding.root_session_id.clone(),
                binding.active_session_id.clone(),
                binding.last_completed_session_id.clone(),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    session_ids.sort();
    session_ids.dedup();
    let before = state.conversation_bindings.len();
    state.conversation_bindings.retain(|binding| {
        !(binding.account_id == account_hash && binding.peer_id_hash == peer_id_hash)
    });
    let removed_bindings = before.saturating_sub(state.conversation_bindings.len());
    if removed_bindings == 0 {
        bail!(
            "weixin session reset found no binding for account={account_hash} peer={peer_id_hash}"
        );
    }
    let now = now_millis_u64();
    state.updated_at_millis = now;
    state.transitioned_at_millis = now;
    state_store
        .save(&state)
        .context("failed to update weixin session binding state")?;
    let session_store = FileSessionStore::for_workspace(workspace);
    let mut archived_sessions = Vec::new();
    let mut missing_sessions = Vec::new();
    for session_id in session_ids {
        match session_store
            .archive(&SessionId::new(session_id.clone()), true)
            .await
            .context("failed to archive YunXi session during weixin reset")?
        {
            Some(_) => archived_sessions.push(session_id),
            None => missing_sessions.push(session_id),
        }
    }
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "version": env!("CARGO_PKG_VERSION"),
                "account": account_hash,
                "peer_id_hash": peer_id_hash,
                "removed_bindings": removed_bindings,
                "archived_sessions": archived_sessions,
                "missing_sessions": missing_sessions,
                "long_term_memory_deleted": false,
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin session reset completed");
        println!("account: {account_hash}");
        println!("peer: {peer_id_hash}");
        println!("removed bindings: {removed_bindings}");
        println!("archived sessions: {}", archived_sessions.len());
        println!("long-term memory deleted: false");
    }
    Ok(())
}

fn print_serve_report(account: &str, report: &WeixinServeReport, json_output: bool) -> Result<()> {
    let stopped_reason = report
        .stopped_reason
        .map(serve_stopped_reason_label)
        .unwrap_or("running");
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "channel": "weixin",
                "version": env!("CARGO_PKG_VERSION"),
                "account": account,
                "service": "foreground_long_polling",
                "stopped_reason": stopped_reason,
                "polls": report.polls,
                "empty_polls": report.empty_polls,
                "accepted_count": report.accepted_count,
                "duplicate_count": report.duplicate_count,
                "pair_request_count": report.pair_request_count,
                "pair_prompt_count": report.pair_prompt_count,
                "pair_prompt_error_count": report.pair_prompt_error_count,
                "skipped_group_count": report.skipped_group_count,
                "skipped_self_count": report.skipped_self_count,
                "skipped_unsupported_count": report.skipped_unsupported_count,
                "skipped_unknown_count": report.skipped_unknown_count,
                "network_error_count": report.network_error_count,
                "runtime_dispatch_enabled": true,
                "runtime_dispatch_count": report.runtime_dispatch_count,
                "runtime_error_count": report.runtime_error_count,
                "runtime_deferred_count": report.runtime_deferred_count,
                "runtime_recovered_count": report.runtime_recovered_count,
                "send_message_enabled": true,
                "send_message_mode": "final_text_spool",
                "voice_input_enabled": true,
                "voice_reply_mode": "text_only",
                "background_delivery_dispatch": true,
                "delivery_attempt_count": report.delivery_attempt_count,
                "delivery_success_count": report.delivery_success_count,
                "delivery_error_count": report.delivery_error_count,
                "delivery_deferred_count": report.delivery_deferred_count,
                "delivery_unknown_outcome_count": report.delivery_unknown_outcome_count,
                "remote_approval_enabled": true,
                "remote_user_input_enabled": true,
                "remote_cancellation_enabled": true,
                "remote_control_mode": "slash_command_agent_run_control",
                "remote_control_count": report.remote_control_count,
                "remote_control_error_count": report.remote_control_error_count,
                "group_chat_enabled": false,
                "secrets_included": false,
            }))?
        );
    } else {
        println!("weixin serve stopped: {stopped_reason}");
        println!("account: {account}");
        println!("polls: {}", report.polls);
        println!("accepted pending inbound: {}", report.accepted_count);
        println!("runtime dispatches: {}", report.runtime_dispatch_count);
        println!("runtime dispatch errors: {}", report.runtime_error_count);
        println!("runtime recovered: {}", report.runtime_recovered_count);
        println!("delivery attempts: {}", report.delivery_attempt_count);
        println!("delivery succeeded: {}", report.delivery_success_count);
        println!("delivery errors: {}", report.delivery_error_count);
        println!(
            "delivery unknown outcomes: {}",
            report.delivery_unknown_outcome_count
        );
        println!("remote controls: {}", report.remote_control_count);
        println!(
            "remote control errors: {}",
            report.remote_control_error_count
        );
        println!("pair requests: {}", report.pair_request_count);
        println!("pair prompts sent: {}", report.pair_prompt_count);
        println!("pair prompt errors: {}", report.pair_prompt_error_count);
        println!("duplicates skipped: {}", report.duplicate_count);
        println!(
            "background delivery drain, final-text sendmessage spool, and slash-command AgentRunControl enabled; group chat remains disabled"
        );
    }
    Ok(())
}

fn serve_stopped_reason_label(reason: WeixinServeStoppedReason) -> &'static str {
    match reason {
        WeixinServeStoppedReason::Cancelled => "cancelled",
        WeixinServeStoppedReason::MaxPolls => "max_polls",
    }
}

fn serve_max_polls_from_env() -> Result<Option<usize>> {
    let Some(value) = std::env::var("YUNXI_WEIXIN_SERVE_MAX_POLLS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let parsed = value
        .parse::<usize>()
        .context("YUNXI_WEIXIN_SERVE_MAX_POLLS must be a positive integer")?;
    if parsed == 0 {
        bail!("YUNXI_WEIXIN_SERVE_MAX_POLLS must be greater than zero");
    }
    Ok(Some(parsed))
}

fn safe_account(account: &str) -> String {
    WeixinAccountId::new(account).to_string()
}

fn credential_record(credential: &WeixinCredentialReference) -> WeixinCredentialReferenceRecord {
    WeixinCredentialReferenceRecord {
        backend: credential.backend.clone(),
        token_target: credential.token_target.clone(),
        data_key_target: credential.data_key_target.clone(),
    }
}

fn now_millis_u64() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::time::Duration;
    use tempfile::TempDir;
    use yunxi_agent_storage::{WEIXIN_STATE_SCHEMA_VERSION, WeixinConnectionStateRecord};
    use yunxi_agent_weixin::ilink::{GetBotQrCodeResponse, GetQrCodeStatusResponse, QrCodeStatus};
    use yunxi_agent_weixin::{
        FakeWeixinSecretStore, SecretString, WeixinLoginFailure, WeixinSecretStore,
    };

    const RAW_ACCOUNT: &str = "private-account-name";
    const QR_PAYLOAD: &str = "https://qr.example/secret-payload";
    const BOT_TOKEN: &str = "bot-token-secret";
    const BOT_ID: &str = "bot-id-secret";
    const USER_ID: &str = "user-id-secret";

    struct ScriptedTransport {
        statuses: VecDeque<GetQrCodeStatusResponse>,
        fetches: usize,
        polls: usize,
    }

    impl ScriptedTransport {
        fn new(statuses: impl IntoIterator<Item = QrCodeStatus>) -> Self {
            Self {
                statuses: statuses
                    .into_iter()
                    .map(|status| GetQrCodeStatusResponse {
                        status,
                        bot_token: (status == QrCodeStatus::Confirmed)
                            .then(|| SecretString::new(BOT_TOKEN)),
                        ilink_bot_id: Some(SecretString::new(BOT_ID)),
                        baseurl: Some(PRODUCTION_ILINK_ENDPOINT.to_string()),
                        ilink_user_id: Some(SecretString::new(USER_ID)),
                        redirect_host: None,
                    })
                    .collect(),
                fetches: 0,
                polls: 0,
            }
        }
    }

    #[async_trait]
    impl WeixinLoginTransport for ScriptedTransport {
        async fn fetch_qr_code(
            &mut self,
        ) -> Result<GetBotQrCodeResponse, yunxi_agent_weixin::WeixinApiError> {
            self.fetches += 1;
            Ok(GetBotQrCodeResponse {
                qrcode: SecretString::new(QR_PAYLOAD),
                qrcode_img_content: SecretString::new("\u{1b}[31mQR\u{1b}[0m"),
            })
        }

        async fn poll_qr_status(
            &mut self,
            _qrcode: &SecretString,
            _verify_code: Option<&SecretString>,
        ) -> Result<GetQrCodeStatusResponse, yunxi_agent_weixin::WeixinApiError> {
            self.polls += 1;
            Ok(self
                .statuses
                .pop_front()
                .unwrap_or(GetQrCodeStatusResponse {
                    status: QrCodeStatus::Wait,
                    bot_token: None,
                    ilink_bot_id: None,
                    baseurl: None,
                    ilink_user_id: None,
                    redirect_host: None,
                }))
        }
    }

    fn fast_machine() -> WeixinLoginStateMachine {
        WeixinLoginStateMachine::new(
            WeixinLoginOptions::bounded(Duration::from_millis(1), Duration::from_millis(100))
                .expect("valid test options"),
        )
    }

    fn assert_secret_free(text: &str) {
        for forbidden in [
            RAW_ACCOUNT,
            QR_PAYLOAD,
            BOT_TOKEN,
            BOT_ID,
            USER_ID,
            "data-key",
        ] {
            assert!(
                !text.contains(forbidden),
                "login output leaked {forbidden:?}: {text}"
            );
        }
    }

    fn status_report_with_store<S: WeixinSecretStore>(
        store: &S,
        account_store: &WeixinAccountStore,
        account_id: &WeixinAccountId,
        workspace: &Path,
    ) -> Result<serde_json::Value> {
        let state_store = FileWeixinStateStore::for_workspace(workspace);
        let Some(record) = account_store.load(account_id)? else {
            let account = account_id.to_string();
            let state = state_store.load(&account)?;
            let lock_state = state_store.lock_state(&account)?;
            return Ok(json!({
                "channel": "weixin",
                "version": env!("CARGO_PKG_VERSION"),
                "account": account,
                "state": WeixinConnectionState::NotConfigured,
                "endpoint": PRODUCTION_ILINK_ENDPOINT,
                "credential_state": "not_configured",
                "state_store_schema_version": state.as_ref().map(|state| state.schema_version),
                "state_store_configured": state.is_some(),
                "account_lock_state": lock_state.state.as_str(),
                "pending_inbound_count": state.as_ref().map(|state| state.pending_inbound_count()).unwrap_or(0),
                "pending_delivery_count": state.as_ref().map(|state| state.pending_delivery_count()).unwrap_or(0),
                "pair_request_count": state.as_ref().map(|state| state.pair_request_count()).unwrap_or(0),
                "last_redacted_error": state.as_ref().and_then(|state| state.last_redacted_error.clone()),
                "secrets_included": false,
            }));
        };
        let state = state_store.load(&record.account_id)?;
        let lock_state = state_store.lock_state(&record.account_id)?;
        Ok(json!({
            "channel": "weixin",
            "version": env!("CARGO_PKG_VERSION"),
            "account": record.account_id,
            "state": record.connection_state,
            "endpoint": record.endpoint,
            "credential_backend": record.credential.backend,
            "credential_reference_present": true,
            "credential_state": credential_state(store, account_id),
            "state_store_schema_version": state.as_ref().map(|state| state.schema_version),
            "state_store_configured": state.is_some(),
            "account_lock_state": lock_state.state.as_str(),
            "pending_inbound_count": state.as_ref().map(|state| state.pending_inbound_count()).unwrap_or(0),
            "pending_delivery_count": state.as_ref().map(|state| state.pending_delivery_count()).unwrap_or(0),
            "pair_request_count": state.as_ref().map(|state| state.pair_request_count()).unwrap_or(0),
            "last_redacted_error": state.as_ref().and_then(|state| state.last_redacted_error.clone()),
            "secrets_included": false,
        }))
    }

    fn outcome() -> WeixinLoginOutcome {
        WeixinLoginOutcome {
            bot_token: SecretString::new(BOT_TOKEN),
            ilink_bot_id: Some(SecretString::new(BOT_ID)),
            ilink_user_id: Some(SecretString::new(USER_ID)),
            base_url: Some(PRODUCTION_ILINK_ENDPOINT.to_string()),
        }
    }

    #[tokio::test]
    async fn cli_login_helper_mock_confirmed_persists_and_stays_secret_free() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let account_store = WeixinAccountStore::new(workspace.path());
        let store = FakeWeixinSecretStore::new();
        let mut transport = ScriptedTransport::new([
            QrCodeStatus::Wait,
            QrCodeStatus::Scanned,
            QrCodeStatus::Confirmed,
        ]);
        let mut output = Vec::new();

        let credential = run_login_with_dependencies(
            RAW_ACCOUNT,
            workspace.path(),
            false,
            &mut transport,
            &store,
            &account_store,
            &WeixinLoginCancellation::default(),
            fast_machine(),
            &mut output,
        )
        .await
        .expect("mock login should succeed");

        assert_eq!(transport.fetches, 1);
        assert_eq!(transport.polls, 3);
        assert!(store.get_token(&account).is_ok());
        assert!(store.get_data_key(&account).is_ok());
        let record = account_store
            .load(&account)
            .expect("metadata read")
            .expect("metadata written");
        assert_eq!(record.credential, credential);
        let state_store = FileWeixinStateStore::for_workspace(workspace.path());
        let state = state_store
            .load(&account.to_string())
            .expect("state read")
            .expect("state written");
        assert_eq!(state.schema_version, WEIXIN_STATE_SCHEMA_VERSION);
        assert_eq!(state.account_id, account.to_string());
        assert_eq!(state.connection_state, WeixinConnectionStateRecord::Ready);
        assert_eq!(
            state.credential.as_ref().map(|c| &c.backend),
            Some(&credential.backend)
        );
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(state.pending_delivery_count(), 0);
        assert_eq!(state.pair_request_count(), 0);
        let text = String::from_utf8(output).expect("utf8 output");
        assert!(text.contains("weixin login succeeded"));
        assert!(text.contains("waiting for scan"));
        assert!(text.contains("scanned; waiting for confirmation"));
        assert_secret_free(&text);

        let status = status_report_with_store(&store, &account_store, &account, workspace.path())
            .expect("status report with fake store");
        assert_eq!(status["credential_state"].as_str(), Some("present"));
        assert_eq!(status["state_store_configured"].as_bool(), Some(true));
        assert_eq!(
            status["state_store_schema_version"].as_u64(),
            Some(WEIXIN_STATE_SCHEMA_VERSION as u64)
        );
        assert_eq!(status["pending_inbound_count"].as_u64(), Some(0));
        assert_eq!(status["pending_delivery_count"].as_u64(), Some(0));
        assert_eq!(status["pair_request_count"].as_u64(), Some(0));
        assert!(status.get("metadata_path").is_none());
        assert_eq!(status["secrets_included"].as_bool(), Some(false));
        assert_secret_free(&status.to_string());
    }

    #[tokio::test]
    async fn cli_login_helper_mock_expired_writes_no_credentials_or_metadata() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let account_store = WeixinAccountStore::new(workspace.path());
        let store = FakeWeixinSecretStore::new();
        let mut transport = ScriptedTransport::new([QrCodeStatus::Expired]);
        let mut output = Vec::new();

        let error = run_login_with_dependencies(
            RAW_ACCOUNT,
            workspace.path(),
            false,
            &mut transport,
            &store,
            &account_store,
            &WeixinLoginCancellation::default(),
            fast_machine(),
            &mut output,
        )
        .await
        .expect_err("expired login must fail");

        assert!(error.to_string().contains("weixin QR login failed"));
        assert_eq!(
            store.get_token(&account),
            Err(WeixinSecretStoreError::NotFound)
        );
        assert_eq!(
            store.get_data_key(&account),
            Err(WeixinSecretStoreError::NotFound)
        );
        assert!(
            account_store
                .load(&account)
                .expect("metadata read")
                .is_none()
        );
        assert!(
            FileWeixinStateStore::for_workspace(workspace.path())
                .load(&account.to_string())
                .expect("state read")
                .is_none()
        );
        assert_secret_free(&String::from_utf8(output).expect("utf8 output"));
    }

    #[tokio::test]
    async fn cli_login_helper_mock_cancelled_writes_no_credentials_or_metadata() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let account_store = WeixinAccountStore::new(workspace.path());
        let store = FakeWeixinSecretStore::new();
        let mut transport = ScriptedTransport::new([QrCodeStatus::Confirmed]);
        let cancellation = WeixinLoginCancellation::default();
        cancellation.cancel();
        let mut output = Vec::new();

        let error = run_login_with_dependencies(
            RAW_ACCOUNT,
            workspace.path(),
            false,
            &mut transport,
            &store,
            &account_store,
            &cancellation,
            fast_machine(),
            &mut output,
        )
        .await
        .expect_err("cancelled login must fail");

        assert!(matches!(
            error.downcast_ref::<WeixinLoginFailure>(),
            Some(WeixinLoginFailure::Cancelled)
        ));
        assert_eq!(transport.fetches, 0);
        assert_eq!(
            store.get_token(&account),
            Err(WeixinSecretStoreError::NotFound)
        );
        assert!(
            account_store
                .load(&account)
                .expect("metadata read")
                .is_none()
        );
        assert!(
            FileWeixinStateStore::for_workspace(workspace.path())
                .load(&account.to_string())
                .expect("state read")
                .is_none()
        );
        let text = String::from_utf8(output).expect("utf8 output");
        assert!(text.contains("cancelled"));
        assert_secret_free(&text);
    }

    #[tokio::test]
    async fn cli_login_helper_mock_credential_unavailable_writes_no_metadata() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let account_store = WeixinAccountStore::new(workspace.path());
        let store = FakeWeixinSecretStore::unavailable();
        let mut transport = ScriptedTransport::new([QrCodeStatus::Confirmed]);
        let mut output = Vec::new();

        let error = run_login_with_dependencies(
            RAW_ACCOUNT,
            workspace.path(),
            false,
            &mut transport,
            &store,
            &account_store,
            &WeixinLoginCancellation::default(),
            fast_machine(),
            &mut output,
        )
        .await
        .expect_err("unavailable credential store must fail");

        assert!(error.to_string().contains("secure data-key storage"));
        assert!(
            account_store
                .load(&account)
                .expect("metadata read")
                .is_none()
        );
        assert!(
            FileWeixinStateStore::for_workspace(workspace.path())
                .load(&account.to_string())
                .expect("state read")
                .is_none()
        );
        assert!(
            !String::from_utf8(output)
                .expect("utf8 output")
                .contains("login succeeded")
        );
    }

    #[tokio::test]
    async fn cli_login_helper_mock_metadata_failure_rolls_back_credentials() {
        let workspace = TempDir::new().expect("workspace");
        let blocker = workspace.path().join("not-a-directory");
        std::fs::write(&blocker, "metadata writes should fail below this file")
            .expect("blocker file");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let account_store = WeixinAccountStore::new(&blocker);
        let store = FakeWeixinSecretStore::new();
        let mut transport = ScriptedTransport::new([QrCodeStatus::Confirmed]);
        let mut output = Vec::new();

        let error = run_login_with_dependencies(
            RAW_ACCOUNT,
            &blocker,
            false,
            &mut transport,
            &store,
            &account_store,
            &WeixinLoginCancellation::default(),
            fast_machine(),
            &mut output,
        )
        .await
        .expect_err("metadata write failure must fail login");

        assert!(error.to_string().contains("account metadata failed"));
        assert_eq!(
            store.get_token(&account),
            Err(WeixinSecretStoreError::NotFound)
        );
        assert_eq!(
            store.get_data_key(&account),
            Err(WeixinSecretStoreError::NotFound)
        );
        assert!(
            FileWeixinStateStore::for_workspace(&blocker)
                .load(&account.to_string())
                .expect("state read")
                .is_none()
        );
        assert!(
            !String::from_utf8(output)
                .expect("utf8 output")
                .contains("login succeeded")
        );
    }

    #[tokio::test]
    async fn cli_login_helper_rejects_json_without_network_or_output() {
        let workspace = TempDir::new().expect("workspace");
        let account_store = WeixinAccountStore::new(workspace.path());
        let store = FakeWeixinSecretStore::new();
        let mut transport = ScriptedTransport::new([QrCodeStatus::Confirmed]);
        let mut output = Vec::new();

        let error = run_login_with_dependencies(
            RAW_ACCOUNT,
            workspace.path(),
            true,
            &mut transport,
            &store,
            &account_store,
            &WeixinLoginCancellation::default(),
            fast_machine(),
            &mut output,
        )
        .await
        .expect_err("json login must be rejected before QR fetch");

        assert!(error.to_string().contains("requires interactive output"));
        assert_eq!(transport.fetches, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn login_persistence_writes_only_safe_metadata_after_secure_secret_store() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let store = FakeWeixinSecretStore::new();
        let account_store = WeixinAccountStore::new(workspace.path());
        let credential = persist_login(
            &store,
            &account,
            outcome(),
            &account_store,
            workspace.path(),
        )
        .expect("fake secure login persistence");
        let record = account_store
            .load(&account)
            .expect("metadata read")
            .expect("metadata record");
        let json = serde_json::to_string(&record).expect("metadata JSON");
        assert_eq!(record.credential, credential);
        assert!(!json.contains(BOT_TOKEN));
        assert!(!json.contains(USER_ID));
        assert!(store.get_token(&account).is_ok());
        assert!(store.get_data_key(&account).is_ok());
        let state = FileWeixinStateStore::for_workspace(workspace.path())
            .load(&account.to_string())
            .expect("state read")
            .expect("state record");
        assert_eq!(state.schema_version, WEIXIN_STATE_SCHEMA_VERSION);
        assert_eq!(state.account_id, account.to_string());
        assert_eq!(state.connection_state, WeixinConnectionStateRecord::Ready);
        assert_eq!(
            state.credential.as_ref().map(|c| &c.backend),
            Some(&credential.backend)
        );
        assert!(
            !serde_json::to_string(&state)
                .expect("state JSON")
                .contains(BOT_TOKEN)
        );
        assert!(
            !serde_json::to_string(&state)
                .expect("state JSON")
                .contains(USER_ID)
        );
    }

    #[test]
    fn login_persistence_rejects_unavailable_secure_store_without_metadata() {
        let workspace = TempDir::new().expect("workspace");
        let account = WeixinAccountId::new(RAW_ACCOUNT);
        let store = FakeWeixinSecretStore::unavailable();
        let account_store = WeixinAccountStore::new(workspace.path());
        let error = persist_login(
            &store,
            &account,
            outcome(),
            &account_store,
            workspace.path(),
        )
        .expect_err("secure store failure must stop login");
        let text = error.to_string();
        assert!(text.contains("secure data-key storage"));
        assert!(!text.contains(BOT_TOKEN));
        assert!(
            account_store
                .load(&account)
                .expect("metadata read")
                .is_none()
        );
        assert!(
            FileWeixinStateStore::for_workspace(workspace.path())
                .load(&account.to_string())
                .expect("state read")
                .is_none()
        );
    }
}
