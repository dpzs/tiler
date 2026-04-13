//! Tests for the core mission rules:
//! 1. New windows always open on screen 1 (stack), screen 1 is a vertical stack
//! 2. Windows can be moved to screen 2 or 3
//! 3. Fullscreen on screen 2/3: return all others to stack
//! 4. Split on screen 2/3: new left/top, old right/bottom, extras to stack
//! 5. Windows NEVER move from their virtual desktop
//! 6. Switching virtual desktops should NOT reflow windows
//! 7. Virtual desktops maintain independent order/positions

use tiler::config::StackScreenPosition;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::menu::state::{MenuInput, MenuState};
use tiler::model::LayoutPreset;
use tiler::tiling::engine::TilingEngine;

fn three_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 2, name: "DP-3".into(), x: 3840, y: 0, width: 1920, height: 1080 },
    ]
}

fn make_proxy(monitors: Vec<MonitorInfo>, windows: Vec<WindowInfo>) -> MockGnomeProxy {
    let mut proxy = MockGnomeProxy::new();
    proxy.set_monitors(monitors);
    proxy.set_windows(windows.clone());
    for w in &windows {
        proxy.set_window_type(w.id, "toplevel".into());
    }
    proxy
}

async fn engine_3mon(windows: Vec<WindowInfo>) -> TilingEngine<MockGnomeProxy> {
    let proxy = make_proxy(three_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();
    engine
}

// =========================================================================
// Rule 1: New windows always open on screen 1 (stack)
// =========================================================================

#[tokio::test]
async fn new_window_on_non_stack_is_moved_to_stack() {
    let mut engine = engine_3mon(vec![]).await;

    engine.proxy_mut().set_window_type(1, "toplevel".into());
    engine.handle_window_opened(1, "App".into(), "app".into(), 1).await.unwrap();

    // Window should have been moved to stack screen (monitor 0)
    let calls = engine.proxy().move_resize_calls();
    let moved_to_stack = calls.iter().any(|c| c.0 == 1 && c.1 < 1920);
    assert!(moved_to_stack, "window opened on monitor 1 should be moved to stack (monitor 0)");
}

// =========================================================================
// Rule 3: Fullscreen on screen 2 returns all others to stack
// =========================================================================

#[tokio::test]
async fn fullscreen_on_screen2_returns_others_to_stack() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;
    engine.desktop_mut(0).push_window(1);
    engine.desktop_mut(0).push_window(2);

    // Focus window 3 (on stack), then apply fullscreen to monitor 1
    engine.proxy_mut().set_window_type(3, "toplevel".into());
    engine.handle_window_opened(3, "C".into(), "c".into(), 0).await.unwrap();
    engine.handle_focus_changed(3);

    // Open menu → select monitor 1 → fullscreen
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(1));

    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_menu_input(MenuInput::Digit(1)).await.unwrap(); // Fullscreen
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Window 3 (focused) should be moved to monitor 1
    let calls = &engine.proxy().move_resize_calls()[calls_before..];
    let win3_on_mon1 = calls.iter().any(|c| c.0 == 3 && c.1 >= 1920 && c.1 < 3840);
    assert!(win3_on_mon1, "focused window 3 should be placed on monitor 1");

    // Excess windows (1, 2) should be returned to stack (x < 1920)
    let win1_to_stack = calls.iter().any(|c| c.0 == 1 && c.1 < 1920);
    let win2_to_stack = calls.iter().any(|c| c.0 == 2 && c.1 < 1920);
    assert!(win1_to_stack, "window 1 should be returned to stack");
    assert!(win2_to_stack, "window 2 should be returned to stack");
}

// =========================================================================
// Rule 4: Split on screen 2 — new left, old right, extras to stack
// =========================================================================

#[tokio::test]
async fn side_by_side_on_screen2_places_new_left_old_right() {
    let windows = vec![
        WindowInfo { id: 1, title: "Old".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;
    engine.desktop_mut(0).push_window(1);

    // Open a new window on stack, focus it
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine.handle_window_opened(2, "New".into(), "b".into(), 0).await.unwrap();
    engine.handle_focus_changed(2);

    // Apply SideBySide on monitor 1 via menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap(); // SideBySide
    let calls = &engine.proxy().move_resize_calls()[calls_before..];

    // New window (2, focused) should be on left half of monitor 1 (x=1920)
    let win2_left = calls.iter().find(|c| c.0 == 2 && c.1 == 1920);
    assert!(win2_left.is_some(), "focused window 2 should be on left half of monitor 1");

    // Old window (1) should be on right half of monitor 1 (x=2880)
    let win1_right = calls.iter().find(|c| c.0 == 1 && c.1 == 2880);
    assert!(win1_right.is_some(), "old window 1 should be on right half of monitor 1");
}

#[tokio::test]
async fn side_by_side_extras_returned_to_stack() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;
    engine.desktop_mut(0).push_window(1);
    engine.desktop_mut(0).push_window(2);

    // Create and focus window 3 on stack
    engine.proxy_mut().set_window_type(3, "toplevel".into());
    engine.handle_window_opened(3, "New".into(), "c".into(), 0).await.unwrap();
    engine.handle_focus_changed(3);

    // Apply SideBySide on monitor 1
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap();
    let calls = &engine.proxy().move_resize_calls()[calls_before..];

    // SideBySide has 2 slots: window 3 (focused, moved first) + window 2 (was already there)
    // Window 1 is excess → returned to stack
    let win1_to_stack = calls.iter().any(|c| c.0 == 1 && c.1 < 1920);
    assert!(win1_to_stack, "excess window 1 should be returned to stack");
}

// =========================================================================
// Rule 5 & 6: Virtual desktop isolation — no reflow on workspace switch
// =========================================================================

#[tokio::test]
async fn workspace_switch_does_not_reflow_visited_workspace() {
    let windows = vec![
        WindowInfo { id: 1, title: "WS0".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;

    // Use menu on workspace 0 (sets a layout preset)
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Switch to workspace 1 (first visit — tiles stack for ws1, which is empty)
    engine.handle_workspace_changed(1).await.unwrap();
    // Switch back to workspace 0 (already visited — should NOT retile)
    engine.handle_workspace_changed(0).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after, calls_before,
        "switching back to visited workspace should NOT cause any move_resize calls"
    );
}

#[tokio::test]
async fn workspace_switch_tiles_on_first_visit_only() {
    let windows = vec![
        WindowInfo { id: 1, title: "WS0".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "WS1".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let mut engine = engine_3mon(windows).await;

    let calls_after_startup = engine.proxy().move_resize_calls().len();

    // First visit to workspace 1 — should tile
    engine.handle_workspace_changed(1).await.unwrap();
    let calls_after_first = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after_first > calls_after_startup,
        "first visit to workspace 1 should tile its windows"
    );

    // Second visit to workspace 1 — should NOT retile
    engine.handle_workspace_changed(0).await.unwrap();
    let calls_before_second = engine.proxy().move_resize_calls().len();
    engine.handle_workspace_changed(1).await.unwrap();
    let calls_after_second = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after_second, calls_before_second,
        "second visit to workspace 1 should NOT retile"
    );
}

// =========================================================================
// Rule 7: Virtual desktops maintain independent layouts
// =========================================================================

#[tokio::test]
async fn workspaces_have_independent_layouts() {
    let mut engine = engine_3mon(vec![]).await;

    // Set different layouts on different workspaces
    engine.desktop_mut(0).set_layout(1, LayoutPreset::Fullscreen);
    engine.desktop_mut(1).set_layout(1, LayoutPreset::SideBySide);

    assert_eq!(
        engine.desktop_ref(0).unwrap().get_layout(1),
        Some(LayoutPreset::Fullscreen)
    );
    assert_eq!(
        engine.desktop_ref(1).unwrap().get_layout(1),
        Some(LayoutPreset::SideBySide)
    );

    // Switch workspaces — layouts should not bleed
    engine.handle_workspace_changed(1).await.unwrap();
    engine.handle_workspace_changed(0).await.unwrap();

    assert_eq!(
        engine.desktop_ref(0).unwrap().get_layout(1),
        Some(LayoutPreset::Fullscreen),
        "workspace 0 layout should be preserved"
    );
    assert_eq!(
        engine.desktop_ref(1).unwrap().get_layout(1),
        Some(LayoutPreset::SideBySide),
        "workspace 1 layout should be preserved"
    );
}

// =========================================================================
// Stack screen protection: selecting screen 1 in menu moves to stack
// =========================================================================

#[tokio::test]
async fn menu_press_n_on_stack_screen_moves_window_to_stack() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;
    engine.handle_focus_changed(1);

    // Open menu, press 0 (monitor 0 = stack screen)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_menu_input(MenuInput::PressN(0)).await.unwrap();

    // Should NOT zoom in — should close menu and move window to stack
    assert_eq!(engine.menu_state(), MenuState::Closed, "menu should close when selecting stack screen");

    let calls = &engine.proxy().move_resize_calls()[calls_before..];
    let win1_to_stack = calls.iter().any(|c| c.0 == 1 && c.1 < 1920);
    assert!(win1_to_stack, "window should be moved to stack screen");
}

// =========================================================================
// ApplyLayout moves focused window to target screen
// =========================================================================

#[tokio::test]
async fn apply_layout_moves_focused_window_to_target() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut engine = engine_3mon(windows).await;
    engine.handle_focus_changed(1);

    // Menu: select monitor 2, apply fullscreen
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(2)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(2));

    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_menu_input(MenuInput::Digit(1)).await.unwrap(); // Fullscreen

    let calls = &engine.proxy().move_resize_calls()[calls_before..];
    let win1_on_mon2 = calls.iter().any(|c| c.0 == 1 && c.1 >= 3840);
    assert!(win1_on_mon2, "focused window 1 should be moved to monitor 2 (x >= 3840)");
}
