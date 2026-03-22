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
