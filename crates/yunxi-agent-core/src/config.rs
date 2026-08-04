use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub codex_home: Option<PathBuf>,
    pub approval_mode: ApprovalMode,
    pub sandbox_mode: SandboxMode,
    #[serde(default)]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub session_title: Option<String>,
    #[serde(default)]
    pub context_window_tokens: Option<i64>,
    #[serde(default)]
    pub auto_compact_threshold_tokens: Option<i64>,
    #[serde(default)]
    pub memory_extraction_mode: MemoryExtractionMode,
    #[serde(default)]
    pub companion: CompanionSettings,
}

impl AgentConfig {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            model: None,
            provider: None,
            codex_home: None,
            approval_mode: ApprovalMode::OnRequest,
            sandbox_mode: SandboxMode::WorkspaceWrite,
            parent_session_id: None,
            session_id: None,
            session_title: None,
            context_window_tokens: None,
            auto_compact_threshold_tokens: None,
            memory_extraction_mode: MemoryExtractionMode::Auto,
            companion: CompanionSettings::default(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_codex_home(mut self, codex_home: impl Into<PathBuf>) -> Self {
        self.codex_home = Some(codex_home.into());
        self
    }

    pub fn with_approval_mode(mut self, approval_mode: ApprovalMode) -> Self {
        self.approval_mode = approval_mode;
        self
    }

    pub fn with_sandbox_mode(mut self, sandbox_mode: SandboxMode) -> Self {
        self.sandbox_mode = sandbox_mode;
        self
    }

    pub fn with_parent_session_id(mut self, parent_session_id: impl Into<String>) -> Self {
        self.parent_session_id = Some(parent_session_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_session_title(mut self, session_title: impl Into<String>) -> Self {
        self.session_title = Some(session_title.into());
        self
    }

    pub fn with_context_window_tokens(mut self, context_window_tokens: i64) -> Self {
        self.context_window_tokens = Some(context_window_tokens.max(1));
        self
    }

    pub fn with_auto_compact_threshold_tokens(
        mut self,
        auto_compact_threshold_tokens: i64,
    ) -> Self {
        self.auto_compact_threshold_tokens = Some(auto_compact_threshold_tokens.max(1));
        self
    }

    pub fn with_memory_extraction_mode(mut self, mode: MemoryExtractionMode) -> Self {
        self.memory_extraction_mode = mode;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
    #[serde(default = "default_max_proactive_per_session")]
    pub max_proactive_per_session: u32,
    #[serde(default = "default_max_proactive_per_day")]
    pub max_proactive_per_day: u32,
    #[serde(default = "default_true")]
    pub require_reason: bool,
    #[serde(default)]
    pub allow_tool_requests: bool,
    #[serde(default)]
    pub cloud_control_enabled: bool,
    #[serde(default = "default_true")]
    pub clear_requires_confirmation: bool,
    #[serde(default)]
    pub love_letters: LoveLetterSettings,
}

impl Default for CompanionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            quiet_hours: None,
            max_proactive_per_session: default_max_proactive_per_session(),
            max_proactive_per_day: default_max_proactive_per_day(),
            require_reason: true,
            allow_tool_requests: false,
            cloud_control_enabled: false,
            clear_requires_confirmation: true,
            love_letters: LoveLetterSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoveLetterSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_love_letter_minimum_active_memories")]
    pub minimum_active_memories: usize,
    #[serde(default = "default_love_letter_minimum_new_memories")]
    pub minimum_new_memories: usize,
    #[serde(default = "default_love_letter_cooldown_min_days")]
    pub cooldown_min_days: u32,
    #[serde(default = "default_love_letter_cooldown_max_days")]
    pub cooldown_max_days: u32,
    #[serde(default = "default_love_letter_max_per_day")]
    pub max_per_day: u32,
    #[serde(default = "default_love_letter_generation_timeout_seconds")]
    pub generation_timeout_seconds: u64,
    #[serde(default = "default_love_letter_max_content_chars")]
    pub max_content_chars: usize,
    #[serde(default = "default_love_letter_max_generation_attempts")]
    pub max_generation_attempts: u32,
    #[serde(default = "default_love_letter_retry_backoff_seconds")]
    pub retry_backoff_seconds: u64,
}

impl Default for LoveLetterSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            minimum_active_memories: default_love_letter_minimum_active_memories(),
            minimum_new_memories: default_love_letter_minimum_new_memories(),
            cooldown_min_days: default_love_letter_cooldown_min_days(),
            cooldown_max_days: default_love_letter_cooldown_max_days(),
            max_per_day: default_love_letter_max_per_day(),
            generation_timeout_seconds: default_love_letter_generation_timeout_seconds(),
            max_content_chars: default_love_letter_max_content_chars(),
            max_generation_attempts: default_love_letter_max_generation_attempts(),
            retry_backoff_seconds: default_love_letter_retry_backoff_seconds(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_minute: u16,
    pub end_minute: u16,
}

impl QuietHours {
    pub fn new(start_minute: u16, end_minute: u16) -> Option<Self> {
        (start_minute < 1440 && end_minute < 1440).then_some(Self {
            start_minute,
            end_minute,
        })
    }

    pub fn contains(self, minute_of_day: u16) -> bool {
        if self.start_minute <= self.end_minute {
            (self.start_minute..=self.end_minute).contains(&minute_of_day)
        } else {
            minute_of_day >= self.start_minute || minute_of_day <= self.end_minute
        }
    }
}

fn default_max_proactive_per_session() -> u32 {
    3
}
fn default_max_proactive_per_day() -> u32 {
    8
}
fn default_true() -> bool {
    true
}
fn default_love_letter_minimum_active_memories() -> usize {
    3
}
fn default_love_letter_minimum_new_memories() -> usize {
    1
}
fn default_love_letter_cooldown_min_days() -> u32 {
    3
}
fn default_love_letter_cooldown_max_days() -> u32 {
    10
}
fn default_love_letter_max_per_day() -> u32 {
    1
}
fn default_love_letter_generation_timeout_seconds() -> u64 {
    20
}
fn default_love_letter_max_content_chars() -> usize {
    4_000
}
fn default_love_letter_max_generation_attempts() -> u32 {
    3
}
fn default_love_letter_retry_backoff_seconds() -> u64 {
    21_600
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryExtractionMode {
    #[default]
    Auto,
    RuleOnly,
    Provider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}
