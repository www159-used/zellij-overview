use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use overview::{Action, Key, Overview, SessionFact, TabFact};
use zellij_tile::prelude::*;

mod floating_state;

use floating_state::FloatingLayerState;

const PLUGIN_NAME: &str = "overview";
const PREVIOUS_JUMP_PATH: &str = "/cache/previous";

#[derive(Default)]
struct State {
    overview: Overview,
    own_plugin_id: Option<u32>,
    client_id: Option<ClientId>,
    permissions_granted: bool,
    pane_manifest: Option<PaneManifest>,
    floating_layer: FloatingLayerState,
    fetched_sessions: bool,
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
                    self.refresh_sessions();
                    self.restore_previous_jump();
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
                if let Some(session) = sessions.iter().find(|session| session.is_current_session) {
                    let previous_tab_id = self.client_id.and_then(|client_id| {
                        session
                            .tab_history
                            .get(&client_id)
                            .and_then(|history| history.last())
                            .copied()
                    });
                    self.overview.set_previous_tab_id(previous_tab_id);
                    self.floating_layer
                        .capture(self.previous_pane_was_floating(session));
                }
                if sessions.len() > 1 {
                    self.overview
                        .apply_sessions(sessions.into_iter().map(session_fact).collect());
                } else if let Some(session) = sessions.into_iter().find(|s| s.is_current_session) {
                    self.overview.touch_current_session(session_fact(session));
                }
                self.restore_previous_jump();
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

    fn previous_pane_was_floating(&self, session: &SessionInfo) -> Option<bool> {
        let previous_pane = self.nth_previous_pane(session)?;
        session
            .panes
            .panes
            .values()
            .flatten()
            .find(|pane| pane_id(pane) == previous_pane)
            .map(|pane| pane.is_floating)
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
            Action::SwitchSession { name } => {
                let landing_tab = read_previous_jump()
                    .filter(|jump| jump.session == name)
                    .and_then(|jump| jump.tab_position);
                self.remember_current_location();
                self.dismiss();
                if let Some(tab_position) = landing_tab {
                    switch_session_with_focus(&name, Some(tab_position), None);
                } else {
                    switch_session(Some(&name));
                }
                false
            }
        }
    }

    fn refresh_sessions(&mut self) {
        if !self.permissions_granted || self.fetched_sessions {
            return;
        }
        let Ok(snapshot) = get_session_list() else {
            return;
        };
        self.fetched_sessions = true;
        self.overview.apply_sessions(
            snapshot
                .live_sessions
                .into_iter()
                .map(session_fact)
                .collect(),
        );
        self.restore_previous_jump();
    }

    fn restore_previous_jump(&mut self) {
        self.overview
            .set_previous_session_name(read_previous_jump().map(|jump| jump.session));
    }

    fn remember_current_location(&self) {
        let Some(session) = self.overview.current_session_name() else {
            return;
        };
        write_previous_jump(&PreviousJump {
            session: session.to_owned(),
            tab_position: self.overview.active_tab_position(),
        });
    }

    fn nth_previous_pane(&self, session: &SessionInfo) -> Option<PaneId> {
        let own_pane = PaneId::Plugin(self.own_plugin_id?);
        session
            .pane_history
            .get(&self.client_id?)?
            .iter()
            .rev()
            .find(|pane_id| **pane_id != own_pane)
            .copied()
    }
}

fn pane_id(pane: &PaneInfo) -> PaneId {
    if pane.is_plugin {
        PaneId::Plugin(pane.id)
    } else {
        PaneId::Terminal(pane.id)
    }
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

fn tab_fact(tab: TabInfo) -> TabFact {
    TabFact {
        id: tab.tab_id,
        position: tab.position,
        name: tab.name,
        active: tab.active,
    }
}

struct PreviousJump {
    session: String,
    tab_position: Option<usize>,
}

fn read_previous_jump() -> Option<PreviousJump> {
    let raw = fs::read_to_string(PREVIOUS_JUMP_PATH).ok()?;
    parse_previous_jump(&raw)
}

fn write_previous_jump(jump: &PreviousJump) {
    let mut raw = jump.session.clone();
    if let Some(position) = jump.tab_position {
        raw.push('\n');
        raw.push_str(&position.to_string());
    }
    let _ = fs::create_dir_all(
        Path::new(PREVIOUS_JUMP_PATH)
            .parent()
            .unwrap_or(Path::new("/cache")),
    );
    let _ = fs::write(PREVIOUS_JUMP_PATH, raw);
}

fn parse_previous_jump(raw: &str) -> Option<PreviousJump> {
    let mut lines = raw.lines();
    let session = lines.next()?.trim();
    if session.is_empty() {
        return None;
    }
    let tab_position = lines.next().and_then(|line| line.trim().parse().ok());
    Some(PreviousJump {
        session: session.to_owned(),
        tab_position,
    })
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
            BareKey::Char('s') if !hinting => Some(Key::StartHint),
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
