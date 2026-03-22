use crate::model::Rect;

const COLUMN_CAPACITY: usize = 5;

/// Compute tiled positions for a stack of windows.
///
/// Windows are laid out in columns of up to 5, left to right.
/// The input order is preserved: `window_ids[0]` lands at the top of the first column.
pub fn stack_layout(window_ids: &[u64], screen: Rect) -> Vec<(u64, Rect)> {
    let n = window_ids.len();
    if n == 0 {
        return Vec::new();
    }

    let num_columns = (n + COLUMN_CAPACITY - 1) / COLUMN_CAPACITY;
    let col_width = screen.width / num_columns as i32;

    let mut result = Vec::with_capacity(n);

    for (i, &id) in window_ids.iter().enumerate() {
        let col = i / COLUMN_CAPACITY;
        let row = i % COLUMN_CAPACITY;

        // Count how many windows are in this column
        let col_start = col * COLUMN_CAPACITY;
        let col_count = (n - col_start).min(COLUMN_CAPACITY);
        let row_height = screen.height / col_count as i32;

        let w = if col == num_columns - 1 {
            screen.width - col_width * (num_columns - 1) as i32
        } else {
            col_width
        };
        let h = if row == col_count - 1 {
            screen.height - row_height * (col_count - 1) as i32
        } else {
            row_height
        };

        let rect = Rect {
            x: screen.x + col_width * col as i32,
            y: screen.y + row_height * row as i32,
            width: w,
            height: h,
        };

        result.push((id, rect));
    }

    result
}
