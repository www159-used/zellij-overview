#[derive(Debug, Default)]
pub(crate) struct FloatingLayerState {
    /// Pre-open visibility. `None` until a snapshot arrives before this pane is listed.
    layer_was_visible: Option<bool>,
}

impl FloatingLayerState {
    pub(crate) fn capture(&mut self, layer_was_visible: Option<bool>) {
        if self.layer_was_visible.is_some() {
            return;
        }
        self.layer_was_visible = layer_was_visible;
    }

    pub(crate) fn should_hide_on_close(&self) -> bool {
        self.layer_was_visible == Some(false)
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
    fn leaves_the_layer_alone_when_pre_open_visibility_is_unknown() {
        let mut state = FloatingLayerState::default();
        state.capture(None);
        assert!(!state.should_hide_on_close());
    }
}
