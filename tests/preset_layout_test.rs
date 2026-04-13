use tiler::model::Rect;
use tiler::tiling::preset::{apply_fullscreen, apply_side_by_side, apply_top_bottom, apply_quadrants};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn monitor() -> Rect {
    Rect { x: 0, y: 0, width: 1920, height: 1080 }
}

// ===========================================================================
// Fullscreen
// ===========================================================================

#[test]
fn test_fullscreen_single_window() {
    // Arrange
    let ids = vec![1];
    let m = monitor();

    // Act
    let result = apply_fullscreen(&ids, m);

    // Assert
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 1);
    assert_eq!(result[0].1, m, "single window fills monitor");
}

#[test]
fn test_fullscreen_empty() {
    // Arrange / Act
    let result = apply_fullscreen(&[], monitor());

    // Assert
    assert!(result.is_empty());
}

#[test]
fn test_fullscreen_excess_windows() {
    // Arrange — preset has 1 slot, but 3 windows given
    let ids = vec![10, 20, 30];

    // Act
    let result = apply_fullscreen(&ids, monitor());

    // Assert — only first window gets positioned
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 10);
}

// ===========================================================================
// SideBySide
// ===========================================================================

#[test]
fn test_side_by_side_two_windows() {
    // Arrange
    let ids = vec![1, 2];
    let m = monitor();

    // Act
    let result = apply_side_by_side(&ids, m);

    // Assert
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, 1);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 1080 });
    assert_eq!(result[1].0, 2);
    assert_eq!(result[1].1, Rect { x: 960, y: 0, width: 960, height: 1080 });
}

#[test]
fn test_side_by_side_single_window() {
    // Arrange — fewer windows than slots
    let ids = vec![5];
    let m = monitor();

    // Act
    let result = apply_side_by_side(&ids, m);

    // Assert — only 1 window positioned (left half)
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 5);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 1080 });
}

#[test]
fn test_side_by_side_empty() {
    let result = apply_side_by_side(&[], monitor());
    assert!(result.is_empty());
}

#[test]
fn test_side_by_side_excess_windows() {
    // Arrange — 2 slots, 4 windows
    let ids = vec![1, 2, 3, 4];

    // Act
    let result = apply_side_by_side(&ids, monitor());

    // Assert — only first 2 get positioned
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, 1);
    assert_eq!(result[1].0, 2);
}

// ===========================================================================
// TopBottom
// ===========================================================================

#[test]
fn test_top_bottom_two_windows() {
    // Arrange
    let ids = vec![1, 2];
    let m = monitor();

    // Act
    let result = apply_top_bottom(&ids, m);

    // Assert
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, 1);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 1920, height: 540 });
    assert_eq!(result[1].0, 2);
    assert_eq!(result[1].1, Rect { x: 0, y: 540, width: 1920, height: 540 });
}

#[test]
fn test_top_bottom_single_window() {
    let ids = vec![7];
    let m = monitor();

    let result = apply_top_bottom(&ids, m);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 7);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 1920, height: 540 });
}

#[test]
fn test_top_bottom_empty() {
    let result = apply_top_bottom(&[], monitor());
    assert!(result.is_empty());
}

#[test]
fn test_top_bottom_excess_windows() {
    let ids = vec![1, 2, 3];
    let result = apply_top_bottom(&ids, monitor());
    assert_eq!(result.len(), 2);
}

// ===========================================================================
// Quadrants
// ===========================================================================

#[test]
fn test_quadrants_four_windows() {
    // Arrange
    let ids = vec![1, 2, 3, 4];
    let m = monitor();

    // Act
    let result = apply_quadrants(&ids, m);

    // Assert — 2x2 grid
    assert_eq!(result.len(), 4);

    // Top-left
    assert_eq!(result[0].0, 1);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 540 });

    // Top-right
    assert_eq!(result[1].0, 2);
    assert_eq!(result[1].1, Rect { x: 960, y: 0, width: 960, height: 540 });

    // Bottom-left
    assert_eq!(result[2].0, 3);
    assert_eq!(result[2].1, Rect { x: 0, y: 540, width: 960, height: 540 });

    // Bottom-right
    assert_eq!(result[3].0, 4);
    assert_eq!(result[3].1, Rect { x: 960, y: 540, width: 960, height: 540 });
}

#[test]
fn test_quadrants_two_windows() {
    // Arrange — fewer than 4
    let ids = vec![1, 2];
    let m = monitor();

    // Act
    let result = apply_quadrants(&ids, m);

    // Assert — only first 2 slots filled (top-left, top-right)
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 540 });
    assert_eq!(result[1].1, Rect { x: 960, y: 0, width: 960, height: 540 });
}

#[test]
fn test_quadrants_single_window() {
    let ids = vec![1];
    let result = apply_quadrants(&ids, monitor());
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 540 });
}

#[test]
fn test_quadrants_empty() {
    let result = apply_quadrants(&[], monitor());
    assert!(result.is_empty());
}

#[test]
fn test_quadrants_excess_windows() {
    let ids = vec![1, 2, 3, 4, 5, 6];
    let result = apply_quadrants(&ids, monitor());
    assert_eq!(result.len(), 4, "only 4 slots available");
}

#[test]
fn test_quadrants_three_windows() {
    // Arrange — 3 windows, 4 slots: top-left, top-right, bottom-left filled
    let ids = vec![1, 2, 3];
    let m = monitor();

    let result = apply_quadrants(&ids, m);

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 960, height: 540 });
    assert_eq!(result[1].1, Rect { x: 960, y: 0, width: 960, height: 540 });
    assert_eq!(result[2].1, Rect { x: 0, y: 540, width: 960, height: 540 });
}

// ===========================================================================
// Screen offset
// ===========================================================================

#[test]
fn test_preset_with_screen_offset() {
    // Arrange — monitor not at origin
    let m = Rect { x: 1920, y: 0, width: 1920, height: 1080 };
    let ids = vec![1, 2];

    // Act
    let result = apply_side_by_side(&ids, m);

    // Assert — positions offset by monitor x
    assert_eq!(result[0].1, Rect { x: 1920, y: 0, width: 960, height: 1080 });
    assert_eq!(result[1].1, Rect { x: 2880, y: 0, width: 960, height: 1080 });
}

// ===========================================================================
// Pixel-rounding: odd dimensions must not leave gaps
// ===========================================================================

#[test]
fn test_side_by_side_odd_width_no_gap() {
    // Arrange — odd width: 1441 / 2 = 720 remainder 1
    let ids = vec![1, 2];
    let m = Rect { x: 0, y: 0, width: 1441, height: 1080 };

    // Act
    let result = apply_side_by_side(&ids, m);

    // Assert — last slot absorbs the extra pixel
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].1.width, 720, "left half: 1441 / 2 = 720");
    assert_eq!(result[1].1.width, 721, "right half absorbs remainder: 1441 - 720 = 721");
    assert_eq!(result[1].1.x, 720, "right slot starts at left_w");
    assert_eq!(
        result[0].1.width + result[1].1.width,
        1441,
        "total width must equal monitor width"
    );
}

#[test]
fn test_top_bottom_odd_height_no_gap() {
    // Arrange — odd height: 1081 / 2 = 540 remainder 1
    let ids = vec![1, 2];
    let m = Rect { x: 0, y: 0, width: 1920, height: 1081 };

    // Act
    let result = apply_top_bottom(&ids, m);

    // Assert — last slot absorbs the extra pixel
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].1.height, 540, "top half: 1081 / 2 = 540");
    assert_eq!(result[1].1.height, 541, "bottom half absorbs remainder: 1081 - 540 = 541");
    assert_eq!(result[1].1.y, 540, "bottom slot starts at top_h");
    assert_eq!(
        result[0].1.height + result[1].1.height,
        1081,
        "total height must equal monitor height"
    );
}

#[test]
fn test_quadrants_odd_dimensions_no_gap() {
    // Arrange — both dimensions odd: 1441x1081
    let ids = vec![1, 2, 3, 4];
    let m = Rect { x: 0, y: 0, width: 1441, height: 1081 };

    // Act
    let result = apply_quadrants(&ids, m);

    // Assert — right column and bottom row absorb remainders
    assert_eq!(result.len(), 4);

    // Top-left
    assert_eq!(result[0].1, Rect { x: 0, y: 0, width: 720, height: 540 });
    // Top-right (extra pixel in width)
    assert_eq!(result[1].1, Rect { x: 720, y: 0, width: 721, height: 540 });
    // Bottom-left (extra pixel in height)
    assert_eq!(result[2].1, Rect { x: 0, y: 540, width: 720, height: 541 });
    // Bottom-right (extra pixel in both)
    assert_eq!(result[3].1, Rect { x: 720, y: 540, width: 721, height: 541 });

    // Coverage invariants
    assert_eq!(
        result[0].1.width + result[1].1.width,
        1441,
        "top row width must equal monitor width"
    );
    assert_eq!(
        result[0].1.height + result[2].1.height,
        1081,
        "left column height must equal monitor height"
    );
}

// ===========================================================================
// Coverage invariant for all presets with offset monitors
// ===========================================================================

#[test]
fn test_all_presets_cover_monitor_with_offset() {
    let m = Rect { x: 1920, y: 100, width: 2560, height: 1440 };

    // Fullscreen
    let result = apply_fullscreen(&[1], m);
    assert_eq!(result[0].1, m, "fullscreen covers entire monitor");

    // SideBySide
    let result = apply_side_by_side(&[1, 2], m);
    assert_eq!(
        result[0].1.width + result[1].1.width,
        m.width,
        "side-by-side covers full width"
    );
    assert_eq!(result[0].1.x, m.x);
    assert_eq!(result[1].1.x, m.x + result[0].1.width);

    // TopBottom
    let result = apply_top_bottom(&[1, 2], m);
    assert_eq!(
        result[0].1.height + result[1].1.height,
        m.height,
        "top-bottom covers full height"
    );
    assert_eq!(result[0].1.y, m.y);
    assert_eq!(result[1].1.y, m.y + result[0].1.height);

    // Quadrants
    let result = apply_quadrants(&[1, 2, 3, 4], m);
    let total_area: i64 = result
        .iter()
        .map(|(_, r)| r.width as i64 * r.height as i64)
        .sum();
    assert_eq!(
        total_area,
        m.width as i64 * m.height as i64,
        "quadrants cover full area"
    );
}

// ===========================================================================
// Presets with width=1 and height=1 (minimum valid dimensions)
// ===========================================================================

#[test]
fn test_side_by_side_width_1() {
    let m = Rect { x: 0, y: 0, width: 1, height: 100 };
    let result = apply_side_by_side(&[1, 2], m);
    // 1/2 = 0, so left window gets width 0 and right gets width 1
    // This is a degenerate case but should not panic
    assert_eq!(result.len(), 2);
    assert_eq!(
        result[0].1.width + result[1].1.width,
        1,
        "total width must equal monitor width"
    );
}

#[test]
fn test_top_bottom_height_1() {
    let m = Rect { x: 0, y: 0, width: 100, height: 1 };
    let result = apply_top_bottom(&[1, 2], m);
    assert_eq!(result.len(), 2);
    assert_eq!(
        result[0].1.height + result[1].1.height,
        1,
        "total height must equal monitor height"
    );
}

#[test]
fn test_quadrants_width_1_height_1() {
    let m = Rect { x: 0, y: 0, width: 1, height: 1 };
    let result = apply_quadrants(&[1, 2, 3, 4], m);
    assert_eq!(result.len(), 4);
    // Degenerate but should not panic
}
