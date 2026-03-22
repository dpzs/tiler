use tiler::menu::state::{MenuAction, MenuInput, MenuState};
use tiler::model::{LayoutPreset, Rect, VirtualDesktop};
use tiler::tiling::preset::{apply_fullscreen, apply_quadrants, apply_side_by_side, apply_top_bottom};
use tiler::tiling::stack::stack_layout;

// ===========================================================================
// Helper
// ===========================================================================

fn screen() -> Rect {
    Rect { x: 0, y: 0, width: 1920, height: 1080 }
}

// ===========================================================================
// VirtualDesktop + preset layout
// ===========================================================================

#[test]
fn test_virtual_desktop_drives_preset_layout() {
    // Arrange
    let mut vd = VirtualDesktop::new(0);
    vd.set_layout(0, LayoutPreset::SideBySide);
    vd.set_enforcement(0, true);

    let window_ids = vec![10, 20];

    // Act — look up preset and apply it
    let preset = vd.get_layout(0).unwrap();
    let layout = match preset {
        LayoutPreset::Fullscreen => apply_fullscreen(&window_ids, screen()),
        LayoutPreset::SideBySide => apply_side_by_side(&window_ids, screen()),
        LayoutPreset::TopBottom => apply_top_bottom(&window_ids, screen()),
        LayoutPreset::Quadrants => apply_quadrants(&window_ids, screen()),
    };

    // Assert
    assert!(vd.is_enforced(0));
    assert_eq!(layout.len(), 2);
    assert_eq!(layout[0].1.width, 960);
    assert_eq!(layout[1].1.x, 960);
}

// ===========================================================================
// Menu state machine -> layout action
// ===========================================================================

#[test]
fn test_menu_flow_zoom_then_apply_layout() {
    // Start closed
    let mut state = MenuState::Closed;

    // Open menu
    let (next, action) = state.transition(MenuInput::ToggleMenu);
    state = next;
    assert_eq!(state, MenuState::Overview);
    assert_eq!(action, None);

    // Zoom into monitor 1
    let (next, action) = state.transition(MenuInput::PressN(1));
    state = next;
    assert_eq!(state, MenuState::ZoomedIn(1));
    assert_eq!(action, Some(MenuAction::ZoomIn(1)));

    // Apply Quadrants (digit 4)
    let (next, action) = state.transition(MenuInput::Digit(4));
    state = next;
    assert_eq!(state, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::ApplyLayout(1, 4)));
}

#[test]
fn test_menu_flow_move_window() {
    let mut state = MenuState::Closed;

    // Open
    let (next, _) = state.transition(MenuInput::ToggleMenu);
    state = next;

    // Shift+3 = move window to monitor 3
    let (next, action) = state.transition(MenuInput::ShiftN(3));
    state = next;
    assert_eq!(state, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::MoveWindow(3)));
}

// ===========================================================================
// VirtualDesktop window stack ordering
// ===========================================================================

#[test]
fn test_desktop_window_stack_ordering() {
    let mut vd = VirtualDesktop::new(0);
    let window_ids: Vec<u64> = vec![1, 2, 3, 4];
    for &id in &window_ids {
        vd.push_window(id);
    }
    assert_eq!(vd.stack_windows, vec![4, 3, 2, 1]);
    let layout = stack_layout(&vd.stack_windows, screen());
    assert_eq!(layout[0].0, 4, "newest window at top");
    assert_eq!(layout[3].0, 1, "oldest window at bottom");
}

// ===========================================================================
// Multiple desktops with different layouts
// ===========================================================================

#[test]
fn test_multiple_desktops_independent() {
    let mut desktop_a = VirtualDesktop::new(0);
    let mut desktop_b = VirtualDesktop::new(1);

    // Desktop A: SideBySide, enforced
    desktop_a.set_layout(0, LayoutPreset::SideBySide);
    desktop_a.set_enforcement(0, true);
    desktop_a.push_window(1);
    desktop_a.push_window(2);

    // Desktop B: Quadrants, not enforced
    desktop_b.set_layout(0, LayoutPreset::Quadrants);
    desktop_b.set_enforcement(0, false);
    desktop_b.push_window(10);
    desktop_b.push_window(20);
    desktop_b.push_window(30);
    desktop_b.push_window(40);

    // Apply layouts
    let layout_a = apply_side_by_side(&desktop_a.stack_windows, screen());
    let layout_b = apply_quadrants(&desktop_b.stack_windows, screen());

    // Desktop A: 2 windows side by side
    assert_eq!(layout_a.len(), 2);
    assert_eq!(layout_a[0].1.width, 960);

    // Desktop B: 4 windows in quadrants
    assert_eq!(layout_b.len(), 4);
    assert_eq!(layout_b[0].1.width, 960);
    assert_eq!(layout_b[0].1.height, 540);

    // They don't interfere
    assert_eq!(desktop_a.stack_windows.len(), 2);
    assert_eq!(desktop_b.stack_windows.len(), 4);
}

// ===========================================================================
// Enforcement toggle via menu
// ===========================================================================

#[test]
fn test_enforcement_via_menu_actions() {
    let mut vd = VirtualDesktop::new(0);
    let mut state = MenuState::Closed;

    // Open menu -> zoom into monitor 0 -> press 9 (enforce on)
    let (next, _) = state.transition(MenuInput::ToggleMenu);
    state = next;
    let (next, _) = state.transition(MenuInput::PressN(0));
    state = next;
    let (next, action) = state.transition(MenuInput::Digit(9));
    state = next;

    assert_eq!(state, MenuState::Closed);
    if let Some(MenuAction::EnforceOn(monitor)) = action {
        vd.set_enforcement(monitor, true);
    }
    assert!(vd.is_enforced(0));

    // Re-open -> zoom -> press 0 (enforce off)
    let (next, _) = state.transition(MenuInput::ToggleMenu);
    state = next;
    let (next, _) = state.transition(MenuInput::PressN(0));
    state = next;
    let (next, action) = state.transition(MenuInput::Digit(0));
    let _state = next;

    if let Some(MenuAction::EnforceOff(monitor)) = action {
        vd.set_enforcement(monitor, false);
    }
    assert!(!vd.is_enforced(0));
}
