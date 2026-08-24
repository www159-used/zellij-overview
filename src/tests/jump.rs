use crate::test_support::tab;
use crate::{display_name, Action, Key, Overview};

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
