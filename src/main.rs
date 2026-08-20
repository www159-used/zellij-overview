use std::collections::BTreeMap;

use overview::{Action, Key, Overview, TabFact};
use zellij_tile::prelude::*;

const PLUGIN_NAME: &str = "overview";

#[derive(Default)]
struct State {
    overview: Overview,
    own_plugin_id: Option<u32>,
    client_id: Option<ClientId>,
    permissions_granted: bool,
    pane_manifest: Option<PaneManifest>,
    /// Snap cursor to the active tab on the first TabUpdate after open.
    pending_initial_cursor: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        let ids = get_plugin_ids();
        self.own_plugin_id = Some(ids.plugin_id);
        self.client_id = Some(ids.client_id);
        self.pending_initial_cursor = true;
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::SessionUpdate,
            EventType::Key,
            EventType::PermissionRequestResult,
        ]);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.permissions_granted = true;
                    if let Some(id) = self.own_plugin_id {
                        rename_plugin_pane(id, PLUGIN_NAME);
                    }
                    self.close_if_duplicate();
                }
                true
            }
            Event::PaneUpdate(manifest) => {
                self.pane_manifest = Some(manifest);
                self.close_if_duplicate();
                false
            }
            Event::TabUpdate(tabs) => {
                self.overview
                    .apply_tabs(tabs.into_iter().map(tab_fact).collect());
                if self.pending_initial_cursor {
                    self.overview.reset_cursor_to_active();
                    self.pending_initial_cursor = false;
                }
                true
            }
            Event::SessionUpdate(sessions, _) => {
                let previous_tab_id = self.client_id.and_then(|client_id| {
                    sessions
                        .iter()
                        .find(|session| session.is_current_session)
                        .and_then(|session| session.tab_history.get(&client_id))
                        .and_then(|history| history.last())
                        .copied()
                });
                self.overview.set_previous_tab_id(previous_tab_id);
                true
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let frame = self.overview.paint(rows, cols);
        for line in frame.lines {
            println!("{line}");
        }
    }
}

impl State {
    fn close_if_duplicate(&self) {
        if !self.permissions_granted {
            return;
        }
        let Some(manifest) = self.pane_manifest.as_ref() else {
            return;
        };
        let Some(own_id) = self.own_plugin_id else {
            return;
        };
        let panes = manifest.panes.values().flatten().collect::<Vec<_>>();
        let Some(own_url) = panes
            .iter()
            .find(|pane| pane.is_plugin && pane.id == own_id)
            .and_then(|pane| pane.plugin_url.as_deref())
        else {
            return;
        };
        let overview_panes: Vec<PaneId> = manifest
            .panes
            .values()
            .flatten()
            .filter(|pane| pane.plugin_url.as_deref() == Some(own_url))
            .map(pane_id)
            .collect();
        if overview_panes.len() > 1 {
            let _ = hide_floating_panes(None);
            for pane_id in overview_panes {
                close_pane_with_id(pane_id);
            }
        }
    }

    fn dismiss(&self) {
        let _ = hide_floating_panes(None);
        close_self();
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let Some(mapped) = map_key(&key, self.overview.is_hinting()) else {
            return false;
        };
        let action = self.overview.decide(mapped);
        match action {
            Action::None => true,
            Action::Dismiss => {
                self.dismiss();
                false
            }
            Action::Commit { tab_index } => {
                self.dismiss();
                go_to_tab(tab_index);
                false
            }
            Action::PreviousTab => {
                self.dismiss();
                toggle_tab();
                false
            }
        }
    }
}

fn pane_id(pane: &PaneInfo) -> PaneId {
    if pane.is_plugin {
        PaneId::Plugin(pane.id)
    } else {
        PaneId::Terminal(pane.id)
    }
}

fn tab_fact(tab: TabInfo) -> TabFact {
    TabFact {
        id: tab.tab_id,
        position: tab.position,
        name: tab.name,
        active: tab.active,
    }
}

fn map_key(key: &KeyWithModifier, hinting: bool) -> Option<Key> {
    if key.has_no_modifiers() {
        return match key.bare_key {
            BareKey::Left => Some(Key::Left),
            BareKey::Down => Some(Key::Down),
            BareKey::Up => Some(Key::Up),
            BareKey::Right => Some(Key::Right),
            BareKey::Enter => Some(Key::Confirm),
            BareKey::Esc => Some(Key::Dismiss),
            BareKey::Backspace if hinting => Some(Key::Backspace),
            BareKey::Char('s') if !hinting => Some(Key::StartHint),
            BareKey::Char('q') => Some(Key::Dismiss),
            BareKey::Char('h') => Some(Key::Left),
            BareKey::Char('j') => Some(Key::Down),
            BareKey::Char('k') => Some(Key::Up),
            BareKey::Char('l') => Some(Key::Right),
            BareKey::Char('e') => Some(Key::Confirm),
            BareKey::Char('-') => Some(Key::PreviousTab),
            BareKey::Char(c) if hinting => Some(Key::Input(c)),
            _ => None,
        };
    }
    None
}
