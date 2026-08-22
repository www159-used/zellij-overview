use ratatui::layout::Rect;

const FRAMED_CARD_HEIGHT: usize = 3;
const FRAMED_MIN_WIDTH: usize = 6;
const COMPACT_CARD_HEIGHT: usize = 1;
const COMPACT_MIN_WIDTH: usize = 4;
const MAX_ROW_STRETCH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Framed,
    Compact,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardPlacement {
    pub index: usize,
    pub row: usize,
    pub area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPlan {
    pub mode: LayoutMode,
    pub columns: usize,
    pub total_rows: usize,
    pub tab_count: usize,
    pub first_visible: usize,
    pub visible_count: usize,
    pub cards: Vec<CardPlacement>,
    pub separator_y: Option<u16>,
}

impl LayoutPlan {
    pub fn calculate(item_widths: &[usize], area: Rect, scroll_offset: usize) -> Self {
        Self::calculate_with_pins(item_widths, area, scroll_offset, 0)
    }

    pub fn calculate_with_pins(
        item_widths: &[usize],
        area: Rect,
        scroll_offset: usize,
        pin_count: usize,
    ) -> Self {
        if item_widths.is_empty() || area.width == 0 || area.height == 0 {
            return Self::empty();
        }
        if let Some(plan) = flow_plan(
            LayoutMode::Framed,
            item_widths,
            area,
            FRAMED_CARD_HEIGHT,
            FRAMED_MIN_WIDTH,
            2,
            pin_count,
        ) {
            return plan;
        }
        if let Some(plan) = flow_plan(
            LayoutMode::Compact,
            item_widths,
            area,
            COMPACT_CARD_HEIGHT,
            COMPACT_MIN_WIDTH,
            0,
            pin_count,
        ) {
            return plan;
        }
        scroll_plan(item_widths.len(), area, scroll_offset, pin_count)
    }

    pub fn visible_end(&self) -> usize {
        self.first_visible.saturating_add(self.visible_count)
    }

    pub fn horizontal_neighbor(&self, index: usize, direction: isize) -> usize {
        if self.tab_count == 0 {
            return 0;
        }
        let last = self.tab_count.saturating_sub(1) as isize;
        (index as isize + direction).clamp(0, last) as usize
    }

    pub fn vertical_neighbor(&self, index: usize, direction: isize) -> usize {
        if self.tab_count == 0 {
            return 0;
        }
        if self.mode == LayoutMode::Scroll || self.total_rows == 1 {
            return self.horizontal_neighbor(index, direction);
        }
        let Some(current) = self.cards.iter().find(|card| card.index == index) else {
            return index.min(self.tab_count - 1);
        };
        let target_row = current.row as isize + direction;
        if target_row < 0 || target_row >= self.total_rows as isize {
            return index;
        }
        let target_row = target_row as usize;
        let current_center = center_x(current.area);
        self.cards
            .iter()
            .filter(|card| card.row == target_row)
            .min_by_key(|card| center_x(card.area).abs_diff(current_center))
            .map_or(index, |card| card.index)
    }

    fn empty() -> Self {
        Self {
            mode: LayoutMode::Compact,
            columns: 1,
            total_rows: 0,
            tab_count: 0,
            first_visible: 0,
            visible_count: 0,
            cards: Vec::new(),
            separator_y: None,
        }
    }
}

fn flow_plan(
    mode: LayoutMode,
    item_widths: &[usize],
    area: Rect,
    card_height: usize,
    minimum_width: usize,
    frame_width: usize,
    pin_count: usize,
) -> Option<LayoutPlan> {
    let available_width = usize::from(area.width);
    if available_width < minimum_width {
        return None;
    }
    if item_widths
        .iter()
        .any(|width| width.saturating_add(frame_width) > available_width)
    {
        return None;
    }
    let widths: Vec<usize> = item_widths
        .iter()
        .map(|width| width.saturating_add(frame_width).max(minimum_width))
        .collect();
    let split = (pin_count > 0 && pin_count < widths.len()).then_some(pin_count);
    let pin_rows = split
        .map(|count| wrap_rows(&widths[..count], available_width))
        .unwrap_or_default();
    let rest_rows = wrap_rows(
        split.map_or(widths.as_slice(), |count| &widths[count..]),
        available_width,
    );
    let separator = usize::from(split.is_some());
    let pin_height = pin_rows.len() * card_height;
    let rest_height = rest_rows.len() * card_height;
    let grid_height = pin_height + separator + rest_height;
    if grid_height > usize::from(area.height) {
        return None;
    }

    let origin_y = usize::from(area.y) + (usize::from(area.height) - grid_height) / 2;
    let rest_origin_y = origin_y + pin_height + separator;
    let rest_row_base = pin_rows.len();
    let mut cards = Vec::with_capacity(item_widths.len());
    place_rows(
        &pin_rows,
        origin_y,
        0,
        0,
        card_height,
        available_width,
        usize::from(area.x),
        &mut cards,
    );
    place_rows(
        &rest_rows,
        rest_origin_y,
        rest_row_base,
        split.unwrap_or(0),
        card_height,
        available_width,
        usize::from(area.x),
        &mut cards,
    );
    let columns = pin_rows
        .iter()
        .chain(&rest_rows)
        .map(Vec::len)
        .max()
        .unwrap_or(1);

    Some(LayoutPlan {
        mode,
        columns,
        total_rows: pin_rows.len() + rest_rows.len(),
        tab_count: item_widths.len(),
        first_visible: 0,
        visible_count: item_widths.len(),
        cards,
        separator_y: split.map(|_| (origin_y + pin_height) as u16),
    })
}

fn place_rows(
    rows: &[Vec<(usize, usize)>],
    origin_y: usize,
    row_base: usize,
    index_base: usize,
    card_height: usize,
    available_width: usize,
    origin_x: usize,
    cards: &mut Vec<CardPlacement>,
) {
    for (row_index, row) in rows.iter().enumerate() {
        let row_width: usize = row.iter().map(|(_, width)| *width).sum();
        let stretch = ((available_width - row_width) / row.len()).min(MAX_ROW_STRETCH);
        let stretched_width = row_width + stretch * row.len();
        let mut x = origin_x + (available_width - stretched_width) / 2;
        for (index, width) in row {
            let width = width + stretch;
            cards.push(CardPlacement {
                index: index_base + *index,
                row: row_base + row_index,
                area: Rect::new(
                    x as u16,
                    (origin_y + row_index * card_height) as u16,
                    width as u16,
                    card_height as u16,
                ),
            });
            x += width;
        }
    }
}

fn wrap_rows(widths: &[usize], available_width: usize) -> Vec<Vec<(usize, usize)>> {
    let mut rows: Vec<Vec<(usize, usize)>> = vec![Vec::new()];
    let mut used_width = 0;
    for (index, width) in widths.iter().copied().enumerate() {
        if used_width > 0 && used_width + width > available_width {
            rows.push(Vec::new());
            used_width = 0;
        }
        rows.last_mut()
            .expect("flow layout always has a row")
            .push((index, width));
        used_width += width;
    }
    rows
}

fn scroll_plan(tab_count: usize, area: Rect, scroll_offset: usize, pin_count: usize) -> LayoutPlan {
    let first_visible = scroll_offset.min(tab_count.saturating_sub(1));
    let show_separator = pin_count > 0 && pin_count < tab_count;
    let has_hidden_items =
        first_visible > 0 || first_visible.saturating_add(usize::from(area.height)) < tab_count;
    let content_width = area
        .width
        .saturating_sub(u16::from(area.width > 1 && has_hidden_items));
    let mut cards = Vec::new();
    let mut separator_y = None;
    let mut y = area.y;
    let end_y = area.y.saturating_add(area.height);
    let mut index = first_visible;
    while index < tab_count && y < end_y {
        cards.push(CardPlacement {
            index,
            row: index,
            area: Rect::new(area.x, y, content_width, 1),
        });
        y = y.saturating_add(1);
        if show_separator && index + 1 == pin_count && y < end_y {
            separator_y = Some(y);
            y = y.saturating_add(1);
        }
        index += 1;
    }

    LayoutPlan {
        mode: LayoutMode::Scroll,
        columns: 1,
        total_rows: tab_count,
        tab_count,
        first_visible,
        visible_count: cards.len(),
        cards,
        separator_y,
    }
}

fn center_x(area: Rect) -> usize {
    usize::from(area.x) * 2 + usize::from(area.width)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: u16, height: u16) -> Rect {
        Rect::new(0, 0, width, height)
    }

    #[test]
    fn pins_wrap_above_a_separator_band() {
        let plan = LayoutPlan::calculate_with_pins(&[8, 8, 8, 8], rect(40, 10), 0, 2);
        assert_eq!(plan.mode, LayoutMode::Framed);
        let last_pin = plan.cards.iter().find(|card| card.index == 1).unwrap();
        let first_rest = plan.cards.iter().find(|card| card.index == 2).unwrap();
        assert!(last_pin.area.y + last_pin.area.height < first_rest.area.y);
        assert_eq!(
            plan.separator_y,
            Some(last_pin.area.y + last_pin.area.height)
        );
    }

    #[test]
    fn framed_cards_follow_title_widths() {
        let plan = LayoutPlan::calculate(&[4, 18, 7], rect(50, 6), 0);
        assert_eq!(plan.mode, LayoutMode::Framed);
        assert!(plan.cards[0].area.width < plan.cards[1].area.width);
        assert!(plan.cards[2].area.width < plan.cards[1].area.width);
        assert!(plan
            .cards
            .iter()
            .all(|card| card.area.height == FRAMED_CARD_HEIGHT as u16));
    }

    #[test]
    fn flow_wraps_in_tab_order() {
        let plan = LayoutPlan::calculate(&[8, 14, 8, 14], rect(30, 7), 0);
        assert_eq!(plan.mode, LayoutMode::Framed);
        assert_eq!(plan.total_rows, 2);
        assert_eq!(plan.cards[0].row, 0);
        assert_eq!(plan.cards[1].row, 0);
        assert_eq!(plan.cards[2].row, 1);
    }

    #[test]
    fn medium_viewport_drops_frames() {
        let plan = LayoutPlan::calculate(&[8; 12], rect(30, 4), 0);
        assert_eq!(plan.mode, LayoutMode::Compact);
        assert_eq!(plan.visible_count, 12);
    }

    #[test]
    fn tiny_viewport_scrolls_a_readable_single_column() {
        let plan = LayoutPlan::calculate(&[12; 20], rect(12, 4), 7);
        assert_eq!(plan.mode, LayoutMode::Scroll);
        assert_eq!(plan.first_visible, 7);
        assert_eq!(plan.visible_count, 4);
        assert_eq!(plan.cards[0].index, 7);
        assert_eq!(plan.cards[3].index, 10);
        assert_eq!(plan.cards[0].area.width, 11);
    }

    #[test]
    fn title_wider_than_viewport_uses_scroll_without_clipping_a_frame() {
        let plan = LayoutPlan::calculate(&[30], rect(12, 20), 0);
        assert_eq!(plan.mode, LayoutMode::Scroll);
        assert_eq!(plan.cards[0].area.width, 12);
    }

    #[test]
    fn vertical_navigation_uses_nearest_horizontal_center() {
        let plan = LayoutPlan::calculate(&[6, 18, 6, 12], rect(30, 7), 0);
        assert_eq!(plan.total_rows, 2);
        assert_eq!(plan.vertical_neighbor(1, 1), 3);
        assert_eq!(plan.vertical_neighbor(2, -1), 0);
    }

    #[test]
    fn neighbors_stop_at_both_ends() {
        let plan = LayoutPlan::calculate(&[12; 20], rect(12, 4), 0);
        assert_eq!(plan.mode, LayoutMode::Scroll);
        assert_eq!(plan.horizontal_neighbor(0, -1), 0);
        assert_eq!(plan.vertical_neighbor(0, -1), 0);
        assert_eq!(plan.horizontal_neighbor(19, 1), 19);
        assert_eq!(plan.vertical_neighbor(19, 1), 19);

        let flow = LayoutPlan::calculate(&[6, 18, 6, 12], rect(30, 7), 0);
        assert_eq!(flow.total_rows, 2);
        assert_eq!(flow.vertical_neighbor(1, -1), 1);
        assert_eq!(flow.vertical_neighbor(3, 1), 3);
        assert_eq!(flow.horizontal_neighbor(0, -1), 0);
        assert_eq!(flow.horizontal_neighbor(3, 1), 3);
    }
}
