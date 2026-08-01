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

    if let Some(memory) = detect_explicit_remembered_memory(trimmed, &lower) {
        out.push(memory);
    }
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
    if trimmed.contains("关系") || lower.contains("relationship") {
        out.push((
            MemoryKind::RelationshipNote,
            format!("关系事件候选：{}", compact(trimmed, 160)),
            "rule:relationship-note".to_string(),
        ));
    }
    if trimmed.contains("情绪")
        || trimmed.contains("感到")
        || lower.contains("i feel")
        || lower.contains("emotion")
    {
        out.push((
            MemoryKind::EmotionalState,
            format!("情绪状态候选：{}", compact(trimmed, 160)),
            "rule:emotional-state".to_string(),
        ));
    }
    if trimmed.contains("目标") || lower.contains("my goal") {
        out.push((
            MemoryKind::Goal,
            format!("目标候选：{}", compact(trimmed, 160)),
            "rule:goal".to_string(),
        ));
    }

    out
}

fn detect_explicit_remembered_memory(
    value: &str,
    lower: &str,
) -> Option<(MemoryKind, String, String)> {
    let payload = explicit_memory_payload(value, lower)?;
    let payload = trim_followup_instruction(payload);
    let payload = normalize_payload(payload);
    if payload.is_empty() {
        return None;
    }

    let kind = classify_explicit_memory_kind(payload);
    Some((
        kind,
        normalize_explicit_memory_content(kind, payload),
        explicit_memory_reason(kind).to_string(),
    ))
}

fn explicit_memory_payload<'a>(value: &'a str, lower: &str) -> Option<&'a str> {
    let markers = [
        "我希望你记住",
        "请你记住",
        "请记住",
        "帮我记住",
        "你要记住",
        "please remember that",
        "please remember",
        "remember that",
        "keep in mind that",
    ];
    markers.iter().find_map(|marker| {
        lower
            .find(marker)
            .map(|start| normalize_payload(&value[start + marker.len()..]))
            .filter(|payload| !payload.is_empty())
    })
}

fn normalize_payload(value: &str) -> &str {
    let mut out = trim_memory_payload_punctuation(value);
    loop {
        let before = out;
        for prefix in ["一个", "一下", "这点", "这件事", "this", "that"] {
            if let Some(stripped) = out.strip_prefix(prefix) {
                out = trim_memory_payload_punctuation(stripped);
            }
        }
        if out == before {
            return out;
        }
    }
}

fn trim_memory_payload_punctuation(value: &str) -> &str {
    value
        .trim()
        .trim_start_matches(['：', ':', '，', ',', '。', '.', '；', ';', ' '])
        .trim()
        .trim_end_matches(['。', '.', '；', ';', '，', ',', ' '])
        .trim()
}

fn trim_followup_instruction(value: &str) -> &str {
    let lower = value.to_ascii_lowercase();
    let mut end = value.len();
    for marker in [
        "请只回复",
        "只回复",
        "不用解释",
        "不要解释",
        "only reply",
        "reply only",
        "respond only",
        "do not explain",
        "don't explain",
    ] {
        if let Some(index) = lower.find(marker) {
            end = end.min(index);
        }
    }
    &value[..end]
}

fn classify_explicit_memory_kind(value: &str) -> MemoryKind {
    let lower = value.to_ascii_lowercase();
    if value.contains("偏好")
        || value.contains("喜欢")
        || value.contains("希望")
        || value.contains("以后")
        || lower.contains("prefer")
        || lower.contains("preference")
        || lower.contains("from now on")
        || lower.contains("by default")
    {
        MemoryKind::Preference
    } else if value.contains("目标") || lower.contains("goal") {
        MemoryKind::Goal
    } else if value.contains("项目")
        || value.contains("硬性要求")
        || value.contains("硬约束")
        || lower.contains("project")
    {
        MemoryKind::ProjectContext
    } else if value.contains("关系") || lower.contains("relationship") {
        MemoryKind::RelationshipNote
    } else if value.contains("情绪") || value.contains("感到") || lower.contains("emotion") {
        MemoryKind::EmotionalState
    } else {
        MemoryKind::PersonalFact
    }
}

fn normalize_explicit_memory_content(kind: MemoryKind, payload: &str) -> String {
    let payload = compact(payload, 180);
    match kind {
        MemoryKind::Preference => {
            if payload.starts_with("用户偏好") {
                payload
            } else {
                format!("用户偏好：{payload}")
            }
        }
        MemoryKind::Goal => format!("用户目标：{payload}"),
        MemoryKind::ProjectContext => format!("项目上下文：{payload}"),
        MemoryKind::RelationshipNote => format!("关系事件候选：{payload}"),
        MemoryKind::EmotionalState => format!("情绪状态候选：{payload}"),
        MemoryKind::PersonalFact => format!("用户自述事实候选：{payload}"),
        MemoryKind::Correction => format!("用户纠正/限制：{payload}"),
        MemoryKind::Event => format!("事件候选：{payload}"),
        MemoryKind::ToolTraceSummary => format!("工具轨迹摘要：{payload}"),
    }
}

fn explicit_memory_reason(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Preference => "rule:explicit-remember-preference",
        MemoryKind::Goal => "rule:explicit-remember-goal",
        MemoryKind::ProjectContext => "rule:explicit-remember-project-context",
        MemoryKind::RelationshipNote => "rule:explicit-remember-relationship-note",
        MemoryKind::EmotionalState => "rule:explicit-remember-emotional-state",
        MemoryKind::PersonalFact => "rule:explicit-remember-personal-fact",
        MemoryKind::Correction => "rule:explicit-remember-correction",
        MemoryKind::Event => "rule:explicit-remember-event",
        MemoryKind::ToolTraceSummary => "rule:explicit-remember-tool-trace",
    }
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
