use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlScope {
    Companion,
    Memory,
    Persona,
    Relationship,
}

impl ControlScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Companion => "companion",
            Self::Memory => "memory",
            Self::Persona => "persona",
            Self::Relationship => "relationship",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlVerb {
    Show,
    Enable,
    Disable,
    Clear,
    Refresh,
    Update,
}

impl ControlVerb {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Clear => "clear",
            Self::Refresh => "refresh",
            Self::Update => "update",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlRequest {
    pub scope: ControlScope,
    pub verb: ControlVerb,
    pub confirm_required: bool,
    pub confirm_token: Option<String>,
}

impl ControlRequest {
    pub fn new(scope: ControlScope, verb: ControlVerb) -> Self {
        Self {
            scope,
            verb,
            confirm_required: verb == ControlVerb::Clear,
            confirm_token: None,
        }
    }

    pub fn confirmed(mut self) -> Self {
        self.confirm_token = Some("confirmed".to_string());
        self
    }

    pub fn confirmation_satisfied(&self) -> bool {
        !self.confirm_required || self.confirm_token.as_deref() == Some("confirmed")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlSource {
    CurrentConfig,
    PersistedSettings,
    RuntimeSnapshot,
    ReadOnlyHistory,
}

impl ControlSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CurrentConfig => "current_config",
            Self::PersistedSettings => "persisted_settings",
            Self::RuntimeSnapshot => "runtime_snapshot",
            Self::ReadOnlyHistory => "read_only_history",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlScopeSnapshot {
    pub scope: ControlScope,
    pub enabled: Option<bool>,
    pub summary: String,
    pub source: ControlSource,
    pub clear_effect: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlSnapshot {
    pub companion_enabled: bool,
    pub cloud_control_enabled: bool,
    pub quiet_hours: Option<String>,
    pub persona_summary: String,
    pub memory_summary: String,
    pub relationship_summary: String,
    pub scopes: Vec<ControlScopeSnapshot>,
    pub recent_change: Option<String>,
}

impl ControlSnapshot {
    pub fn scope(&self, scope: ControlScope) -> Option<&ControlScopeSnapshot> {
        self.scopes.iter().find(|state| state.scope == scope)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlAuditRecord {
    pub timestamp_millis: u64,
    pub scope: ControlScope,
    pub verb: ControlVerb,
    pub outcome: String,
    pub detail: String,
    pub source: String,
}

impl ControlAuditRecord {
    pub fn new(
        request: &ControlRequest,
        outcome: impl Into<String>,
        detail: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            timestamp_millis: now_millis(),
            scope: request.scope,
            verb: request.verb,
            outcome: outcome.into(),
            detail: detail.into(),
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionHistoryRecord {
    pub timestamp_millis: u64,
    pub trigger: String,
    pub reason: String,
    pub message: String,
    pub requires_user_confirmation: bool,
}

impl CompanionHistoryRecord {
    pub fn new(
        trigger: impl Into<String>,
        reason: impl Into<String>,
        message: impl Into<String>,
        requires_user_confirmation: bool,
    ) -> Self {
        Self {
            timestamp_millis: now_millis(),
            trigger: trigger.into(),
            reason: reason.into(),
            message: message.into(),
            requires_user_confirmation,
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_requests_always_require_explicit_confirmation() {
        let request = ControlRequest::new(ControlScope::Memory, ControlVerb::Clear);
        assert!(request.confirm_required);
        assert!(!request.confirmation_satisfied());
        assert!(request.confirmed().confirmation_satisfied());
    }

    #[test]
    fn non_clear_requests_do_not_require_confirmation() {
        let request = ControlRequest::new(ControlScope::Companion, ControlVerb::Disable);
        assert!(!request.confirm_required);
        assert!(request.confirmation_satisfied());
    }

    #[test]
    fn companion_and_cloud_controls_are_conservative_by_default() {
        let settings = crate::CompanionSettings::default();
        assert!(!settings.enabled);
        assert!(!settings.cloud_control_enabled);
        assert!(settings.clear_requires_confirmation);
    }
}
