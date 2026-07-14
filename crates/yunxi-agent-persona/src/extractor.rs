use crate::dedup::deduplicate_candidates;
use crate::memory::{MemoryCandidate, MemoryKind, MemoryRecord, MemoryStatus, now_millis};
use crate::policy::{MemoryWritePolicy, MemoryWritePolicyEngine};
use crate::scope::MemoryScopeRouter;

#[derive(Clone, Debug, Default)]
pub struct MemoryRuleExtractor {
    policy: MemoryWritePolicyEngine,
    scope_router: MemoryScopeRouter,
}

impl MemoryRuleExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(
        &self,
        prompt: &str,
        assistant_response: Option<&str>,
        source_session_id: Option<&str>,
        workspace_fingerprint: Option<&str>,
        memory_enabled: bool,
    ) -> Vec<MemoryCandidate> {
        let mut candidates = Vec::new();
        let now = now_millis();

        for (index, (kind, content, reason)) in
            detect_prompt_memories(prompt).into_iter().enumerate()
        {
            let scope =
                self.scope_router
                    .route(kind, &content, &reason, workspace_fingerprint, None);
            let sensitivity = self.policy.classify_content(&content);
            let write_policy = self
                .policy
                .policy_for(kind, sensitivity, &content, memory_enabled);
            let mut record = MemoryRecord::new(
                format!("mem-{now}-{index}"),
                scope,
                kind,
                content.clone(),
                now,
            )
            .with_scores(0.82, 0.65)
            .with_sensitivity(sensitivity)
            .with_status(crate::dedup::status_for_policy(write_policy));
            if let Some(source_session_id) = source_session_id {
                record = record.with_source_session_id(source_session_id);
            }
            candidates.push(MemoryCandidate {
                proposed_record: record,
                evidence: prompt.trim().to_string(),
                write_policy,
                reason,
            });
        }

        if let Some(response) = assistant_response {
            if response.contains("YunXi autonomous runtime accepted prompt")
                && prompt.contains("项目")
                && prompt.contains("硬性")
            {
                let content = "用户强调当前项目开发需要遵守既定硬性约束。".to_string();
                let reason = "rule:project-constraint-summary".to_string();
                let scope = self.scope_router.route(
                    MemoryKind::ProjectContext,
                    &content,
                    &reason,
                    workspace_fingerprint,
                    Some("workspace"),
                );
                let record = MemoryRecord::new(
                    format!("mem-{now}-assistant-summary"),
                    scope,
                    MemoryKind::ProjectContext,
                    content,
                    now,
                )
                .with_status(MemoryStatus::Active);
                candidates.push(MemoryCandidate {
                    proposed_record: record,
                    evidence: response.to_string(),
                    write_policy: MemoryWritePolicy::Auto,
                    reason,
                });
            }
        }

        deduplicate_candidates(candidates)
    }
}

fn detect_prompt_memories(prompt: &str) -> Vec<(MemoryKind, String, String)> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let lower = trimmed.to_ascii_lowercase();

    if is_future_chinese_language_preference(trimmed) {
        out.push((
            MemoryKind::Preference,
            "用户偏好后续默认使用中文交流。".to_string(),
            "rule:language-preference".to_string(),
        ));
    }
    if is_direct_chinese_language_preference(trimmed) {
        out.push((
            MemoryKind::Preference,
            "用户偏好使用中文回答。".to_string(),
            "rule:language-preference-direct".to_string(),
        ));
    }
    if is_future_english_language_preference(trimmed, &lower) {
        out.push((
            MemoryKind::Preference,
            "用户偏好后续默认使用英文交流。".to_string(),
            "rule:language-preference".to_string(),
        ));
    }
    if is_direct_english_language_preference(trimmed, &lower) {
        out.push((
            MemoryKind::Preference,
            "用户偏好使用英文回答。".to_string(),
            "rule:language-preference-direct".to_string(),
        ));
    }
    if trimmed.contains("不要") || lower.contains("do not") {
        out.push((
            MemoryKind::Correction,
            format!("用户纠正/限制：{}", compact(trimmed, 160)),
            "rule:correction".to_string(),
        ));
    }
    if trimmed.contains("硬性要求") || trimmed.contains("硬约束") {
        out.push((
            MemoryKind::ProjectContext,
            format!("项目硬性约束：{}", compact(trimmed, 180)),
            "rule:project-hard-constraint".to_string(),
        ));
    }
    if trimmed.contains("我的") || lower.contains("my ") {
        out.push((
            MemoryKind::PersonalFact,
            format!("用户自述事实候选：{}", compact(trimmed, 160)),
            "rule:self-disclosure".to_string(),
        ));
    }

    out
}

fn is_future_chinese_language_preference(value: &str) -> bool {
    value.contains("以后") && (value.contains("中文") || value.contains("说中文"))
}

fn is_direct_chinese_language_preference(value: &str) -> bool {
    value.contains("说中文") || value.contains("用中文")
}

fn is_future_english_language_preference(value: &str, lower: &str) -> bool {
    let mentions_english =
        value.contains("英文") || value.contains("英语") || lower.contains("english");
    mentions_english
        && (value.contains("以后") || lower.contains("from now on") || lower.contains("by default"))
}

fn is_direct_english_language_preference(value: &str, lower: &str) -> bool {
    value.contains("说英文")
        || value.contains("用英文")
        || value.contains("说英语")
        || value.contains("用英语")
        || lower.contains("answer in english")
        || lower.contains("reply in english")
        || lower.contains("respond in english")
        || lower.contains("use english")
        || lower.contains("english please")
}

fn compact(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}
