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
    pub memory_summary: CompanionMemorySummary,
    pub emotional_clues: Vec<String>,
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
                || !self.emotional_clues.is_empty())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionPolicyDecision {
    pub tone: CompanionTone,
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
        let use_persona_context =
            context_available && (context.persona_id.is_some() || context.display_name.is_some());
        let supportive = context
            .emotional_clues
            .iter()
            .any(|clue| contains_emotional_signal(clue));
        let tone = if supportive {
            CompanionTone::Supportive
        } else if use_persona_context || use_memory_context {
            CompanionTone::Warm
        } else if input.tool_request.is_some() {
            CompanionTone::Direct
        } else {
            CompanionTone::Neutral
        };
        let proactive_care = self.settings.enabled && input.has_signal();
        let follow_up = if proactive_care
            && context_available
            && input.tool_request.is_none()
            && (input.unfinished_task.is_some() || input.topic_continuation.is_some())
        {
            Some(CompanionFollowUp {
                prompt: "你希望我继续跟进这件事吗？".to_string(),
                required: false,
            })
        } else {
            None
        };
        CompanionPolicyDecision {
            tone,
            proactive_care,
            follow_up,
            use_persona_context,
            use_memory_context,
            fallback_to_existing_reply: !context_available,
        }
    }
}

fn contains_emotional_signal(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "焦虑", "难过", "疲惫", "压力", "担心", "sad", "anxious", "tired", "stress", "worried",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
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
    }
}
