use crate::model::Rect;

/// Fullscreen: 1 slot, window fills the entire monitor.
pub fn apply_fullscreen(window_ids: &[u64], monitor: Rect) -> Vec<(u64, Rect)> {
    window_ids
        .iter()
        .take(1)
        .map(|&id| (id, monitor))
        .collect()
}

/// SideBySide: 2 slots, left and right halves.
pub fn apply_side_by_side(window_ids: &[u64], monitor: Rect) -> Vec<(u64, Rect)> {
    let half_w = monitor.width / 2;
    let slots = [
        Rect { x: monitor.x, y: monitor.y, width: half_w, height: monitor.height },
        Rect { x: monitor.x + half_w, y: monitor.y, width: half_w, height: monitor.height },
    ];
    window_ids
        .iter()
        .zip(slots.iter())
        .map(|(&id, &rect)| (id, rect))
        .collect()
}

/// TopBottom: 2 slots, top and bottom halves.
pub fn apply_top_bottom(window_ids: &[u64], monitor: Rect) -> Vec<(u64, Rect)> {
    let half_h = monitor.height / 2;
    let slots = [
        Rect { x: monitor.x, y: monitor.y, width: monitor.width, height: half_h },
        Rect { x: monitor.x, y: monitor.y + half_h, width: monitor.width, height: half_h },
    ];
    window_ids
        .iter()
        .zip(slots.iter())
        .map(|(&id, &rect)| (id, rect))
        .collect()
}

/// Quadrants: 4 slots in a 2x2 grid (top-left, top-right, bottom-left, bottom-right).
pub fn apply_quadrants(window_ids: &[u64], monitor: Rect) -> Vec<(u64, Rect)> {
    let half_w = monitor.width / 2;
    let half_h = monitor.height / 2;
    let slots = [
        Rect { x: monitor.x, y: monitor.y, width: half_w, height: half_h },
        Rect { x: monitor.x + half_w, y: monitor.y, width: half_w, height: half_h },
        Rect { x: monitor.x, y: monitor.y + half_h, width: half_w, height: half_h },
        Rect { x: monitor.x + half_w, y: monitor.y + half_h, width: half_w, height: half_h },
    ];
    window_ids
        .iter()
        .zip(slots.iter())
        .map(|(&id, &rect)| (id, rect))
        .collect()
}
