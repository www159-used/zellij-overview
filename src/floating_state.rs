#[derive(Debug, Default)]
pub(crate) struct FloatingLayerState {
    captured: bool,
    previous_pane_was_floating: bool,
}

impl FloatingLayerState {
    pub(crate) fn capture(&mut self, previous_pane_was_floating: Option<bool>) {
        if self.captured {
            return;
        }
        self.captured = true;
        self.previous_pane_was_floating = previous_pane_was_floating.unwrap_or(false);
    }

    pub(crate) fn should_hide_on_close(&self) -> bool {
        !self.previous_pane_was_floating
    }
}

#[cfg(test)]
mod tests {
    use super::FloatingLayerState;

    #[test]
    fn keeps_a_previously_visible_floating_layer_visible() {
        let mut state = FloatingLayerState::default();
        state.capture(Some(true));
        assert!(!state.should_hide_on_close());
    }

    #[test]
    fn restores_a_previously_hidden_floating_layer() {
        let mut state = FloatingLayerState::default();
        state.capture(Some(false));
        assert!(state.should_hide_on_close());
    }

    #[test]
    fn captures_only_the_state_from_before_overview_opened() {
        let mut state = FloatingLayerState::default();
        state.capture(Some(false));
        state.capture(Some(true));
        assert!(state.should_hide_on_close());
    }

    #[test]
    fn defaults_to_hiding_when_history_is_unavailable() {
        let mut state = FloatingLayerState::default();
        state.capture(None);
        assert!(state.should_hide_on_close());
    }

    #[test]
    fn hides_when_no_snapshot_arrives_before_close() {
        let state = FloatingLayerState::default();
        assert!(state.should_hide_on_close());
    }
}
