use tiler::model::{LayoutPreset, VirtualDesktop};

// ===========================================================================
// Layout preset per monitor
// ===========================================================================

#[test]
fn test_set_layout_preset() {
    let mut vd = VirtualDesktop::new(0);
    vd.set_layout(1, LayoutPreset::SideBySide);
    assert_eq!(vd.get_layout(1), Some(LayoutPreset::SideBySide));
}

#[test]
fn test_get_layout_missing_monitor() {
    let vd = VirtualDesktop::new(0);
    assert_eq!(vd.get_layout(99), None);
}

#[test]
fn test_set_layout_overwrites() {
    let mut vd = VirtualDesktop::new(0);
    vd.set_layout(1, LayoutPreset::Fullscreen);
    vd.set_layout(1, LayoutPreset::Quadrants);
    assert_eq!(vd.get_layout(1), Some(LayoutPreset::Quadrants));
}

// ===========================================================================
// Enforcement mode per monitor
// ===========================================================================

#[test]
fn test_set_enforcement() {
    let mut vd = VirtualDesktop::new(0);
    vd.set_enforcement(2, true);
    assert_eq!(vd.is_enforced(2), true);
}

#[test]
fn test_enforcement_default_false() {
    let vd = VirtualDesktop::new(0);
    assert_eq!(vd.is_enforced(99), false);
}

#[test]
fn test_toggle_enforcement() {
    let mut vd = VirtualDesktop::new(0);
    vd.set_enforcement(0, true);
    assert!(vd.is_enforced(0));
    vd.set_enforcement(0, false);
    assert!(!vd.is_enforced(0));
}

// ===========================================================================
// Stack window order
// ===========================================================================

#[test]
fn test_push_window_to_front() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(100);
    vd.push_window(200);
    vd.push_window(300);

    assert_eq!(vd.stack_windows, vec![300, 200, 100], "newest first");
}

#[test]
fn test_push_existing_window_moves_to_front() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);
    vd.push_window(3);
    // Re-push window 1 — should move to front
    vd.push_window(1);

    assert_eq!(vd.stack_windows, vec![1, 3, 2], "re-pushed window at front, no duplicates");
}

#[test]
fn test_remove_window() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);
    vd.push_window(3);
    vd.remove_window(2);

    assert_eq!(vd.stack_windows, vec![3, 1]);
}

#[test]
fn test_remove_nonexistent_window() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.remove_window(99); // no-op
    assert_eq!(vd.stack_windows, vec![1]);
}

// ===========================================================================
// Per-desktop isolation
// ===========================================================================

#[test]
fn test_desktops_are_isolated() {
    let mut vd1 = VirtualDesktop::new(0);
    let mut vd2 = VirtualDesktop::new(1);

    vd1.set_layout(0, LayoutPreset::Fullscreen);
    vd1.set_enforcement(0, true);
    vd1.push_window(100);

    vd2.set_layout(0, LayoutPreset::Quadrants);
    vd2.set_enforcement(0, false);
    vd2.push_window(200);

    // vd1 is unchanged by vd2 operations
    assert_eq!(vd1.get_layout(0), Some(LayoutPreset::Fullscreen));
    assert!(vd1.is_enforced(0));
    assert_eq!(vd1.stack_windows, vec![100]);

    // vd2 has its own state
    assert_eq!(vd2.get_layout(0), Some(LayoutPreset::Quadrants));
    assert!(!vd2.is_enforced(0));
    assert_eq!(vd2.stack_windows, vec![200]);
}

// ===========================================================================
// Constructor
// ===========================================================================

#[test]
fn test_new_virtual_desktop() {
    let vd = VirtualDesktop::new(5);
    assert_eq!(vd.id, 5);
    assert!(vd.layout_presets.is_empty());
    assert!(vd.enforcement_modes.is_empty());
    assert!(vd.stack_windows.is_empty());
}
