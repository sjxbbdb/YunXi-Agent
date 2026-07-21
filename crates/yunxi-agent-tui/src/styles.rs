use ratatui::style::{Color, Modifier, Style, Stylize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TuiColorCapability {
    Full,
    Ansi16,
    Monochrome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TuiSemanticStyle {
    User,
    Assistant,
    Progress,
    Tool,
    ActionRequired,
    Notice,
    Warning,
    Error,
    Muted,
    Header,
    Subheader,
    Footer,
    Border,
    Focus,
    Success,
    Selection,
}

impl TuiSemanticStyle {
    #[cfg(test)]
    const ALL: [Self; 16] = [
        Self::User,
        Self::Assistant,
        Self::Progress,
        Self::Tool,
        Self::ActionRequired,
        Self::Notice,
        Self::Warning,
        Self::Error,
        Self::Muted,
        Self::Header,
        Self::Subheader,
        Self::Footer,
        Self::Border,
        Self::Focus,
        Self::Success,
        Self::Selection,
    ];
}

pub(crate) type TuiStyle = Style;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiStyleSet {
    capability: TuiColorCapability,
}

impl TuiStyleSet {
    pub(crate) const fn new(capability: TuiColorCapability) -> Self {
        Self { capability }
    }

    pub(crate) fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some()
            || std::env::var("TERM").is_ok_and(|term| term.eq_ignore_ascii_case("dumb"))
        {
            return Self::new(TuiColorCapability::Monochrome);
        }

        let rich_terminal = std::env::var("COLORTERM").is_ok_and(|value| !value.is_empty())
            || std::env::var("TERM").is_ok_and(|term| {
                let term = term.to_ascii_lowercase();
                term.contains("256color") || term.contains("truecolor")
            });
        Self::new(if rich_terminal {
            TuiColorCapability::Full
        } else {
            TuiColorCapability::Ansi16
        })
    }

    pub(crate) fn style(self, semantic: TuiSemanticStyle) -> TuiStyle {
        match self.capability {
            TuiColorCapability::Full => rich_style(semantic),
            TuiColorCapability::Ansi16 => ansi16_style(semantic),
            TuiColorCapability::Monochrome => monochrome_style(semantic),
        }
    }
}

fn rich_style(semantic: TuiSemanticStyle) -> TuiStyle {
    match semantic {
        TuiSemanticStyle::User => Style::default().fg(Color::Cyan).bold(),
        TuiSemanticStyle::Assistant => Style::default().fg(Color::LightGreen),
        TuiSemanticStyle::Progress => Style::default().fg(Color::LightBlue),
        TuiSemanticStyle::Tool => Style::default().fg(Color::Magenta).bold(),
        TuiSemanticStyle::ActionRequired => Style::default().fg(Color::Yellow).bold(),
        TuiSemanticStyle::Notice => Style::default().fg(Color::LightBlue),
        TuiSemanticStyle::Warning => Style::default().fg(Color::Yellow).bold(),
        TuiSemanticStyle::Error => Style::default().fg(Color::LightRed).bold(),
        TuiSemanticStyle::Muted => Style::default().fg(Color::DarkGray),
        TuiSemanticStyle::Header => Style::default().fg(Color::LightCyan).bold(),
        TuiSemanticStyle::Subheader => Style::default().fg(Color::Gray),
        TuiSemanticStyle::Footer => Style::default().fg(Color::DarkGray),
        TuiSemanticStyle::Border => Style::default().fg(Color::DarkGray),
        TuiSemanticStyle::Focus => Style::default().fg(Color::Cyan).bold(),
        TuiSemanticStyle::Success => Style::default().fg(Color::Green).bold(),
        TuiSemanticStyle::Selection => Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
    }
}

fn ansi16_style(semantic: TuiSemanticStyle) -> TuiStyle {
    match semantic {
        TuiSemanticStyle::User => Style::default().fg(Color::Cyan).bold(),
        TuiSemanticStyle::Assistant => Style::default().fg(Color::Green),
        TuiSemanticStyle::Progress => Style::default().fg(Color::Blue),
        TuiSemanticStyle::Tool => Style::default().fg(Color::Magenta).bold(),
        TuiSemanticStyle::ActionRequired | TuiSemanticStyle::Warning => {
            Style::default().fg(Color::Yellow).bold()
        }
        TuiSemanticStyle::Notice => Style::default().fg(Color::Blue),
        TuiSemanticStyle::Error => Style::default().fg(Color::Red).bold(),
        TuiSemanticStyle::Muted | TuiSemanticStyle::Footer | TuiSemanticStyle::Border => {
            Style::default().fg(Color::DarkGray)
        }
        TuiSemanticStyle::Header | TuiSemanticStyle::Focus => {
            Style::default().fg(Color::Cyan).bold()
        }
        TuiSemanticStyle::Subheader => Style::default().fg(Color::Gray),
        TuiSemanticStyle::Success => Style::default().fg(Color::Green).bold(),
        TuiSemanticStyle::Selection => Style::default().reversed().bold(),
    }
}

fn monochrome_style(semantic: TuiSemanticStyle) -> TuiStyle {
    match semantic {
        TuiSemanticStyle::Muted
        | TuiSemanticStyle::Subheader
        | TuiSemanticStyle::Footer
        | TuiSemanticStyle::Border => Style::default().add_modifier(Modifier::DIM),
        TuiSemanticStyle::Selection => Style::default().reversed().bold(),
        TuiSemanticStyle::Header
        | TuiSemanticStyle::Focus
        | TuiSemanticStyle::ActionRequired
        | TuiSemanticStyle::Warning
        | TuiSemanticStyle::Error
        | TuiSemanticStyle::Tool
        | TuiSemanticStyle::User
        | TuiSemanticStyle::Success => Style::default().bold(),
        TuiSemanticStyle::Assistant | TuiSemanticStyle::Progress | TuiSemanticStyle::Notice => {
            Style::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_semantic_style_is_stable_in_each_color_capability() {
        for capability in [
            TuiColorCapability::Full,
            TuiColorCapability::Ansi16,
            TuiColorCapability::Monochrome,
        ] {
            let styles = TuiStyleSet::new(capability);
            for semantic in TuiSemanticStyle::ALL {
                assert_eq!(styles.style(semantic), styles.style(semantic));
            }
        }
    }

    #[test]
    fn monochrome_keeps_important_states_distinct_without_color() {
        let styles = TuiStyleSet::new(TuiColorCapability::Monochrome);
        for semantic in [
            TuiSemanticStyle::ActionRequired,
            TuiSemanticStyle::Warning,
            TuiSemanticStyle::Error,
            TuiSemanticStyle::Focus,
        ] {
            let style = styles.style(semantic);
            assert_eq!(style.fg, None);
            assert_eq!(style.bg, None);
            assert!(style.add_modifier.contains(Modifier::BOLD));
        }
        assert!(
            styles
                .style(TuiSemanticStyle::Selection)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
    }

    #[test]
    fn rich_and_ansi_palettes_use_only_terminal_colors() {
        let rich = TuiStyleSet::new(TuiColorCapability::Full);
        let ansi = TuiStyleSet::new(TuiColorCapability::Ansi16);

        assert_eq!(
            rich.style(TuiSemanticStyle::Error).fg,
            Some(Color::LightRed)
        );
        assert_eq!(ansi.style(TuiSemanticStyle::Error).fg, Some(Color::Red));
        assert_eq!(
            rich.style(TuiSemanticStyle::ActionRequired).fg,
            Some(Color::Yellow)
        );
    }
}
