use crate::memory::{MemoryKind, MemoryScope};

#[derive(Clone, Debug, Default)]
pub struct MemoryScopeRouter;

impl MemoryScopeRouter {
    pub fn route(
        &self,
        kind: MemoryKind,
        content: &str,
        reason: &str,
        workspace_fingerprint: Option<&str>,
        scope_hint: Option<&str>,
    ) -> MemoryScope {
        if matches!(
            kind,
            MemoryKind::RelationshipNote | MemoryKind::EmotionalState
        ) {
            return MemoryScope::Relationship;
        }

        if matches!(kind, MemoryKind::Preference)
            && is_global_interaction_preference(content, reason)
        {
            return MemoryScope::GlobalUser;
        }

        if matches!(kind, MemoryKind::PersonalFact) && !has_project_marker(content, reason) {
            return MemoryScope::GlobalUser;
        }

        if matches!(kind, MemoryKind::Correction) && !has_project_marker(content, reason) {
            return MemoryScope::GlobalUser;
        }

        if matches!(
            kind,
            MemoryKind::ProjectContext | MemoryKind::ToolTraceSummary
        ) || has_workspace_hint(scope_hint)
            || has_project_marker(content, reason)
        {
            if let Some(root_fingerprint) = workspace_fingerprint {
                return MemoryScope::Workspace {
                    root_fingerprint: root_fingerprint.to_string(),
                };
            }
        }

        match kind {
            MemoryKind::RelationshipNote | MemoryKind::EmotionalState => MemoryScope::Relationship,
            MemoryKind::ProjectContext | MemoryKind::ToolTraceSummary => workspace_fingerprint
                .map(|root_fingerprint| MemoryScope::Workspace {
                    root_fingerprint: root_fingerprint.to_string(),
                })
                .unwrap_or(MemoryScope::GlobalUser),
            _ => MemoryScope::GlobalUser,
        }
    }
}

fn has_workspace_hint(scope_hint: Option<&str>) -> bool {
    scope_hint
        .map(|hint| {
            let hint = hint.to_ascii_lowercase();
            hint.contains("workspace") || hint.contains("project")
        })
        .unwrap_or(false)
}

fn is_global_interaction_preference(content: &str, reason: &str) -> bool {
    let combined = format!("{content}\n{reason}");
    contains_any(
        &combined,
        &[
            "中文", "英文", "语言", "回答", "交流", "称呼", "语气", "表达", "language", "answer",
            "reply", "tone",
        ],
    ) && !has_project_marker(content, reason)
}

fn has_project_marker(content: &str, reason: &str) -> bool {
    let combined = format!("{content}\n{reason}");
    contains_any(
        &combined,
        &[
            "当前项目",
            "这个项目",
            "项目",
            "仓库",
            "代码库",
            "工作区",
            "硬性要求",
            "硬约束",
            "开发报告",
            "源码",
            "workspace",
            "project",
            "repo",
            "repository",
            "codebase",
        ],
    )
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| value.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
}
