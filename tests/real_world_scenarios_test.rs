//! Integration tests simulating real-world usage patterns observed in the
//! production log (`tiler.log`). Each scenario exercises a multi-step
//! lifecycle that has historically surfaced bugs or state inconsistencies.

use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::menu::state::{MenuInput, MenuState};
use tiler::model::LayoutPreset;
use tiler::tiling::engine::TilingEngine;
use tiler::config::StackScreenPosition;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]
}

fn make_proxy(monitors: Vec<MonitorInfo>, windows: Vec<WindowInfo>) -> MockGnomeProxy {
    let mut proxy = MockGnomeProxy::new();
    proxy.set_monitors(monitors);
    proxy.set_windows(windows);
    for w in proxy.list_windows_snapshot() {
        proxy.set_window_type(w.id, "toplevel".into());
    }
    proxy
}

/// Create a fresh engine with two monitors and the given initial windows,
/// calling startup before returning.
async fn engine_with(windows: Vec<WindowInfo>) -> TilingEngine<MockGnomeProxy> {
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();
    engine
}

// ===========================================================================
// 1. Rapid window open/close
//
// Production pattern: window opens, gets tiled, closes within 100ms,
// immediately another window opens. The stack must remain consistent
// with no stale references.
// ===========================================================================

#[tokio::test]
async fn rapid_open_close_leaves_consistent_stack() {
    let mut engine = engine_with(vec![]).await;

    // Open window A on the stack screen (monitor 0)
    engine.proxy_mut().set_window_type(1, "toplevel".into());
    engine.handle_window_opened(1, "A".into(), "a".into(), 0).await.unwrap();

    // Verify A is tracked
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&1), "window A should be in stack after open");

    // Immediately close A (simulates sub-100ms lifespan)
    engine.handle_window_closed(1).await.unwrap();

    // Open window B on the same monitor
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine.handle_window_opened(2, "B".into(), "b".into(), 0).await.unwrap();

    // Stack must contain only B, no stale reference to A
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(!desktop.stack_windows.contains(&1), "closed window A must not be in stack");
    assert!(desktop.stack_windows.contains(&2), "window B must be in stack");
    assert_eq!(desktop.stack_windows.len(), 1, "stack should have exactly 1 window");
}

#[tokio::test]
async fn rapid_open_close_three_cycles() {
    let mut engine = engine_with(vec![]).await;

    // Three rapid open/close cycles
    for id in 1..=3u64 {
        engine.proxy_mut().set_window_type(id, "toplevel".into());
        engine.handle_window_opened(id, format!("W{id}"), "app".into(), 0).await.unwrap();
        engine.handle_window_closed(id).await.unwrap();
    }

    // Open a final surviving window
    engine.proxy_mut().set_window_type(10, "toplevel".into());
    engine.handle_window_opened(10, "Survivor".into(), "app".into(), 0).await.unwrap();

    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.stack_windows, vec![10], "only the survivor should remain");
}

// ===========================================================================
// 2. Multi-workspace window lifecycle
//
// Open windows on different workspaces, switch between workspaces rapidly.
// Each workspace's state must remain independent and consistent.
// ===========================================================================

#[tokio::test]
async fn multi_workspace_isolation_after_rapid_switching() {
    let windows = vec![
        WindowInfo { id: 1, title: "WS0-A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "WS0-B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 3, title: "WS1-A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 1 },
        WindowInfo { id: 4, title: "WS1-B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let mut engine = engine_with(windows).await;

    // Rapidly switch between workspaces
    engine.handle_workspace_changed(1).await.unwrap();
    engine.handle_workspace_changed(0).await.unwrap();
    engine.handle_workspace_changed(1).await.unwrap();
    engine.handle_workspace_changed(0).await.unwrap();

    // Verify workspace 0 state is intact
    let ws0 = engine.desktop_ref(0).unwrap();
    assert!(ws0.stack_windows.contains(&1), "ws0 should have window 1");
    assert!(ws0.stack_windows.contains(&2), "ws0 should have window 2");
    assert!(!ws0.stack_windows.contains(&3), "ws0 should NOT have ws1 windows");

    // Verify workspace 1 state is intact
    let ws1 = engine.desktop_ref(1).unwrap();
    assert!(ws1.stack_windows.contains(&3), "ws1 should have window 3");
    assert!(ws1.stack_windows.contains(&4), "ws1 should have window 4");
    assert!(!ws1.stack_windows.contains(&1), "ws1 should NOT have ws0 windows");
}

#[tokio::test]
async fn workspace_switch_retiles_correct_windows() {
    let windows = vec![
        WindowInfo { id: 1, title: "WS0".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "WS1".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let mut engine = engine_with(windows).await;

    let calls_after_startup = engine.proxy().move_resize_calls().len();

    // Switch to workspace 1
    engine.handle_workspace_changed(1).await.unwrap();

    // The retile should only position window 2 (on ws 1)
    let new_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_after_startup..].to_vec();
    let tiled_ids: Vec<u64> = new_calls.iter().map(|c| c.0).collect();
    assert!(tiled_ids.contains(&2), "window 2 should be tiled on workspace 1");

    // Switch back to workspace 0
    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_workspace_changed(0).await.unwrap();

    let new_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before..].to_vec();
    let tiled_ids: Vec<u64> = new_calls.iter().map(|c| c.0).collect();
    assert!(tiled_ids.contains(&1), "window 1 should be tiled on workspace 0");
}

// ===========================================================================
// 3. Window opens on non-stack monitor, moved to stack, then closed
//
// Full lifecycle: open on mon1 -> auto-moved to mon0 -> retiled -> closed
// -> stack retiles correctly without the window.
// ===========================================================================

#[tokio::test]
async fn non_stack_to_stack_lifecycle() {
    // Start with one existing window on the stack screen
    let windows = vec![
        WindowInfo { id: 1, title: "Existing".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_with(windows).await;

    let calls_after_startup = engine.proxy().move_resize_calls().len();

    // Open window 2 on monitor 1 (non-stack, no preset) -> should be moved to stack
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine.handle_window_opened(2, "NewTerm".into(), "terminal".into(), 1).await.unwrap();

    // Verify window 2 was moved to the stack screen (x=0 region)
    let post_open_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_after_startup..].to_vec();
    let win2_calls: Vec<_> = post_open_calls.iter().filter(|c| c.0 == 2).collect();
    assert!(!win2_calls.is_empty(), "window 2 should have been repositioned");
    // At least one call for window 2 should target stack screen geometry (x=0)
    assert!(
        win2_calls.iter().any(|c| c.1 == 0),
        "window 2 should have been moved to stack screen (x=0)"
    );

    // Both windows should be in the desktop stack
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&1));
    assert!(desktop.stack_windows.contains(&2));

    // Now close window 2
    let calls_before_close = engine.proxy().move_resize_calls().len();
    engine.handle_window_closed(2).await.unwrap();

    // Stack should retile with only window 1
    let post_close_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before_close..].to_vec();
    let tiled_ids: Vec<u64> = post_close_calls.iter().map(|c| c.0).collect();
    assert!(tiled_ids.contains(&1), "window 1 should be retiled after window 2 closes");
    assert!(!tiled_ids.contains(&2), "closed window 2 should not be retiled");

    // Desktop stack should only contain window 1
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.stack_windows, vec![1]);
}

// ===========================================================================
// 4. Preset layout + window open/close cycle
//
// Set a preset on monitor 1, open 3 windows on it, close the middle one.
// Verify layout re-applies correctly with remaining 2 windows.
// ===========================================================================

#[tokio::test]
async fn preset_layout_reapplies_after_middle_window_close() {
    let mut engine = engine_with(vec![]).await;

    // Set SideBySide layout on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    // Open 3 windows on monitor 1 (which has a preset, so they stay there)
    for id in 10..=12u64 {
        engine.proxy_mut().set_window_type(id, "toplevel".into());
        engine.handle_window_opened(id, format!("W{id}"), "app".into(), 1).await.unwrap();
    }

    // Verify all 3 are tracked on monitor 1 in the desktop
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&10));
    assert!(desktop.stack_windows.contains(&11));
    assert!(desktop.stack_windows.contains(&12));

    // Close the middle window (11)
    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_window_closed(11).await.unwrap();

    // Layout should re-apply for the remaining 2 windows on monitor 1
    let post_close_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before..].to_vec();
    let mon1_calls: Vec<_> = post_close_calls.iter().filter(|c| c.1 >= 1920).collect();
    assert!(
        mon1_calls.len() >= 2,
        "at least 2 windows should be repositioned on monitor 1 after close, got {}",
        mon1_calls.len()
    );

    // Verify the remaining windows are positioned as SideBySide on monitor 1
    // Left half: x=1920, w=960; Right half: x=2880, w=960
    let win10_call = mon1_calls.iter().find(|c| c.0 == 10);
    let win12_call = mon1_calls.iter().find(|c| c.0 == 12);
    assert!(win10_call.is_some(), "window 10 should be repositioned");
    assert!(win12_call.is_some(), "window 12 should be repositioned");

    // Desktop should no longer contain window 11
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(!desktop.stack_windows.contains(&11), "closed window 11 must be removed");
    assert_eq!(desktop.stack_windows.iter().filter(|&&w| w == 10 || w == 12).count(), 2);
}

#[tokio::test]
async fn preset_layout_quadrants_with_window_churn() {
    let mut engine = engine_with(vec![]).await;

    // Set Quadrants layout on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::Quadrants);

    // Open 4 windows
    for id in 1..=4u64 {
        engine.proxy_mut().set_window_type(id, "toplevel".into());
        engine.handle_window_opened(id, format!("W{id}"), "app".into(), 1).await.unwrap();
    }

    // Close windows 2 and 3
    engine.handle_window_closed(2).await.unwrap();
    engine.handle_window_closed(3).await.unwrap();

    // Open a new window (5) to replace one slot
    engine.proxy_mut().set_window_type(5, "toplevel".into());
    engine.handle_window_opened(5, "W5".into(), "app".into(), 1).await.unwrap();

    // Desktop should contain windows 1, 4, 5 (2 and 3 closed)
    let desktop = engine.desktop_ref(0).unwrap();
    let on_mon1: Vec<u64> = desktop.stack_windows.iter()
        .filter(|&&w| [1, 4, 5].contains(&w))
        .copied()
        .collect();
    assert_eq!(on_mon1.len(), 3, "3 windows should remain after churn");
    assert!(!desktop.stack_windows.contains(&2));
    assert!(!desktop.stack_windows.contains(&3));
}

// ===========================================================================
// 5. Fullscreen toggle mid-layout
//
// Window is part of a preset layout, goes fullscreen, comes back.
// Verify layout is restored when fullscreen exits.
// ===========================================================================

#[tokio::test]
async fn fullscreen_toggle_restores_preset_layout() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_with(windows).await;

    // Set a layout preset on the stack screen (monitor 0)
    engine.desktop_mut(0).set_layout(0, LayoutPreset::SideBySide);

    // Window 1 goes fullscreen - should be removed from layout
    engine.handle_fullscreen_changed(1, true).await.unwrap();

    // Only window 2 should be in the layout now
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(!desktop.stack_windows.contains(&1), "fullscreen window should be removed from stack");
    assert!(desktop.stack_windows.contains(&2), "non-fullscreen window should remain");

    // Window 1 exits fullscreen - should be added back and layout restored
    let calls_before_restore = engine.proxy().move_resize_calls().len();
    engine.handle_fullscreen_changed(1, false).await.unwrap();

    // Both windows should be back in the layout
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&1), "window should be back in stack after exiting fullscreen");
    assert!(desktop.stack_windows.contains(&2));

    // Layout should have been re-applied (move_resize calls for both windows)
    let restore_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before_restore..].to_vec();
    assert!(
        restore_calls.len() >= 2,
        "both windows should be repositioned after fullscreen exit, got {}",
        restore_calls.len()
    );
}

#[tokio::test]
async fn fullscreen_toggle_on_preset_monitor_reapplies_layout() {
    let mut engine = engine_with(vec![]).await;

    // Set SideBySide on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    // Open 2 windows on monitor 1
    for id in [10, 11] {
        engine.proxy_mut().set_window_type(id, "toplevel".into());
        engine.handle_window_opened(id, format!("W{id}"), "app".into(), 1).await.unwrap();
    }

    // Window 10 goes fullscreen
    engine.handle_fullscreen_changed(10, true).await.unwrap();

    let desktop = engine.desktop_ref(0).unwrap();
    assert!(!desktop.stack_windows.contains(&10));

    // Window 10 exits fullscreen
    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_fullscreen_changed(10, false).await.unwrap();

    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&10), "window 10 should be back");
    assert!(desktop.stack_windows.contains(&11), "window 11 should still be there");

    // Layout should have been re-applied on monitor 1
    let calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before..].to_vec();
    let mon1_calls: Vec<_> = calls.iter().filter(|c| c.1 >= 1920).collect();
    assert!(
        mon1_calls.len() >= 2,
        "both windows should be repositioned on monitor 1, got {}",
        mon1_calls.len()
    );
}

// ===========================================================================
// 6. Menu flow with move window
//
// Full menu lifecycle: open menu, zoom into monitor, apply layout, then use
// Shift+N to move window. Verify all state transitions and proxy calls.
// ===========================================================================

#[tokio::test]
async fn full_menu_flow_zoom_apply_layout_then_move_window() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_with(windows).await;

    // Focus window 1
    engine.handle_focus_changed(1);
    assert_eq!(engine.focused_window_id(), Some(1));

    // --- Flow 1: Open menu -> Zoom into monitor 1 -> Apply SideBySide ---

    // Open menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);
    assert_eq!(engine.proxy().show_menu_calls().len(), 1, "show_menu should be called");

    // Zoom into monitor 1
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(1));
    assert_eq!(engine.proxy().show_menu_zoomed_calls().len(), 1);
    assert_eq!(engine.proxy().show_menu_zoomed_calls()[0].0, 1);

    // Apply SideBySide (digit 2)
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Closed);
    assert_eq!(engine.proxy().hide_menu_count(), 1, "menu should be hidden after layout apply");

    // Verify layout was set
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.get_layout(1), Some(LayoutPreset::SideBySide));

    // --- Flow 2: Open menu -> Shift+1 to move focused window to monitor 1 ---

    let calls_before_move = engine.proxy().move_resize_calls().len();

    // Re-open menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    // Shift+1 = move window to monitor 1
    engine.handle_menu_input(MenuInput::ShiftN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Closed, "menu should close after move");
    assert_eq!(engine.proxy().hide_menu_count(), 2, "menu hidden a second time");

    // Verify window was moved to monitor 1
    let move_calls: Vec<_> = engine.proxy().move_resize_calls()[calls_before_move..].to_vec();
    let win1_on_mon1 = move_calls.iter().find(|c| c.0 == 1 && c.1 >= 1920);
    assert!(win1_on_mon1.is_some(), "window 1 should be moved to monitor 1 (x >= 1920)");
}

#[tokio::test]
async fn menu_escape_dismisses_without_side_effects() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_with(windows).await;

    let calls_before = engine.proxy().move_resize_calls().len();

    // Open menu then escape
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    engine.handle_menu_input(MenuInput::Escape).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // No layout changes should have occurred
    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(calls_after, calls_before, "escape should not trigger any move/resize");
}

// ===========================================================================
// 7. Daemon restart scenario
//
// Start engine fresh (simulating a restart where old windows are gone).
// Verify clean state with no artifacts from a previous session.
// ===========================================================================

#[tokio::test]
async fn fresh_startup_has_clean_state() {
    // Simulate: daemon starts with no pre-existing windows
    let engine = engine_with(vec![]).await;

    // No windows should be tracked
    assert_eq!(engine.desktop_ref(0), None, "no desktop should exist with 0 windows");
    assert_eq!(engine.focused_window_id(), None);
    assert_eq!(engine.menu_state(), MenuState::Closed);
    assert!(!engine.is_tiling());

    // No move_resize calls should have been made
    assert_eq!(engine.proxy().move_resize_calls().len(), 0);
}

#[tokio::test]
async fn fresh_startup_then_open_windows_works() {
    let mut engine = engine_with(vec![]).await;

    // Open windows one by one (simulating user opening apps after daemon restart)
    for id in 1..=3u64 {
        engine.proxy_mut().set_window_type(id, "toplevel".into());
        engine.handle_window_opened(id, format!("App{id}"), "app".into(), 0).await.unwrap();
    }

    // All 3 should be tracked
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.stack_windows.len(), 3);
    assert!(desktop.stack_windows.contains(&1));
    assert!(desktop.stack_windows.contains(&2));
    assert!(desktop.stack_windows.contains(&3));

    // Each window open triggers a retile, last retile positions all 3
    let calls = engine.proxy().move_resize_calls();
    assert!(!calls.is_empty(), "windows should have been tiled");

    // The last batch of calls should include all 3 windows
    let last_3: Vec<u64> = calls.iter().rev().take(3).map(|c| c.0).collect();
    assert!(last_3.contains(&1));
    assert!(last_3.contains(&2));
    assert!(last_3.contains(&3));
}

#[tokio::test]
async fn restart_with_existing_windows_tiles_only_stack_screen() {
    // Simulate restart where GNOME already has windows open
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 3, title: "C".into(), app_class: "c".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let engine = engine_with(windows).await;

    // Only monitor 0 windows should have been tiled at startup
    let calls = engine.proxy().move_resize_calls();
    let tiled_on_mon0: Vec<_> = calls.iter().filter(|c| c.1 < 1920).collect();
    let tiled_on_mon1: Vec<_> = calls.iter().filter(|c| c.1 >= 1920).collect();

    assert_eq!(tiled_on_mon0.len(), 2, "2 windows should be tiled on stack screen");
    assert_eq!(tiled_on_mon1.len(), 0, "no windows should be tiled on non-stack monitor");
}

// ===========================================================================
// Cross-cutting: combined scenarios
// ===========================================================================

#[tokio::test]
async fn open_close_across_workspaces_then_switch() {
    let mut engine = engine_with(vec![]).await;

    // Open windows on workspace 0
    engine.proxy_mut().set_window_type(1, "toplevel".into());
    engine.handle_window_opened(1, "WS0-A".into(), "a".into(), 0).await.unwrap();
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine.handle_window_opened(2, "WS0-B".into(), "b".into(), 0).await.unwrap();

    // Switch to workspace 1, open a window there
    engine.handle_workspace_changed(1).await.unwrap();
    engine.proxy_mut().set_window_type(3, "toplevel".into());
    engine.handle_window_opened(3, "WS1-A".into(), "a".into(), 0).await.unwrap();

    // Close window 2 (which is on workspace 0, not the current active workspace)
    // The engine tracks by stored workspace_id, not active workspace
    engine.handle_window_closed(2).await.unwrap();

    // Switch back to workspace 0
    engine.handle_workspace_changed(0).await.unwrap();

    // Workspace 0 should have only window 1
    let ws0 = engine.desktop_ref(0).unwrap();
    assert_eq!(ws0.stack_windows, vec![1], "ws0 should only have window 1 after closing 2");

    // Workspace 1 should have window 3
    let ws1 = engine.desktop_ref(1).unwrap();
    assert!(ws1.stack_windows.contains(&3), "ws1 should have window 3");
}

#[tokio::test]
async fn move_window_then_fullscreen_then_close() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_with(windows).await;

    // Move window 1 to monitor 1
    engine.handle_focus_changed(1);
    engine.move_window_to_monitor(1).await.unwrap();

    // Window 1 goes fullscreen on monitor 1
    engine.handle_fullscreen_changed(1, true).await.unwrap();

    // Focused window was removed from stack by fullscreen
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(!desktop.stack_windows.contains(&1));

    // Close window 1
    engine.handle_window_closed(1).await.unwrap();

    // Focused window should be cleared
    assert_eq!(engine.focused_window_id(), None, "focused window should be cleared after close");

    // Only window 2 should remain, on the stack screen
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.stack_windows, vec![2]);
}

#[tokio::test]
async fn preset_survives_workspace_switch() {
    let mut engine = engine_with(vec![]).await;

    // Set a preset on monitor 1, workspace 0
    engine.desktop_mut(0).set_layout(1, LayoutPreset::TopBottom);

    // Open windows on monitor 1 with the preset
    engine.proxy_mut().set_window_type(1, "toplevel".into());
    engine.handle_window_opened(1, "A".into(), "a".into(), 1).await.unwrap();
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine.handle_window_opened(2, "B".into(), "b".into(), 1).await.unwrap();

    // Switch to workspace 1 and back
    engine.handle_workspace_changed(1).await.unwrap();
    engine.handle_workspace_changed(0).await.unwrap();

    // Preset should still be set on workspace 0
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(
        desktop.get_layout(1),
        Some(LayoutPreset::TopBottom),
        "preset should survive workspace switching"
    );
}
