#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ErrorCategory {
    Provider,
    Tool,
    Approval,
    Cancel,
    Terminal,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ErrorPresentation {
    pub(crate) category: ErrorCategory,
    pub(crate) code: &'static str,
    pub(crate) summary: String,
    pub(crate) next_step: &'static str,
    pub(crate) retryable: bool,
    pub(crate) detail_ref: Option<String>,
}

impl ErrorPresentation {
    pub(crate) fn for_category(category: ErrorCategory, context: impl Into<String>) -> Self {
        let context = context.into();
        let (code, summary, next_step, retryable) = match category {
            ErrorCategory::Provider => (
                "YX-PROVIDER-001",
                format!("provider request failed: {context}"),
                "Check provider settings or retry the request.",
                true,
            ),
            ErrorCategory::Tool => (
                "YX-TOOL-001",
                format!("tool execution failed: {context}"),
                "Review tool details and retry when safe.",
                true,
            ),
            ErrorCategory::Approval => (
                "YX-APPROVAL-001",
                format!("approval was not granted: {context}"),
                "Review the request and choose an explicit safe action.",
                false,
            ),
            ErrorCategory::Cancel => (
                "YX-CANCEL-001",
                format!("operation cancelled: {context}"),
                "Enter a new request when ready.",
                true,
            ),
            ErrorCategory::Terminal => (
                "YX-TERMINAL-001",
                format!("terminal interaction failed: {context}"),
                "Check terminal dimensions or restart the session.",
                true,
            ),
            ErrorCategory::Unknown => (
                "YX-UNKNOWN-001",
                format!("operation failed: {context}"),
                "Open details for diagnostics, then retry if appropriate.",
                true,
            ),
        };
        Self {
            category,
            code,
            summary,
            next_step,
            retryable,
            detail_ref: None,
        }
    }

    pub(crate) fn with_detail_ref(mut self, detail_ref: impl Into<String>) -> Self {
        self.detail_ref = Some(detail_ref.into());
        self
    }

    pub(crate) fn display_summary(&self) -> String {
        format!(
            "{} {}; retryable={}; next: {}",
            self.code,
            self.summary,
            if self.retryable { "yes" } else { "no" },
            self.next_step
        )
    }
}

pub(crate) fn classify_error(message: &str) -> ErrorCategory {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("provider") || lowered.contains("http") || lowered.contains("model") {
        ErrorCategory::Provider
    } else if lowered.contains("approval") || lowered.contains("policy") {
        ErrorCategory::Approval
    } else if lowered.contains("cancel") {
        ErrorCategory::Cancel
    } else if lowered.contains("terminal") || lowered.contains("tui") || lowered.contains("pty") {
        ErrorCategory::Terminal
    } else if lowered.contains("tool")
        || lowered.contains("command")
        || lowered.contains("exit code")
    {
        ErrorCategory::Tool
    } else {
        ErrorCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_category_has_stable_code_and_next_step() {
        for category in [
            ErrorCategory::Provider,
            ErrorCategory::Tool,
            ErrorCategory::Approval,
            ErrorCategory::Cancel,
            ErrorCategory::Terminal,
            ErrorCategory::Unknown,
        ] {
            let presentation = ErrorPresentation::for_category(category, "test context");
            assert!(presentation.code.starts_with("YX-"));
            assert!(presentation.display_summary().contains("next:"));
            assert!(presentation.display_summary().contains("retryable="));
        }
    }

    #[test]
    fn detail_reference_is_explicit_and_not_part_of_the_safe_summary() {
        let presentation = ErrorPresentation::for_category(ErrorCategory::Unknown, "failure")
            .with_detail_ref("detail:unknown:1");

        assert_eq!(presentation.detail_ref.as_deref(), Some("detail:unknown:1"));
        assert!(!presentation.display_summary().contains("detail:unknown:1"));
    }

    #[test]
    fn message_classification_is_stable_and_conservative() {
        assert_eq!(
            classify_error("provider timed out"),
            ErrorCategory::Provider
        );
        assert_eq!(classify_error("tool exit code 7"), ErrorCategory::Tool);
        assert_eq!(
            classify_error("approval declined by policy"),
            ErrorCategory::Approval
        );
        assert_eq!(
            classify_error("current turn cancelled"),
            ErrorCategory::Cancel
        );
        assert_eq!(
            classify_error("terminal resize failed"),
            ErrorCategory::Terminal
        );
        assert_eq!(
            classify_error("unexpected condition"),
            ErrorCategory::Unknown
        );
    }
}
