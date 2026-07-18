use crate::presentation::{PresentationVisibility, TuiEvent};

/// Pure visibility gate. Runtime-event semantics belong to `presentation.rs`;
/// this module only applies the user's debug visibility preference.
pub(crate) fn should_show(event: &TuiEvent, debug_enabled: bool) -> bool {
    match event.visibility {
        PresentationVisibility::Transcript => true,
        PresentationVisibility::DebugOnly => debug_enabled,
        PresentationVisibility::Hidden => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::{TuiCellId, TuiCellKind};

    fn event(visibility: PresentationVisibility) -> TuiEvent {
        TuiEvent {
            id: TuiCellId::from_test("filter"),
            kind: TuiCellKind::Notice,
            visible_text: String::new(),
            detail: None,
            stream: None,
            visibility,
            tool_update: None,
        }
    }

    #[test]
    fn visibility_gate_only_depends_on_presentation_visibility_and_debug_flag() {
        assert!(should_show(
            &event(PresentationVisibility::Transcript),
            false
        ));
        assert!(!should_show(
            &event(PresentationVisibility::DebugOnly),
            false
        ));
        assert!(should_show(&event(PresentationVisibility::DebugOnly), true));
        assert!(!should_show(&event(PresentationVisibility::Hidden), true));
    }
}
