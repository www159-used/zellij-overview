use ratatui::{
    buffer::{Buffer, CellWidth},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    grid::{LayoutMode, LayoutPlan},
    theme::theme,
    Overview,
};

const CARD_FOREGROUND: Color = Color::Reset;
const SESSION_MARK: &str = "◆ ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub lines: Vec<String>,
}

pub fn paint(overview: &Overview, rows: usize, cols: usize) -> Frame {
    if rows == 0 || cols == 0 {
        return Frame { lines: Vec::new() };
    }

    let area = Rect::new(
        0,
        0,
        cols.min(u16::MAX as usize) as u16,
        rows.min(u16::MAX as usize) as u16,
    );
    let mut buffer = Buffer::empty(area);
    let (content, footer) = if rows >= 3 {
        let [content, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        (content, Some(footer))
    } else {
        (area, None)
    };

    render_cards(overview, content, &mut buffer);
    if let Some(footer) = footer {
        render_footer(overview, footer, &mut buffer);
    }

    Frame {
        lines: crate::ansi::encode_lines(&buffer),
    }
}

fn render_cards(overview: &Overview, area: Rect, buffer: &mut Buffer) {
    if overview.is_help_visible() {
        render_help(area, buffer);
        return;
    }
    if overview.item_count() == 0 {
        Paragraph::new("no tabs")
            .style(Style::default().fg(Color::DarkGray))
            .render(area, buffer);
        return;
    }
    if area.height == 0 || area.width == 0 {
        return;
    }

    let plan = overview.layout_plan(area);
    for card in &plan.cards {
        render_card(overview, card.index, card.area, plan.mode, buffer);
    }
    render_pin_separator(&plan, area, buffer);
    render_scroll_indicators(&plan, overview.item_count(), area, buffer);
}

fn render_pin_separator(plan: &LayoutPlan, area: Rect, buffer: &mut Buffer) {
    let Some(y) = plan.separator_y else {
        return;
    };
    if y < area.y || y >= area.y.saturating_add(area.height) || area.width == 0 {
        return;
    }
    let line = "─".repeat(usize::from(area.width));
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(theme().pin_border),
    )))
    .render(Rect::new(area.x, y, area.width, 1), buffer);
}

fn render_help(area: Rect, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default()
        .title(" help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme().focus));
    let inner = block.inner(area);
    block.render(area, buffer);
    let lines = vec![
        Line::from("h/j/k/l, arrows  move"),
        Line::from("Ctrl+d/u           half page down/up"),
        Line::from("Ctrl+f/b, PgDn/Up  full page down/up"),
        Line::from("gg / G             first / last item"),
        Line::from("zt / zz / zb       align top / center / bottom"),
        Line::from("s, then query/tip   Flash search / jump"),
        Line::from("Backspace / Esc     delete / cancel search"),
        Line::from("e / Enter           open session tabs, or jump"),
        Line::from("p                   pin or unpin a tab"),
        Line::from("-                   previous tab or session last tab"),
        Line::from("q / Esc / ?         back or close / close help"),
        Line::from("Ctrl+y              toggle overview (global)"),
    ];
    Paragraph::new(lines).render(inner, buffer);
}

fn render_card(
    overview: &Overview,
    index: usize,
    area: Rect,
    mode: LayoutMode,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let title = overview.item_title(index).unwrap_or("");
    let match_range = overview.hint_match_range(index);
    let candidate = match_range.is_some() && !overview.hint_query().is_empty();
    let mut spans = Vec::new();
    if mode == LayoutMode::Scroll
        && overview.viewing_session().is_none()
        && !overview.item_is_session(index)
        && !overview.item_is_pinned(index)
    {
        spans.push(Span::raw("  "));
    }
    if mode != LayoutMode::Framed && overview.cursor() == index {
        spans.push(Span::styled(
            "› ",
            Style::default()
                .fg(theme().focus)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if overview.item_is_session(index) {
        spans.push(Span::styled(
            SESSION_MARK,
            Style::default().fg(theme().session),
        ));
    }
    if overview.item_is_pinned(index) {
        spans.push(Span::styled(
            "* ",
            Style::default()
                .fg(theme().pin_mark)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if overview.item_is_active(index) {
        spans.push(Span::styled("● ", Style::default().fg(theme().focus)));
    }
    if overview.is_previous_item(index) {
        spans.push(Span::styled(
            "[-] ",
            Style::default()
                .fg(theme().tip_bg)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if overview.is_hinting() {
        spans.extend(highlighted_title(
            title,
            match_range.filter(|_| candidate),
            overview.hint_label(index),
            overview.hint_jump_prefix().len(),
        ));
    } else {
        spans.push(Span::raw(title));
    }
    if let Some(session) = overview.item_pin_session(index) {
        spans.push(Span::styled(
            format!("  {session}"),
            Style::default().fg(theme().pin_border),
        ));
    }
    if let Some(count) = overview.item_tab_count(index) {
        spans.push(Span::styled(
            format!("  {count}"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let text_area = match mode {
        LayoutMode::Framed => {
            let border_color = if candidate {
                theme().match_border
            } else if overview.cursor() == index {
                theme().focus
            } else if overview.item_is_pinned(index) {
                theme().pin_border
            } else if overview.item_is_session(index) {
                theme().session
            } else {
                theme().card_border
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));
            let inner = block.inner(area);
            block.render(area, buffer);
            buffer.set_style(inner, Style::default().fg(CARD_FOREGROUND).bg(Color::Reset));
            Rect::new(
                area.x.saturating_add(1),
                area.y.saturating_add(area.height / 2),
                area.width.saturating_sub(2),
                1,
            )
        }
        LayoutMode::Compact => Rect::new(
            area.x,
            area.y.saturating_add(area.height / 2),
            area.width,
            1,
        ),
        LayoutMode::Scroll => area,
    };
    let spans = truncate_spans(spans, usize::from(text_area.width));
    Paragraph::new(Line::from(spans))
        .alignment(if mode == LayoutMode::Scroll {
            Alignment::Left
        } else {
            Alignment::Center
        })
        .render(text_area, buffer);
}

fn truncate_spans<'a>(spans: Vec<Span<'a>>, max_width: usize) -> Vec<Span<'a>> {
    let total_width: usize = spans.iter().map(Span::width).sum();
    if total_width <= max_width {
        return spans;
    }
    if max_width == 0 {
        return Vec::new();
    }

    let target_width = max_width - 1;
    let mut used_width = 0;
    let mut output = Vec::new();
    let mut ellipsis_style = Style::default().fg(CARD_FOREGROUND);

    'spans: for span in spans {
        ellipsis_style = span.style;
        let span_width = span.width();
        if used_width + span_width <= target_width {
            used_width += span_width;
            output.push(span);
            continue;
        }
        let mut partial = String::new();
        for grapheme in span.styled_graphemes(Style::default()) {
            let grapheme_width = usize::from(grapheme.symbol.cell_width());
            if used_width + grapheme_width > target_width {
                if !partial.is_empty() {
                    output.push(Span::styled(partial, span.style));
                }
                break 'spans;
            }
            partial.push_str(grapheme.symbol);
            used_width += grapheme_width;
        }
        if !partial.is_empty() {
            output.push(Span::styled(partial, span.style));
        }
        if used_width >= target_width {
            break;
        }
    }
    output.push(Span::styled("…", ellipsis_style));
    output
}

fn render_scroll_indicators(plan: &LayoutPlan, tab_count: usize, area: Rect, buffer: &mut Buffer) {
    if plan.mode != LayoutMode::Scroll || area.width == 0 || area.height == 0 {
        return;
    }
    let style = Style::default().fg(theme().tip_bg);
    let x = area.right().saturating_sub(1);
    if plan.first_visible > 0 {
        buffer.set_string(x, area.y, "↑", style);
    }
    if plan.visible_end() < tab_count {
        buffer.set_string(x, area.bottom().saturating_sub(1), "↓", style);
    }
}

fn highlighted_title<'a>(
    title: &'a str,
    match_range: Option<(usize, usize)>,
    label: Option<&'a str>,
    jump_prefix_len: usize,
) -> Vec<Span<'a>> {
    let normal = Style::default().fg(CARD_FOREGROUND);
    let Some((start, len)) = match_range else {
        return vec![Span::raw(title)];
    };
    let chars: Vec<char> = title.chars().collect();
    let before: String = chars[..start].iter().collect();
    let matched: String = chars[start..start + len].iter().collect();
    let after: String = chars[start + len..].iter().collect();
    let mut spans = vec![Span::styled(before, normal)];
    if let Some(label) = label {
        spans.extend(hint_badge(label, jump_prefix_len));
    }
    spans.push(Span::styled(
        matched,
        Style::default()
            .fg(theme().match_fg)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(after, normal));
    spans
}

fn hint_badge<'a>(label: &'a str, prefix_len: usize) -> Vec<Span<'a>> {
    let prefix_len = prefix_len.min(label.len());
    let (typed, remaining) = label.split_at(prefix_len);
    let mut spans = Vec::new();
    if !typed.is_empty() {
        spans.push(Span::styled(
            typed,
            Style::default()
                .fg(theme().tip_typed)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if !remaining.is_empty() {
        spans.push(Span::styled(
            remaining,
            Style::default().fg(theme().tip_fg).bg(theme().tip_bg),
        ));
    }
    spans
}

fn render_footer(overview: &Overview, area: Rect, buffer: &mut Buffer) {
    let line = if overview.is_help_visible() {
        Line::from("? / q / Esc close help")
    } else if overview.is_hinting() {
        Line::from(vec![
            Span::styled(
                " FLASH ",
                Style::default().fg(theme().tip_fg).bg(theme().tip_bg),
            ),
            Span::raw(" type to search"),
            Span::styled(
                format!("  /{}█", overview.hint_query()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  tip:{}", overview.hint_jump_prefix()),
                Style::default()
                    .fg(theme().tip_typed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Esc cancel", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        let mut spans = Vec::new();
        if let Some(name) = overview.viewing_session() {
            spans.push(Span::styled(
                format!(" {name} "),
                Style::default().fg(theme().tip_fg).bg(theme().session),
            ));
            spans.push(Span::raw(
                "  tabs   s search   e go   p pin   - prev   Esc/q back   ? help",
            ));
        } else {
            if let Some(name) = overview.current_session_name() {
                spans.push(Span::styled(
                    format!(" {name} "),
                    Style::default().fg(theme().tip_fg).bg(theme().session),
                ));
                spans.push(Span::raw("  "));
            }
            spans.push(Span::raw(
                "hjkl move   s search   e go   p pin   - prev   q close   ? help",
            ));
        }
        Line::from(spans)
    };
    Paragraph::new(line).render(area, buffer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Overview, TabFact};

    fn fg_sgr(color: Color) -> String {
        let Color::Rgb(red, green, blue) = color else {
            panic!("theme colors are rgb");
        };
        format!("\u{1b}[38;2;{red};{green};{blue}m")
    }

    fn bg_sgr(color: Color) -> String {
        let Color::Rgb(red, green, blue) = color else {
            panic!("theme colors are rgb");
        };
        format!("\u{1b}[48;2;{red};{green};{blue}m")
    }

    fn overview() -> Overview {
        let mut overview = Overview::new();
        overview.apply_tabs(vec![
            TabFact {
                id: 1,
                position: 0,
                name: "ww".into(),
                active: true,
            },
            TabFact {
                id: 2,
                position: 1,
                name: "feat/geo-db".into(),
                active: false,
            },
        ]);
        overview
    }

    #[test]
    fn paints_title_cards_and_lavender_selection() {
        let frame = paint(&overview(), 12, 40);
        let joined = frame.lines.join("\n");
        assert!(joined.contains("ww"));
        assert!(joined.contains("feat/geo-db"));
        assert!(joined.contains(&fg_sgr(theme().focus)));
        assert!(joined.contains("s"));
    }

    #[test]
    fn flash_mode_paints_high_contrast_labels() {
        let mut overview = overview();
        overview.decide(Key::StartHint);
        overview.decide(Key::Input('g'));
        let frame = paint(&overview, 12, 40);
        let joined = frame.lines.join("\n");
        assert!(joined.contains("FLASH"));
        assert!(joined.contains(&bg_sgr(theme().tip_bg)));
        assert!(joined.contains(&fg_sgr(theme().tip_fg)));
        assert!(joined.contains(&fg_sgr(theme().match_fg)));
        assert!(!joined.contains("\u{1b}[2m"));
        assert!(joined.contains('a'));

        let area = Rect::new(0, 0, 40, 11);
        let mut buffer = Buffer::empty(area);
        render_cards(&overview, area, &mut buffer);
        let plan = overview.layout_plan(area);
        let second = plan.cards[1].area;
        assert_eq!(buffer[(second.x, second.y)].fg, theme().match_border);
        assert_eq!(buffer[(second.x, second.y)].bg, Color::Reset);
        assert_eq!(buffer[(second.x + 1, second.y + 1)].bg, Color::Reset);
    }

    #[test]
    fn marks_the_previous_tab_destination() {
        let mut overview = overview();
        overview.set_previous_tab_id(Some(2));
        let joined = paint(&overview, 12, 40).lines.join("\n");
        assert!(joined.contains("[-]"));
    }

    #[test]
    fn session_cards_do_not_mark_a_previous_destination() {
        let mut overview = overview();
        overview.apply_sessions(vec![
            crate::SessionFact {
                name: "dev".into(),
                current: true,
                tab_count: 2,
                tabs: vec![],
            },
            crate::SessionFact {
                name: "ops".into(),
                current: false,
                tab_count: 1,
                tabs: vec![],
            },
        ]);
        overview.set_previous_tab_id(Some(2));
        let joined = paint(&overview, 12, 60).lines.join("\n");
        assert!(joined.contains("◆"));
        assert!(joined.contains("dev"));
        assert!(joined.contains("ops"));
        assert!(joined.contains("[-]"));
    }

    #[test]
    fn narrow_frames_do_not_panic() {
        assert_eq!(paint(&overview(), 1, 1).lines.len(), 1);
        assert_eq!(paint(&overview(), 2, 3).lines.len(), 2);
    }

    #[test]
    fn short_wide_frames_keep_every_card_visible() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..8)
                .map(|position| TabFact {
                    id: position,
                    position,
                    name: format!("tab-{position}"),
                    active: position == 0,
                })
                .collect(),
        );
        let joined = paint(&overview, 7, 80).lines.join("\n");
        for position in 0..8 {
            assert!(joined.contains(&format!("tab-{position}")));
        }
    }

    #[test]
    fn card_rows_have_identical_dimensions() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| TabFact {
                    id: position,
                    position,
                    name: format!("tab-{position}"),
                    active: position == 0,
                })
                .collect(),
        );
        let plan = overview.layout_plan(Rect::new(0, 0, 81, 19));
        assert_eq!(plan.cards.len(), 20);
        assert!(plan
            .cards
            .iter()
            .all(|card| card.area.height == plan.cards[0].area.height));
        assert!(plan
            .cards
            .iter()
            .any(|card| card.area.width != plan.cards[0].area.width));
    }

    #[test]
    fn compact_mode_drops_card_borders() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..12)
                .map(|position| TabFact {
                    id: position,
                    position,
                    name: format!("tab-{position}"),
                    active: position == 0,
                })
                .collect(),
        );
        let joined = paint(&overview, 7, 50).lines.join("\n");
        assert!(!joined.contains('┌'));
        assert!(joined.contains("tab-"));
    }

    #[test]
    fn scroll_mode_shows_more_indicator() {
        let mut overview = Overview::new();
        overview.apply_tabs(
            (0..20)
                .map(|position| TabFact {
                    id: position,
                    position,
                    name: format!("tab-{position}"),
                    active: position == 0,
                })
                .collect(),
        );
        let joined = paint(&overview, 6, 12).lines.join("\n");
        assert!(joined.contains('↓'));
        assert!(joined.contains("tab-0"));
        assert!(!joined.contains("tab-19"));
    }

    #[test]
    fn help_overlay_contains_advanced_shortcuts() {
        let mut overview = overview();
        overview.decide(Key::ToggleHelp);
        let joined = paint(&overview, 14, 50).lines.join("\n");
        assert!(joined.contains("Ctrl+d/u"));
        assert!(joined.contains("zt / zz / zb"));
        assert!(!joined.contains("Space"));
        assert!(joined.contains("? / q / Esc close help"));
    }

    #[test]
    fn fused_board_paints_session_marks_and_tab_counts() {
        let mut overview = overview();
        overview.apply_sessions(vec![
            crate::SessionFact {
                name: "ww".into(),
                current: true,
                tab_count: 4,
                tabs: vec![],
            },
            crate::SessionFact {
                name: "lp".into(),
                current: false,
                tab_count: 2,
                tabs: vec![],
            },
        ]);
        let joined = paint(&overview, 12, 60).lines.join("\n");
        assert!(joined.contains("◆"));
        assert!(joined.contains("ww"));
        assert!(joined.contains("lp"));
        assert!(joined.contains('4'));
        assert!(joined.contains('2'));
        assert!(joined.contains("feat/geo-db"));
        assert!(!joined.contains("SESSIONS"));
        assert!(!joined.contains("SPACE"));
    }

    #[test]
    fn scroll_mode_indents_tabs_after_sessions() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![crate::SessionFact {
            name: "ww".into(),
            current: true,
            tab_count: 20,
            tabs: vec![],
        }]);
        overview.apply_tabs(
            (0..20)
                .map(|position| TabFact {
                    id: position,
                    position,
                    name: format!("tab-{position}"),
                    active: position == 0,
                })
                .collect(),
        );
        let joined = paint(&overview, 6, 12).lines.join("\n");
        assert!(joined.contains("◆"));
        assert!(joined.contains("ww"));
        assert!(joined.contains("tab-0"));
        assert!(joined.contains('↓'));
    }

    #[test]
    fn pinned_tab_shows_session_rose_frame_and_divider() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![
            crate::SessionFact {
                name: "lp".into(),
                current: false,
                tab_count: 3,
                tabs: (0..3)
                    .map(|position| TabFact {
                        id: 100 + position,
                        position,
                        name: format!("lp-{position}"),
                        active: false,
                    })
                    .collect(),
            },
            crate::SessionFact {
                name: "ww".into(),
                current: true,
                tab_count: 1,
                tabs: vec![],
            },
        ]);
        overview.apply_tabs(vec![TabFact {
            id: 1,
            position: 0,
            name: "notes".into(),
            active: true,
        }]);
        overview.apply_pins(vec![crate::Pin {
            session: "lp".into(),
            tab_name: "lp-1".into(),
        }]);

        let joined = paint(&overview, 14, 70).lines.join("\n");
        assert!(joined.contains("lp-1"));
        assert!(joined.contains("lp"));
        assert!(joined.contains('─'));
        assert!(joined.contains(&fg_sgr(theme().pin_mark)));
        assert!(joined.contains(&fg_sgr(theme().pin_border)));

        let area = Rect::new(0, 0, 70, 13);
        let mut buffer = Buffer::empty(area);
        render_cards(&overview, area, &mut buffer);
        let plan = overview.layout_plan(area);
        let pin = plan.cards[0].area;
        let rest = plan
            .cards
            .iter()
            .find(|card| !overview.item_is_pinned(card.index))
            .unwrap()
            .area;
        assert_eq!(buffer[(pin.x, pin.y)].fg, theme().pin_border);
        assert!(pin.y + pin.height < rest.y);
        assert_eq!(plan.separator_y, Some(pin.y + pin.height));
    }

    #[test]
    fn pinned_previous_tab_shows_dash_without_session_name() {
        let mut overview = Overview::new();
        overview.apply_sessions(vec![crate::SessionFact {
            name: "ww".into(),
            current: true,
            tab_count: 2,
            tabs: vec![],
        }]);
        overview.apply_tabs(vec![
            TabFact {
                id: 1,
                position: 0,
                name: "notes".into(),
                active: true,
            },
            TabFact {
                id: 2,
                position: 1,
                name: "logs".into(),
                active: false,
            },
        ]);
        overview.apply_pins(vec![crate::Pin {
            session: "ww".into(),
            tab_name: "logs".into(),
        }]);
        overview.set_previous_tab_id(Some(2));
        let joined = paint(&overview, 12, 50).lines.join("\n");
        assert!(joined.contains("[-]"));
        assert!(joined.contains("logs"));
        assert_eq!(overview.item_pin_session(0), None);
    }

    #[test]
    fn truncated_unicode_titles_end_with_an_ellipsis() {
        let spans = truncate_spans(vec![Span::raw("你好世界")], 5);
        let text: String = spans.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(text, "你好…");
    }

    #[test]
    fn truncation_keeps_emoji_graphemes_intact() {
        let spans = truncate_spans(vec![Span::raw("👩‍💻xy")], 3);
        let text: String = spans.iter().map(|span| span.content.as_ref()).collect();
        assert_eq!(text, "👩‍💻…");
    }
}
