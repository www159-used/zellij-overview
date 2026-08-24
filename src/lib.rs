//! Tab overview core. No Zellij types — the WASM adapter maps host events in.

mod ansi;
mod float_size;
mod floating_state;
mod grid;
mod render;
mod theme;
mod usage;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

pub use float_size::{float_size_from_config, FloatSize};
pub use floating_state::FloatingLayerState;
use ratatui::{layout::Rect, text::Line};
pub use render::{paint, Frame};
pub use theme::{apply_theme_overlay, PACKED_THEME_CSS};
pub use usage::{append_usage_log, Usage, UsageEnd, USAGE_CAP};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabFact {
    pub id: usize,
    pub position: usize,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFact {
    pub name: String,
    pub current: bool,
    pub tab_count: usize,
    pub tabs: Vec<TabFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Down,
    Up,
    Right,
    Confirm,
    PreviousTab,
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    GoPrefix,
    Last,
    ZPrefix,
    AlignTop,
    AlignBottom,
    ToggleHelp,
    Dismiss,
    Toggle,
    StartHint,
    Pin,
    Input(char),
    Backspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Dismiss,
    /// Zero-based index expected by zellij-tile's `go_to_tab`.
    Commit {
        tab_index: u32,
    },
    PreviousTab,
    SwitchSession {
        name: String,
        tab_position: Option<usize>,
    },
    PersistPins,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub session: String,
    pub tab_name: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CachePrune {
    pub pins: bool,
    pub previous: bool,
    pub session_last: bool,
}

#[derive(Debug, Clone, Copy)]
enum Slot {
    Pin(usize),
    Session(usize),
    LiveTab(usize),
    SnapTab { session: usize, tab: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BoardIdentity {
    Session(String),
    Tab(usize),
    Pinned { session: String, name: String },
}

#[derive(Debug, Clone, Copy)]
enum ScrollAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Default)]
struct HintState {
    labels: Vec<Option<String>>,
    query: String,
    jump_prefix: String,
}

#[derive(Debug, Default)]
pub struct Overview {
    tabs: Vec<TabFact>,
    sessions: Vec<SessionFact>,
    /// `None` is the home board (sessions + current tabs).
    drilled_session: Option<String>,
    session_last_tabs: BTreeMap<String, usize>,
    pins: Vec<Pin>,
    /// Combined board: pins, then sessions, then unpinned current tabs.
    cursor: usize,
    previous_tab_id: Option<usize>,
    previous_session_name: Option<String>,
    viewport: Option<(usize, usize)>,
    scroll_offset: usize,
    pending_g: bool,
    pending_z: bool,
    show_help: bool,
    hint: Option<HintState>,
    stale_cache: CachePrune,
}

impl Overview {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_tabs(&mut self, mut tabs: Vec<TabFact>) {
        tabs.sort_by_key(|t| t.position);
        let selected = self.selected_identity();
        self.tabs = tabs;
        self.prune_stale_cache();
        self.reseat_cursor(selected);
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
    }

    pub fn apply_sessions(&mut self, mut sessions: Vec<SessionFact>) {
        sessions.sort_by(|left, right| {
            right
                .current
                .cmp(&left.current)
                .then_with(|| left.name.cmp(&right.name))
        });
        let selected = self.selected_identity();
        self.sessions = sessions;
        if self
            .drilled_session
            .as_ref()
            .is_some_and(|name| !self.sessions.iter().any(|session| session.name == *name))
        {
            self.drilled_session = None;
        }
        self.prune_stale_cache();
        self.reseat_cursor(selected);
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
    }

    pub fn touch_current_session(&mut self, session: SessionFact) {
        let selected = self.selected_identity();
        let name = session.name.clone();
        for candidate in &mut self.sessions {
            candidate.current = false;
        }
        if let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|candidate| candidate.name == name)
        {
            let tabs = if session.tabs.is_empty() {
                std::mem::take(&mut existing.tabs)
            } else {
                session.tabs.clone()
            };
            *existing = SessionFact {
                current: true,
                tabs,
                ..session
            };
        } else {
            self.sessions.push(SessionFact {
                current: true,
                ..session
            });
        }
        self.sessions.sort_by(|left, right| {
            right
                .current
                .cmp(&left.current)
                .then_with(|| left.name.cmp(&right.name))
        });
        self.prune_stale_cache();
        self.reseat_cursor(selected);
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
    }

    pub fn reset_cursor_to_active(&mut self) {
        self.cursor = self.active_index().unwrap_or(0);
    }

    pub fn tabs(&self) -> &[TabFact] {
        &self.tabs
    }

    pub fn sessions(&self) -> &[SessionFact] {
        &self.sessions
    }

    pub fn current_session_name(&self) -> Option<&str> {
        self.sessions
            .iter()
            .find(|session| session.current)
            .map(|session| session.name.as_str())
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_previous_tab_id(&mut self, tab_id: Option<usize>) {
        self.previous_tab_id = tab_id;
    }

    pub fn set_previous_session_name(&mut self, name: Option<String>) {
        self.previous_session_name = name.filter(|name| !name.is_empty());
    }

    pub fn set_session_last_tab(&mut self, session: String, position: usize) {
        self.session_last_tabs.insert(session, position);
    }

    pub fn apply_pins(&mut self, pins: Vec<Pin>) {
        let selected = self.selected_identity();
        self.pins = pins;
        self.prune_stale_cache();
        self.reseat_cursor(selected);
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
    }

    pub fn session_last_tabs(&self) -> &BTreeMap<String, usize> {
        &self.session_last_tabs
    }

    pub fn take_stale_cache(&mut self) -> CachePrune {
        std::mem::take(&mut self.stale_cache)
    }

    pub fn prune_stale_cache(&mut self) {
        let pin_count = self.pins.len();
        let pins = std::mem::take(&mut self.pins);
        self.pins = pins
            .into_iter()
            .filter(|pin| !self.pin_is_dead(pin))
            .collect();
        if self.pins.len() != pin_count {
            self.stale_cache.pins = true;
        }

        if let Some(name) = self.previous_session_name.clone() {
            let alive = self
                .sessions
                .iter()
                .any(|session| session.name == name && !session.current);
            if !self.sessions.is_empty() && !alive {
                self.previous_session_name = None;
                self.stale_cache.previous = true;
            }
        }

        let last_count = self.session_last_tabs.len();
        let last = std::mem::take(&mut self.session_last_tabs);
        self.session_last_tabs = last
            .into_iter()
            .filter(|(name, position)| !self.session_last_is_dead(name, *position))
            .collect();
        if self.session_last_tabs.len() != last_count {
            self.stale_cache.session_last = true;
        }
    }

    fn pin_is_dead(&self, pin: &Pin) -> bool {
        if self.current_session_name() == Some(pin.session.as_str()) {
            // Only TabUpdate is complete for the current session. A session
            // snapshot often arrives first with a partial tab list.
            if self.tabs.is_empty() {
                return false;
            }
            return self
                .tabs
                .iter()
                .all(|tab| display_name(tab) != pin.tab_name);
        }
        match self
            .sessions
            .iter()
            .find(|session| session.name == pin.session)
        {
            None => !self.sessions.is_empty(),
            Some(session) => {
                if session.tabs.is_empty() || session.tabs.len() < session.tab_count {
                    return false;
                }
                session
                    .tabs
                    .iter()
                    .all(|tab| display_name(tab) != pin.tab_name)
            }
        }
    }

    fn session_last_is_dead(&self, name: &str, position: usize) -> bool {
        if self.sessions.is_empty() {
            return false;
        }
        let Some(session) = self.sessions.iter().find(|session| session.name == name) else {
            return true;
        };
        if session.current && !self.tabs.is_empty() {
            return position >= self.tabs.len();
        }
        if !session.tabs.is_empty() {
            return position >= session.tabs.len();
        }
        session.tab_count > 0 && position >= session.tab_count
    }

    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }

    pub fn item_is_pinned(&self, index: usize) -> bool {
        matches!(self.slot(index), Some(Slot::Pin(_)))
    }

    pub fn pin_count(&self) -> usize {
        self.slots()
            .iter()
            .take_while(|slot| matches!(slot, Slot::Pin(_)))
            .count()
    }

    pub(crate) fn item_pin_session(&self, index: usize) -> Option<&str> {
        if !self.item_is_pinned(index) || self.is_previous_item(index) {
            return None;
        }
        self.item_session_name(index)
    }

    pub fn viewing_session(&self) -> Option<&str> {
        self.drilled_session.as_deref()
    }

    pub fn needs_session_tabs(&self) -> bool {
        let Some(name) = self.drilled_session.as_deref() else {
            return false;
        };
        if self.drilled_session_is_current() {
            return false;
        }
        self.sessions
            .iter()
            .find(|session| session.name == name)
            .is_some_and(|session| session.tabs.is_empty())
    }

    pub fn active_tab_position(&self) -> Option<usize> {
        self.tabs
            .iter()
            .find(|tab| tab.active)
            .map(|tab| tab.position)
    }

    pub fn set_viewport(&mut self, rows: usize, cols: usize) {
        self.viewport = Some((rows, cols));
        self.ensure_index_visible(self.cursor);
    }

    pub fn is_previous_item(&self, index: usize) -> bool {
        if let Some(name) = self.drilled_session.as_deref() {
            if self.drilled_session_is_current() {
                return self
                    .tab_at(index)
                    .is_some_and(|tab| Some(tab.id) == self.previous_tab_id);
            }
            return self.session_last_tabs.get(name).is_some_and(|position| {
                self.tab_at(index)
                    .is_some_and(|tab| tab.position == *position)
            });
        }
        if let Some(name) = self.other_previous_session() {
            if let Some(pin) = self.previous_session_pin_index(name) {
                return index == pin;
            }
            return self
                .session_at(index)
                .is_some_and(|session| session.name == name);
        }
        self.tab_at(index)
            .is_some_and(|tab| Some(tab.id) == self.previous_tab_id)
    }

    pub fn is_previous_tab(&self, index: usize) -> bool {
        self.is_previous_item(index)
    }

    pub(crate) fn item_is_session(&self, index: usize) -> bool {
        matches!(self.slot(index), Some(Slot::Session(_)))
    }

    pub fn is_hinting(&self) -> bool {
        self.hint.is_some()
    }

    pub fn is_help_visible(&self) -> bool {
        self.show_help
    }

    pub fn visible_tabs(&self) -> Vec<&TabFact> {
        self.tabs.iter().collect()
    }

    pub fn hint_query(&self) -> &str {
        self.hint
            .as_ref()
            .map(|hint| hint.query.as_str())
            .unwrap_or("")
    }

    pub fn hint_jump_prefix(&self) -> &str {
        self.hint
            .as_ref()
            .map(|hint| hint.jump_prefix.as_str())
            .unwrap_or("")
    }

    pub fn hint_label(&self, index: usize) -> Option<&str> {
        self.hint
            .as_ref()
            .and_then(|hint| hint.labels.get(index))
            .and_then(Option::as_deref)
    }

    pub fn hint_match_range(&self, index: usize) -> Option<(usize, usize)> {
        let hint = self.hint.as_ref()?;
        title_match_range(self.item_title(index)?, &hint.query)
    }

    pub fn item_title(&self, index: usize) -> Option<&str> {
        if let Some(session) = self.session_at(index) {
            return Some(session.name.as_str());
        }
        self.tab_at(index).map(display_name)
    }

    pub fn item_is_active(&self, index: usize) -> bool {
        if let Some(session) = self.session_at(index) {
            return session.current;
        }
        self.tab_at(index).is_some_and(|tab| tab.active)
    }

    pub fn item_count(&self) -> usize {
        self.slots().len()
    }

    /// Opening the float grows through a compact size. Hold that frame so the
    /// first paint the user sees is already framed.
    pub fn should_hold_opening_paint(&self, opening: bool) -> bool {
        opening && self.current_layout_plan().mode == grid::LayoutMode::Compact
    }

    pub fn decide(&mut self, key: Key) -> Action {
        if self.show_help {
            return match key {
                Key::ToggleHelp | Key::Dismiss => {
                    self.show_help = false;
                    Action::None
                }
                _ => Action::None,
            };
        }
        let plan = self.current_layout_plan();
        let completes_gg = key == Key::GoPrefix && self.pending_g;
        if key != Key::GoPrefix {
            self.pending_g = false;
        }
        let z_was_pending = self.pending_z;
        if !matches!(key, Key::ZPrefix | Key::AlignTop | Key::AlignBottom) {
            self.pending_z = false;
        }
        let action = match key {
            Key::Left => {
                self.cursor = plan.horizontal_neighbor(self.cursor, -1);
                Action::None
            }
            Key::Right => {
                self.cursor = plan.horizontal_neighbor(self.cursor, 1);
                Action::None
            }
            Key::Up => {
                self.cursor = plan.vertical_neighbor(self.cursor, -1);
                Action::None
            }
            Key::Down => {
                self.cursor = plan.vertical_neighbor(self.cursor, 1);
                Action::None
            }
            Key::HalfPageDown => {
                self.move_cursor_by((plan.visible_count / 2).max(1) as isize);
                Action::None
            }
            Key::HalfPageUp => {
                self.move_cursor_by(-((plan.visible_count / 2).max(1) as isize));
                Action::None
            }
            Key::PageDown => {
                self.move_cursor_by(plan.visible_count.max(1) as isize);
                Action::None
            }
            Key::PageUp => {
                self.move_cursor_by(-(plan.visible_count.max(1) as isize));
                Action::None
            }
            Key::GoPrefix if completes_gg => {
                self.pending_g = false;
                self.cursor = 0;
                Action::None
            }
            Key::GoPrefix => {
                self.pending_g = true;
                Action::None
            }
            Key::Last => {
                self.cursor = self.item_count().saturating_sub(1);
                Action::None
            }
            Key::ZPrefix if z_was_pending => {
                self.pending_z = false;
                self.align_cursor(ScrollAlignment::Center);
                Action::None
            }
            Key::ZPrefix => {
                self.pending_z = true;
                Action::None
            }
            Key::AlignTop if z_was_pending => {
                self.pending_z = false;
                self.align_cursor(ScrollAlignment::Top);
                Action::None
            }
            Key::AlignBottom if z_was_pending => {
                self.pending_z = false;
                self.align_cursor(ScrollAlignment::Bottom);
                Action::None
            }
            Key::AlignTop | Key::AlignBottom => Action::None,
            Key::ToggleHelp => {
                self.show_help = true;
                Action::None
            }
            Key::Confirm => self.commit_cursor(),
            Key::PreviousTab => self.commit_previous(),
            Key::Pin => {
                if self.toggle_pin_at_cursor() {
                    Action::PersistPins
                } else {
                    Action::None
                }
            }
            Key::Dismiss if self.is_hinting() => {
                self.hint = None;
                self.reset_cursor_to_active();
                Action::None
            }
            Key::Dismiss if self.drilled_session.is_some() => {
                self.leave_session_board();
                Action::None
            }
            Key::Dismiss | Key::Toggle => Action::Dismiss,
            Key::StartHint => {
                self.hint = Some(HintState {
                    labels: vec![None; self.item_count()],
                    query: String::new(),
                    jump_prefix: String::new(),
                });
                Action::None
            }
            Key::Input(ch) if self.is_hinting() => self.apply_hint_input(ch),
            Key::Backspace if self.is_hinting() => {
                if let Some(hint) = self.hint.as_mut() {
                    if hint.jump_prefix.is_empty() {
                        hint.query.pop();
                    } else {
                        hint.jump_prefix.pop();
                    }
                }
                self.recompute_hint_labels();
                let reveal_index = self
                    .hint
                    .as_ref()
                    .and_then(|hint| hint.labels.iter().position(Option::is_some))
                    .unwrap_or(self.cursor);
                self.ensure_index_visible(reveal_index);
                Action::None
            }
            Key::Input(_) | Key::Backspace => Action::None,
        };
        if matches!(
            key,
            Key::Left
                | Key::Right
                | Key::Up
                | Key::Down
                | Key::HalfPageDown
                | Key::HalfPageUp
                | Key::PageDown
                | Key::PageUp
                | Key::GoPrefix
                | Key::Last
                | Key::ZPrefix
                | Key::AlignTop
                | Key::AlignBottom
                | Key::Dismiss
        ) {
            self.ensure_index_visible(self.cursor);
        }
        action
    }

    pub fn paint(&self, rows: usize, cols: usize) -> Frame {
        render::paint(self, rows, cols)
    }

    fn commit_cursor(&mut self) -> Action {
        self.commit_index(self.cursor)
    }

    fn commit_previous(&mut self) -> Action {
        if let Some(name) = self.drilled_session.clone() {
            return self.jump_session_tab(&name, self.session_last_tabs.get(&name).copied());
        }
        if let Some(name) = self.other_previous_session().map(str::to_owned) {
            if let Some(position) = self.session_last_tabs.get(&name).copied() {
                if self.previous_session_pin_index(&name).is_some() {
                    return self.jump_session_tab(&name, Some(position));
                }
            }
            self.enter_session_board(&name);
            return Action::None;
        }
        Action::PreviousTab
    }

    fn previous_session_pin_index(&self, session: &str) -> Option<usize> {
        let position = *self.session_last_tabs.get(session)?;
        (0..self.item_count()).find(|&index| {
            self.item_is_pinned(index)
                && self.item_session_name(index) == Some(session)
                && self
                    .tab_at(index)
                    .is_some_and(|tab| tab.position == position)
        })
    }

    fn other_previous_session(&self) -> Option<&str> {
        let name = self.previous_session_name.as_deref()?;
        self.sessions
            .iter()
            .find(|session| session.name == name && !session.current)
            .map(|session| session.name.as_str())
    }

    fn commit_index(&mut self, index: usize) -> Action {
        if let Some(name) = self.session_at(index).map(|session| session.name.clone()) {
            self.enter_session_board(&name);
            return Action::None;
        }
        let Some(tab) = self.tab_at(index) else {
            return Action::Dismiss;
        };
        let position = tab.position;
        match self.item_session_name(index).map(str::to_owned) {
            Some(session) => self.jump_session_tab(&session, Some(position)),
            None => Action::Commit {
                tab_index: position as u32,
            },
        }
    }

    fn toggle_pin_at_cursor(&mut self) -> bool {
        let Some(session) = self.item_session_name(self.cursor).map(str::to_owned) else {
            return false;
        };
        if self.session_at(self.cursor).is_some() {
            return false;
        }
        let Some(name) = self
            .tab_at(self.cursor)
            .map(display_name)
            .map(str::to_owned)
        else {
            return false;
        };
        let selected = BoardIdentity::Pinned {
            session: session.clone(),
            name: name.clone(),
        };
        if let Some(index) = self
            .pins
            .iter()
            .position(|pin| pin.session == session && pin.tab_name == name)
        {
            self.pins.remove(index);
        } else {
            self.pins.push(Pin {
                session,
                tab_name: name,
            });
        }
        self.reseat_cursor(Some(selected));
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
        true
    }

    fn jump_session_tab(&self, name: &str, tab_position: Option<usize>) -> Action {
        if self
            .sessions
            .iter()
            .any(|session| session.name == name && session.current)
        {
            return tab_position
                .map(|tab_index| Action::Commit {
                    tab_index: tab_index as u32,
                })
                .unwrap_or(Action::PreviousTab);
        }
        Action::SwitchSession {
            name: name.to_owned(),
            tab_position,
        }
    }

    fn enter_session_board(&mut self, name: &str) {
        if !self.sessions.iter().any(|session| session.name == name) {
            return;
        }
        self.drilled_session = Some(name.to_owned());
        self.hint = None;
        self.pending_g = false;
        self.pending_z = false;
        self.scroll_offset = 0;
        self.reset_cursor_to_active();
        self.ensure_index_visible(self.cursor);
    }

    fn leave_session_board(&mut self) {
        self.drilled_session = None;
        self.hint = None;
        self.pending_g = false;
        self.pending_z = false;
        self.scroll_offset = 0;
        self.reset_cursor_to_active();
        self.ensure_index_visible(self.cursor);
    }

    pub(crate) fn layout_plan(&self, area: Rect) -> grid::LayoutPlan {
        grid::LayoutPlan::calculate_with_bands(
            &self.item_widths(),
            area,
            self.scroll_offset,
            &self.band_ends(),
        )
    }

    fn band_ends(&self) -> Vec<usize> {
        let total = self.item_count();
        let mut ends = Vec::new();
        let pins = self.pin_count();
        if pins > 0 && pins < total {
            ends.push(pins);
        }
        if self.drilled_session.is_none() {
            let sessions = self.sessions.len();
            if sessions > 0 {
                let session_end = pins + sessions;
                if session_end < total {
                    ends.push(session_end);
                }
            }
        }
        ends
    }

    pub(crate) fn current_layout_plan(&self) -> grid::LayoutPlan {
        let (rows, cols) = self.viewport.unwrap_or((1, 1));
        self.layout_plan(Rect::new(
            0,
            0,
            cols.min(u16::MAX as usize) as u16,
            content_rows(rows).min(u16::MAX as usize) as u16,
        ))
    }

    fn ensure_index_visible(&mut self, index: usize) {
        let plan = self.current_layout_plan();
        if plan.mode != grid::LayoutMode::Scroll || plan.visible_count == 0 {
            self.scroll_offset = 0;
            return;
        }
        if index < plan.first_visible {
            self.scroll_offset = index;
        } else if index >= plan.visible_end() {
            self.scroll_offset = index + 1 - plan.visible_count;
        }
    }

    fn move_cursor_by(&mut self, delta: isize) {
        let last = self.item_count().saturating_sub(1) as isize;
        self.cursor = (self.cursor as isize + delta).clamp(0, last) as usize;
    }

    fn align_cursor(&mut self, alignment: ScrollAlignment) {
        if self.current_layout_plan().mode != grid::LayoutMode::Scroll {
            return;
        }
        let visible_capacity = self
            .viewport
            .map_or(1, |(rows, _)| content_rows(rows))
            .max(1);
        self.scroll_offset = match alignment {
            ScrollAlignment::Top => self.cursor,
            ScrollAlignment::Center => self.cursor.saturating_sub(visible_capacity / 2),
            ScrollAlignment::Bottom => self
                .cursor
                .saturating_add(1)
                .saturating_sub(visible_capacity),
        };
    }

    fn item_widths(&self) -> Vec<usize> {
        (0..self.item_count())
            .map(|index| {
                let title_width = Line::from(self.item_title(index).unwrap_or("")).width();
                let active_width = usize::from(self.item_is_active(index)) * 2;
                let previous_width = usize::from(self.is_previous_item(index)) * 4;
                let session_width = usize::from(self.item_is_session(index)) * SESSION_MARK_WIDTH;
                let pin_width = usize::from(self.item_is_pinned(index)) * PIN_MARK_WIDTH;
                let pin_session_width = self
                    .item_pin_session(index)
                    .map(|name| 2 + name.len())
                    .unwrap_or(0);
                title_width
                    + active_width
                    + previous_width
                    + session_width
                    + pin_width
                    + pin_session_width
                    + 2
            })
            .collect()
    }

    fn reseat_cursor(&mut self, selected: Option<BoardIdentity>) {
        if let Some(index) = selected.and_then(|identity| self.index_of(&identity)) {
            self.cursor = index;
            return;
        }
        self.reset_cursor_to_active();
    }

    fn selected_identity(&self) -> Option<BoardIdentity> {
        if let Some(session) = self.session_at(self.cursor) {
            return Some(BoardIdentity::Session(session.name.clone()));
        }
        if self.item_is_pinned(self.cursor) {
            let session = self.item_session_name(self.cursor)?.to_owned();
            let name = self.tab_at(self.cursor).map(display_name)?.to_owned();
            return Some(BoardIdentity::Pinned { session, name });
        }
        self.tab_at(self.cursor)
            .map(|tab| BoardIdentity::Tab(tab.id))
    }

    fn index_of(&self, identity: &BoardIdentity) -> Option<usize> {
        match identity {
            BoardIdentity::Session(name) => (0..self.item_count()).find(|&index| {
                self.session_at(index)
                    .is_some_and(|session| session.name == *name)
            }),
            BoardIdentity::Pinned { session, name } => (0..self.item_count()).find(|&index| {
                self.item_is_pinned(index)
                    && self.item_session_name(index) == Some(session.as_str())
                    && self
                        .tab_at(index)
                        .is_some_and(|tab| display_name(tab) == name)
            }),
            BoardIdentity::Tab(id) => (0..self.item_count())
                .find(|&index| self.tab_at(index).is_some_and(|tab| tab.id == *id)),
        }
    }

    fn session_at(&self, index: usize) -> Option<&SessionFact> {
        match self.slot(index)? {
            Slot::Session(session) => self.sessions.get(session),
            _ => None,
        }
    }

    fn tab_at(&self, index: usize) -> Option<&TabFact> {
        match self.slot(index)? {
            Slot::Pin(pin) => self.resolve_pin(&self.pins[pin]),
            Slot::LiveTab(tab) => self.tabs.get(tab),
            Slot::SnapTab { session, tab } => self.sessions.get(session)?.tabs.get(tab),
            Slot::Session(_) => None,
        }
    }

    fn item_session_name(&self, index: usize) -> Option<&str> {
        match self.slot(index)? {
            Slot::Pin(pin) => Some(self.pins[pin].session.as_str()),
            Slot::Session(session) => Some(self.sessions[session].name.as_str()),
            Slot::LiveTab(_) => self.current_session_name(),
            Slot::SnapTab { session, .. } => Some(self.sessions[session].name.as_str()),
        }
    }

    fn slot(&self, index: usize) -> Option<Slot> {
        self.slots().get(index).copied()
    }

    fn slots(&self) -> Vec<Slot> {
        if let Some(name) = self.drilled_session.as_deref() {
            return self.drilled_slots(name);
        }
        let mut slots = Vec::new();
        for (index, pin) in self.pins.iter().enumerate() {
            if self.resolve_pin(pin).is_some() {
                slots.push(Slot::Pin(index));
            }
        }
        for index in 0..self.sessions.len() {
            slots.push(Slot::Session(index));
        }
        let current = self.current_session_name().unwrap_or("");
        for (index, tab) in self.tabs.iter().enumerate() {
            if !self.is_pinned_name(current, display_name(tab)) {
                slots.push(Slot::LiveTab(index));
            }
        }
        slots
    }

    fn drilled_slots(&self, name: &str) -> Vec<Slot> {
        let mut slots = Vec::new();
        for (index, pin) in self.pins.iter().enumerate() {
            if pin.session == name && self.resolve_pin(pin).is_some() {
                slots.push(Slot::Pin(index));
            }
        }
        if self.drilled_session_is_current() {
            for (index, tab) in self.tabs.iter().enumerate() {
                if !self.is_pinned_name(name, display_name(tab)) {
                    slots.push(Slot::LiveTab(index));
                }
            }
            return slots;
        }
        let Some(session) = self
            .sessions
            .iter()
            .position(|session| session.name == name)
        else {
            return slots;
        };
        for (tab, fact) in self.sessions[session].tabs.iter().enumerate() {
            if !self.is_pinned_name(name, display_name(fact)) {
                slots.push(Slot::SnapTab { session, tab });
            }
        }
        slots
    }

    fn resolve_pin(&self, pin: &Pin) -> Option<&TabFact> {
        if self.current_session_name() == Some(pin.session.as_str()) {
            return self
                .tabs
                .iter()
                .find(|tab| display_name(tab) == pin.tab_name);
        }
        self.sessions
            .iter()
            .find(|session| session.name == pin.session)
            .and_then(|session| {
                session
                    .tabs
                    .iter()
                    .find(|tab| display_name(tab) == pin.tab_name)
            })
    }

    fn is_pinned_name(&self, session: &str, tab_name: &str) -> bool {
        self.pins
            .iter()
            .any(|pin| pin.session == session && pin.tab_name == tab_name)
    }

    fn drilled_session_is_current(&self) -> bool {
        self.drilled_session.as_deref() == self.current_session_name()
    }

    fn active_index(&self) -> Option<usize> {
        if let Some(name) = self.drilled_session.as_deref() {
            if self.drilled_session_is_current() {
                if let Some(id) = self.previous_tab_id {
                    if let Some(index) = self.index_of(&BoardIdentity::Tab(id)) {
                        return Some(index);
                    }
                }
            } else if let Some(position) = self.session_last_tabs.get(name) {
                if let Some(index) = (0..self.item_count()).find(|&index| {
                    self.tab_at(index)
                        .is_some_and(|tab| tab.position == *position)
                }) {
                    return Some(index);
                }
            }
            return (0..self.item_count())
                .find(|&index| self.tab_at(index).is_some_and(|tab| tab.active))
                .or_else(|| (self.item_count() > 0).then_some(0));
        }
        (0..self.item_count())
            .find(|&index| !self.item_is_session(index) && self.item_is_active(index))
            .or_else(|| {
                (0..self.item_count())
                    .find(|&index| self.item_is_session(index) && self.item_is_active(index))
            })
    }

    fn apply_hint_input(&mut self, ch: char) -> Action {
        let ch = ch.to_ascii_lowercase();
        let Some(hint) = self.hint.as_ref() else {
            return Action::None;
        };
        if !hint.query.is_empty() {
            let mut jump_prefix = hint.jump_prefix.clone();
            jump_prefix.push(ch);
            let label_matches: Vec<usize> = hint
                .labels
                .iter()
                .enumerate()
                .filter_map(|(index, label)| {
                    label
                        .as_ref()
                        .is_some_and(|label| label.starts_with(&jump_prefix))
                        .then_some(index)
                })
                .collect();
            if label_matches.len() == 1
                && hint.labels[label_matches[0]].as_deref() == Some(jump_prefix.as_str())
            {
                return self.commit_index(label_matches[0]);
            }
            if !label_matches.is_empty() {
                if let Some(hint) = self.hint.as_mut() {
                    hint.jump_prefix = jump_prefix;
                }
                return Action::None;
            }
        }

        let mut query = hint.query.clone();
        query.push(ch);
        let has_matches = (0..self.item_count()).any(|index| {
            self.item_title(index)
                .is_some_and(|title| title_match_range(title, &query).is_some())
        });
        if !has_matches {
            return Action::None;
        }
        if let Some(hint) = self.hint.as_mut() {
            hint.query = query;
            hint.jump_prefix.clear();
        }
        self.recompute_hint_labels();
        let sole_match = self.hint.as_ref().and_then(|hint| {
            let matched: Vec<usize> = hint
                .labels
                .iter()
                .enumerate()
                .filter_map(|(index, label)| label.as_ref().map(|_| index))
                .collect();
            (matched.len() == 1).then_some(matched[0])
        });
        if let Some(index) = sole_match {
            return self.commit_index(index);
        }
        if let Some(first_match) = self
            .hint
            .as_ref()
            .and_then(|hint| hint.labels.iter().position(Option::is_some))
        {
            self.ensure_index_visible(first_match);
        }
        Action::None
    }

    fn recompute_hint_labels(&mut self) {
        let Some(hint) = self.hint.as_ref() else {
            return;
        };
        let query = hint.query.clone();
        let matches: Vec<usize> = (0..self.item_count())
            .filter(|index| {
                !query.is_empty()
                    && self
                        .item_title(*index)
                        .is_some_and(|title| title_match_range(title, &query).is_some())
            })
            .collect();
        let mut available: Vec<u8> = HINT_ALPHABET
            .iter()
            .copied()
            .filter(|candidate| {
                let mut extended = query.clone();
                extended.push(char::from(*candidate));
                !(0..self.item_count()).any(|index| {
                    self.item_title(index)
                        .is_some_and(|title| title_match_range(title, &extended).is_some())
                })
            })
            .collect();
        if available.is_empty() {
            available.extend_from_slice(HINT_ALPHABET);
        }
        let generated = labels_for(matches.len(), &available);
        let item_count = self.item_count();
        if let Some(hint) = self.hint.as_mut() {
            hint.labels = vec![None; item_count];
            for (index, label) in matches.into_iter().zip(generated) {
                hint.labels[index] = Some(label);
            }
        }
    }
}

pub(crate) fn content_rows(rows: usize) -> usize {
    if rows >= 3 {
        rows - 1
    } else {
        rows
    }
}

const HINT_ALPHABET: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
const SESSION_MARK_WIDTH: usize = 2;
const PIN_MARK_WIDTH: usize = 2;

fn labels_for(count: usize, alphabet: &[u8]) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let base = alphabet.len();
    let mut width = 1;
    let mut capacity = base;
    while capacity < count {
        width += 1;
        capacity = capacity.saturating_mul(base);
    }
    (0..count)
        .map(|mut index| {
            let mut label = vec![alphabet[0]; width];
            for slot in label.iter_mut().rev() {
                *slot = alphabet[index % base];
                index /= base;
            }
            String::from_utf8(label).expect("hint alphabet is ASCII")
        })
        .collect()
}

fn title_match_range(title: &str, query: &str) -> Option<(usize, usize)> {
    if query.is_empty() {
        return Some((0, 0));
    }
    let title: Vec<char> = title.chars().map(|ch| ch.to_ascii_lowercase()).collect();
    let query: Vec<char> = query.chars().map(|ch| ch.to_ascii_lowercase()).collect();
    title
        .windows(query.len())
        .position(|window| window == query)
        .map(|start| (start, query.len()))
}

pub fn display_name(tab: &TabFact) -> &str {
    let name = tab.name.trim();
    if name.is_empty() {
        return "untitled";
    }
    name
}
