use tiler::menu::state::{MenuAction, MenuInput, MenuState};
use tiler::model::{LayoutPreset, Rect, VirtualDesktop, Window, WindowType};
use tiler::tiling::filter::is_toplevel;
use tiler::tiling::preset::{apply_fullscreen, apply_quadrants, apply_side_by_side, apply_top_bottom};
use tiler::tiling::stack::stack_layout;

// ===========================================================================
// Helper
// ===========================================================================

fn make_window(id: u64, wtype: WindowType, fullscreen: bool) -> Window {
    Window {
        id,
        title: format!("win-{}", id),
        app_class: "test".to_string(),
        monitor_id: 0,
        tile_position: Rect { x: 0, y: 0, width: 0, height: 0 },
        virtual_desktop_id: 0,
        is_fullscreen: fullscreen,
        window_type: wtype,
    }
}

fn screen() -> Rect {
    Rect { x: 0, y: 0, width: 1920, height: 1080 }
}

// ===========================================================================
// Filter + Stack layout end-to-end
// ===========================================================================

#[test]
fn test_filter_then_stack_layout() {
    // Arrange — mix of window types
    let windows = vec![
        make_window(1, WindowType::Normal, false),
        make_window(2, WindowType::Dialog, false),
        make_window(3, WindowType::Normal, false),
        make_window(4, WindowType::Normal, true), // fullscreen
        make_window(5, WindowType::Popup, false),
        make_window(6, WindowType::Normal, false),
    ];

    // Act — filter to toplevel, then stack layout
    let toplevel_ids: Vec<u64> = windows
        .iter()
        .filter(|w| is_toplevel(w))
        .map(|w| w.id)
        .collect();

    let layout = stack_layout(&toplevel_ids, screen());

    // Assert — only 3 normal, non-fullscreen windows
    assert_eq!(layout.len(), 3);
    assert_eq!(layout[0].0, 1);
    assert_eq!(layout[1].0, 3);
    assert_eq!(layout[2].0, 6);

    // Single column, 3 rows of 360px each
    let row_h = 1080 / 3;
    assert_eq!(layout[0].1.height, row_h);
    assert_eq!(layout[1].1.y, row_h);
    assert_eq!(layout[2].1.y, row_h * 2);
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
// VirtualDesktop window stack + filter integration
// ===========================================================================

#[test]
fn test_desktop_window_stack_with_filtering() {
    // Arrange — build up desktop state
    let mut vd = VirtualDesktop::new(0);

    let windows = vec![
        make_window(1, WindowType::Normal, false),
        make_window(2, WindowType::Normal, false),
        make_window(3, WindowType::Dialog, false),
        make_window(4, WindowType::Normal, false),
    ];

    // Push windows onto desktop stack
    for w in &windows {
        vd.push_window(w.id);
    }

    // Newest (4) should be first
    assert_eq!(vd.stack_windows, vec![4, 3, 2, 1]);

    // Filter to toplevel only
    let toplevel_ids: Vec<u64> = vd
        .stack_windows
        .iter()
        .filter(|&&id| {
            windows
                .iter()
                .find(|w| w.id == id)
                .map(|w| is_toplevel(w))
                .unwrap_or(false)
        })
        .copied()
        .collect();

    // Window 3 (dialog) is filtered out
    assert_eq!(toplevel_ids, vec![4, 2, 1]);

    // Stack layout preserves desktop order
    let layout = stack_layout(&toplevel_ids, screen());
    assert_eq!(layout[0].0, 4, "newest window at top");
    assert_eq!(layout[2].0, 1, "oldest window at bottom");
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
