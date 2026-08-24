use crate::test_support::{pin, session, tab};
use crate::{Action, Key, Overview, SessionFact};

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
fn pin_moves_a_tab_to_the_front_and_does_not_pin_sessions() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 2)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
    overview.reset_cursor_to_active();
    assert_eq!(overview.item_title(overview.cursor()), Some("notes"));
    assert_eq!(overview.decide(Key::Pin), Action::PersistPins);
    assert_eq!(overview.item_title(0), Some("notes"));
    assert!(overview.item_is_pinned(0));
    assert_eq!(overview.item_title(1), Some("ww"));
    assert_eq!(overview.item_title(3), Some("logs"));
    assert_eq!(overview.item_count(), 4);
    overview.decide(Key::Down);
    assert_eq!(overview.decide(Key::Pin), Action::None);
    assert_eq!(overview.pins().len(), 1);
    overview.decide(Key::Up);
    assert_eq!(overview.decide(Key::Pin), Action::PersistPins);
    assert!(!overview.item_is_pinned(0));
    assert_eq!(overview.item_title(2), Some("notes"));
}

#[test]
fn pinned_foreign_tab_sits_first_and_jumps() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_pins(vec![pin("lp", "lp-1")]);
    assert_eq!(overview.item_title(0), Some("lp-1"));
    assert!(overview.item_is_pinned(0));
    assert_eq!(overview.pin_count(), 1);
    assert_eq!(overview.item_pin_session(0), Some("lp"));
    assert_eq!(overview.item_title(1), Some("ww"));
    overview.decide(Key::GoPrefix);
    overview.decide(Key::GoPrefix);
    assert_eq!(
        overview.decide(Key::Confirm),
        Action::SwitchSession {
            name: "lp".into(),
            tab_position: Some(1),
        }
    );
}

#[test]
fn session_board_only_shows_its_own_pins() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_pins(vec![pin("ww", "notes"), pin("lp", "lp-2")]);
    assert_eq!(overview.item_title(0), Some("notes"));
    overview.decide(Key::Last);
    overview.decide(Key::Confirm);
    assert_eq!(overview.viewing_session(), Some("lp"));
    assert_eq!(overview.item_title(0), Some("lp-2"));
    assert!(overview.item_is_pinned(0));
    assert_eq!(overview.item_title(1), Some("lp-0"));
    assert_ne!(overview.item_title(0), Some("notes"));
}

#[test]
fn current_session_pin_survives_a_partial_session_snapshot() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![SessionFact {
        name: "ww".into(),
        current: true,
        tab_count: 2,
        tabs: vec![tab(1, 0, "notes", true)],
    }]);
    overview.apply_pins(vec![pin("ww", "logs")]);
    assert_eq!(overview.pins().len(), 1);
    assert!(!overview.take_stale_cache().pins);
    overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
    assert_eq!(overview.pins().len(), 1);
    assert!(overview.item_is_pinned(0));
}

#[test]
fn unmatched_pins_are_dropped() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("ww", true, 1)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_pins(vec![pin("gone", "old")]);
    assert_eq!(overview.item_title(0), Some("ww"));
    assert!(!overview.item_is_pinned(0));
    assert!(overview.pins().is_empty());
    assert!(overview.take_stale_cache().pins);
}

#[test]
fn deleted_tab_drops_its_pin() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("ww", true, 2)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
    overview.apply_pins(vec![pin("ww", "logs")]);
    assert_eq!(overview.pins().len(), 1);
    overview.take_stale_cache();
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    assert!(overview.pins().is_empty());
    assert!(overview.take_stale_cache().pins);
}

#[test]
fn foreign_pin_stays_until_that_session_tabs_are_known() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![SessionFact {
        name: "lp".into(),
        current: false,
        tab_count: 3,
        tabs: vec![],
    }]);
    overview.apply_pins(vec![pin("lp", "lp-2")]);
    assert_eq!(overview.pins().len(), 1);
    overview.apply_sessions(vec![session("lp", false, 3)]);
    assert_eq!(overview.pins().len(), 1);
    overview.apply_sessions(vec![session("lp", false, 2)]);
    assert!(overview.pins().is_empty());
}

#[test]
fn foreign_pin_survives_a_short_tab_list_when_count_says_more() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![SessionFact {
        name: "lp".into(),
        current: false,
        tab_count: 3,
        tabs: vec![tab(100, 0, "lp-0", false)],
    }]);
    overview.apply_pins(vec![pin("lp", "lp-2")]);
    assert_eq!(overview.pins().len(), 1);
    assert!(!overview.take_stale_cache().pins);
}

#[test]
fn stale_previous_and_session_last_are_dropped() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.set_previous_session_name(Some("lp".into()));
    overview.set_session_last_tab("lp".into(), 2);
    overview.set_session_last_tab("gone".into(), 0);
    overview.prune_stale_cache();
    assert_eq!(overview.session_last_tabs().get("lp"), Some(&2));
    assert!(!overview.session_last_tabs().contains_key("gone"));
    overview.apply_sessions(vec![session("ww", true, 1)]);
    assert!(overview.take_stale_cache().previous);
    assert!(!overview.session_last_tabs().contains_key("lp"));
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
fn current_session_board_uses_zellij_previous_tab() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("ww", true, 2)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
    overview.set_session_last_tab("ww".into(), 0);
    overview.set_previous_tab_id(Some(2));
    assert_eq!(overview.decide(Key::Confirm), Action::None);
    assert_eq!(overview.viewing_session(), Some("ww"));
    assert!(!overview.is_previous_item(0));
    assert!(overview.is_previous_item(1));
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
fn pinned_previous_tab_keeps_the_dash_mark() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("ww", true, 2)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true), tab(2, 1, "logs", false)]);
    overview.apply_pins(vec![pin("ww", "logs")]);
    overview.set_previous_tab_id(Some(2));
    assert!(overview.item_is_pinned(0));
    assert!(overview.is_previous_item(0));
    assert!(!overview.is_previous_item(1));
    assert_eq!(overview.item_pin_session(0), None);
}

#[test]
fn dash_jumps_a_pinned_previous_session_tab() {
    let mut overview = Overview::new();
    overview.apply_sessions(vec![session("lp", false, 3), session("ww", true, 1)]);
    overview.apply_tabs(vec![tab(1, 0, "notes", true)]);
    overview.apply_pins(vec![pin("lp", "lp-2")]);
    overview.set_previous_session_name(Some("lp".into()));
    overview.set_session_last_tab("lp".into(), 2);
    assert_eq!(overview.item_title(0), Some("lp-2"));
    assert_eq!(overview.item_title(1), Some("ww"));
    assert_eq!(overview.item_title(2), Some("notes"));
    assert_eq!(overview.item_count(), 3);
    assert!(overview.is_previous_item(0));
    assert!(!overview.is_previous_item(1));
    assert_eq!(overview.item_pin_session(0), None);
    assert_eq!(
        overview.decide(Key::PreviousTab),
        Action::SwitchSession {
            name: "lp".into(),
            tab_position: Some(2),
        }
    );
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
