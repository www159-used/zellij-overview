//! Tab overview core. No Zellij types — the WASM adapter maps host events in.

mod ansi;
#[cfg(test)]
mod floating_state;
mod grid;
mod render;
mod usage;

use std::collections::BTreeMap;

use ratatui::{layout::Rect, text::Line};
pub use render::{paint, Frame};
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BoardIdentity {
    Session(String),
    Tab(usize),
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
    /// Combined board: sessions first, then the current session's tabs.
    cursor: usize,
    previous_tab_id: Option<usize>,
    previous_session_name: Option<String>,
    viewport: Option<(usize, usize)>,
    scroll_offset: usize,
    pending_g: bool,
    pending_z: bool,
    show_help: bool,
    hint: Option<HintState>,
}

impl Overview {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_tabs(&mut self, mut tabs: Vec<TabFact>) {
        tabs.sort_by_key(|t| t.position);
        let selected = self.selected_identity();
        self.tabs = tabs;
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
            if let Some(position) = self.session_last_tabs.get(name) {
                return self
                    .tab_at(index)
                    .is_some_and(|tab| tab.position == *position);
            }
            return self.drilled_session_is_current()
                && self
                    .tab_at(index)
                    .is_some_and(|tab| Some(tab.id) == self.previous_tab_id);
        }
        if let Some(name) = self.other_previous_session() {
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
        self.drilled_session.is_none() && index < self.sessions.len()
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
        if self.drilled_session.is_some() {
            return self.viewed_tabs().len();
        }
        self.sessions.len() + self.tabs.len()
    }

    pub(crate) fn item_tab_count(&self, index: usize) -> Option<usize> {
        self.session_at(index).map(|session| session.tab_count)
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
            self.enter_session_board(&name);
            return Action::None;
        }
        Action::PreviousTab
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
        if let Some(name) = self.drilled_session.clone() {
            return self.jump_session_tab(&name, Some(position));
        }
        Action::Commit {
            tab_index: position as u32,
        }
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
        grid::LayoutPlan::calculate(&self.item_widths(), area, self.scroll_offset)
    }

    fn current_layout_plan(&self) -> grid::LayoutPlan {
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
                let session_width = self
                    .item_tab_count(index)
                    .map(|count| SESSION_MARK_WIDTH + 2 + count.to_string().len())
                    .unwrap_or(0);
                title_width + active_width + previous_width + session_width + 2
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
        self.tab_at(self.cursor)
            .map(|tab| BoardIdentity::Tab(tab.id))
    }

    fn index_of(&self, identity: &BoardIdentity) -> Option<usize> {
        match identity {
            BoardIdentity::Session(name) if self.drilled_session.is_none() => self
                .sessions
                .iter()
                .position(|session| session.name == *name),
            BoardIdentity::Session(_) => None,
            BoardIdentity::Tab(id) if self.drilled_session.is_some() => {
                self.viewed_tabs().iter().position(|tab| tab.id == *id)
            }
            BoardIdentity::Tab(id) => self
                .tabs
                .iter()
                .position(|tab| tab.id == *id)
                .map(|index| self.sessions.len() + index),
        }
    }

    fn session_at(&self, index: usize) -> Option<&SessionFact> {
        if self.drilled_session.is_some() {
            return None;
        }
        self.sessions.get(index)
    }

    fn tab_at(&self, index: usize) -> Option<&TabFact> {
        if self.drilled_session.is_some() {
            return self.viewed_tabs().get(index);
        }
        self.tabs.get(index.checked_sub(self.sessions.len())?)
    }

    fn viewed_tabs(&self) -> &[TabFact] {
        let Some(name) = self.drilled_session.as_deref() else {
            return &[];
        };
        if self.drilled_session_is_current() {
            return &self.tabs;
        }
        self.sessions
            .iter()
            .find(|session| session.name == name)
            .map(|session| session.tabs.as_slice())
            .unwrap_or(&[])
    }

    fn drilled_session_is_current(&self) -> bool {
        self.drilled_session.as_deref() == self.current_session_name()
    }

    fn active_index(&self) -> Option<usize> {
        if let Some(name) = self.drilled_session.as_deref() {
            if let Some(position) = self.session_last_tabs.get(name) {
                if let Some(index) = self
                    .viewed_tabs()
                    .iter()
                    .position(|tab| tab.position == *position)
                {
                    return Some(index);
                }
            }
            return self
                .viewed_tabs()
                .iter()
                .position(|tab| tab.active)
                .or_else(|| (!self.viewed_tabs().is_empty()).then_some(0));
        }
        self.tabs
            .iter()
            .position(|tab| tab.active)
            .map(|index| self.sessions.len() + index)
            .or_else(|| self.sessions.iter().position(|session| session.current))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: usize, position: usize, name: &str, active: bool) -> TabFact {
        TabFact {
            id,
            position,
            name: name.to_owned(),
            active,
        }
    }

    #[test]
    fn enter_commits_cursor_tab() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![
            tab(10, 0, "ww", false),
            tab(11, 1, "feat/geo-db", true),
            tab(12, 2, "notes", false),
        ]);
        overview.reset_cursor_to_active();
        assert_eq!(overview.cursor(), 1);
        assert_eq!(
            overview.decide(Key::Confirm),
            Action::Commit { tab_index: 1 }
        );
    }

    #[test]
    fn previous_tab_delegates_to_host_history() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(10, 0, "ww", true)]);
        assert_eq!(overview.decide(Key::PreviousTab), Action::PreviousTab);
    }

    #[test]
    fn previous_tab_is_identified_by_stable_tab_id() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![
            tab(10, 0, "current", true),
            tab(11, 1, "previous", false),
        ]);
        overview.set_previous_tab_id(Some(11));
        assert!(!overview.is_previous_tab(0));
        assert!(overview.is_previous_tab(1));
    }

    #[test]
    fn navigation_uses_the_rendered_responsive_grid() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..8)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.set_viewport(7, 80);
        overview.decide(Key::Down);
        assert_eq!(overview.cursor(), 1);
    }

    #[test]
    fn scrolling_stops_at_both_ends_and_keeps_the_camera_on_the_cursor() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.set_viewport(6, 12);

        overview.decide(Key::Left);
        overview.decide(Key::Up);
        overview.decide(Key::HalfPageUp);
        assert_eq!(overview.cursor(), 0);
        assert_eq!(overview.current_layout_plan().first_visible, 0);

        overview.decide(Key::Last);
        overview.decide(Key::Right);
        overview.decide(Key::Down);
        overview.decide(Key::HalfPageDown);
        assert_eq!(overview.cursor(), 19);
        let plan = overview.current_layout_plan();
        assert!(overview.cursor() >= plan.first_visible);
        assert!(overview.cursor() < plan.visible_end());
    }

    #[test]
    fn scrolling_keeps_the_moved_cursor_visible() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.set_viewport(6, 12);
        for _ in 0..5 {
            overview.decide(Key::Down);
        }
        let plan = overview.current_layout_plan();
        assert_eq!(overview.cursor(), 5);
        assert_eq!(plan.first_visible, 1);
        assert!(overview.cursor() < plan.visible_end());
    }

    #[test]
    fn vim_page_keys_scroll_by_half_and_full_viewports() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.set_viewport(6, 12);
        overview.decide(Key::HalfPageDown);
        assert_eq!(overview.cursor(), 2);
        overview.decide(Key::PageDown);
        assert_eq!(overview.cursor(), 7);
        overview.decide(Key::HalfPageUp);
        assert_eq!(overview.cursor(), 5);
        overview.decide(Key::PageUp);
        assert_eq!(overview.cursor(), 0);
    }

    #[test]
    fn vim_gg_and_uppercase_g_jump_to_the_ends() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.decide(Key::Last);
        assert_eq!(overview.cursor(), 19);
        overview.decide(Key::GoPrefix);
        overview.decide(Key::GoPrefix);
        assert_eq!(overview.cursor(), 0);
    }

    #[test]
    fn vim_z_commands_align_the_cursor_in_the_scroll_viewport() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.set_viewport(6, 12);
        overview.decide(Key::PageDown);
        overview.decide(Key::PageDown);
        assert_eq!(overview.cursor(), 10);

        overview.decide(Key::ZPrefix);
        overview.decide(Key::AlignTop);
        assert_eq!(overview.current_layout_plan().first_visible, 10);

        overview.decide(Key::ZPrefix);
        overview.decide(Key::ZPrefix);
        assert_eq!(overview.current_layout_plan().first_visible, 8);

        overview.decide(Key::ZPrefix);
        overview.decide(Key::AlignBottom);
        assert_eq!(overview.current_layout_plan().first_visible, 6);
    }

    #[test]
    fn help_overlay_is_closed_before_the_overview() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "ww", true)]);
        assert_eq!(overview.decide(Key::ToggleHelp), Action::None);
        assert!(overview.is_help_visible());
        assert_eq!(overview.decide(Key::Dismiss), Action::None);
        assert!(!overview.is_help_visible());
        assert_eq!(overview.decide(Key::Dismiss), Action::Dismiss);
    }

    #[test]
    fn flash_reveals_an_offscreen_match_without_moving_cursor() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| {
                    let name = if position == 19 {
                        "zebra".to_owned()
                    } else {
                        format!("tab-{position}")
                    };
                    tab(position, position, &name, position == 0)
                })
                .collect(),
        );
        overview.set_viewport(6, 12);
        overview.decide(Key::StartHint);
        overview.decide(Key::Input('z'));
        let plan = overview.current_layout_plan();
        assert_eq!(overview.cursor(), 0);
        assert_eq!(plan.first_visible, 15);
        assert_eq!(overview.hint_label(19), Some("a"));
    }

    #[test]
    fn esc_dismisses_without_commit() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "ww", true)]);
        overview.decide(Key::Right);
        assert_eq!(overview.decide(Key::Dismiss), Action::Dismiss);
    }

    #[test]
    fn cursor_follows_tab_id_after_reorder() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(10, 0, "a", true), tab(11, 1, "b", false)]);
        overview.decide(Key::Right);
        assert_eq!(overview.tabs()[overview.cursor()].id, 11);
        overview.apply_tabs(vec![tab(11, 0, "b", false), tab(10, 1, "a", true)]);
        assert_eq!(overview.tabs()[overview.cursor()].id, 11);
        assert_eq!(overview.cursor(), 0);
    }

    #[test]
    fn missing_cursor_tab_snaps_to_active() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(10, 0, "a", false), tab(11, 1, "b", true)]);
        overview.decide(Key::Right);
        overview.apply_tabs(vec![tab(10, 0, "a", false)]);
        assert_eq!(overview.cursor(), 0);
    }

    #[test]
    fn default_tab_name_is_preserved() {
        let tab = tab(1, 2, "Tab #3", true);
        assert_eq!(display_name(&tab), "Tab #3");
    }

    #[test]
    fn search_then_tip_commits_without_confirmation() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![
            tab(10, 0, "notes", true),
            tab(11, 1, "Feature/Geo-DB", false),
        ]);
        overview.decide(Key::StartHint);
        assert_eq!(overview.hint_label(0), None);
        assert_eq!(overview.decide(Key::Input('g')), Action::None);
        assert_eq!(overview.hint_query(), "g");
        assert_eq!(overview.hint_match_range(1), Some((8, 1)));
        assert_eq!(overview.cursor(), 0);
        let label = overview.hint_label(1).unwrap().to_owned();
        assert_eq!(
            overview.decide(Key::Input(label.chars().next().unwrap())),
            Action::Commit { tab_index: 1 }
        );
    }

    #[test]
    fn two_character_tips_narrow_then_commit() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..52)
                .map(|position| {
                    tab(
                        position,
                        position,
                        &format!("tab-{position}"),
                        position == 0,
                    )
                })
                .collect(),
        );
        overview.decide(Key::StartHint);
        assert_eq!(overview.decide(Key::Input('t')), Action::None);
        let label = overview.hint_label(27).unwrap().to_owned();
        assert_eq!(label.len(), 2);
        let mut chars = label.chars();
        assert_eq!(
            overview.decide(Key::Input(chars.next().unwrap())),
            Action::None
        );
        assert_eq!(
            overview.decide(Key::Input(chars.next().unwrap())),
            Action::Commit { tab_index: 27 }
        );
    }

    #[test]
    fn invalid_search_key_keeps_current_query() {
        let mut overview = Overview::new();
        overview.apply_tabs((0..52).map(|i| tab(i, i, "tab", i == 0)).collect());
        overview.decide(Key::StartHint);
        overview.decide(Key::Input('t'));
        assert_eq!(overview.decide(Key::Input('!')), Action::None);
        assert_eq!(overview.hint_query(), "t");
    }

    #[test]
    fn first_hint_character_always_builds_the_query() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(0, 0, "shell", true), tab(1, 1, "notes", false)]);
        overview.decide(Key::StartHint);
        assert_eq!(overview.decide(Key::Input('h')), Action::None);
        assert_eq!(overview.hint_query(), "h");
        assert_eq!(overview.hint_match_range(0), Some((1, 1)));
    }

    #[test]
    fn escape_cancels_hint_before_dismissing() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(10, 0, "notes", true)]);
        overview.decide(Key::StartHint);
        assert_eq!(overview.decide(Key::Dismiss), Action::None);
        assert!(!overview.is_hinting());
        assert_eq!(overview.decide(Key::Dismiss), Action::Dismiss);
    }

    fn session(name: &str, current: bool, tab_count: usize) -> SessionFact {
        SessionFact {
            name: name.to_owned(),
            current,
            tab_count,
            tabs: (0..tab_count)
                .map(|position| {
                    tab(
                        100 + position,
                        position,
                        &format!("{name}-{position}"),
                        false,
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn board_puts_sessions_before_tabs() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 2)]);
        assert_eq!(overview.item_count(), 4);
        assert_eq!(overview.item_title(0), Some("ww"));
        assert_eq!(overview.item_title(1), Some("lp"));
        assert_eq!(overview.item_title(2), Some("notes"));
        assert_eq!(overview.item_title(3), Some("logs"));
        assert!(overview.item_is_session(0));
        assert!(!overview.item_is_session(2));
        overview.reset_cursor_to_active();
        assert_eq!(overview.cursor(), 2);
        assert_eq!(
            overview.decide(Key::Confirm),
            Action::Commit { tab_index: 0 }
        );
    }

    #[test]
    fn confirm_on_a_session_card_opens_its_tabs() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
        overview.reset_cursor_to_active();
        overview.decide(Key::Left);
        assert_eq!(overview.decide(Key::Confirm), Action::None);
        assert_eq!(overview.viewing_session(), Some("lp"));
        assert_eq!(overview.item_count(), 3);
        assert_eq!(overview.item_title(0), Some("lp-0"));
        assert!(!overview.item_is_session(0));
        assert_eq!(
            overview.decide(Key::Confirm),
            Action::SwitchSession {
                name: "lp".into(),
                tab_position: Some(overview.cursor()),
            }
        );
    }

    #[test]
    fn flash_tip_on_a_session_opens_its_tabs() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.apply_sessions(vec![session("geo", false, 3), session("notes", true, 1)]);
        overview.decide(Key::StartHint);
        assert_eq!(overview.decide(Key::Input('g')), Action::None);
        let label = overview.hint_label(1).unwrap().to_owned();
        assert_eq!(
            overview.decide(Key::Input(label.chars().next().unwrap())),
            Action::None
        );
        assert_eq!(overview.viewing_session(), Some("geo"));
        assert!(!overview.is_hinting());
        assert_eq!(overview.item_title(0), Some("geo-0"));
    }

    #[test]
    fn apply_sessions_replaces_with_the_live_snapshot() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("t", false, 2), session("ww", true, 4)]);
        overview.apply_sessions(vec![session("ww", true, 5)]);
        assert_eq!(
            overview
                .sessions()
                .iter()
                .map(|session| session.name.as_str())
                .collect::<Vec<_>>(),
            vec!["ww"]
        );
        assert_eq!(overview.sessions()[0].tab_count, 5);
    }

    #[test]
    fn touch_current_session_keeps_other_live_sessions() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 2)]);
        overview.touch_current_session(session("ww", true, 5));
        assert_eq!(
            overview
                .sessions()
                .iter()
                .map(|session| (session.name.as_str(), session.tab_count, session.current))
                .collect::<Vec<_>>(),
            vec![("ww", 5, true), ("lp", 3, false)]
        );
    }

    #[test]
    fn dash_uses_the_previous_tab_after_a_same_session_jump() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 2)]);
        overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
        overview.set_previous_session_name(Some("lp".into()));
        overview.set_previous_tab_id(Some(2));
        overview.set_previous_session_name(None);
        assert!(!overview.is_previous_item(1));
        assert!(overview.is_previous_item(3));
        assert_eq!(overview.decide(Key::PreviousTab), Action::PreviousTab);
    }

    #[test]
    fn dash_always_goes_to_the_previous_tab() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("ww", true, 1)]);
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.set_previous_tab_id(Some(1));
        assert!(!overview.is_previous_item(0));
        assert!(overview.is_previous_item(1));
        assert_eq!(overview.decide(Key::PreviousTab), Action::PreviousTab);
    }

    #[test]
    fn dash_returns_to_the_previous_session_after_a_jump() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 2)]);
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.set_previous_session_name(Some("lp".into()));
        overview.set_previous_tab_id(Some(1));
        assert!(overview.is_previous_item(1));
        assert!(!overview.is_previous_item(0));
        assert!(!overview.is_previous_item(2));
        assert_eq!(overview.decide(Key::PreviousTab), Action::None);
        assert_eq!(overview.viewing_session(), Some("lp"));
        assert_eq!(overview.item_title(0), Some("lp-0"));
        overview.set_session_last_tab("lp".into(), 2);
        assert_eq!(
            overview.decide(Key::PreviousTab),
            Action::SwitchSession {
                name: "lp".into(),
                tab_position: Some(2),
            }
        );
    }

    #[test]
    fn dash_on_a_session_board_jumps_that_session_last_tab() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.set_session_last_tab("lp".into(), 2);
        overview.reset_cursor_to_active();
        overview.decide(Key::Left);
        assert_eq!(overview.decide(Key::Confirm), Action::None);
        assert_eq!(overview.viewing_session(), Some("lp"));
        assert!(overview.is_previous_item(2));
        assert!(!overview.is_previous_item(0));
        assert_eq!(
            overview.decide(Key::PreviousTab),
            Action::SwitchSession {
                name: "lp".into(),
                tab_position: Some(2),
            }
        );
    }

    #[test]
    fn dismiss_from_a_session_board_returns_home() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
        overview.reset_cursor_to_active();
        overview.decide(Key::Left);
        overview.decide(Key::Confirm);
        assert_eq!(overview.decide(Key::Dismiss), Action::None);
        assert_eq!(overview.viewing_session(), None);
        assert_eq!(overview.item_title(0), Some("ww"));
        assert_eq!(overview.decide(Key::Dismiss), Action::Dismiss);
    }

    #[test]
    fn dismiss_closes_from_the_fused_board() {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
        overview.apply_sessions(vec![session("ww", true, 1)]);
        assert_eq!(overview.decide(Key::Dismiss), Action::Dismiss);
    }
}
