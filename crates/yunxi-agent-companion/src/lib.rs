use serde::{Deserialize, Serialize};
use yunxi_agent_core::{CompanionSettings, QuietHours};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionMemorySummary {
    pub boot_summary: Option<String>,
    pub dynamic_summary: Option<String>,
    pub record_count: usize,
    pub stable_fact_count: usize,
}

impl CompanionMemorySummary {
    pub fn is_empty(&self) -> bool {
        self.record_count == 0
            && self.boot_summary.as_deref().is_none_or(str::is_empty)
            && self.dynamic_summary.as_deref().is_none_or(str::is_empty)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionRelationshipStage {
    #[default]
    New,
    Familiar,
    Established,
}

impl CompanionRelationshipStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Familiar => "familiar",
            Self::Established => "established",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionEmotionKind {
    #[default]
    None,
    Anxiety,
    Sadness,
    Fatigue,
    Frustration,
    Joy,
    Uncertainty,
    Loneliness,
}

impl CompanionEmotionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Anxiety => "anxiety",
            Self::Sadness => "sadness",
            Self::Fatigue => "fatigue",
            Self::Frustration => "frustration",
            Self::Joy => "joy",
            Self::Uncertainty => "uncertainty",
            Self::Loneliness => "loneliness",
        }
    }

    fn needs_supportive_tone(self) -> bool {
        matches!(
            self,
            Self::Anxiety | Self::Sadness | Self::Fatigue | Self::Loneliness
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionEmotion {
    pub kind: CompanionEmotionKind,
    pub intensity: u8,
    pub confidence: u8,
}

impl CompanionEmotion {
    pub fn is_detected(self) -> bool {
        self.kind != CompanionEmotionKind::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionPersonaStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soul_signature: Option<String>,
    #[serde(default = "default_style_level")]
    pub warmth: u8,
    #[serde(default = "default_style_level")]
    pub directness: u8,
    #[serde(default = "default_style_level")]
    pub initiative: u8,
    #[serde(default)]
    pub humor: u8,
    #[serde(default = "default_style_level")]
    pub emotional_attunement: u8,
    #[serde(default)]
    pub reply_rules: Vec<String>,
    #[serde(default)]
    pub forbidden_styles: Vec<String>,
}

impl Default for CompanionPersonaStyle {
    fn default() -> Self {
        Self {
            soul_signature: None,
            warmth: default_style_level(),
            directness: default_style_level(),
            initiative: default_style_level(),
            humor: 0,
            emotional_attunement: default_style_level(),
            reply_rules: Vec::new(),
            forbidden_styles: Vec::new(),
        }
    }
}

impl CompanionPersonaStyle {
    pub fn has_rules(&self) -> bool {
        self.soul_signature
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || !self.reply_rules.is_empty()
            || !self.forbidden_styles.is_empty()
    }
}

fn default_style_level() -> u8 {
    50
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionTone {
    #[default]
    Neutral,
    Warm,
    Direct,
    Supportive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionFollowUp {
    pub prompt: String,
    pub required: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionContext {
    pub persona_id: Option<String>,
    pub display_name: Option<String>,
    pub relationship_state: Option<String>,
    #[serde(default)]
    pub relationship_stage: CompanionRelationshipStage,
    pub memory_summary: CompanionMemorySummary,
    pub emotional_clues: Vec<String>,
    #[serde(default)]
    pub emotion: CompanionEmotion,
    #[serde(default)]
    pub persona_style: CompanionPersonaStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistency_key: Option<String>,
    pub available: bool,
}

impl CompanionContext {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            ..Self::default()
        }
    }

    pub fn has_context(&self) -> bool {
        self.available
            && (self.persona_id.is_some()
                || self.relationship_state.is_some()
                || !self.memory_summary.is_empty()
                || !self.emotional_clues.is_empty()
                || self.emotion.is_detected()
                || self.persona_style.has_rules())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionPolicyDecision {
    pub tone: CompanionTone,
    pub emotion: CompanionEmotion,
    pub relationship_stage: CompanionRelationshipStage,
    pub consistency_key: Option<String>,
    pub proactive_care: bool,
    pub follow_up: Option<CompanionFollowUp>,
    pub use_persona_context: bool,
    pub use_memory_context: bool,
    pub fallback_to_existing_reply: bool,
}

pub trait CompanionPolicy {
    fn decide(&self, context: &CompanionContext, input: &CompanionInput)
    -> CompanionPolicyDecision;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicCompanionPolicy {
    settings: CompanionSettings,
}

impl DeterministicCompanionPolicy {
    pub fn new(settings: CompanionSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &CompanionSettings {
        &self.settings
    }
}

impl CompanionPolicy for DeterministicCompanionPolicy {
    fn decide(
        &self,
        context: &CompanionContext,
        input: &CompanionInput,
    ) -> CompanionPolicyDecision {
        let context_available = context.has_context();
        let use_memory_context = context_available && !context.memory_summary.is_empty();
        let use_persona_context = context_available
            && (context.persona_id.is_some()
                || context.display_name.is_some()
                || context.persona_style.has_rules());
        let emotion = if context.emotion.is_detected() {
            context.emotion
        } else {
            classify_companion_emotion(context.emotional_clues.iter().map(String::as_str))
        };
        let tone = if input.tool_request.is_some() {
            CompanionTone::Direct
        } else if emotion.kind.needs_supportive_tone() {
            CompanionTone::Supportive
        } else if emotion.kind == CompanionEmotionKind::Frustration
            && context.persona_style.directness >= 50
        {
            CompanionTone::Direct
        } else if use_persona_context || use_memory_context || emotion.is_detected() {
            CompanionTone::Warm
        } else {
            CompanionTone::Neutral
        };
        let proactive_care = self.settings.enabled && input.has_signal();
        let follow_up = if proactive_care
            && context_available
            && input.tool_request.is_none()
            && (emotion.is_detected()
                || input.unfinished_task.is_some()
                || input.topic_continuation.is_some())
        {
            Some(CompanionFollowUp {
                prompt: follow_up_prompt_for_emotion(emotion.kind).to_string(),
                required: false,
            })
        } else {
            None
        };
        CompanionPolicyDecision {
            tone,
            emotion,
            relationship_stage: context.relationship_stage,
            consistency_key: context.consistency_key.clone(),
            proactive_care,
            follow_up,
            use_persona_context,
            use_memory_context,
            fallback_to_existing_reply: !context_available,
        }
    }
}

pub fn classify_companion_emotion<I, S>(values: I) -> CompanionEmotion
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut best = CompanionEmotion::default();
    for value in values {
        let value = value.as_ref();
        let lower = value.to_ascii_lowercase();
        for (kind, needles) in EMOTION_PATTERNS {
            let matches = needles
                .iter()
                .filter(|needle| lower.contains(*needle) || value.contains(*needle))
                .count();
            if matches == 0 {
                continue;
            }
            let intensity = emotional_intensity(value, &lower, *kind, matches);
            let confidence = (55usize + matches * 10).min(95) as u8;
            let score = intensity.saturating_add(confidence / 2);
            let best_score = best.intensity.saturating_add(best.confidence / 2);
            if score > best_score {
                best = CompanionEmotion {
                    kind: *kind,
                    intensity,
                    confidence,
                };
            }
        }
    }
    best
}

const EMOTION_PATTERNS: &[(CompanionEmotionKind, &[&str])] = &[
    (
        CompanionEmotionKind::Anxiety,
        &[
            "焦虑", "紧张", "压力", "担心", "慌", "害怕", "怕", "anxious", "anxiety", "stress",
            "stressed", "worried", "panic",
        ],
    ),
    (
        CompanionEmotionKind::Sadness,
        &[
            "难过",
            "伤心",
            "低落",
            "委屈",
            "沮丧",
            "sad",
            "depressed",
            "down",
            "upset",
        ],
    ),
    (
        CompanionEmotionKind::Fatigue,
        &[
            "疲惫",
            "很累",
            "太累",
            "累了",
            "困",
            "熬不住",
            "撑不住",
            "tired",
            "exhausted",
            "burnout",
            "burned out",
        ],
    ),
    (
        CompanionEmotionKind::Frustration,
        &[
            "烦",
            "生气",
            "火大",
            "无语",
            "崩溃",
            "卡住",
            "恼火",
            "angry",
            "frustrated",
            "annoyed",
            "stuck",
        ],
    ),
    (
        CompanionEmotionKind::Joy,
        &[
            "开心",
            "高兴",
            "太好了",
            "有起色",
            "顺了",
            "完成了",
            "happy",
            "great",
            "nice",
            "progress",
            "done",
        ],
    ),
    (
        CompanionEmotionKind::Uncertainty,
        &[
            "不知道",
            "不确定",
            "困惑",
            "疑惑",
            "迷茫",
            "怎么做",
            "没思路",
            "confused",
            "uncertain",
            "not sure",
            "lost",
        ],
    ),
    (
        CompanionEmotionKind::Loneliness,
        &[
            "孤独",
            "孤单",
            "没人",
            "陪我",
            "想有人",
            "lonely",
            "alone",
            "companionship",
        ],
    ),
];

fn emotional_intensity(value: &str, lower: &str, kind: CompanionEmotionKind, matches: usize) -> u8 {
    let mut intensity = match kind {
        CompanionEmotionKind::Joy => 45,
        CompanionEmotionKind::Uncertainty => 50,
        CompanionEmotionKind::Frustration => 55,
        CompanionEmotionKind::Anxiety
        | CompanionEmotionKind::Sadness
        | CompanionEmotionKind::Fatigue
        | CompanionEmotionKind::Loneliness => 60,
        CompanionEmotionKind::None => 0,
    };
    intensity += matches.saturating_sub(1).min(3) as u8 * 8;
    if [
        "很",
        "特别",
        "非常",
        "太",
        "真的",
        "崩溃",
        "撑不住",
        "受不了",
        "extremely",
        "very",
        "really",
        "so ",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || value.contains(marker))
    {
        intensity = intensity.saturating_add(25);
    }
    if [
        "有点", "一点", "稍微", "somewhat", "a little", "kind of", "kinda",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || value.contains(marker))
    {
        intensity = intensity.saturating_sub(15);
    }
    intensity.min(100)
}

fn follow_up_prompt_for_emotion(kind: CompanionEmotionKind) -> &'static str {
    match kind {
        CompanionEmotionKind::Anxiety => "要不要先把最压着你的点拆成一小步？",
        CompanionEmotionKind::Sadness => "你希望我先听你说完，还是一起整理下一步？",
        CompanionEmotionKind::Fatigue => "要不要先把当前任务收束成一个最小可做步骤？",
        CompanionEmotionKind::Frustration => "要不要我先帮你定位最卡住的具体点？",
        CompanionEmotionKind::Joy => "要不要顺手把这次有效的做法记录下来？",
        CompanionEmotionKind::Uncertainty => "要不要我给你两个可选方向，再一起取舍？",
        CompanionEmotionKind::Loneliness => "你希望我先陪你聊一会儿，还是一起做点轻量的事？",
        CompanionEmotionKind::None => "你希望我继续跟进这件事吗？",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionTrigger {
    ReminderDue,
    UnfinishedTask,
    LongIdleCheckIn,
    TopicContinuation,
    PeriodicSummary,
    RelationshipMilestone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionAction {
    MessageOnly,
    SuggestNextStep,
    SummarizeStage,
    AskPermissionForTool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionPlan {
    pub trigger: CompanionTrigger,
    pub action: CompanionAction,
    pub reason: String,
    pub message: String,
    pub requires_user_confirmation: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompanionInput {
    pub now_minute_of_day: u16,
    pub proactive_in_session: u32,
    pub proactive_today: u32,
    pub idle_minutes: u64,
    pub reminder_due: bool,
    pub unfinished_task: Option<String>,
    pub topic_continuation: Option<String>,
    pub periodic_summary_due: bool,
    pub relationship_milestone: Option<String>,
    pub tool_request: Option<String>,
}

impl CompanionInput {
    pub fn from_prompt(prompt: &str, relationship_milestone: Option<String>) -> Self {
        let normalized = prompt
            .trim()
            .strip_prefix("companion check:")
            .map(str::trim)
            .unwrap_or_else(|| prompt.trim());
        let lower = normalized.to_ascii_lowercase();
        Self {
            reminder_due: lower.contains("reminder due") || normalized.contains("提醒到期"),
            unfinished_task: normalized
                .strip_prefix("unfinished task:")
                .or_else(|| normalized.strip_prefix("未完成任务："))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            topic_continuation: normalized
                .strip_prefix("continue topic:")
                .or_else(|| normalized.strip_prefix("继续话题："))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            periodic_summary_due: lower.contains("periodic summary")
                || normalized.contains("阶段总结"),
            relationship_milestone,
            tool_request: normalized
                .strip_prefix("tool request:")
                .or_else(|| normalized.strip_prefix("工具请求："))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            idle_minutes: if lower.contains("long idle") || normalized.contains("长时间空闲") {
                120
            } else {
                0
            },
            ..Self::default()
        }
    }

    pub fn has_signal(&self) -> bool {
        self.reminder_due
            || self.unfinished_task.is_some()
            || self.topic_continuation.is_some()
            || self.periodic_summary_due
            || self.relationship_milestone.is_some()
            || self.tool_request.is_some()
            || self.idle_minutes >= 120
    }
}

pub trait CompanionPlanner {
    fn plan(&self, input: CompanionInput) -> Vec<CompanionPlan>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeCompanionPlanner {
    settings: CompanionSettings,
}

impl SafeCompanionPlanner {
    pub fn new(settings: CompanionSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &CompanionSettings {
        &self.settings
    }

    fn allowed(&self, input: &CompanionInput) -> bool {
        self.settings.enabled
            && input.proactive_in_session < self.settings.max_proactive_per_session
            && input.proactive_today < self.settings.max_proactive_per_day
            && !self
                .settings
                .quiet_hours
                .is_some_and(|hours| hours.contains(input.now_minute_of_day))
    }

    fn push(
        &self,
        plans: &mut Vec<CompanionPlan>,
        trigger: CompanionTrigger,
        action: CompanionAction,
        reason: impl Into<String>,
        message: impl Into<String>,
    ) {
        let reason = reason.into();
        if self.settings.require_reason && reason.trim().is_empty() {
            return;
        }
        let requires_user_confirmation = matches!(action, CompanionAction::AskPermissionForTool);
        plans.push(CompanionPlan {
            trigger,
            action,
            reason,
            message: message.into(),
            requires_user_confirmation,
        });
    }
}

impl CompanionPlanner for SafeCompanionPlanner {
    fn plan(&self, input: CompanionInput) -> Vec<CompanionPlan> {
        if !self.allowed(&input) || !input.has_signal() {
            return Vec::new();
        }
        let mut plans = Vec::new();
        if input.reminder_due {
            self.push(
                &mut plans,
                CompanionTrigger::ReminderDue,
                CompanionAction::MessageOnly,
                "a reminder is due",
                "提醒：有一项到期事项需要你留意。",
            );
        } else if let Some(task) = input.unfinished_task.as_deref() {
            self.push(
                &mut plans,
                CompanionTrigger::UnfinishedTask,
                CompanionAction::SuggestNextStep,
                "an unfinished task was observed",
                format!("轻提示：还可以继续处理“{}”。", compact(task)),
            );
        } else if input.idle_minutes >= 120 {
            self.push(
                &mut plans,
                CompanionTrigger::LongIdleCheckIn,
                CompanionAction::MessageOnly,
                "the session has been idle for an extended period",
                "好久没有继续了，回来时可以从上次停下的地方接着做。",
            );
        } else if let Some(topic) = input.topic_continuation.as_deref() {
            self.push(
                &mut plans,
                CompanionTrigger::TopicContinuation,
                CompanionAction::SuggestNextStep,
                "recent context suggests a topic can be continued",
                format!("可以继续关注“{}”。", compact(topic)),
            );
        } else if input.periodic_summary_due {
            self.push(
                &mut plans,
                CompanionTrigger::PeriodicSummary,
                CompanionAction::SummarizeStage,
                "a bounded stage summary is due",
                "阶段小结：可以整理一下当前进展、未完成事项和下一步。",
            );
        } else if let Some(change) = input.relationship_milestone.as_deref() {
            self.push(
                &mut plans,
                CompanionTrigger::RelationshipMilestone,
                CompanionAction::MessageOnly,
                "a recent relationship or context milestone changed",
                format!("最近的上下文有变化：{}。", compact(change)),
            );
        }
        if let Some(tool) = input.tool_request.as_deref() {
            plans.clear();
            if self.settings.allow_tool_requests {
                self.push(
                    &mut plans,
                    CompanionTrigger::UnfinishedTask,
                    CompanionAction::AskPermissionForTool,
                    "a proactive tool action was suggested",
                    format!("如果你确认，我可以请求执行：{}。", compact(tool)),
                );
            }
        }
        plans
    }
}

fn compact(value: &str) -> String {
    let value = value.trim();
    if value.chars().count() <= 80 {
        return value.to_string();
    }
    let mut out = value.chars().take(77).collect::<String>();
    out.push_str("...");
    out
}

pub fn quiet_hours(start_minute: u16, end_minute: u16) -> Option<QuietHours> {
    QuietHours::new(start_minute, end_minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> SafeCompanionPlanner {
        SafeCompanionPlanner::new(CompanionSettings {
            enabled: true,
            ..CompanionSettings::default()
        })
    }

    #[test]
    fn default_settings_do_not_plan() {
        assert!(
            SafeCompanionPlanner::new(CompanionSettings::default())
                .plan(CompanionInput {
                    reminder_due: true,
                    ..CompanionInput::default()
                })
                .is_empty()
        );
    }

    #[test]
    fn plans_reminder_with_reason() {
        let plans = enabled().plan(CompanionInput {
            reminder_due: true,
            ..CompanionInput::default()
        });
        assert_eq!(plans[0].trigger, CompanionTrigger::ReminderDue);
        assert!(!plans[0].reason.is_empty());
    }

    #[test]
    fn quiet_hours_suppress_all_plans() {
        let planner = SafeCompanionPlanner::new(CompanionSettings {
            enabled: true,
            quiet_hours: quiet_hours(22 * 60, 7 * 60),
            ..CompanionSettings::default()
        });
        assert!(
            planner
                .plan(CompanionInput {
                    now_minute_of_day: 23 * 60,
                    reminder_due: true,
                    ..CompanionInput::default()
                })
                .is_empty()
        );
    }

    #[test]
    fn frequency_limits_apply_per_session_and_day() {
        let planner = enabled();
        assert!(
            planner
                .plan(CompanionInput {
                    proactive_in_session: 3,
                    reminder_due: true,
                    ..CompanionInput::default()
                })
                .is_empty()
        );
        assert!(
            planner
                .plan(CompanionInput {
                    proactive_today: 8,
                    reminder_due: true,
                    ..CompanionInput::default()
                })
                .is_empty()
        );
    }

    #[test]
    fn tool_action_requires_confirmation_and_never_executes() {
        let planner = SafeCompanionPlanner::new(CompanionSettings {
            enabled: true,
            allow_tool_requests: true,
            ..CompanionSettings::default()
        });
        let plans = planner.plan(CompanionInput {
            tool_request: Some("open notes".to_string()),
            ..CompanionInput::default()
        });
        assert_eq!(plans[0].action, CompanionAction::AskPermissionForTool);
        assert!(plans[0].requires_user_confirmation);
    }

    #[test]
    fn relationship_signal_is_redacted_to_bounded_reason() {
        let plans = enabled().plan(CompanionInput {
            relationship_milestone: Some("a".repeat(200)),
            ..CompanionInput::default()
        });
        assert!(plans[0].reason.contains("milestone"));
        assert!(plans[0].message.chars().count() < 120);
    }

    #[test]
    fn prompt_parser_extracts_shared_companion_signals() {
        let input = CompanionInput::from_prompt(
            "companion check: 未完成任务：整理陪伴层",
            Some("relationship changed".to_string()),
        );
        assert_eq!(input.unfinished_task.as_deref(), Some("整理陪伴层"));
        assert_eq!(
            input.relationship_milestone.as_deref(),
            Some("relationship changed")
        );
        assert!(input.has_signal());
    }

    #[test]
    fn prompt_parser_keeps_unrelated_prompts_inert() {
        let input = CompanionInput::from_prompt("你好，今天怎么样？", None);
        assert!(!input.has_signal());
        assert!(enabled().plan(input).is_empty());
    }

    #[test]
    fn deterministic_policy_uses_context_without_model_calls() {
        let policy = DeterministicCompanionPolicy::new(CompanionSettings {
            enabled: true,
            ..CompanionSettings::default()
        });
        let decision = policy.decide(
            &CompanionContext {
                persona_id: Some("default".to_string()),
                display_name: Some("YunXi".to_string()),
                relationship_state: Some("steady".to_string()),
                memory_summary: CompanionMemorySummary {
                    boot_summary: Some("用户偏好中文".to_string()),
                    record_count: 1,
                    stable_fact_count: 1,
                    ..CompanionMemorySummary::default()
                },
                available: true,
                ..CompanionContext::default()
            },
            &CompanionInput {
                unfinished_task: Some("测试陪伴层".to_string()),
                ..CompanionInput::default()
            },
        );
        assert!(decision.proactive_care);
        assert!(decision.use_persona_context);
        assert!(decision.use_memory_context);
        assert_eq!(decision.tone, CompanionTone::Warm);
        assert!(decision.follow_up.is_some());
        assert!(!decision.fallback_to_existing_reply);
    }

    #[test]
    fn missing_context_falls_back_without_proactive_follow_up() {
        let policy = DeterministicCompanionPolicy::new(CompanionSettings {
            enabled: true,
            ..CompanionSettings::default()
        });
        let decision = policy.decide(
            &CompanionContext::unavailable(),
            &CompanionInput {
                topic_continuation: Some("旧话题".to_string()),
                ..CompanionInput::default()
            },
        );
        assert!(decision.proactive_care);
        assert!(decision.fallback_to_existing_reply);
        assert!(decision.follow_up.is_none());
        assert!(!decision.use_memory_context);
        assert_eq!(decision.tone, CompanionTone::Neutral);
    }

    #[test]
    fn emotional_clue_selects_supportive_tone() {
        let policy = DeterministicCompanionPolicy::new(CompanionSettings {
            enabled: true,
            ..CompanionSettings::default()
        });
        let decision = policy.decide(
            &CompanionContext {
                available: true,
                emotional_clues: vec!["用户最近有些焦虑".to_string()],
                ..CompanionContext::default()
            },
            &CompanionInput {
                reminder_due: true,
                ..CompanionInput::default()
            },
        );
        assert_eq!(decision.tone, CompanionTone::Supportive);
        assert_eq!(decision.emotion.kind, CompanionEmotionKind::Anxiety);
    }

    #[test]
    fn emotion_classifier_handles_nuanced_companion_signals() {
        let emotion = classify_companion_emotion([
            "我今天真的有点撑不住，压力特别大",
            "但是项目终于有起色了",
        ]);

        assert_eq!(emotion.kind, CompanionEmotionKind::Anxiety);
        assert!(emotion.intensity >= 70);
        assert!(emotion.confidence >= 60);
    }

    #[test]
    fn policy_keeps_stable_persona_and_relationship_metadata() {
        let policy = DeterministicCompanionPolicy::new(CompanionSettings {
            enabled: true,
            ..CompanionSettings::default()
        });
        let decision = policy.decide(
            &CompanionContext {
                available: true,
                persona_id: Some("subjective_soul".to_string()),
                relationship_stage: CompanionRelationshipStage::Established,
                persona_style: CompanionPersonaStyle {
                    soul_signature: Some("sharp-warm-local".to_string()),
                    warmth: 80,
                    directness: 65,
                    initiative: 70,
                    emotional_attunement: 85,
                    reply_rules: vec!["先接住情绪，再给行动建议".to_string()],
                    ..CompanionPersonaStyle::default()
                },
                consistency_key: Some("subjective_soul:sharp-warm-local".to_string()),
                ..CompanionContext::default()
            },
            &CompanionInput {
                topic_continuation: Some("长期陪伴测试".to_string()),
                ..CompanionInput::default()
            },
        );

        assert_eq!(
            decision.relationship_stage,
            CompanionRelationshipStage::Established
        );
        assert_eq!(
            decision.consistency_key.as_deref(),
            Some("subjective_soul:sharp-warm-local")
        );
        assert!(decision.use_persona_context);
        assert_eq!(decision.tone, CompanionTone::Warm);
    }
}
