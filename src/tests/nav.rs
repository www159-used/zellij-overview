use crate::test_support::numbered_tabs;
use crate::{Key, Overview};

#[test]
fn opening_holds_compact_until_the_viewport_can_frame() {
    let mut overview = Overview::new();
    overview.apply_tabs(numbered_tabs(12));
    overview.set_viewport(7, 50);
    assert!(overview.should_hold_opening_paint(true));
    overview.set_viewport(12, 80);
    assert!(!overview.should_hold_opening_paint(true));
    overview.set_viewport(7, 50);
    assert!(!overview.should_hold_opening_paint(false));
}

#[test]
fn navigation_uses_the_rendered_responsive_grid() {
    let mut overview = Overview::new();
    overview.apply_tabs(numbered_tabs(8));
    overview.set_viewport(7, 96);
    overview.decide(Key::Down);
    assert_eq!(overview.cursor(), 1);
}

#[test]
fn scrolling_stops_at_both_ends_and_keeps_the_camera_on_the_cursor() {
    let mut overview = Overview::new();
    overview.apply_tabs(numbered_tabs(20));
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
    overview.apply_tabs(numbered_tabs(20));
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
    overview.apply_tabs(numbered_tabs(20));
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
    overview.apply_tabs(numbered_tabs(20));
    overview.decide(Key::Last);
    assert_eq!(overview.cursor(), 19);
    overview.decide(Key::GoPrefix);
    overview.decide(Key::GoPrefix);
    assert_eq!(overview.cursor(), 0);
}

#[test]
fn vim_z_commands_align_the_cursor_in_the_scroll_viewport() {
    let mut overview = Overview::new();
    overview.apply_tabs(numbered_tabs(20));
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
