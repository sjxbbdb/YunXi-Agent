#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedTerminalMode {
    Plain,
    Tui,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvocationKind {
    Interactive,
    OneShot,
    Command,
    Json,
    Jsonl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalModeReason {
    InteractiveTerminal,
    NoTuiRequested,
    StructuredOutput,
    NonInteractiveInvocation,
    ContinuousIntegration,
    StdinNotTerminal,
    StdoutNotTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TerminalModeResolution {
    pub mode: ResolvedTerminalMode,
    pub reason: TerminalModeReason,
    pub fallback_notice: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TerminalModeRequest {
    pub tui: bool,
    pub no_tui: bool,
    pub stdin_is_terminal: bool,
    pub stdout_is_terminal: bool,
    pub ci: bool,
    pub invocation: InvocationKind,
}

impl TerminalModeRequest {
    pub(crate) fn resolve(self) -> TerminalModeResolution {
        let plain = |reason, fallback_notice| TerminalModeResolution {
            mode: ResolvedTerminalMode::Plain,
            reason,
            fallback_notice,
        };

        if matches!(self.invocation, InvocationKind::Json | InvocationKind::Jsonl) {
            return plain(TerminalModeReason::StructuredOutput, None);
        }
        if matches!(self.invocation, InvocationKind::OneShot | InvocationKind::Command) {
            return plain(TerminalModeReason::NonInteractiveInvocation, None);
        }
        if self.no_tui {
            return plain(TerminalModeReason::NoTuiRequested, None);
        }

        let fallback_notice = self.tui.then_some(
            "[notice] --tui requested, but this terminal cannot host the TUI; using plain mode",
        );
        if self.ci {
            return plain(TerminalModeReason::ContinuousIntegration, fallback_notice);
        }
        if !self.stdin_is_terminal {
            return plain(TerminalModeReason::StdinNotTerminal, fallback_notice);
        }
        if !self.stdout_is_terminal {
            return plain(TerminalModeReason::StdoutNotTerminal, fallback_notice);
        }

        TerminalModeResolution {
            mode: ResolvedTerminalMode::Tui,
            reason: TerminalModeReason::InteractiveTerminal,
            fallback_notice: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(invocation: InvocationKind) -> TerminalModeRequest {
        TerminalModeRequest {
            tui: false,
            no_tui: false,
            stdin_is_terminal: true,
            stdout_is_terminal: true,
            ci: false,
            invocation,
        }
    }

    #[test]
    fn interactive_terminal_uses_tui() {
        assert_eq!(
            request(InvocationKind::Interactive).resolve(),
            TerminalModeResolution {
                mode: ResolvedTerminalMode::Tui,
                reason: TerminalModeReason::InteractiveTerminal,
                fallback_notice: None,
            }
        );
    }

    #[test]
    fn plain_mode_matrix_has_one_result_for_every_non_tui_path() {
        let cases = [
            (
                TerminalModeRequest {
                    stdin_is_terminal: false,
                    ..request(InvocationKind::Interactive)
                },
                TerminalModeReason::StdinNotTerminal,
            ),
            (
                TerminalModeRequest {
                    stdout_is_terminal: false,
                    ..request(InvocationKind::Interactive)
                },
                TerminalModeReason::StdoutNotTerminal,
            ),
            (
                TerminalModeRequest {
                    ci: true,
                    ..request(InvocationKind::Interactive)
                },
                TerminalModeReason::ContinuousIntegration,
            ),
            (
                TerminalModeRequest {
                    no_tui: true,
                    ..request(InvocationKind::Interactive)
                },
                TerminalModeReason::NoTuiRequested,
            ),
            (
                request(InvocationKind::OneShot),
                TerminalModeReason::NonInteractiveInvocation,
            ),
            (
                request(InvocationKind::Command),
                TerminalModeReason::NonInteractiveInvocation,
            ),
            (
                request(InvocationKind::Json),
                TerminalModeReason::StructuredOutput,
            ),
            (
                request(InvocationKind::Jsonl),
                TerminalModeReason::StructuredOutput,
            ),
        ];

        for (request, reason) in cases {
            let resolution = request.resolve();
            assert_eq!(resolution.mode, ResolvedTerminalMode::Plain);
            assert_eq!(resolution.reason, reason);
        }
    }

    #[test]
    fn forced_tui_fallback_is_visible_only_for_interactive_plain_mode() {
        let forced = TerminalModeRequest {
            tui: true,
            stdout_is_terminal: false,
            ..request(InvocationKind::Interactive)
        }
        .resolve();
        assert_eq!(forced.mode, ResolvedTerminalMode::Plain);
        assert!(forced.fallback_notice.is_some());

        for invocation in [InvocationKind::Json, InvocationKind::Jsonl] {
            let structured = TerminalModeRequest {
                tui: true,
                stdout_is_terminal: false,
                ..request(invocation)
            }
            .resolve();
            assert_eq!(structured.mode, ResolvedTerminalMode::Plain);
            assert_eq!(structured.fallback_notice, None);
        }
    }
}
