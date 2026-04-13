use crate::model::Rect;

const COLUMN_CAPACITY: usize = 5;

/// Minimum tile dimension in pixels. Tiles smaller than this are not usable
/// on any display, so we cap the number of columns to avoid producing them.
const MIN_TILE_PX: i32 = 80;

/// Compute tiled positions for a stack of windows.
///
/// Windows are laid out in columns of up to 5, left to right.
/// The input order is preserved: `window_ids[0]` lands at the top of the first column.
///
/// If the screen dimensions are zero or negative, an empty vec is returned.
/// The number of columns is capped so that every tile is at least
/// [`MIN_TILE_PX`] wide and tall; excess windows beyond the grid capacity
/// are omitted from the result (the caller is responsible for stashing them).
#[must_use]
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub fn stack_layout(window_ids: &[u64], screen: Rect) -> Vec<(u64, Rect)> {
    let n = window_ids.len();
    if n == 0 || screen.width <= 0 || screen.height <= 0 {
        return Vec::new();
    }

    let num_columns = {
        let needed = n.div_ceil(COLUMN_CAPACITY);
        // Cap columns so each column is at least MIN_TILE_PX wide.
        // Row height depends on how many windows land in each column
        // (at most COLUMN_CAPACITY), not on the column count, so it
        // does not constrain the number of columns.
        let max_by_width = (screen.width / MIN_TILE_PX).max(1) as usize;
        needed.min(max_by_width).max(1)
    };
    let max_windows = num_columns * COLUMN_CAPACITY;
    let effective_n = n.min(max_windows);
    let col_width = screen.width / num_columns as i32;

    let mut result = Vec::with_capacity(effective_n);

    for (i, &id) in window_ids.iter().take(effective_n).enumerate() {
        let col = i / COLUMN_CAPACITY;
        let row = i % COLUMN_CAPACITY;

        // Count how many windows are in this column
        let col_start = col * COLUMN_CAPACITY;
        let col_count = (effective_n - col_start).min(COLUMN_CAPACITY);
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
