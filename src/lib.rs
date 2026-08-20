//! Tab overview core. No Zellij types — the WASM adapter maps host events in.

mod ansi;
#[cfg(test)]
mod floating_state;
mod grid;
mod render;

use ratatui::{layout::Rect, text::Line};
pub use render::{paint, Frame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabFact {
    pub id: usize,
    pub position: usize,
    pub name: String,
    pub active: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Dismiss,
    /// Zero-based index expected by zellij-tile's `go_to_tab`.
    Commit {
        tab_index: u32,
    },
    PreviousTab,
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
    /// Index into `tabs` (tab-bar order).
    cursor: usize,
    previous_tab_id: Option<usize>,
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
        let cursor_id = self.selected_tab().map(|t| t.id);
        self.tabs = tabs;
        self.reseat_cursor(cursor_id);
        if self.hint.is_some() {
            self.recompute_hint_labels();
        }
    }

    pub fn reset_cursor_to_active(&mut self) {
        if let Some(index) = self.tabs.iter().position(|t| t.active) {
            self.cursor = index;
        } else {
            self.cursor = 0;
        }
    }

    pub fn tabs(&self) -> &[TabFact] {
        &self.tabs
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_previous_tab_id(&mut self, tab_id: Option<usize>) {
        self.previous_tab_id = tab_id;
    }

    pub fn set_viewport(&mut self, rows: usize, cols: usize) {
        self.viewport = Some((rows, cols));
        self.ensure_index_visible(self.cursor);
    }

    pub fn is_previous_tab(&self, index: usize) -> bool {
        self.tabs
            .get(index)
            .is_some_and(|tab| Some(tab.id) == self.previous_tab_id)
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
        let tab = self.tabs.get(index)?;
        title_match_range(display_name(tab), &hint.query)
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
                self.cursor = self.tabs.len().saturating_sub(1);
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
            Key::PreviousTab => Action::PreviousTab,
            Key::Dismiss if self.is_hinting() => {
                self.hint = None;
                self.reset_cursor_to_active();
                Action::None
            }
            Key::Dismiss | Key::Toggle => Action::Dismiss,
            Key::StartHint => {
                self.hint = Some(HintState {
                    labels: vec![None; self.tabs.len()],
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

    fn commit_cursor(&self) -> Action {
        match self.selected_tab() {
            Some(tab) => Action::Commit {
                tab_index: tab.position as u32,
            },
            None => Action::Dismiss,
        }
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
        let last = self.tabs.len().saturating_sub(1) as isize;
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
        self.tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let title_width = Line::from(display_name(tab)).width();
                let active_width = usize::from(tab.active) * 2;
                let previous_width = usize::from(self.is_previous_tab(index)) * 4;
                title_width + active_width + previous_width + 2
            })
            .collect()
    }

    fn reseat_cursor(&mut self, previous_id: Option<usize>) {
        if let Some(id) = previous_id {
            if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
                self.cursor = index;
                return;
            }
        }
        self.reset_cursor_to_active();
    }

    fn selected_tab(&self) -> Option<&TabFact> {
        self.tabs.get(self.cursor)
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
                return self
                    .tabs
                    .get(label_matches[0])
                    .map(|tab| Action::Commit {
                        tab_index: tab.position as u32,
                    })
                    .unwrap_or(Action::None);
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
        let has_matches = self
            .tabs
            .iter()
            .any(|tab| title_match_range(display_name(tab), &query).is_some());
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
        let matches: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter_map(|(index, tab)| {
                title_match_range(display_name(tab), &query)
                    .filter(|_| !query.is_empty())
                    .map(|_| index)
            })
            .collect();
        let mut available: Vec<u8> = HINT_ALPHABET
            .iter()
            .copied()
            .filter(|candidate| {
                let mut extended = query.clone();
                extended.push(char::from(*candidate));
                !self
                    .tabs
                    .iter()
                    .any(|tab| title_match_range(display_name(tab), &extended).is_some())
            })
            .collect();
        if available.is_empty() {
            available.extend_from_slice(HINT_ALPHABET);
        }
        let generated = labels_for(matches.len(), &available);
        if let Some(hint) = self.hint.as_mut() {
            hint.labels = vec![None; self.tabs.len()];
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
}
