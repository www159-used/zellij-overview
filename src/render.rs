use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{columns, display_name, Overview, TabFact};

const MATCH_COLOR: Color = Color::Rgb(255, 184, 108);
const CARD_FOREGROUND: Color = Color::Reset;
const CARD_BORDER: Color = Color::Rgb(90, 96, 114);
const FOCUS_BORDER: Color = Color::Rgb(189, 147, 249);
const TIP_FOREGROUND: Color = Color::Rgb(16, 19, 26);
const TIP_BACKGROUND: Color = Color::Rgb(139, 233, 253);
const TIP_TYPED: Color = FOCUS_BORDER;

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
    let [content, footer] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    render_cards(overview, content, &mut buffer);
    render_footer(overview, footer, &mut buffer);

    Frame {
        lines: crate::ansi::encode_lines(&buffer),
    }
}

fn render_cards(overview: &Overview, area: Rect, buffer: &mut Buffer) {
    let tabs = overview.visible_tabs();
    if tabs.is_empty() {
        Paragraph::new("no tabs")
            .style(Style::default().fg(Color::DarkGray))
            .render(area, buffer);
        return;
    }
    if area.height < 3 || area.width < 3 {
        return;
    }

    let grid_cols = columns(tabs.len());
    let grid_rows = crate::rows(tabs.len());
    let row_constraints = vec![Constraint::Ratio(1, grid_rows as u32); grid_rows];
    let row_areas = Layout::vertical(row_constraints).split(area);

    for (row, row_area) in row_areas.iter().enumerate() {
        let col_constraints = vec![Constraint::Ratio(1, grid_cols as u32); grid_cols];
        let col_areas = Layout::horizontal(col_constraints).split(*row_area);
        for (col, card_area) in col_areas.iter().enumerate() {
            let index = row * grid_cols + col;
            let Some(tab) = tabs.get(index) else {
                continue;
            };
            render_card(overview, index, tab, *card_area, buffer);
        }
    }
}

fn render_card(overview: &Overview, index: usize, tab: &TabFact, area: Rect, buffer: &mut Buffer) {
    if area.width < 3 || area.height < 3 {
        return;
    }

    let match_range = overview.hint_match_range(index);
    let candidate = match_range.is_some() && !overview.hint_query().is_empty();
    let border_color = if overview.cursor() == index {
        FOCUS_BORDER
    } else {
        CARD_BORDER
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    block.render(area, buffer);
    buffer.set_style(inner, Style::default().fg(CARD_FOREGROUND).bg(Color::Reset));

    let mut spans = Vec::new();
    if overview.is_hinting() {
        spans.extend(highlighted_title(
            display_name(tab),
            match_range.filter(|_| candidate),
            overview.hint_label(index),
            overview.hint_jump_prefix().len(),
        ));
    } else {
        spans.push(Span::raw(display_name(tab)));
    }
    if overview.is_previous_tab(index) {
        spans.push(Span::styled(
            " [-]",
            Style::default()
                .fg(TIP_BACKGROUND)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if tab.active {
        spans.push(Span::styled(" ●", Style::default().fg(FOCUS_BORDER)));
    }
    let text_area = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(area.height / 2),
        area.width.saturating_sub(2),
        1,
    );
    Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .render(text_area, buffer);
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
            .fg(MATCH_COLOR)
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
            Style::default().fg(TIP_TYPED).add_modifier(Modifier::BOLD),
        ));
    }
    if !remaining.is_empty() {
        spans.push(Span::styled(
            remaining,
            Style::default().fg(TIP_FOREGROUND).bg(TIP_BACKGROUND),
        ));
    }
    spans
}

fn render_footer(overview: &Overview, area: Rect, buffer: &mut Buffer) {
    let line = if overview.is_hinting() {
        Line::from(vec![
            Span::styled(
                " FLASH ",
                Style::default().fg(TIP_FOREGROUND).bg(TIP_BACKGROUND),
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
                Style::default().fg(TIP_TYPED).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Esc cancel", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::raw("hjkl/arrows move   "),
            Span::raw("s"),
            Span::raw(" flash   - previous   e/Enter go   q/Esc back"),
        ])
    };
    Paragraph::new(line).render(area, buffer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Overview, TabFact};

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
        assert!(joined.contains("\u{1b}[38;2;189;147;249m"));
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
        assert!(joined.contains("\u{1b}[48;2;139;233;253m"));
        assert!(joined.contains("\u{1b}[38;2;16;19;26m"));
        assert!(joined.contains("\u{1b}[38;2;255;184;108m"));
        assert!(!joined.contains("\u{1b}[2m"));
        assert!(joined.contains('a'));

        let area = Rect::new(0, 0, 40, 11);
        let mut buffer = Buffer::empty(area);
        render_cards(&overview, area, &mut buffer);
        assert_eq!(buffer[(20, 0)].fg, CARD_BORDER);
        assert_eq!(buffer[(20, 0)].bg, Color::Reset);
        assert_eq!(buffer[(21, 1)].bg, Color::Reset);
        assert_eq!(buffer[(24, 5)].bg, Color::Reset);
    }

    #[test]
    fn marks_the_previous_tab_destination() {
        let mut overview = overview();
        overview.set_previous_tab_id(Some(2));
        let joined = paint(&overview, 12, 40).lines.join("\n");
        assert!(joined.contains("[-]"));
    }

    #[test]
    fn narrow_frames_do_not_panic() {
        assert_eq!(paint(&overview(), 1, 1).lines.len(), 1);
        assert_eq!(paint(&overview(), 2, 3).lines.len(), 2);
    }
}
