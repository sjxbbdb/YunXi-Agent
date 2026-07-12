#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedTerminalMode {
    Plain,
    Tui,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TerminalModeRequest {
    pub tui: bool,
    pub no_tui: bool,
    pub stdin_is_terminal: bool,
    pub stdout_is_terminal: bool,
}

impl TerminalModeRequest {
    pub(crate) fn resolve(self) -> ResolvedTerminalMode {
        if self.no_tui {
            return ResolvedTerminalMode::Plain;
        }
        if self.stdin_is_terminal && self.stdout_is_terminal {
            return ResolvedTerminalMode::Tui;
        }
        ResolvedTerminalMode::Plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_terminal_uses_tui() {
        assert_eq!(
            TerminalModeRequest {
                tui: false,
                no_tui: false,
                stdin_is_terminal: true,
                stdout_is_terminal: true,
            }
            .resolve(),
            ResolvedTerminalMode::Tui
        );
    }

    #[test]
    fn piped_stdin_uses_plain() {
        assert_eq!(
            TerminalModeRequest {
                tui: false,
                no_tui: false,
                stdin_is_terminal: false,
                stdout_is_terminal: true,
            }
            .resolve(),
            ResolvedTerminalMode::Plain
        );
    }

    #[test]
    fn no_tui_forces_plain() {
        assert_eq!(
            TerminalModeRequest {
                tui: false,
                no_tui: true,
                stdin_is_terminal: true,
                stdout_is_terminal: true,
            }
            .resolve(),
            ResolvedTerminalMode::Plain
        );
    }

    #[test]
    fn forced_tui_without_terminal_falls_back_to_plain() {
        assert_eq!(
            TerminalModeRequest {
                tui: true,
                no_tui: false,
                stdin_is_terminal: true,
                stdout_is_terminal: false,
            }
            .resolve(),
            ResolvedTerminalMode::Plain
        );
    }
}
