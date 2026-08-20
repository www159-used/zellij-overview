/// Column count for the tab card grid.
///
/// 1–4 tabs stay on 2 columns so a left/right split remains readable.
/// 5–9 use 3; 10+ use 4. Never 5 — mosaics collapse into unreadable stripes.
pub fn columns(tab_count: usize) -> usize {
    match tab_count {
        0 => 1,
        1..=4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

pub fn rows(tab_count: usize) -> usize {
    let cols = columns(tab_count);
    tab_count.div_ceil(cols)
}

/// Index of the card at `(row, col)`, if that cell is occupied.
pub fn index_at(row: usize, col: usize, tab_count: usize) -> Option<usize> {
    let cols = columns(tab_count);
    let index = row.saturating_mul(cols).saturating_add(col);
    if index < tab_count {
        Some(index)
    } else {
        None
    }
}

/// Move the cursor on the grid. Empty cells are skipped by landing on the
/// last occupied cell of that row, or wrapping to the other side.
pub fn step(cursor: usize, tab_count: usize, drow: isize, dcol: isize) -> usize {
    if tab_count == 0 {
        return 0;
    }
    let cols = columns(tab_count) as isize;
    let rows = rows(tab_count) as isize;
    let cursor = cursor.min(tab_count - 1);
    let row = (cursor as isize) / cols;
    let col = (cursor as isize) % cols;

    if dcol != 0 {
        let next = cursor as isize + dcol;
        return next.rem_euclid(tab_count as isize) as usize;
    }
    if rows == 1 {
        let next = cursor as isize + drow;
        return next.rem_euclid(tab_count as isize) as usize;
    }

    let mut new_row = (row + drow).rem_euclid(rows);
    let new_col = col;
    if let Some(index) = index_at(new_row as usize, new_col as usize, tab_count) {
        return index;
    }
    // empty cell: last occupied in this row, else keep wrapping rows
    for _ in 0..rows {
        let start = (new_row as usize) * (cols as usize);
        if start < tab_count {
            return tab_count - 1;
        }
        new_row = (new_row + drow).rem_euclid(rows);
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_policy() {
        assert_eq!(columns(1), 2);
        assert_eq!(columns(4), 2);
        assert_eq!(columns(5), 3);
        assert_eq!(columns(9), 3);
        assert_eq!(columns(10), 4);
        assert_eq!(columns(20), 4);
    }

    #[test]
    fn twelve_tabs_are_four_by_three() {
        assert_eq!(columns(12), 4);
        assert_eq!(rows(12), 3);
    }

    #[test]
    fn left_right_wrap_in_reading_order() {
        assert_eq!(step(0, 5, 0, -1), 4);
        assert_eq!(step(4, 5, 0, 1), 0);
        assert_eq!(step(2, 5, 0, 1), 3);
    }

    #[test]
    fn up_down_move_in_a_single_row() {
        assert_eq!(step(0, 2, 1, 0), 1);
        assert_eq!(step(1, 2, 1, 0), 0);
        assert_eq!(step(0, 2, -1, 0), 1);
    }

    #[test]
    fn down_from_last_row_wraps() {
        // 5 tabs, 3 cols:
        // 0 1 2
        // 3 4 _
        assert_eq!(step(0, 5, 1, 0), 3);
        assert_eq!(step(1, 5, 1, 0), 4);
        assert_eq!(step(2, 5, 1, 0), 4); // empty → last in that row
        assert_eq!(step(3, 5, 1, 0), 0);
    }
}
