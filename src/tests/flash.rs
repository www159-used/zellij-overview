use crate::test_support::{numbered_tabs, session, tab};
use crate::{Action, Key, Overview};

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
    // "t" matches tab-* and keeps flash open; unique "z" would jump.
    overview.decide(Key::Input('t'));
    let plan = overview.current_layout_plan();
    assert_eq!(overview.cursor(), 0);
    assert!(overview.is_hinting());
    assert!(plan.first_visible <= 19);
    assert!(overview.hint_label(0).is_some() || overview.hint_label(1).is_some());
}

#[test]
fn search_then_tip_commits_without_confirmation() {
    let mut overview = Overview::new();
    overview.apply_tabs(vec![
        tab(10, 0, "notes", true),
        tab(11, 1, "Feature/Geo-DB", false),
        tab(12, 2, "logs", false),
    ]);
    overview.decide(Key::StartHint);
    assert_eq!(overview.hint_label(0), None);
    // "o" matches notes and logs — tip still required.
    assert_eq!(overview.decide(Key::Input('o')), Action::None);
    assert_eq!(overview.hint_query(), "o");
    assert!(overview.hint_label(0).is_some());
    assert!(overview.hint_label(2).is_some());
    let label = overview.hint_label(0).unwrap().to_owned();
    assert_eq!(
        overview.decide(Key::Input(label.chars().next().unwrap())),
        Action::Commit { tab_index: 0 }
    );
}

#[test]
fn sole_search_match_commits_immediately() {
    let mut overview = Overview::new();
    overview.apply_tabs(vec![
        tab(10, 0, "notes", true),
        tab(11, 1, "Feature/Geo-DB", false),
    ]);
    overview.decide(Key::StartHint);
    assert_eq!(
        overview.decide(Key::Input('g')),
        Action::Commit { tab_index: 1 }
    );
}

#[test]
fn two_character_tips_narrow_then_commit() {
    let mut overview = Overview::new();
    overview.apply_tabs(numbered_tabs(52));
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
fn first_hint_character_builds_query_when_ambiguous() {
    let mut overview = Overview::new();
    overview.apply_tabs(vec![
        tab(0, 0, "shell", true),
        tab(1, 1, "share", false),
        tab(2, 2, "notes", false),
    ]);
    overview.decide(Key::StartHint);
    assert_eq!(overview.decide(Key::Input('h')), Action::None);
    assert_eq!(overview.hint_query(), "h");
    assert_eq!(overview.hint_match_range(0), Some((1, 1)));
    assert_eq!(overview.hint_match_range(1), Some((1, 1)));
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

#[test]
fn flash_sole_session_match_opens_its_tabs() {
    let mut overview = Overview::new();
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_sessions(vec![session("geo", false, 3), session("notes", true, 1)]);
    overview.decide(Key::StartHint);
    assert_eq!(overview.decide(Key::Input('g')), Action::None);
    assert_eq!(overview.viewing_session(), Some("geo"));
    assert!(!overview.is_hinting());
    assert_eq!(overview.item_title(0), Some("geo-0"));
}

#[test]
fn flash_tip_on_ambiguous_session_opens_its_tabs() {
    let mut overview = Overview::new();
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_sessions(vec![
        session("geo", false, 3),
        session("gitea", false, 2),
        session("notes", true, 1),
    ]);
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
