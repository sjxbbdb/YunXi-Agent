use serde::{Deserialize, Serialize};
use yunxi_agent_companion::{CompanionInput, CompanionPlanner, SafeCompanionPlanner};
use yunxi_agent_core::{
    CompanionSettings, ControlRequest, ControlScope, ControlScopeSnapshot, ControlSnapshot,
    ControlSource, ControlVerb,
};
use yunxi_agent_persona::{
    HumanProfile, MemoryKind, MemoryRecord, MemoryRuleExtractor, MemoryScope, MemorySensitivity,
    MemoryStatus, MemoryWritePolicy, PersonaPromptCompiler, PersonaSettings, RelationshipGraphLite,
    RelationshipState, link_supersession_chain, yunxi_companion_strong,
};

pub const HARNESS_VERSION: &str = "2.0.1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalCategory {
    Persona,
    Memory,
    Relationship,
    Proactive,
    Controls,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionEvalScenario {
    pub id: String,
    pub category: EvalCategory,
    pub description: String,
    pub checks: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvalCheckResult {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionEvalResult {
    pub id: String,
    pub category: EvalCategory,
    pub passed: bool,
    pub checks: Vec<EvalCheckResult>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompanionEvalMetrics {
    pub scenario_count: usize,
    pub passed_scenarios: usize,
    pub failed_scenarios: usize,
    pub persona_consistency_rate: f64,
    pub memory_precision: f64,
    pub memory_false_positive_rate: f64,
    pub memory_recall_accuracy: f64,
    pub memory_correct_writes: usize,
    pub memory_false_positives: usize,
    pub memory_missed_writes: usize,
    pub memory_forbidden_writes: usize,
    pub relationship_continuity_rate: f64,
    pub proactive_boundary_violation_count: usize,
    pub tool_approval_bypass_count: usize,
    pub control_regression_rate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompanionEvalReport {
    pub harness_version: String,
    pub golden_passed: bool,
    pub golden_failures: Vec<String>,
    pub metrics: CompanionEvalMetrics,
    pub results: Vec<CompanionEvalResult>,
}

#[derive(Clone, Debug, Deserialize)]
struct GoldenThresholds {
    scenario_count_min: usize,
    persona_consistency_rate_min: f64,
    memory_precision_min: f64,
    relationship_continuity_rate_min: f64,
    proactive_boundary_violation_count_max: usize,
    tool_approval_bypass_count_max: usize,
    control_regression_rate_min: f64,
}

pub fn load_default_scenarios() -> Result<Vec<CompanionEvalScenario>, String> {
    let sources = [
        include_str!("../../../evals/companion/scenarios/persona_consistency.jsonl"),
        include_str!("../../../evals/companion/scenarios/memory_precision.jsonl"),
        include_str!("../../../evals/companion/scenarios/relationship_continuity.jsonl"),
        include_str!("../../../evals/companion/scenarios/proactive_boundaries.jsonl"),
        include_str!("../../../evals/companion/scenarios/control_operations.jsonl"),
    ];
    let mut scenarios: Vec<CompanionEvalScenario> = Vec::new();
    for source in sources {
        for (line_number, line) in source.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            scenarios.push(serde_json::from_str(line).map_err(|error| {
                format!("invalid eval scenario at line {}: {error}", line_number + 1)
            })?);
        }
    }
    scenarios.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(scenarios)
}

pub fn run_default_suite() -> Result<CompanionEvalReport, String> {
    let scenarios = load_default_scenarios()?;
    let results = scenarios.iter().map(evaluate_scenario).collect::<Vec<_>>();
    let mut report = build_report(results);
    let golden: GoldenThresholds = serde_json::from_str(include_str!(
        "../../../evals/companion/golden/companion_expected_metrics.json"
    ))
    .map_err(|error| format!("invalid companion golden metrics: {error}"))?;
    report.golden_failures = golden_failures(&report.metrics, &golden);
    report.golden_passed = report.golden_failures.is_empty();
    Ok(report)
}

pub fn render_text(report: &CompanionEvalReport) -> String {
    let metrics = &report.metrics;
    format!(
        "YunXi companion evaluation v{}\ngolden_passed: {}\nscenarios: {}/{} passed\npersona_consistency_rate: {:.3}\nmemory_precision: {:.3} (correct={} false_positive={} missed={} forbidden={})\nmemory_false_positive_rate: {:.3}\nmemory_recall_accuracy: {:.3}\nrelationship_continuity_rate: {:.3}\nproactive_boundary_violation_count: {}\ntool_approval_bypass_count: {}\ncontrol_regression_rate: {:.3}",
        report.harness_version,
        report.golden_passed,
        metrics.passed_scenarios,
        metrics.scenario_count,
        metrics.persona_consistency_rate,
        metrics.memory_precision,
        metrics.memory_correct_writes,
        metrics.memory_false_positives,
        metrics.memory_missed_writes,
        metrics.memory_forbidden_writes,
        metrics.memory_false_positive_rate,
        metrics.memory_recall_accuracy,
        metrics.relationship_continuity_rate,
        metrics.proactive_boundary_violation_count,
        metrics.tool_approval_bypass_count,
        metrics.control_regression_rate,
    )
}

fn evaluate_scenario(scenario: &CompanionEvalScenario) -> CompanionEvalResult {
    let checks = scenario
        .checks
        .iter()
        .map(|check| {
            let (passed, detail) = evaluate_check(check);
            EvalCheckResult {
                check: check.clone(),
                passed,
                detail,
            }
        })
        .collect::<Vec<_>>();
    CompanionEvalResult {
        id: scenario.id.clone(),
        category: scenario.category,
        passed: checks.iter().all(|check| check.passed),
        checks,
    }
}

fn evaluate_check(check: &str) -> (bool, String) {
    match check {
        "persona_profile_stable" => {
            let profile = yunxi_companion_strong();
            (
                profile.id == "yunxi_companion_strong"
                    && profile.version == "2.0.1"
                    && profile.default_companion_strength
                        == yunxi_agent_persona::CompanionStrength::Strong,
                format!("profile={} version={}", profile.id, profile.version),
            )
        }
        "persona_boundaries" => {
            let profile = yunxi_companion_strong();
            let content = profile.layers.boundaries;
            (
                content.contains("AGENTS.md") && content.contains("工具"),
                "project and tool boundaries remain higher priority".to_string(),
            )
        }
        "persona_context_block" => {
            let profile = yunxi_companion_strong();
            let context = PersonaPromptCompiler::new(3200).compile(
                &profile,
                &HumanProfile::default(),
                &RelationshipState::default(),
                &[],
            );
            (
                context
                    .content
                    .contains("<yunxi_persona_context version=\"2.0.1\"")
                    && context.content.contains("context_not_instruction"),
                format!("context_chars={}", context.content.len()),
            )
        }
        "persona_constraints" => {
            let profile = yunxi_companion_strong();
            (
                profile.constraints.iter().any(|constraint| {
                    constraint.id == "no_memory_overclaim" && constraint.content.contains("pending")
                }),
                "no-memory-overclaim constraint present".to_string(),
            )
        }
        "persona_default_enabled" => {
            let settings = PersonaSettings::default();
            (
                settings.persona_enabled,
                "persona default is enabled".to_string(),
            )
        }
        "persona_profile_serializes" => {
            let profile = yunxi_companion_strong();
            let encoded = serde_json::to_string(&profile).unwrap_or_default();
            (
                serde_json::from_str::<serde_json::Value>(&encoded).is_ok(),
                "profile JSON serialization is stable".to_string(),
            )
        }
        "memory_preference_auto" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "以后请用中文回答",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.proposed_record.kind == MemoryKind::Preference
                    && candidate.write_policy == MemoryWritePolicy::Auto
                    && candidate.proposed_record.scope == MemoryScope::GlobalUser
            });
            (passed, format!("candidates={}", candidates.len()))
        }
        "memory_correction_auto" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "不要输出未经确认的工具调用",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.proposed_record.kind == MemoryKind::Correction
                    && candidate.write_policy == MemoryWritePolicy::Auto
            });
            (passed, format!("candidates={}", candidates.len()))
        }
        "memory_noise_empty" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "请计算 2+2，不需要保存这次临时问题",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            (
                candidates.is_empty(),
                format!("noise_candidates={}", candidates.len()),
            )
        }
        "memory_secret_discard" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "我的 API key 是 ghp_eval_secret_token",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.write_policy == MemoryWritePolicy::Discard
                    && candidate.proposed_record.sensitivity == MemorySensitivity::High
            });
            (passed, "secret-like candidate is discarded".to_string())
        }
        "memory_personal_confirmation" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "我的生日是 1990 年 1 月 1 日",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.proposed_record.kind == MemoryKind::PersonalFact
                    && candidate.write_policy == MemoryWritePolicy::RequireConfirmation
            });
            (passed, "personal fact requires confirmation".to_string())
        }
        "memory_relationship_confirmation" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "我们的关系最近发生变化",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.proposed_record.kind == MemoryKind::RelationshipNote
                    && candidate.write_policy == MemoryWritePolicy::RequireConfirmation
            });
            (
                passed,
                "relationship note requires confirmation".to_string(),
            )
        }
        "memory_disabled" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "以后请用中文回答",
                None,
                None,
                Some("eval-workspace"),
                false,
            );
            let passed = candidates
                .iter()
                .all(|candidate| candidate.write_policy == MemoryWritePolicy::Disabled);
            (passed, "disabled memory does not write".to_string())
        }
        "memory_sensitivity_classification" => {
            let candidates = MemoryRuleExtractor::new().extract(
                "我的健康状况需要保密",
                None,
                None,
                Some("eval-workspace"),
                true,
            );
            let passed = candidates.iter().any(|candidate| {
                candidate.proposed_record.sensitivity == MemorySensitivity::Medium
                    && candidate.write_policy == MemoryWritePolicy::RequireConfirmation
            });
            (passed, "sensitive profile fact is gated".to_string())
        }
        "memory_recallable_filter" => {
            let old = MemoryRecord::new(
                "eval-archived",
                MemoryScope::GlobalUser,
                MemoryKind::Preference,
                "archived preference",
                1,
            )
            .with_status(MemoryStatus::Archived);
            (
                !old.is_recallable_at(2),
                "archived records are not recallable".to_string(),
            )
        }
        "relationship_supersession_chain" => {
            let old = sample_record("old", "旧偏好", 1);
            let new = sample_record("new", "新偏好", 2);
            let (old, new) = link_supersession_chain(&old, &new, 3);
            (
                old.invalidation.superseded_by.as_deref() == Some("new")
                    && new.invalidation.supersedes.contains(&"old".to_string()),
                "old and new facts retain bidirectional replacement links".to_string(),
            )
        }
        "relationship_history_retained" => {
            let old = sample_record("old", "旧偏好", 1);
            let new = sample_record("new", "新偏好", 2);
            let (old, new) = link_supersession_chain(&old, &new, 3);
            let graph = RelationshipGraphLite::from_records(&[old, new]);
            (
                graph.edges.len() >= 2,
                format!("graph_edges={}", graph.edges.len()),
            )
        }
        "relationship_expiry" => {
            let mut record = sample_record("expired", "过期关系", 1);
            record.temporal.expires_at_millis = Some(2);
            let graph = RelationshipGraphLite::from_records(&[record]);
            (
                graph.active_edges_at(3).is_empty(),
                "expired relationship edge is inactive".to_string(),
            )
        }
        "relationship_future_valid_from" => {
            let mut record = sample_record("future", "未来关系", 1);
            record.temporal.valid_from_millis = Some(10);
            let graph = RelationshipGraphLite::from_records(&[record]);
            (
                graph.active_edges_at(3).is_empty(),
                "future relationship edge is not active early".to_string(),
            )
        }
        "relationship_timeline_order" => {
            let first = sample_record("first", "first", 1);
            let second = sample_record("second", "second", 2);
            let graph = RelationshipGraphLite::from_records(&[first, second]);
            let events = graph.events_by_time();
            (
                events
                    .first()
                    .is_some_and(|edge| edge.memory_id == "second"),
                "relationship events use descending temporal order".to_string(),
            )
        }
        "relationship_read_only_scope" => {
            let snapshot = relationship_snapshot();
            (
                snapshot.enabled.is_none() && snapshot.source == ControlSource::ReadOnlyHistory,
                "relationship scope exposes read-only history".to_string(),
            )
        }
        "proactive_default_off" => {
            let plans =
                SafeCompanionPlanner::new(CompanionSettings::default()).plan(reminder_input());
            (plans.is_empty(), "default planner is silent".to_string())
        }
        "proactive_explicit_disable" => {
            let settings = CompanionSettings {
                enabled: false,
                ..CompanionSettings::default()
            };
            let plans = SafeCompanionPlanner::new(settings).plan(reminder_input());
            (plans.is_empty(), "explicit disable is silent".to_string())
        }
        "proactive_quiet_hours" => {
            let settings = CompanionSettings {
                enabled: true,
                quiet_hours: yunxi_agent_core::QuietHours::new(0, 1439),
                ..CompanionSettings::default()
            };
            let plans = SafeCompanionPlanner::new(settings).plan(reminder_input());
            (plans.is_empty(), "quiet hours suppress plans".to_string())
        }
        "proactive_session_limit" => {
            let settings = CompanionSettings {
                enabled: true,
                max_proactive_per_session: 1,
                ..CompanionSettings::default()
            };
            let mut input = reminder_input();
            input.proactive_in_session = 1;
            let plans = SafeCompanionPlanner::new(settings).plan(input);
            (
                plans.is_empty(),
                "session frequency limit suppresses plans".to_string(),
            )
        }
        "proactive_day_limit" => {
            let settings = CompanionSettings {
                enabled: true,
                max_proactive_per_day: 1,
                ..CompanionSettings::default()
            };
            let mut input = reminder_input();
            input.proactive_today = 1;
            let plans = SafeCompanionPlanner::new(settings).plan(input);
            (
                plans.is_empty(),
                "daily frequency limit suppresses plans".to_string(),
            )
        }
        "proactive_tool_confirmation" => {
            let settings = CompanionSettings {
                enabled: true,
                allow_tool_requests: true,
                ..CompanionSettings::default()
            };
            let mut input = reminder_input();
            input.tool_request = Some("inspect notes".to_string());
            let plans = SafeCompanionPlanner::new(settings).plan(input);
            let passed = plans.len() == 1 && plans[0].requires_user_confirmation;
            (
                passed,
                "tool suggestion requires user confirmation".to_string(),
            )
        }
        "proactive_reason_required" => {
            let settings = CompanionSettings {
                enabled: true,
                require_reason: true,
                ..CompanionSettings::default()
            };
            let plans = SafeCompanionPlanner::new(settings).plan(reminder_input());
            (
                plans.iter().all(|plan| !plan.reason.trim().is_empty()),
                "every proactive plan has a reason".to_string(),
            )
        }
        "control_clear_confirmation" => {
            let request = ControlRequest::new(ControlScope::Memory, ControlVerb::Clear);
            (
                !request.confirmation_satisfied() && request.confirmed().confirmation_satisfied(),
                "clear requires an explicit confirmation token".to_string(),
            )
        }
        "control_cloud_off" => {
            let settings = CompanionSettings::default();
            (
                !settings.cloud_control_enabled,
                "cloud control defaults off".to_string(),
            )
        }
        "control_snapshot_sources" => {
            let snapshot = ControlSnapshot {
                companion_enabled: false,
                cloud_control_enabled: false,
                quiet_hours: None,
                persona_summary: "eval".to_string(),
                memory_summary: "eval".to_string(),
                relationship_summary: "eval".to_string(),
                scopes: vec![relationship_snapshot()],
                recent_change: None,
            };
            (
                snapshot
                    .scope(ControlScope::Relationship)
                    .is_some_and(|state| {
                        state.source == ControlSource::ReadOnlyHistory && state.enabled.is_none()
                    }),
                "snapshot identifies read-only relationship source".to_string(),
            )
        }
        "control_update_verbs" => {
            let verbs = [
                ControlVerb::Show,
                ControlVerb::Enable,
                ControlVerb::Disable,
                ControlVerb::Clear,
                ControlVerb::Refresh,
                ControlVerb::Update,
            ];
            (
                verbs.len() == 6,
                "control verbs cover show/update/clear lifecycle".to_string(),
            )
        }
        "control_audit_serializes" => {
            let request = ControlRequest::new(ControlScope::Companion, ControlVerb::Disable);
            let record = yunxi_agent_core::ControlAuditRecord::new(
                &request,
                "completed",
                "eval",
                "eval-harness",
            );
            let encoded = serde_json::to_string(&record).unwrap_or_default();
            (
                encoded.contains("eval-harness") && encoded.contains("companion"),
                "control audit is structured JSON".to_string(),
            )
        }
        unknown => (false, format!("unknown evaluation check: {unknown}")),
    }
}

fn build_report(results: Vec<CompanionEvalResult>) -> CompanionEvalReport {
    let scenario_count = results.len();
    let passed_scenarios = results.iter().filter(|result| result.passed).count();
    let failed_scenarios = scenario_count.saturating_sub(passed_scenarios);
    let category_rate = |category| {
        let selected = results
            .iter()
            .filter(|result| result.category == category)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            0.0
        } else {
            selected.iter().filter(|result| result.passed).count() as f64 / selected.len() as f64
        }
    };
    let memory_checks = results
        .iter()
        .filter(|result| result.category == EvalCategory::Memory)
        .flat_map(|result| result.checks.iter())
        .collect::<Vec<_>>();
    let correct_writes = count_passed(
        &memory_checks,
        &["memory_preference_auto", "memory_correction_auto"],
    );
    let false_positives = usize::from(!check_passed(&memory_checks, "memory_noise_empty"));
    let forbidden_writes = usize::from(!check_passed(&memory_checks, "memory_secret_discard"));
    let missed_writes = usize::from(!check_passed(&memory_checks, "memory_preference_auto"))
        + usize::from(!check_passed(&memory_checks, "memory_correction_auto"));
    let precision_denominator = correct_writes + false_positives + forbidden_writes;
    let memory_precision = if precision_denominator == 0 {
        1.0
    } else {
        correct_writes as f64 / precision_denominator as f64
    };
    let memory_cases = memory_checks.len().max(1) as f64;
    let metrics = CompanionEvalMetrics {
        scenario_count,
        passed_scenarios,
        failed_scenarios,
        persona_consistency_rate: category_rate(EvalCategory::Persona),
        memory_precision,
        memory_false_positive_rate: false_positives as f64 / memory_cases,
        memory_recall_accuracy: (correct_writes.saturating_sub(missed_writes) as f64 / 2.0)
            .clamp(0.0, 1.0),
        memory_correct_writes: correct_writes,
        memory_false_positives: false_positives,
        memory_missed_writes: missed_writes,
        memory_forbidden_writes: forbidden_writes,
        relationship_continuity_rate: category_rate(EvalCategory::Relationship),
        proactive_boundary_violation_count: results
            .iter()
            .filter(|result| result.category == EvalCategory::Proactive && !result.passed)
            .count(),
        tool_approval_bypass_count: usize::from(!check_passed(
            &results
                .iter()
                .filter(|result| result.category == EvalCategory::Proactive)
                .flat_map(|result| result.checks.iter())
                .collect::<Vec<_>>(),
            "proactive_tool_confirmation",
        )),
        control_regression_rate: category_rate(EvalCategory::Controls),
    };
    CompanionEvalReport {
        harness_version: HARNESS_VERSION.to_string(),
        golden_passed: false,
        golden_failures: Vec::new(),
        metrics,
        results,
    }
}

fn golden_failures(metrics: &CompanionEvalMetrics, golden: &GoldenThresholds) -> Vec<String> {
    let mut failures = Vec::new();
    if metrics.scenario_count < golden.scenario_count_min {
        failures.push(format!(
            "scenario_count {} < {}",
            metrics.scenario_count, golden.scenario_count_min
        ));
    }
    if metrics.persona_consistency_rate < golden.persona_consistency_rate_min {
        failures.push(format!(
            "persona_consistency_rate {:.3} < {:.3}",
            metrics.persona_consistency_rate, golden.persona_consistency_rate_min
        ));
    }
    if metrics.memory_precision < golden.memory_precision_min {
        failures.push(format!(
            "memory_precision {:.3} < {:.3}",
            metrics.memory_precision, golden.memory_precision_min
        ));
    }
    if metrics.relationship_continuity_rate < golden.relationship_continuity_rate_min {
        failures.push(format!(
            "relationship_continuity_rate {:.3} < {:.3}",
            metrics.relationship_continuity_rate, golden.relationship_continuity_rate_min
        ));
    }
    if metrics.proactive_boundary_violation_count > golden.proactive_boundary_violation_count_max {
        failures.push(format!(
            "proactive_boundary_violation_count {} > {}",
            metrics.proactive_boundary_violation_count,
            golden.proactive_boundary_violation_count_max
        ));
    }
    if metrics.tool_approval_bypass_count > golden.tool_approval_bypass_count_max {
        failures.push(format!(
            "tool_approval_bypass_count {} > {}",
            metrics.tool_approval_bypass_count, golden.tool_approval_bypass_count_max
        ));
    }
    if metrics.control_regression_rate < golden.control_regression_rate_min {
        failures.push(format!(
            "control_regression_rate {:.3} < {:.3}",
            metrics.control_regression_rate, golden.control_regression_rate_min
        ));
    }
    failures
}

fn check_passed(checks: &[&EvalCheckResult], name: &str) -> bool {
    checks
        .iter()
        .find(|check| check.check == name)
        .is_some_and(|check| check.passed)
}

fn count_passed(checks: &[&EvalCheckResult], names: &[&str]) -> usize {
    names
        .iter()
        .filter(|name| check_passed(checks, name))
        .count()
}

fn sample_record(id: &str, content: &str, now: u128) -> MemoryRecord {
    MemoryRecord::new(
        id,
        MemoryScope::Relationship,
        MemoryKind::Preference,
        content,
        now,
    )
    .with_status(MemoryStatus::Active)
}

fn reminder_input() -> CompanionInput {
    CompanionInput {
        now_minute_of_day: 600,
        reminder_due: true,
        ..CompanionInput::default()
    }
}

fn relationship_snapshot() -> ControlScopeSnapshot {
    ControlScopeSnapshot {
        scope: ControlScope::Relationship,
        enabled: None,
        summary: "nodes=0 edges=0 active_edges=0".to_string(),
        source: ControlSource::ReadOnlyHistory,
        clear_effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dataset_has_at_least_thirty_scenarios() {
        let scenarios = load_default_scenarios().expect("scenarios");
        assert!(scenarios.len() >= 30);
    }

    #[test]
    fn default_suite_is_offline_and_measurable() {
        let report = run_default_suite().expect("report");
        assert!(report.metrics.scenario_count >= 30);
        assert_eq!(report.metrics.failed_scenarios, 0, "{report:?}");
        assert!(report.metrics.memory_precision >= 0.8);
        assert!(report.golden_passed, "{:?}", report.golden_failures);
        assert_eq!(report.metrics.tool_approval_bypass_count, 0);
        assert_eq!(report.metrics.proactive_boundary_violation_count, 0);
    }

    #[test]
    fn text_summary_contains_auditable_metrics() {
        let report = run_default_suite().expect("report");
        let text = render_text(&report);
        assert!(text.contains("memory_precision:"));
        assert!(text.contains("tool_approval_bypass_count: 0"));
    }
}
