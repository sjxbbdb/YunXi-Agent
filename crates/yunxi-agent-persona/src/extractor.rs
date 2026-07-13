use crate::memory::{
    MemoryCandidate, MemoryKind, MemoryRecord, MemoryScope, MemoryStatus, now_millis,
};
use crate::policy::{MemoryWritePolicy, MemoryWritePolicyEngine};

#[derive(Clone, Debug, Default)]
pub struct MemoryRuleExtractor {
    policy: MemoryWritePolicyEngine,
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
        let scope = workspace_fingerprint
            .map(|root_fingerprint| MemoryScope::Workspace {
                root_fingerprint: root_fingerprint.to_string(),
            })
            .unwrap_or(MemoryScope::GlobalUser);

        for (index, (kind, content, reason)) in
            detect_prompt_memories(prompt).into_iter().enumerate()
        {
            let sensitivity = self.policy.classify_content(&content);
            let write_policy = self
                .policy
                .policy_for(kind, sensitivity, &content, memory_enabled);
            let status = match write_policy {
                MemoryWritePolicy::Auto => MemoryStatus::Active,
                MemoryWritePolicy::RequireConfirmation => MemoryStatus::Pending,
                MemoryWritePolicy::Discard | MemoryWritePolicy::Disabled => MemoryStatus::Rejected,
            };
            let mut record = MemoryRecord::new(
                format!("mem-{now}-{index}"),
                scope.clone(),
                kind,
                content.clone(),
                now,
            )
            .with_scores(0.82, 0.65)
            .with_sensitivity(sensitivity)
            .with_status(status);
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
                    reason: "rule:project-constraint-summary".to_string(),
                });
            }
        }

        candidates
    }
}

fn detect_prompt_memories(prompt: &str) -> Vec<(MemoryKind, String, String)> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let lower = trimmed.to_ascii_lowercase();

    if trimmed.contains("以后") && (trimmed.contains("中文") || trimmed.contains("说中文")) {
        out.push((
            MemoryKind::Preference,
            "用户偏好后续默认使用中文交流。".to_string(),
            "rule:language-preference".to_string(),
        ));
    }
    if trimmed.contains("说中文") || trimmed.contains("用中文") {
        out.push((
            MemoryKind::Preference,
            "用户偏好使用中文回答。".to_string(),
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
