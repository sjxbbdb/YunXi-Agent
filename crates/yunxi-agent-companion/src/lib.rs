use serde::{Deserialize, Serialize};
use yunxi_agent_core::{CompanionSettings, QuietHours};

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
        if !self.allowed(&input) {
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
}
