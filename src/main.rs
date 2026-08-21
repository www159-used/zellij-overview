use std::collections::BTreeMap;

use overview::{Action, Key, Overview, PaneFact, SessionFact, TabFact};
use zellij_tile::prelude::*;

mod floating_state;

use floating_state::FloatingLayerState;

const PLUGIN_NAME: &str = "overview";

#[derive(Default)]
struct State {
    overview: Overview,
    own_plugin_id: Option<u32>,
    client_id: Option<ClientId>,
    permissions_granted: bool,
    pane_manifest: Option<PaneManifest>,
    floating_layer: FloatingLayerState,
    active_tab_position: Option<usize>,
    previous_focused_pane: Option<PaneId>,
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
                self.refresh_panes();
                true
            }
            Event::TabUpdate(tabs) => {
                self.capture_floating_layer(&tabs);
                self.active_tab_position =
                    tabs.iter().find(|tab| tab.active).map(|tab| tab.position);
                self.overview
                    .apply_tabs(tabs.into_iter().map(tab_fact).collect());
                if self.pending_initial_cursor {
                    self.overview.reset_cursor_to_active();
                    self.pending_initial_cursor = false;
                }
                self.refresh_panes();
                true
            }
            Event::SessionUpdate(sessions, _) => {
                if let Some(session) = sessions.iter().find(|session| session.is_current_session)
                {
                    let previous_tab_id = self.client_id.and_then(|client_id| {
                        session
                            .tab_history
                            .get(&client_id)
                            .and_then(|history| history.last())
                            .copied()
                    });
                    self.previous_focused_pane = self.nth_previous_pane(session, 0);
                    self.overview.set_previous_tab_id(previous_tab_id);
                    self.overview
                        .set_previous_pane(self.nth_previous_pane(session, 1).map(pane_id_parts));
                    self.capture_floating_layer(&session.tabs);
                }
                self.overview
                    .apply_sessions(sessions.into_iter().map(session_fact).collect());
                self.refresh_panes();
                true
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.overview.set_viewport(rows, cols);
        let frame = self.overview.paint(rows, cols);
        // The plugin pane is exactly `rows` high. A newline after the last
        // line scrolls the first card into scrollback (Tab #1 vanishes).
        write_plugin_lines(&frame.lines);
    }
}

impl State {
    fn close_if_duplicate(&self) {
        if !self.permissions_granted || !self.own_pane_is_listed() {
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
        let is_oldest_instance = overview_panes
            .iter()
            .filter_map(|pane_id| match pane_id {
                PaneId::Plugin(id) => Some(*id),
                PaneId::Terminal(_) => None,
            })
            .min()
            == Some(own_id);
        if overview_panes.len() > 1 && is_oldest_instance {
            self.restore_floating_layer();
            for pane_id in overview_panes {
                close_pane_with_id(pane_id);
            }
        }
    }

    fn dismiss(&self) {
        self.restore_floating_layer();
        close_self();
    }

    fn restore_floating_layer(&self) {
        if self.floating_layer.should_hide_on_close() {
            let _ = hide_floating_panes(None);
        }
    }

    fn capture_floating_layer(&mut self, tabs: &[TabInfo]) {
        if self.own_pane_is_listed() {
            return;
        }
        let visible = tabs
            .iter()
            .find(|tab| tab.active)
            .map(|tab| tab.are_floating_panes_visible);
        self.floating_layer.capture(visible);
    }

    fn own_pane_is_listed(&self) -> bool {
        let Some(own_id) = self.own_plugin_id else {
            return false;
        };
        self.pane_manifest.as_ref().is_some_and(|manifest| {
            manifest
                .panes
                .values()
                .flatten()
                .any(|pane| pane.is_plugin && pane.id == own_id)
        })
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let Some(mapped) = map_key(&key, self.overview.is_hinting()) else {
            return false;
        };
        let entering_sessions = !self.overview.is_sessions_layer();
        let action = self.overview.decide(mapped);
        if entering_sessions && self.overview.is_sessions_layer() {
            self.refresh_sessions();
        }
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
            Action::SwitchSession { name } => {
                self.dismiss();
                switch_session(Some(&name));
                false
            }
            Action::FocusPane {
                id,
                is_plugin,
                floating,
            } => {
                if !floating {
                    self.restore_floating_layer();
                }
                close_self();
                focus_pane_with_id(host_pane_id(id, is_plugin), floating, false);
                false
            }
        }
    }

    fn refresh_sessions(&mut self) {
        if !self.permissions_granted {
            return;
        }
        let Ok(snapshot) = get_session_list() else {
            return;
        };
        self.overview.apply_sessions(
            snapshot
                .live_sessions
                .into_iter()
                .map(session_fact)
                .collect(),
        );
    }

    fn refresh_panes(&mut self) {
        let Some(manifest) = self.pane_manifest.as_ref() else {
            return;
        };
        let tab_position = self.active_tab_position.unwrap_or(0);
        self.overview.apply_panes(pane_facts(
            manifest,
            tab_position,
            self.own_plugin_id,
            self.previous_focused_pane,
        ));
    }

    fn nth_previous_pane(&self, session: &SessionInfo, skip: usize) -> Option<PaneId> {
        let own_pane = PaneId::Plugin(self.own_plugin_id?);
        session
            .pane_history
            .get(&self.client_id?)?
            .iter()
            .rev()
            .filter(|pane_id| **pane_id != own_pane)
            .nth(skip)
            .copied()
    }
}

fn pane_id(pane: &PaneInfo) -> PaneId {
    host_pane_id(pane.id, pane.is_plugin)
}

fn write_plugin_lines(lines: &[String]) {
    let Some((last, rest)) = lines.split_last() else {
        return;
    };
    for line in rest {
        println!("{line}");
    }
    print!("{last}");
}

fn host_pane_id(id: u32, is_plugin: bool) -> PaneId {
    if is_plugin {
        PaneId::Plugin(id)
    } else {
        PaneId::Terminal(id)
    }
}

fn pane_id_parts(pane_id: PaneId) -> (u32, bool) {
    match pane_id {
        PaneId::Plugin(id) => (id, true),
        PaneId::Terminal(id) => (id, false),
    }
}

fn pane_facts(
    manifest: &PaneManifest,
    tab_position: usize,
    own_plugin_id: Option<u32>,
    previous_focused: Option<PaneId>,
) -> Vec<PaneFact> {
    manifest
        .panes
        .get(&tab_position)
        .into_iter()
        .flatten()
        .filter(|pane| pane.is_selectable && !pane.is_suppressed)
        .filter(|pane| !(pane.is_plugin && Some(pane.id) == own_plugin_id))
        .map(|pane| {
            let id = pane_id(pane);
            PaneFact {
                id: pane.id,
                is_plugin: pane.is_plugin,
                title: pane.title.clone(),
                active: previous_focused == Some(id),
                floating: pane.is_floating,
            }
        })
        .collect()
}

fn tab_fact(tab: TabInfo) -> TabFact {
    TabFact {
        id: tab.tab_id,
        position: tab.position,
        name: tab.name,
        active: tab.active,
    }
}

fn session_fact(session: SessionInfo) -> SessionFact {
    SessionFact {
        name: session.name,
        current: session.is_current_session,
        tab_count: session.tabs.len(),
    }
}

fn map_key(key: &KeyWithModifier, hinting: bool) -> Option<Key> {
    if key.has_no_modifiers() {
        return match key.bare_key {
            BareKey::Left => Some(Key::Left),
            BareKey::Down => Some(Key::Down),
            BareKey::Up => Some(Key::Up),
            BareKey::Right => Some(Key::Right),
            BareKey::PageDown => Some(Key::PageDown),
            BareKey::PageUp => Some(Key::PageUp),
            BareKey::Enter if !hinting => Some(Key::Confirm),
            BareKey::Esc => Some(Key::Dismiss),
            BareKey::Backspace if hinting => Some(Key::Backspace),
            BareKey::Char(' ') if !hinting => Some(Key::SpacePrefix),
            BareKey::Char('s') if !hinting => Some(Key::StartHint),
            BareKey::Char('p') if !hinting => Some(Key::PanesLayer),
            BareKey::Char(c) if hinting => Some(Key::Input(c)),
            BareKey::Char('z') => Some(Key::ZPrefix),
            BareKey::Char('t') => Some(Key::AlignTop),
            BareKey::Char('b') => Some(Key::AlignBottom),
            BareKey::Char('g') => Some(Key::GoPrefix),
            BareKey::Char('G') => Some(Key::Last),
            BareKey::Char('?') => Some(Key::ToggleHelp),
            BareKey::Char('q') => Some(Key::Dismiss),
            BareKey::Char('h') => Some(Key::Left),
            BareKey::Char('j') => Some(Key::Down),
            BareKey::Char('k') => Some(Key::Up),
            BareKey::Char('l') => Some(Key::Right),
            BareKey::Char('e') => Some(Key::Confirm),
            BareKey::Char('-') => Some(Key::PreviousTab),
            _ => None,
        };
    }
    if !hinting && key.has_modifiers(&[KeyModifier::Ctrl]) {
        return match key.bare_key {
            BareKey::Char('d') => Some(Key::HalfPageDown),
            BareKey::Char('u') => Some(Key::HalfPageUp),
            BareKey::Char('f') => Some(Key::PageDown),
            BareKey::Char('b') => Some(Key::PageUp),
            _ => None,
        };
    }
    None
}
