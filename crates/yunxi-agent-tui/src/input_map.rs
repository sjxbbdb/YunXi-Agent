use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FocusTarget {
    Composer,
    History,
    Approval,
    Details,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TuiAction {
    None,
    Submit,
    Cancel,
    Approve,
    Decline,
    InsertNewline,
    CloseDetails,
    FocusNext,
    FocusPrevious,
    PageUp,
    PageDown,
    Paste,
    ScrollUp,
    ScrollDown,
}

pub(crate) fn resolve_event(focus: FocusTarget, event: &Event) -> TuiAction {
    match event {
        Event::Key(key) => resolve_key(focus, *key),
        Event::Paste(_) if focus == FocusTarget::Composer => TuiAction::Paste,
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollUp => TuiAction::ScrollUp,
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollDown => TuiAction::ScrollDown,
        _ => TuiAction::None,
    }
}

pub(crate) fn resolve_key(focus: FocusTarget, key: KeyEvent) -> TuiAction {
    if key.kind != KeyEventKind::Press {
        return TuiAction::None;
    }

    match focus {
        FocusTarget::Details => match key.code {
            KeyCode::Esc => TuiAction::CloseDetails,
            KeyCode::PageUp => TuiAction::PageUp,
            KeyCode::PageDown => TuiAction::PageDown,
            _ => TuiAction::None,
        },
        FocusTarget::History => match key.code {
            KeyCode::PageUp => TuiAction::PageUp,
            KeyCode::PageDown => TuiAction::PageDown,
            KeyCode::Tab => TuiAction::FocusNext,
            KeyCode::BackTab => TuiAction::FocusPrevious,
            _ => TuiAction::None,
        },
        FocusTarget::Composer | FocusTarget::Approval => {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return TuiAction::Cancel;
            }
            match key.code {
                KeyCode::Esc if matches!(focus, FocusTarget::Approval) => TuiAction::Decline,
                KeyCode::Esc => TuiAction::Cancel,
                KeyCode::Enter
                    if key
                        .modifiers
                        .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT) =>
                {
                    TuiAction::InsertNewline
                }
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    TuiAction::InsertNewline
                }
                KeyCode::Char('y')
                | KeyCode::Char('Y')
                | KeyCode::Char('a')
                | KeyCode::Char('A')
                    if matches!(focus, FocusTarget::Approval) =>
                {
                    TuiAction::Approve
                }
                KeyCode::Char('n')
                | KeyCode::Char('N')
                | KeyCode::Char('d')
                | KeyCode::Char('D')
                    if matches!(focus, FocusTarget::Approval) =>
                {
                    TuiAction::Decline
                }
                KeyCode::Tab if matches!(focus, FocusTarget::Composer) => TuiAction::FocusNext,
                KeyCode::BackTab if matches!(focus, FocusTarget::Composer) => {
                    TuiAction::FocusPrevious
                }
                KeyCode::Enter => TuiAction::Submit,
                _ => TuiAction::None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn text_focus_actions_are_consistent() {
        assert_eq!(
            resolve_key(
                FocusTarget::Composer,
                key(KeyCode::Enter, KeyModifiers::NONE)
            ),
            TuiAction::Submit
        );
        assert_eq!(
            resolve_key(
                FocusTarget::Composer,
                key(KeyCode::Enter, KeyModifiers::ALT)
            ),
            TuiAction::InsertNewline
        );
        assert_eq!(
            resolve_key(
                FocusTarget::Approval,
                key(KeyCode::Char('c'), KeyModifiers::CONTROL)
            ),
            TuiAction::Cancel
        );
    }

    #[test]
    fn navigation_actions_do_not_leak_into_composer() {
        assert_eq!(
            resolve_key(
                FocusTarget::Composer,
                key(KeyCode::PageUp, KeyModifiers::NONE)
            ),
            TuiAction::None
        );
        assert_eq!(
            resolve_key(
                FocusTarget::History,
                key(KeyCode::PageUp, KeyModifiers::NONE)
            ),
            TuiAction::PageUp
        );
        assert_eq!(
            resolve_key(FocusTarget::Details, key(KeyCode::Esc, KeyModifiers::NONE)),
            TuiAction::CloseDetails
        );
    }

    #[test]
    fn paste_and_wheel_are_classified_before_view_handlers() {
        assert_eq!(
            resolve_event(FocusTarget::Composer, &Event::Paste("draft".to_string())),
            TuiAction::Paste
        );
        assert_eq!(
            resolve_event(FocusTarget::Approval, &Event::Paste("ignored".to_string())),
            TuiAction::None
        );
        let wheel_up = Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            resolve_event(FocusTarget::History, &wheel_up),
            TuiAction::ScrollUp
        );
    }
}
