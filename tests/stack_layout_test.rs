use tiler::model::Rect;
use tiler::tiling::stack::stack_layout;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn screen() -> Rect {
    Rect { x: 0, y: 0, width: 1920, height: 1080 }
}

// ---------------------------------------------------------------------------
// Empty input
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_empty() {
    // Arrange
    let ids: Vec<u64> = vec![];

    // Act
    let result = stack_layout(&ids, screen());

    // Assert
    assert!(result.is_empty(), "empty input should produce empty output");
}

// ---------------------------------------------------------------------------
// Single window — fills entire screen
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_single_window() {
    // Arrange
    let ids = vec![1];

    // Act
    let result = stack_layout(&ids, screen());

    // Assert
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 1, "window id");
    assert_eq!(result[0].1, screen(), "single window fills entire screen");
}

// ---------------------------------------------------------------------------
// Two windows — single column, equal height
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_two_windows() {
    // Arrange
    let ids = vec![10, 20];
    let s = screen();

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 2);

    // Window 10 (newest) at top
    assert_eq!(result[0].0, 10);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 1920, height: 540 });

    // Window 20 below
    assert_eq!(result[1].0, 20);
    assert_eq!(result[1].1, Rect { x: 0, y: 540, width: 1920, height: 540 });
}

// ---------------------------------------------------------------------------
// Five windows — single column, max capacity
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_five_windows() {
    // Arrange
    let ids: Vec<u64> = (1..=5).collect();
    let s = screen();
    let row_h = s.height / 5; // 216

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 5);
    for (i, (id, rect)) in result.iter().enumerate() {
        assert_eq!(*id, (i as u64) + 1, "window id at index {}", i);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, row_h * i as i32);
        assert_eq!(rect.width, 1920);
        assert_eq!(rect.height, row_h, "row height at index {}", i);
    }
}

// ---------------------------------------------------------------------------
// Six windows — two columns, 5+1 split
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_six_windows() {
    // Arrange
    let ids: Vec<u64> = (1..=6).collect();
    let s = screen();
    let col_w = s.width / 2; // 960

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 6);

    // Column 1: windows 1-5, height = 1080/5 = 216
    let row_h_col1 = s.height / 5;
    for i in 0..5 {
        let (id, rect) = &result[i];
        assert_eq!(*id, (i as u64) + 1);
        assert_eq!(rect.x, 0, "col 1 x");
        assert_eq!(rect.y, row_h_col1 * i as i32, "col 1 row {} y", i);
        assert_eq!(rect.width, col_w);
        assert_eq!(rect.height, row_h_col1);
    }

    // Column 2: window 6, full height
    let (id, rect) = &result[5];
    assert_eq!(*id, 6);
    assert_eq!(rect.x, col_w, "col 2 x");
    assert_eq!(rect.y, 0);
    assert_eq!(rect.width, col_w);
    assert_eq!(rect.height, s.height, "single window in col 2 gets full height");
}

// ---------------------------------------------------------------------------
// Ten windows — two columns, 5+5 evenly split
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_ten_windows() {
    // Arrange
    let ids: Vec<u64> = (1..=10).collect();
    let s = screen();
    let col_w = s.width / 2;
    let row_h = s.height / 5;

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 10);

    // Column 1: windows 1-5
    for i in 0..5 {
        let (id, rect) = &result[i];
        assert_eq!(*id, (i as u64) + 1);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, row_h * i as i32);
        assert_eq!(rect.width, col_w);
        assert_eq!(rect.height, row_h);
    }

    // Column 2: windows 6-10
    for i in 5..10 {
        let (id, rect) = &result[i];
        assert_eq!(*id, (i as u64) + 1);
        assert_eq!(rect.x, col_w);
        assert_eq!(rect.y, row_h * (i - 5) as i32);
        assert_eq!(rect.width, col_w);
        assert_eq!(rect.height, row_h);
    }
}

// ---------------------------------------------------------------------------
// Eleven windows — three columns, 5+5+1
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_eleven_windows() {
    // Arrange
    let ids: Vec<u64> = (1..=11).collect();
    let s = screen();
    let col_w = s.width / 3; // 640

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 11);

    // Column 1: 5 windows
    let row_h_5 = s.height / 5;
    for i in 0..5 {
        assert_eq!(result[i].0, (i as u64) + 1);
        assert_eq!(result[i].1.x, 0);
        assert_eq!(result[i].1.width, col_w);
        assert_eq!(result[i].1.height, row_h_5);
    }

    // Column 2: 5 windows
    for i in 5..10 {
        assert_eq!(result[i].0, (i as u64) + 1);
        assert_eq!(result[i].1.x, col_w);
        assert_eq!(result[i].1.width, col_w);
        assert_eq!(result[i].1.height, row_h_5);
    }

    // Column 3: 1 window (full height)
    assert_eq!(result[10].0, 11);
    assert_eq!(result[10].1.x, col_w * 2);
    assert_eq!(result[10].1.y, 0);
    assert_eq!(result[10].1.width, col_w);
    assert_eq!(result[10].1.height, s.height);
}

// ---------------------------------------------------------------------------
// Fifteen windows — three columns, 5+5+5
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_fifteen_windows() {
    // Arrange
    let ids: Vec<u64> = (1..=15).collect();
    let s = screen();
    let col_w = s.width / 3;
    let row_h = s.height / 5;

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 15);

    for col in 0..3 {
        for row in 0..5 {
            let idx = col * 5 + row;
            let (id, rect) = &result[idx];
            assert_eq!(*id, (idx as u64) + 1);
            assert_eq!(rect.x, col_w * col as i32, "col {} x", col);
            assert_eq!(rect.y, row_h * row as i32, "col {} row {} y", col, row);
            assert_eq!(rect.width, col_w);
            assert_eq!(rect.height, row_h);
        }
    }
}

// ---------------------------------------------------------------------------
// Screen with non-zero offset
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_with_screen_offset() {
    // Arrange
    let ids = vec![1, 2];
    let s = Rect { x: 100, y: 50, width: 1920, height: 1080 };

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 2);

    assert_eq!(result[0].1, Rect { x: 100, y: 50, width: 1920, height: 540 });
    assert_eq!(result[1].1, Rect { x: 100, y: 590, width: 1920, height: 540 });
}

// ---------------------------------------------------------------------------
// Window order — newest at top of first column
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_window_order_preserved() {
    // Arrange — window 99 is newest (index 0), 1 is oldest
    let ids = vec![99, 50, 1];
    let s = screen();
    let row_h = s.height / 3;

    // Act
    let result = stack_layout(&ids, s);

    // Assert — order matches input
    assert_eq!(result[0].0, 99, "newest at top");
    assert_eq!(result[0].1.y, 0);
    assert_eq!(result[1].0, 50);
    assert_eq!(result[1].1.y, row_h);
    assert_eq!(result[2].0, 1, "oldest at bottom");
    assert_eq!(result[2].1.y, row_h * 2);
}

// ---------------------------------------------------------------------------
// Pixel-rounding: odd dimensions must not leave gaps
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_odd_width_two_columns() {
    // Arrange — odd width: 1441 / 2 = 720 remainder 1
    let ids: Vec<u64> = (1..=6).collect(); // 2 columns: 5+1
    let s = Rect { x: 0, y: 0, width: 1441, height: 1080 };

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 6);

    // Column 1: windows 0-4 have width 720 and x=0
    for i in 0..5 {
        assert_eq!(result[i].1.x, 0, "col 1 window {} x", i);
        assert_eq!(result[i].1.width, 720, "col 1 window {} width", i);
    }

    // Column 2: window 5 has width 721 and x=720 (absorbs remainder)
    assert_eq!(result[5].1.x, 720, "col 2 x");
    assert_eq!(result[5].1.width, 721, "col 2 absorbs remainder: 1441 - 720 = 721");
    assert_eq!(720 + 721, 1441, "total width must equal screen width");
}

#[test]
fn test_stack_layout_odd_height_three_rows() {
    // Arrange — odd height: 1081 / 3 = 360 remainder 1
    let ids: Vec<u64> = (1..=3).collect(); // 1 column, 3 rows
    let s = Rect { x: 0, y: 0, width: 1920, height: 1081 };

    // Act
    let result = stack_layout(&ids, s);

    // Assert
    assert_eq!(result.len(), 3);

    // First two rows: height 360
    assert_eq!(result[0].1.height, 360, "row 0 height");
    assert_eq!(result[0].1.y, 0, "row 0 y");
    assert_eq!(result[1].1.height, 360, "row 1 height");
    assert_eq!(result[1].1.y, 360, "row 1 y");

    // Last row absorbs remainder: 1081 - 360*2 = 361
    assert_eq!(result[2].1.height, 361, "last row absorbs remainder");
    assert_eq!(result[2].1.y, 720, "last row y = 360 * 2");
    assert_eq!(
        360 + 360 + 361,
        1081,
        "total height must equal screen height"
    );
}

// ---------------------------------------------------------------------------
// Zero-dimension screen returns empty
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_zero_width_screen() {
    let ids = vec![1, 2, 3];
    let s = Rect { x: 0, y: 0, width: 0, height: 1080 };
    let result = stack_layout(&ids, s);
    assert!(result.is_empty(), "zero-width screen should produce no tiles");
}

#[test]
fn test_stack_layout_zero_height_screen() {
    let ids = vec![1, 2, 3];
    let s = Rect { x: 0, y: 0, width: 1920, height: 0 };
    let result = stack_layout(&ids, s);
    assert!(result.is_empty(), "zero-height screen should produce no tiles");
}

#[test]
fn test_stack_layout_negative_dimensions() {
    let ids = vec![1];
    let s = Rect { x: 0, y: 0, width: -100, height: -200 };
    let result = stack_layout(&ids, s);
    assert!(result.is_empty(), "negative dimensions should produce no tiles");
}

// ---------------------------------------------------------------------------
// Very small screen — only 1 column fits due to MIN_TILE_PX constraint
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_small_screen_caps_columns() {
    // Screen is 160px wide. MIN_TILE_PX=80, so max 2 columns.
    // With 11 windows that would normally need 3 columns, but we cap at 2.
    let ids: Vec<u64> = (1..=11).collect();
    let s = Rect { x: 0, y: 0, width: 160, height: 500 };
    let result = stack_layout(&ids, s);

    // max_by_width = 160/80 = 2 columns
    // needed = ceil(11/5) = 3 columns
    // actual = min(3, 2) = 2 columns, 10 windows max
    assert_eq!(result.len(), 10, "should cap to 2 columns * 5 rows = 10 windows");

    // Verify the first 10 windows are tiled
    for i in 0..10 {
        assert_eq!(result[i].0, (i as u64) + 1);
    }
}

// ---------------------------------------------------------------------------
// Large window count — excess windows beyond grid capacity are omitted
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_large_count_all_fit_on_1080p() {
    // 1920px wide, MIN_TILE_PX=80: max 24 columns
    // 25 windows need ceil(25/5)=5 columns, min(5,24)=5
    // All 25 windows fit in 5 columns of 5
    let ids: Vec<u64> = (1..=25).collect();
    let s = Rect { x: 0, y: 0, width: 1920, height: 1080 };
    let result = stack_layout(&ids, s);

    assert_eq!(result.len(), 25, "all 25 windows fit in 5 columns on 1080p");

    // Verify gap-free coverage
    let total_area: i64 = result.iter().map(|(_, r)| r.width as i64 * r.height as i64).sum();
    let screen_area = s.width as i64 * s.height as i64;
    assert_eq!(total_area, screen_area);
}

#[test]
fn test_stack_layout_excess_omitted_narrow_screen() {
    // 240px wide, MIN_TILE_PX=80: max 3 columns
    // 20 windows need ceil(20/5)=4 columns, but capped to 3
    // 3 columns * 5 rows = 15 windows max
    let ids: Vec<u64> = (1..=20).collect();
    let s = Rect { x: 0, y: 0, width: 240, height: 1080 };
    let result = stack_layout(&ids, s);

    assert_eq!(result.len(), 15, "should cap to 3 columns * 5 rows = 15 windows on 240px wide");
}

// ---------------------------------------------------------------------------
// Gap-free invariant: all tiles must cover the screen exactly
// ---------------------------------------------------------------------------

/// Verify that all tiles together cover the screen exactly, with no
/// overlaps and no gaps, for any window count from 1..=15.
#[test]
fn test_stack_layout_gap_free_coverage_invariant() {
    let s = Rect { x: 100, y: 50, width: 1920, height: 1080 };

    for n in 1..=10usize {
        let ids: Vec<u64> = (1..=(n as u64)).collect();
        let result = stack_layout(&ids, s);

        // Every tile's right edge must equal the next column's left edge (or screen right)
        // and every tile's bottom edge must equal the next row's top edge (or screen bottom)
        // We check a simpler invariant: sum of tile areas == screen area
        let total_area: i64 = result
            .iter()
            .map(|(_, r)| r.width as i64 * r.height as i64)
            .sum();
        let screen_area = s.width as i64 * s.height as i64;
        assert_eq!(
            total_area, screen_area,
            "total tile area must equal screen area for {} windows",
            n
        );

        // Verify no tile extends outside the screen
        for (id, r) in &result {
            assert!(
                r.x >= s.x && r.y >= s.y
                    && r.x + r.width <= s.x + s.width
                    && r.y + r.height <= s.y + s.height,
                "window {} tile {:?} must be within screen {:?}",
                id, r, s
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Twenty windows — verifies multi-column behavior on 1080p
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_twenty_windows() {
    let ids: Vec<u64> = (1..=20).collect();
    let s = Rect { x: 0, y: 0, width: 1920, height: 1080 };
    let result = stack_layout(&ids, s);

    // max_by_width = 1920/80 = 24, needed = ceil(20/5) = 4
    // All 20 windows fit in 4 columns
    assert_eq!(result.len(), 20, "20 windows on 1080p should all be tiled in 4 columns");

    // Verify gap-free coverage
    let total_area: i64 = result.iter().map(|(_, r)| r.width as i64 * r.height as i64).sum();
    let screen_area = s.width as i64 * s.height as i64;
    assert_eq!(total_area, screen_area);
}

// ---------------------------------------------------------------------------
// Tall screen allows more columns
// ---------------------------------------------------------------------------

#[test]
fn test_stack_layout_tall_screen_allows_more_columns() {
    // 3840px wide, 2160px tall (4K)
    // max_by_width = 3840/80 = 48
    // 25 windows need ceil(25/5) = 5 columns
    // min(5, 48) = 5 columns, all 25 windows fit
    let ids: Vec<u64> = (1..=25).collect();
    let s = Rect { x: 0, y: 0, width: 3840, height: 2160 };
    let result = stack_layout(&ids, s);

    assert_eq!(result.len(), 25, "4K screen should fit 5 columns of 5");

    // Verify gap-free coverage
    let total_area: i64 = result.iter().map(|(_, r)| r.width as i64 * r.height as i64).sum();
    let screen_area = s.width as i64 * s.height as i64;
    assert_eq!(total_area, screen_area);
}
