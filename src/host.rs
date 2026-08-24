use crate::{Action, Key, Overview, Pin, SessionFact, TabFact};

/// Plugin-side ingest + pin persist, without Zellij or the filesystem.
///
/// Open order matches `fetch_sessions` in `main.rs`: session snapshot, then
/// load pins from disk, then prune writes the pin file.
#[derive(Debug)]
pub struct Host {
    pub overview: Overview,
    pins: Vec<Pin>,
    last_action: Action,
    pending_initial_cursor: bool,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Self {
            overview: Overview::new(),
            pins: Vec::new(),
            last_action: Action::None,
            pending_initial_cursor: true,
        }
    }

    pub fn persisted_pins(&self) -> &[Pin] {
        &self.pins
    }

    pub fn last_action(&self) -> &Action {
        &self.last_action
    }

    pub fn focused_title(&self) -> Option<&str> {
        self.overview.item_title(self.overview.cursor())
    }

    /// Like opening the plugin: a new instance, then a session snapshot, then `/cache/pins`.
    pub fn load_from_snapshot(&mut self, sessions: Vec<SessionFact>) {
        self.overview = Overview::new();
        self.pending_initial_cursor = true;
        self.overview.apply_sessions(sessions);
        self.overview.apply_pins(self.pins.clone());
        self.overview.prune_stale_cache();
        self.flush_pins();
    }

    /// Select a card. Same as moving the cursor onto it.
    pub fn focus(&mut self, spec: &str) -> Result<(), String> {
        let (session, title) = split_spec(spec);
        let Some(index) = self.overview.find_item(title, session, false) else {
            return Err(format!("no card {spec}"));
        };
        self.overview.set_cursor(index);
        Ok(())
    }

    /// Press `e` on the focused card.
    pub fn jump(&mut self) -> Action {
        self.key(Key::Confirm)
    }

    /// Select a tab and press `p`. Same path as the CLI / plugin.
    pub fn pin(&mut self, spec: &str) -> Result<Action, String> {
        let (session, title) = split_spec(spec);
        let Some(index) = self.overview.find_tab(title, session) else {
            return Err(format!("no tab {spec}"));
        };
        self.overview.set_cursor(index);
        let action = self.key(Key::Pin);
        if !matches!(action, Action::PersistPins) {
            return Err(format!("pin {spec} did nothing"));
        }
        Ok(action)
    }

    pub fn apply_tabs(&mut self, tabs: Vec<TabFact>) {
        self.overview.apply_tabs(tabs);
        if self.pending_initial_cursor {
            self.overview.reset_cursor_to_active();
            self.pending_initial_cursor = false;
        }
        self.flush_pins();
    }

    pub fn apply_sessions(&mut self, sessions: Vec<SessionFact>) {
        self.overview.apply_sessions(sessions);
        self.flush_pins();
    }

    pub fn key(&mut self, key: Key) -> Action {
        let action = self.overview.decide(key);
        self.last_action = action.clone();
        if matches!(action, Action::PersistPins) {
            self.pins = self.overview.pins().to_vec();
        }
        self.flush_pins();
        action
    }

    fn flush_pins(&mut self) {
        if self.overview.take_stale_cache().pins {
            self.pins = self.overview.pins().to_vec();
        }
    }
}

fn split_spec(spec: &str) -> (Option<&str>, &str) {
    spec.split_once('/')
        .map(|(session, title)| (Some(session), title))
        .unwrap_or((None, spec))
}
