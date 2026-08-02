use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use thiserror::Error;
use yunxi_agent_core::{
    AgentCancellationToken, AgentRunApprovalDecision, AgentRunApprovalRequest,
    AgentRunUserInputRequest, AgentRunUserInputResponse,
};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinRemoteControlCommitItem, WeixinRemoteControlPurposeRecord,
    WeixinRemoteControlState, WeixinStateError,
};

use crate::redaction::redacted_identifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeixinRemoteCommand {
    Status,
    Stop,
    Approve {
        request_id: Option<String>,
    },
    Deny {
        request_id: Option<String>,
        reason: Option<String>,
    },
    Answer {
        request_id: Option<String>,
        text: String,
    },
}

pub fn parse_weixin_remote_command(input: &str) -> Option<WeixinRemoteCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return parse_natural_remote_command(trimmed);
    }
    let (command, arguments) = split_remote_command(trimmed);
    match command.as_str() {
        "/status" => Some(WeixinRemoteCommand::Status),
        "/stop" => Some(WeixinRemoteCommand::Stop),
        "/approve" => Some(WeixinRemoteCommand::Approve {
            request_id: arguments
                .filter(|value| is_remote_request_id(value))
                .map(str::to_string),
        }),
        "/deny" => {
            let (request_id, reason) = split_optional_remote_request_id(arguments);
            Some(WeixinRemoteCommand::Deny {
                request_id: request_id.map(str::to_string),
                reason: reason.map(safe_remote_text),
            })
        }
        "/answer" => {
            let (request_id, text) = split_optional_remote_request_id(arguments);
            Some(WeixinRemoteCommand::Answer {
                request_id: request_id.map(str::to_string),
                text: safe_remote_text(text?),
            })
        }
        _ => None,
    }
}

fn parse_natural_remote_command(trimmed: &str) -> Option<WeixinRemoteCommand> {
    let (command, arguments) = split_remote_command(trimmed);
    let command = command.as_str();
    if natural_approve_command(command) {
        let request_id = parse_natural_approval_request_id(arguments)?;
        return Some(WeixinRemoteCommand::Approve { request_id });
    }
    if natural_deny_command(command) {
        let (request_id, reason) = split_optional_remote_request_id(arguments);
        return Some(WeixinRemoteCommand::Deny {
            request_id: request_id.map(str::to_string),
            reason: reason.map(safe_remote_text),
        });
    }
    None
}

fn parse_natural_approval_request_id(arguments: Option<&str>) -> Option<Option<String>> {
    match arguments {
        None => Some(None),
        Some(value) if is_remote_request_id(value) => Some(Some(value.to_string())),
        Some(_) => None,
    }
}

fn natural_approve_command(command: &str) -> bool {
    matches!(command, "允许" | "同意" | "批准" | "确认" | "准许" | "通过")
}

fn natural_deny_command(command: &str) -> bool {
    matches!(command, "拒绝" | "不同意" | "不允许" | "否决" | "驳回")
}

fn split_remote_command(trimmed: &str) -> (String, Option<&str>) {
    match trimmed.split_once(char::is_whitespace) {
        Some((command, arguments)) => (
            command.to_ascii_lowercase(),
            Some(arguments.trim()).filter(|value| !value.is_empty()),
        ),
        None => (trimmed.to_ascii_lowercase(), None),
    }
}

fn split_optional_remote_request_id(arguments: Option<&str>) -> (Option<&str>, Option<&str>) {
    let Some(arguments) = arguments.map(str::trim).filter(|value| !value.is_empty()) else {
        return (None, None);
    };
    match arguments.split_once(char::is_whitespace) {
        Some((first, rest)) if is_remote_request_id(first) => (
            Some(first),
            Some(rest.trim()).filter(|value| !value.is_empty()),
        ),
        _ if is_remote_request_id(arguments) => (Some(arguments), None),
        _ => (None, Some(arguments)),
    }
}

fn is_remote_request_id(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("wxctl#") else {
        return false;
    };
    hash.len() == 8 && hash.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlScope {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlPrompt {
    pub scope: WeixinRemoteControlScope,
    pub request_id: String,
    pub purpose: WeixinRemoteControlPurpose,
    pub action: String,
    pub reason: String,
    pub cwd_label: Option<String>,
    pub expires_at_millis: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeixinRemoteControlPurpose {
    Approval,
    UserInput,
    Cancellation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlOutcome {
    pub scope: WeixinRemoteControlScope,
    pub request_id: Option<String>,
    pub status: &'static str,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct WeixinRemoteControlHub {
    inner: Arc<Mutex<HashMap<String, RegisteredControlRequest>>>,
    state_store: Option<FileWeixinStateStore>,
}

impl WeixinRemoteControlHub {
    pub fn with_state_store(state_store: FileWeixinStateStore) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            state_store: Some(state_store),
        }
    }

    pub fn pending_count(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.len())
            .unwrap_or_default()
    }

    pub fn register_approval(
        &self,
        scope: WeixinRemoteControlScope,
        request: AgentRunApprovalRequest,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id(
            "approval",
            &scope,
            request.id.as_deref().unwrap_or(&request.tool_name),
        );
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::Approval,
            action: safe_remote_text(&request.tool_name),
            reason: safe_remote_text(&request.reason),
            cwd_label: Some(redacted_identifier("cwd", &request.cwd)),
            expires_at_millis,
        };
        let respond_to = request.respond_to;
        if let Err(error) = self.persist_register(&scope, &prompt, remote_now_millis()) {
            let _ = respond_to.send(AgentRunApprovalDecision {
                approved: false,
                reason: Some("weixin_remote_control_unavailable".to_string()),
            });
            return Err(error);
        }
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some("weixin_remote_control_unavailable".to_string()),
                });
                return Err(WeixinRemoteControlError::Poisoned);
            }
        };
        inner.insert(
            request_id,
            RegisteredControlRequest {
                scope,
                expires_at_millis,
                kind: RegisteredControlKind::Approval { respond_to },
            },
        );
        Ok(prompt)
    }

    pub fn register_user_input(
        &self,
        scope: WeixinRemoteControlScope,
        request: AgentRunUserInputRequest,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id(
            "input",
            &scope,
            request.id.as_deref().unwrap_or("user-input"),
        );
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::UserInput,
            action: "answer".to_string(),
            reason: safe_remote_text(&request.prompt),
            cwd_label: None,
            expires_at_millis,
        };
        let respond_to = request.respond_to;
        if let Err(error) = self.persist_register(&scope, &prompt, remote_now_millis()) {
            let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            return Err(error);
        }
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                return Err(WeixinRemoteControlError::Poisoned);
            }
        };
        inner.insert(
            request_id,
            RegisteredControlRequest {
                scope,
                expires_at_millis,
                kind: RegisteredControlKind::UserInput { respond_to },
            },
        );
        Ok(prompt)
    }

    pub fn register_cancellation(
        &self,
        scope: WeixinRemoteControlScope,
        cancellation_token: AgentCancellationToken,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id("stop", &scope, "turn");
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::Cancellation,
            action: "stop".to_string(),
            reason: "cancel current turn".to_string(),
            cwd_label: None,
            expires_at_millis,
        };
        self.persist_register(&scope, &prompt, remote_now_millis())?;
        self.inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .insert(
                request_id,
                RegisteredControlRequest {
                    scope,
                    expires_at_millis,
                    kind: RegisteredControlKind::Cancellation { cancellation_token },
                },
            );
        Ok(prompt)
    }

    pub fn handle_command(
        &self,
        scope: &WeixinRemoteControlScope,
        command: WeixinRemoteCommand,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        match command {
            WeixinRemoteCommand::Status => Ok(WeixinRemoteControlOutcome {
                scope: scope.clone(),
                request_id: None,
                status: "ok",
                message: format!("pending_control_requests={}", self.pending_count()),
            }),
            WeixinRemoteCommand::Stop => self.stop_scope(scope, now_millis),
            WeixinRemoteCommand::Approve { request_id } => match request_id {
                Some(request_id) => self.approve(scope, &request_id, now_millis),
                None => {
                    let request_id = self.resolve_single_request(
                        scope,
                        WeixinRemoteControlPurpose::Approval,
                        now_millis,
                    )?;
                    self.approve(scope, &request_id, now_millis)
                }
            },
            WeixinRemoteCommand::Deny { request_id, reason } => {
                let request_id = match request_id {
                    Some(request_id) => request_id,
                    None => self.resolve_single_request(
                        scope,
                        WeixinRemoteControlPurpose::Approval,
                        now_millis,
                    )?,
                };
                self.deny(scope, &request_id, reason, now_millis)
            }
            WeixinRemoteCommand::Answer { request_id, text } => {
                let request_id = match request_id {
                    Some(request_id) => request_id,
                    None => self.resolve_single_request(
                        scope,
                        WeixinRemoteControlPurpose::UserInput,
                        now_millis,
                    )?,
                };
                self.answer(scope, &request_id, text, now_millis)
            }
        }
    }

    pub fn expire_request(
        &self,
        request_id: &str,
        now_millis: u64,
    ) -> Result<bool, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id);
        let Some(request) = request else {
            return Ok(false);
        };
        if request.expires_at_millis > now_millis {
            self.inner
                .lock()
                .map_err(|_| WeixinRemoteControlError::Poisoned)?
                .insert(request_id.to_string(), request);
            return Ok(false);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some("weixin_remote_control_timeout".to_string()),
                });
            }
            RegisteredControlKind::UserInput { respond_to } => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            }
            RegisteredControlKind::Cancellation { .. } => {}
        }
        self.persist_transition(
            &request.scope.account_id,
            request_id,
            WeixinRemoteControlState::Expired,
            "timeout",
            now_millis,
        )?;
        Ok(true)
    }

    pub fn expire_due(&self, now_millis: u64) -> Result<usize, WeixinRemoteControlError> {
        let request_ids = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| WeixinRemoteControlError::Poisoned)?;
            inner
                .iter()
                .filter_map(|(request_id, request)| {
                    (request.expires_at_millis <= now_millis).then(|| request_id.clone())
                })
                .collect::<Vec<_>>()
        };
        let mut expired = 0usize;
        for request_id in request_ids {
            if self.expire_request(&request_id, now_millis)? {
                expired += 1;
            }
        }
        Ok(expired)
    }

    pub fn close_scope(
        &self,
        scope: &WeixinRemoteControlScope,
    ) -> Result<usize, WeixinRemoteControlError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?;
        let request_ids = inner
            .iter()
            .filter_map(|(request_id, request)| {
                request.scope.matches(scope).then(|| request_id.clone())
            })
            .collect::<Vec<_>>();
        let mut closed = 0usize;
        for request_id in request_ids {
            if let Some(request) = inner.remove(&request_id) {
                match request.kind {
                    RegisteredControlKind::Approval { respond_to } => {
                        let _ = respond_to.send(AgentRunApprovalDecision {
                            approved: false,
                            reason: Some("weixin_turn_closed".to_string()),
                        });
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Rejected,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                    RegisteredControlKind::UserInput { respond_to } => {
                        let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Rejected,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                    RegisteredControlKind::Cancellation { .. } => {
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Cancelled,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                }
                closed += 1;
            }
        }
        Ok(closed)
    }

    pub fn reject_request(
        &self,
        request_id: &str,
        reason: &'static str,
        now_millis: u64,
    ) -> Result<bool, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id);
        let Some(request) = request else {
            return Ok(false);
        };
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some(reason.to_string()),
                });
            }
            RegisteredControlKind::UserInput { respond_to } => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            }
            RegisteredControlKind::Cancellation { .. } => {}
        }
        self.persist_transition(
            &request.scope.account_id,
            request_id,
            WeixinRemoteControlState::Rejected,
            reason,
            now_millis,
        )?;
        Ok(true)
    }

    fn persist_register(
        &self,
        scope: &WeixinRemoteControlScope,
        prompt: &WeixinRemoteControlPrompt,
        now_millis: u64,
    ) -> Result<(), WeixinRemoteControlError> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(());
        };
        store.record_remote_control_request(
            &scope.account_id,
            WeixinRemoteControlCommitItem {
                request_id: prompt.request_id.clone(),
                peer_id_hash: scope.peer_id_hash.clone(),
                direct_message_key: scope.direct_message_key.clone(),
                item_id: scope.item_id.clone(),
                session_id: scope.session_id.clone(),
                purpose: storage_purpose(prompt.purpose),
                action: prompt.action.clone(),
                reason: prompt.reason.clone(),
                cwd_label: prompt.cwd_label.clone(),
                expires_at_millis: prompt.expires_at_millis,
            },
            now_millis,
        )?;
        Ok(())
    }

    fn persist_transition(
        &self,
        account_id: &str,
        request_id: &str,
        target: WeixinRemoteControlState,
        last_status: &'static str,
        now_millis: u64,
    ) -> Result<(), WeixinRemoteControlError> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(());
        };
        store.transition_remote_control_request(
            account_id,
            request_id,
            target,
            Some(last_status.to_string()),
            now_millis,
        )?;
        Ok(())
    }

    fn resolve_single_request(
        &self,
        scope: &WeixinRemoteControlScope,
        purpose: WeixinRemoteControlPurpose,
        now_millis: u64,
    ) -> Result<String, WeixinRemoteControlError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?;
        let mut matches = inner.iter().filter_map(|(request_id, request)| {
            (request.scope.matches(scope)
                && request.expires_at_millis > now_millis
                && request_kind_matches(&request.kind, purpose))
            .then(|| request_id.clone())
        });
        let Some(request_id) = matches.next() else {
            return Err(WeixinRemoteControlError::RequestNotFound);
        };
        if matches.next().is_some() {
            return Err(WeixinRemoteControlError::AmbiguousRequest);
        }
        Ok(request_id)
    }

    fn approve(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Approval { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                if respond_to
                    .send(AgentRunApprovalDecision {
                        approved: true,
                        reason: Some("approved_from_weixin".to_string()),
                    })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Consumed,
                    "approved",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "approval accepted",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn deny(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        reason: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Approval { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                if respond_to
                    .send(AgentRunApprovalDecision {
                        approved: false,
                        reason: Some(reason.unwrap_or_else(|| "denied_from_weixin".to_string())),
                    })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Rejected,
                    "denied",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "approval denied",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn answer(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        text: String,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::UserInput { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::UserInput { respond_to } => {
                if respond_to
                    .send(AgentRunUserInputResponse { value: Some(text) })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Consumed,
                    "answered",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "answer accepted",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn stop_scope(
        &self,
        scope: &WeixinRemoteControlScope,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request_id = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| WeixinRemoteControlError::Poisoned)?;
            inner.iter().find_map(|(request_id, request)| {
                (request.scope.matches(scope)
                    && request.expires_at_millis > now_millis
                    && matches!(request.kind, RegisteredControlKind::Cancellation { .. }))
                .then(|| request_id.clone())
            })
        }
        .ok_or(WeixinRemoteControlError::RequestNotFound)?;
        let request = self.take_matching(scope, &request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Cancellation { .. }) {
            self.restore_request(&request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Cancellation { cancellation_token } => {
                cancellation_token.cancel();
                self.persist_transition(
                    &request.scope.account_id,
                    &request_id,
                    WeixinRemoteControlState::Cancelled,
                    "cancelled",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    &request_id,
                    "consumed",
                    "turn cancellation requested",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn take_matching(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        now_millis: u64,
    ) -> Result<RegisteredControlRequest, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id)
            .ok_or(WeixinRemoteControlError::RequestNotFound)?;
        if !request.scope.matches(scope) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::ScopeMismatch);
        }
        if request.expires_at_millis <= now_millis {
            match request.kind {
                RegisteredControlKind::Approval { respond_to } => {
                    let _ = respond_to.send(AgentRunApprovalDecision {
                        approved: false,
                        reason: Some("weixin_remote_control_expired".to_string()),
                    });
                }
                RegisteredControlKind::UserInput { respond_to } => {
                    let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                }
                RegisteredControlKind::Cancellation { .. } => {}
            }
            self.persist_transition(
                &request.scope.account_id,
                request_id,
                WeixinRemoteControlState::Expired,
                "expired",
                now_millis,
            )?;
            return Err(WeixinRemoteControlError::Expired);
        }
        Ok(request)
    }

    fn restore_request(
        &self,
        request_id: &str,
        request: RegisteredControlRequest,
    ) -> Result<(), WeixinRemoteControlError> {
        self.inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .insert(request_id.to_string(), request);
        Ok(())
    }
}

struct RegisteredControlRequest {
    scope: WeixinRemoteControlScope,
    expires_at_millis: u64,
    kind: RegisteredControlKind,
}

enum RegisteredControlKind {
    Approval {
        respond_to: tokio::sync::oneshot::Sender<AgentRunApprovalDecision>,
    },
    UserInput {
        respond_to: tokio::sync::oneshot::Sender<AgentRunUserInputResponse>,
    },
    Cancellation {
        cancellation_token: AgentCancellationToken,
    },
}

fn request_kind_matches(kind: &RegisteredControlKind, purpose: WeixinRemoteControlPurpose) -> bool {
    matches!(
        (kind, purpose),
        (
            RegisteredControlKind::Approval { .. },
            WeixinRemoteControlPurpose::Approval
        ) | (
            RegisteredControlKind::UserInput { .. },
            WeixinRemoteControlPurpose::UserInput
        ) | (
            RegisteredControlKind::Cancellation { .. },
            WeixinRemoteControlPurpose::Cancellation
        )
    )
}

impl WeixinRemoteControlScope {
    fn matches(&self, other: &Self) -> bool {
        self.account_id == other.account_id
            && self.peer_id_hash == other.peer_id_hash
            && self.direct_message_key == other.direct_message_key
    }
}

#[derive(Debug, Error)]
pub enum WeixinRemoteControlError {
    #[error("weixin remote control lock was poisoned")]
    Poisoned,
    #[error("weixin remote control state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin remote control request was not found")]
    RequestNotFound,
    #[error("weixin remote control request scope mismatch")]
    ScopeMismatch,
    #[error("weixin remote control request purpose mismatch")]
    PurposeMismatch,
    #[error("multiple weixin remote control requests matched this command")]
    AmbiguousRequest,
    #[error("weixin remote control request expired")]
    Expired,
    #[error("weixin remote control response channel was closed")]
    ResponseChannelClosed,
}

fn remote_request_id(kind: &str, scope: &WeixinRemoteControlScope, source_id: &str) -> String {
    redacted_identifier(
        "wxctl",
        &format!(
            "{}:{}:{}:{}:{}:{}",
            kind,
            scope.account_id,
            scope.peer_id_hash,
            scope.direct_message_key,
            scope.item_id,
            source_id
        ),
    )
}

fn storage_purpose(purpose: WeixinRemoteControlPurpose) -> WeixinRemoteControlPurposeRecord {
    match purpose {
        WeixinRemoteControlPurpose::Approval => WeixinRemoteControlPurposeRecord::Approval,
        WeixinRemoteControlPurpose::UserInput => WeixinRemoteControlPurposeRecord::UserInput,
        WeixinRemoteControlPurpose::Cancellation => WeixinRemoteControlPurposeRecord::Cancellation,
    }
}

fn remote_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn safe_remote_text(value: &str) -> String {
    let trimmed = value.trim();
    let mut sanitized = String::new();
    for ch in trimmed.chars().take(200) {
        if ch.is_control() {
            sanitized.push(' ');
        } else {
            sanitized.push(ch);
        }
    }
    let lower = sanitized.to_ascii_lowercase();
    if ["token", "secret", "context", "data_key", "authorization"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[redacted]".to_string()
    } else {
        sanitized
    }
}

fn outcome(
    scope: WeixinRemoteControlScope,
    request_id: &str,
    status: &'static str,
    message: &str,
) -> WeixinRemoteControlOutcome {
    WeixinRemoteControlOutcome {
        scope,
        request_id: Some(request_id.to_string()),
        status,
        message: message.to_string(),
    }
}

pub fn render_remote_control_prompt(prompt: &WeixinRemoteControlPrompt) -> String {
    match prompt.purpose {
        WeixinRemoteControlPurpose::Approval => format!(
            "[YunXi]\n需要你确认一次操作：{}\n原因：{}\n回复 /approve 或“允许”允许，回复 /deny 或“拒绝”拒绝。\n如同时有多个待确认请求，请带控制码重试：{}",
            safe_remote_text(&prompt.action),
            safe_remote_text(&prompt.reason),
            prompt.request_id,
        ),
        WeixinRemoteControlPurpose::UserInput => format!(
            "[YunXi]\n需要你补充信息：{}\n回复 /answer 你的回答。\n如同时有多个待回答请求，请带控制码重试：{}",
            safe_remote_text(&prompt.reason),
            prompt.request_id,
        ),
        WeixinRemoteControlPurpose::Cancellation => String::new(),
    }
}

pub fn render_remote_control_outcome(outcome: &WeixinRemoteControlOutcome) -> String {
    format!("[YunXi]\n{}", public_outcome_message(outcome))
}

pub fn render_remote_control_error(
    _scope: &WeixinRemoteControlScope,
    error: &WeixinRemoteControlError,
) -> String {
    let message = match error {
        WeixinRemoteControlError::Poisoned | WeixinRemoteControlError::State(_) => {
            "微信控制暂不可用，请稍后重试。"
        }
        WeixinRemoteControlError::RequestNotFound => "没有找到可处理的请求，可能已过期或已完成。",
        WeixinRemoteControlError::ScopeMismatch => "这条控制命令不属于当前会话，已拒绝处理。",
        WeixinRemoteControlError::PurposeMismatch => "控制命令类型不匹配，请按提示使用对应命令。",
        WeixinRemoteControlError::AmbiguousRequest => {
            "当前有多个待处理请求，请按提示里的控制码重试。"
        }
        WeixinRemoteControlError::Expired => "这个控制请求已过期，请重新发起。",
        WeixinRemoteControlError::ResponseChannelClosed => "当前任务已结束，控制命令未生效。",
    };
    format!("[YunXi]\n{message}")
}

fn public_outcome_message(outcome: &WeixinRemoteControlOutcome) -> String {
    match outcome.message.as_str() {
        "approval accepted" => "已允许这次操作。".to_string(),
        "approval denied" => "已拒绝这次操作。".to_string(),
        "answer accepted" => "已收到补充信息。".to_string(),
        "turn cancellation requested" => "已请求中止本轮处理。".to_string(),
        message if message.starts_with("pending_control_requests=") => {
            let count = message
                .strip_prefix("pending_control_requests=")
                .unwrap_or("0");
            format!("当前有 {count} 个待处理控制请求。")
        }
        message => safe_remote_text(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::sync::oneshot;
    use yunxi_agent_storage::{WeixinStateSnapshot, WeixinStateStore};

    fn scope(item_id: &str) -> WeixinRemoteControlScope {
        WeixinRemoteControlScope {
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: item_id.to_string(),
            session_id: "session-1".to_string(),
        }
    }

    fn assert_public_message_hides_control_internals(message: &str) {
        for marker in [
            "purpose=",
            "request_id=",
            "account=",
            "peer=",
            "dm=",
            "item=",
            "session=",
            "cwd=",
            "expires_at_millis=",
            "account#11111111",
            "peer#22222222",
            "dm#33333333",
            "item#11111111",
            "session-1",
        ] {
            assert!(
                !message.contains(marker),
                "public message leaked internal marker {marker}: {message}"
            );
        }
    }

    #[test]
    fn parser_accepts_slash_and_natural_control_commands() {
        assert_eq!(parse_weixin_remote_command("好的"), None);
        assert_eq!(parse_weixin_remote_command("批准一下"), None);
        assert_eq!(
            parse_weixin_remote_command("/status"),
            Some(WeixinRemoteCommand::Status)
        );
        assert_eq!(
            parse_weixin_remote_command("/approve"),
            Some(WeixinRemoteCommand::Approve { request_id: None })
        );
        assert_eq!(
            parse_weixin_remote_command("/approve wxctl#12345678"),
            Some(WeixinRemoteCommand::Approve {
                request_id: Some("wxctl#12345678".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("允许"),
            Some(WeixinRemoteCommand::Approve { request_id: None })
        );
        assert_eq!(
            parse_weixin_remote_command("同意 wxctl#12345678"),
            Some(WeixinRemoteCommand::Approve {
                request_id: Some("wxctl#12345678".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("允许 这个操作"),
            None,
            "natural approve must stay exact to avoid accidental approvals"
        );
        assert_eq!(
            parse_weixin_remote_command("/deny 这次先不要执行"),
            Some(WeixinRemoteCommand::Deny {
                request_id: None,
                reason: Some("这次先不要执行".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("/deny wxctl#12345678 too risky"),
            Some(WeixinRemoteCommand::Deny {
                request_id: Some("wxctl#12345678".to_string()),
                reason: Some("too risky".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("拒绝 wxctl#12345678 太危险"),
            Some(WeixinRemoteCommand::Deny {
                request_id: Some("wxctl#12345678".to_string()),
                reason: Some("太危险".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("不允许"),
            Some(WeixinRemoteCommand::Deny {
                request_id: None,
                reason: None
            })
        );
        assert_eq!(
            parse_weixin_remote_command("/answer 可以"),
            Some(WeixinRemoteCommand::Answer {
                request_id: None,
                text: "可以".to_string()
            })
        );
        assert_eq!(
            parse_weixin_remote_command("/answer wxctl#12345678 yes"),
            Some(WeixinRemoteCommand::Answer {
                request_id: Some("wxctl#12345678".to_string()),
                text: "yes".to_string()
            })
        );
    }

    #[test]
    fn cancellation_prompt_is_suppressed_for_public_weixin_messages() {
        let rendered = render_remote_control_prompt(&WeixinRemoteControlPrompt {
            scope: scope("item#11111111"),
            request_id: "wxctl#12345678".to_string(),
            purpose: WeixinRemoteControlPurpose::Cancellation,
            action: "stop".to_string(),
            reason: "cancel current turn".to_string(),
            cwd_label: Some("cwd#99999999".to_string()),
            expires_at_millis: 123456789,
        });

        assert!(rendered.is_empty());
    }

    #[test]
    fn approval_and_user_input_prompts_hide_scope_fields() {
        let approval = render_remote_control_prompt(&WeixinRemoteControlPrompt {
            scope: scope("item#11111111"),
            request_id: "wxctl#12345678".to_string(),
            purpose: WeixinRemoteControlPurpose::Approval,
            action: "shell".to_string(),
            reason: "safe test".to_string(),
            cwd_label: Some("cwd#99999999".to_string()),
            expires_at_millis: 123456789,
        });
        assert!(approval.contains("/approve"));
        assert!(approval.contains("/deny"));
        assert!(approval.contains("wxctl#12345678"));
        assert_public_message_hides_control_internals(&approval);

        let user_input = render_remote_control_prompt(&WeixinRemoteControlPrompt {
            scope: scope("item#11111111"),
            request_id: "wxctl#87654321".to_string(),
            purpose: WeixinRemoteControlPurpose::UserInput,
            action: "answer".to_string(),
            reason: "请选择下一步".to_string(),
            cwd_label: None,
            expires_at_millis: 123456789,
        });
        assert!(user_input.contains("/answer"));
        assert!(user_input.contains("wxctl#87654321"));
        assert_public_message_hides_control_internals(&user_input);
    }

    #[test]
    fn remote_control_result_messages_are_public_facing() {
        let rendered = render_remote_control_outcome(&WeixinRemoteControlOutcome {
            scope: scope("item#11111111"),
            request_id: Some("wxctl#12345678".to_string()),
            status: "consumed",
            message: "turn cancellation requested".to_string(),
        });
        assert!(rendered.contains("已请求中止"));
        assert_public_message_hides_control_internals(&rendered);
        assert!(!rendered.contains("wxctl#12345678"));

        let error = render_remote_control_error(
            &scope("item#11111111"),
            &WeixinRemoteControlError::AmbiguousRequest,
        );
        assert!(error.contains("多个待处理请求"));
        assert_public_message_hides_control_internals(&error);
    }

    #[tokio::test]
    async fn hub_routes_approval_by_exact_scope_once() {
        let hub = WeixinRemoteControlHub::default();
        let (tx, rx) = oneshot::channel();
        let prompt = hub
            .register_approval(
                scope("item#11111111"),
                AgentRunApprovalRequest {
                    id: Some("approval-1".to_string()),
                    tool_name: "shell".to_string(),
                    reason: "safe test".to_string(),
                    command: Some("echo ok".to_string()),
                    cwd: "D:/YunXi Agent".to_string(),
                    respond_to: tx,
                },
                2000,
            )
            .expect("register");
        assert!(prompt.request_id.starts_with("wxctl#"));
        let outcome = hub
            .handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve {
                    request_id: Some(prompt.request_id.clone()),
                },
                1000,
            )
            .expect("approve");
        assert_eq!(outcome.status, "consumed");
        assert_eq!(
            rx.await.expect("approval response"),
            AgentRunApprovalDecision {
                approved: true,
                reason: Some("approved_from_weixin".to_string())
            }
        );
        assert!(matches!(
            hub.handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve {
                    request_id: Some(prompt.request_id)
                },
                1001,
            ),
            Err(WeixinRemoteControlError::RequestNotFound)
        ));
    }

    #[tokio::test]
    async fn hub_expire_due_denies_registered_approval() {
        let hub = WeixinRemoteControlHub::default();
        let (tx, rx) = oneshot::channel();
        let prompt = hub
            .register_approval(
                scope("item#11111111"),
                AgentRunApprovalRequest {
                    id: Some("approval-1".to_string()),
                    tool_name: "shell".to_string(),
                    reason: "safe test".to_string(),
                    command: Some("echo ok".to_string()),
                    cwd: "D:/YunXi Agent".to_string(),
                    respond_to: tx,
                },
                2000,
            )
            .expect("register");

        assert_eq!(hub.expire_due(1999).expect("not due"), 0);
        assert_eq!(hub.expire_due(2000).expect("expired"), 1);
        assert_eq!(
            rx.await.expect("approval timeout response"),
            AgentRunApprovalDecision {
                approved: false,
                reason: Some("weixin_remote_control_timeout".to_string())
            }
        );
        assert_eq!(hub.expire_due(2001).expect("already expired"), 0);
        assert!(matches!(
            hub.handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve {
                    request_id: Some(prompt.request_id)
                },
                2001,
            ),
            Err(WeixinRemoteControlError::RequestNotFound)
        ));
    }

    #[tokio::test]
    async fn hub_routes_single_approval_without_request_id() {
        let hub = WeixinRemoteControlHub::default();
        let (tx, rx) = oneshot::channel();
        hub.register_approval(
            scope("item#11111111"),
            AgentRunApprovalRequest {
                id: Some("approval-1".to_string()),
                tool_name: "shell".to_string(),
                reason: "safe test".to_string(),
                command: Some("echo ok".to_string()),
                cwd: "D:/YunXi Agent".to_string(),
                respond_to: tx,
            },
            2000,
        )
        .expect("register");

        let outcome = hub
            .handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve { request_id: None },
                1000,
            )
            .expect("approve without id");

        assert_eq!(outcome.status, "consumed");
        assert!(rx.await.expect("approval response").approved);
    }

    #[tokio::test]
    async fn hub_rejects_ambiguous_approval_without_request_id() {
        let hub = WeixinRemoteControlHub::default();
        let (first_tx, _first_rx) = oneshot::channel();
        let (second_tx, _second_rx) = oneshot::channel();
        hub.register_approval(
            scope("item#11111111"),
            AgentRunApprovalRequest {
                id: Some("approval-1".to_string()),
                tool_name: "shell".to_string(),
                reason: "safe test".to_string(),
                command: Some("echo ok".to_string()),
                cwd: "D:/YunXi Agent".to_string(),
                respond_to: first_tx,
            },
            2000,
        )
        .expect("register first");
        hub.register_approval(
            scope("item#11111111"),
            AgentRunApprovalRequest {
                id: Some("approval-2".to_string()),
                tool_name: "shell".to_string(),
                reason: "safe test 2".to_string(),
                command: Some("echo ok 2".to_string()),
                cwd: "D:/YunXi Agent".to_string(),
                respond_to: second_tx,
            },
            2000,
        )
        .expect("register second");

        assert!(matches!(
            hub.handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve { request_id: None },
                1000,
            ),
            Err(WeixinRemoteControlError::AmbiguousRequest)
        ));
        assert_eq!(hub.pending_count(), 2);
    }

    #[tokio::test]
    async fn hub_persists_control_request_lifecycle() {
        let temp = TempDir::new().expect("temp");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let control_scope = scope("item#11111111");
        let snapshot = WeixinStateSnapshot::new(
            &control_scope.account_id,
            "workspace#73521066",
            "https://ilinkai.weixin.qq.com/",
            1000,
        );
        store.save(&snapshot).expect("save state");
        let hub = WeixinRemoteControlHub::with_state_store(store.clone());
        let (tx, rx) = oneshot::channel();
        let prompt = hub
            .register_approval(
                control_scope.clone(),
                AgentRunApprovalRequest {
                    id: Some("approval-1".to_string()),
                    tool_name: "shell".to_string(),
                    reason: "safe test".to_string(),
                    command: Some("echo ok".to_string()),
                    cwd: "D:/YunXi Agent/private".to_string(),
                    respond_to: tx,
                },
                2000,
            )
            .expect("register");
        let state = store
            .load(&control_scope.account_id)
            .expect("load")
            .expect("state");
        assert_eq!(state.pending_remote_control_count_at(1000), 1);
        assert_eq!(state.pending_remote_control_count_at(2000), 0);
        assert_eq!(
            state.remote_control_requests[0].state,
            WeixinRemoteControlState::Pending
        );
        assert_eq!(
            state.remote_control_requests[0].request_id,
            prompt.request_id
        );
        assert!(
            state.remote_control_requests[0]
                .cwd_label
                .as_deref()
                .is_some_and(|cwd| cwd.starts_with("cwd#"))
        );
        let state_json = std::fs::read_to_string(store.state_path_for(&control_scope.account_id))
            .expect("state json");
        assert!(!state_json.contains("D:/YunXi Agent/private"));

        hub.handle_command(
            &control_scope,
            WeixinRemoteCommand::Approve {
                request_id: Some(prompt.request_id.clone()),
            },
            1500,
        )
        .expect("approve");
        assert!(rx.await.expect("approval").approved);
        let state = store
            .load(&control_scope.account_id)
            .expect("load")
            .expect("state");
        assert_eq!(state.pending_remote_control_count_at(1500), 0);
        assert_eq!(
            state.remote_control_requests[0].state,
            WeixinRemoteControlState::Consumed
        );
        assert_eq!(
            state.remote_control_requests[0].last_status.as_deref(),
            Some("approved")
        );
    }

    #[test]
    fn hub_rejects_cross_scope_stop() {
        let hub = WeixinRemoteControlHub::default();
        let token = AgentCancellationToken::new();
        let prompt = hub
            .register_cancellation(scope("item#11111111"), token.clone(), 2000)
            .expect("register stop");
        let mut other_peer = scope("item#22222222");
        other_peer.peer_id_hash = "peer#99999999".to_string();
        assert!(
            hub.handle_command(&other_peer, WeixinRemoteCommand::Stop, 1000)
                .is_err()
        );
        assert!(!token.is_cancelled());
        let outcome = hub
            .handle_command(&scope("item#11111111"), WeixinRemoteCommand::Stop, 1000)
            .expect("stop");
        assert_eq!(outcome.request_id, Some(prompt.request_id));
        assert!(token.is_cancelled());
    }
}
