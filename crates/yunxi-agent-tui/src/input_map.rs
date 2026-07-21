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
    DetailScrollUp,
    DetailScrollDown,
}

pub(crate) fn resolve_event(focus: FocusTarget, event: &Event) -> TuiAction {
    match event {
        Event::Key(key) => resolve_key(focus, *key),
        Event::Paste(_) if focus == FocusTarget::Composer => TuiAction::Paste,
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollUp => match focus {
            FocusTarget::Composer | FocusTarget::History => TuiAction::ScrollUp,
            FocusTarget::Approval => TuiAction::None,
            FocusTarget::Details => TuiAction::DetailScrollUp,
        },
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollDown => match focus {
            FocusTarget::Composer | FocusTarget::History => TuiAction::ScrollDown,
            FocusTarget::Approval => TuiAction::None,
            FocusTarget::Details => TuiAction::DetailScrollDown,
        },
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
                KeyCode::PageUp if matches!(focus, FocusTarget::Composer) => TuiAction::PageUp,
                KeyCode::PageDown if matches!(focus, FocusTarget::Composer) => TuiAction::PageDown,
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
            TuiAction::PageUp
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
    fn focus_action_matrix_is_explicit() {
        let wheel_up = mouse(MouseEventKind::ScrollUp);
        let wheel_down = mouse(MouseEventKind::ScrollDown);
        let paste = Event::Paste("draft".to_string());
        let cases = [
            (
                FocusTarget::Composer,
                [
                    TuiAction::Cancel,
                    TuiAction::Submit,
                    TuiAction::PageUp,
                    TuiAction::PageDown,
                    TuiAction::FocusNext,
                    TuiAction::Cancel,
                    TuiAction::Paste,
                    TuiAction::ScrollUp,
                    TuiAction::ScrollDown,
                ],
            ),
            (
                FocusTarget::History,
                [
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::PageUp,
                    TuiAction::PageDown,
                    TuiAction::FocusNext,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::ScrollUp,
                    TuiAction::ScrollDown,
                ],
            ),
            (
                FocusTarget::Approval,
                [
                    TuiAction::Decline,
                    TuiAction::Submit,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::Cancel,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::None,
                ],
            ),
            (
                FocusTarget::Details,
                [
                    TuiAction::CloseDetails,
                    TuiAction::None,
                    TuiAction::PageUp,
                    TuiAction::PageDown,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::None,
                    TuiAction::DetailScrollUp,
                    TuiAction::DetailScrollDown,
                ],
            ),
        ];

        for (focus, expected) in cases {
            let actual = [
                resolve_key(focus, key(KeyCode::Esc, KeyModifiers::NONE)),
                resolve_key(focus, key(KeyCode::Enter, KeyModifiers::NONE)),
                resolve_key(focus, key(KeyCode::PageUp, KeyModifiers::NONE)),
                resolve_key(focus, key(KeyCode::PageDown, KeyModifiers::NONE)),
                resolve_key(focus, key(KeyCode::Tab, KeyModifiers::NONE)),
                resolve_key(focus, key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
                resolve_event(focus, &paste),
                resolve_event(focus, &wheel_up),
                resolve_event(focus, &wheel_down),
            ];
            assert_eq!(actual, expected, "focus={focus:?}");
        }
    }

    fn mouse(kind: MouseEventKind) -> Event {
        Event::Mouse(crossterm::event::MouseEvent {
            kind,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })
    }
}
