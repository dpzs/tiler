use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::tiling::engine::TilingEngine;
use tiler::config::StackScreenPosition;

// --- Helpers ---

fn two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]
}

fn make_proxy(monitors: Vec<MonitorInfo>) -> MockGnomeProxy {
    let mut proxy = MockGnomeProxy::new();
    proxy.set_monitors(monitors);
    proxy
}

// ===========================================================================
// 1. New engine has no focused window
// ===========================================================================

#[tokio::test]
async fn should_return_none_focused_window_for_new_engine() {
    // Arrange
    let proxy = make_proxy(two_monitors());
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Assert
    assert_eq!(
        engine.focused_window_id(),
        None,
        "a newly started engine should have no focused window"
    );
}

// ===========================================================================
// 2. handle_focus_changed sets focused window
// ===========================================================================

#[tokio::test]
async fn should_set_focused_window_after_handle_focus_changed() {
    // Arrange — window must be tracked for focus to be accepted
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 42, title: "W".into(), app_class: "w".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors);
    proxy.set_windows(windows);
    proxy.set_window_type(42, "toplevel".into());
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Act
    engine.handle_focus_changed(42);

    // Assert
    assert_eq!(
        engine.focused_window_id(),
        Some(42),
        "focused_window_id should return Some(42) after handle_focus_changed(42)"
    );
}

// ===========================================================================
// 3. handle_focus_changed replaces previous focus
// ===========================================================================

#[tokio::test]
async fn should_replace_focused_window_on_subsequent_focus_change() {
    // Arrange — both windows must be tracked
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 42, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 99, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors);
    proxy.set_windows(windows);
    proxy.set_window_type(42, "toplevel".into());
    proxy.set_window_type(99, "toplevel".into());
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Act
    engine.handle_focus_changed(42);
    engine.handle_focus_changed(99);

    // Assert
    assert_eq!(
        engine.focused_window_id(),
        Some(99),
        "focused_window_id should return Some(99) after a second handle_focus_changed call"
    );
}

// ===========================================================================
// 4. handle_focus_changed works for untracked windows
// ===========================================================================

#[tokio::test]
async fn should_accept_focus_change_for_untracked_window() {
    // Arrange — engine with no windows at all
    let proxy = make_proxy(two_monitors());
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Act — window 12345 is not tracked by the engine. Focus signals can
    // arrive before WindowOpened events due to D-Bus signal ordering, so
    // the engine stores them unconditionally. move_window_to_monitor
    // already guards against untracked windows.
    engine.handle_focus_changed(12345);

    // Assert — should store the ID even for untracked windows
    assert_eq!(
        engine.focused_window_id(),
        Some(12345),
        "handle_focus_changed should accept untracked windows"
    );
}

use tiler::menu::state::{MenuInput, MenuState};

// --- Helper for tests with windows ---

fn make_proxy_with_windows(monitors: Vec<MonitorInfo>, windows: Vec<WindowInfo>) -> MockGnomeProxy {
    let mut proxy = MockGnomeProxy::new();
    proxy.set_monitors(monitors);
    proxy.set_windows(windows.clone());
    for w in &windows {
        proxy.set_window_type(w.id, "toplevel".into());
    }
    proxy
}

// ===========================================================================
// 5. move_window_to_monitor: happy path
// ===========================================================================

#[tokio::test]
async fn should_move_focused_window_to_target_monitor() {
    // Arrange: window 1 on monitor 0
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Focus window 1
    engine.handle_focus_changed(1);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Act: move window to monitor 1
    engine.move_window_to_monitor(1).await.unwrap();

    // Assert: move_resize_window was called with monitor 1's geometry (1920, 0, 1920, 1080)
    let calls = engine.proxy().move_resize_calls();
    let move_call = calls.iter().skip(calls_before).find(|c| c.0 == 1);
    assert!(
        move_call.is_some(),
        "move_resize_window should have been called for window 1"
    );
    let (wid, x, y, w, h) = move_call.unwrap();
    assert_eq!(*wid, 1);
    assert_eq!(*x, 1920, "x should match monitor 1");
    assert_eq!(*y, 0, "y should match monitor 1");
    assert_eq!(*w, 1920, "width should match monitor 1");
    assert_eq!(*h, 1080, "height should match monitor 1");
}

// ===========================================================================
// 6. move_window_to_monitor: no-op when no focused window
// ===========================================================================

#[tokio::test]
async fn should_noop_move_when_no_focused_window() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Do NOT set focus
    let calls_before = engine.proxy().move_resize_calls().len();

    engine.move_window_to_monitor(1).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before, 0,
        "no move_resize calls should happen when no window is focused"
    );
}

// ===========================================================================
// 7. move_window_to_monitor: no-op when target monitor doesn't exist
// ===========================================================================

#[tokio::test]
async fn should_noop_move_when_target_monitor_not_found() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    engine.handle_focus_changed(1);
    let calls_before = engine.proxy().move_resize_calls().len();

    // Monitor 99 doesn't exist
    engine.move_window_to_monitor(99).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before, 0,
        "no move_resize calls should happen when target monitor doesn't exist"
    );
}

// ===========================================================================
// 8. End-to-end: ShiftN triggers move via menu
// ===========================================================================

#[tokio::test]
async fn should_move_window_via_shift_n_menu_input() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Focus window 1
    engine.handle_focus_changed(1);

    // Navigate: Closed -> Overview
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    let calls_before = engine.proxy().move_resize_calls().len();

    // ShiftN(1) should trigger MoveWindow(1)
    engine.handle_menu_input(MenuInput::ShiftN(1)).await.unwrap();

    // Menu should transition to Closed
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Should have called move_resize_window for window 1 to monitor 1's geometry
    let calls = engine.proxy().move_resize_calls();
    let move_call = calls.iter().skip(calls_before).find(|c| c.0 == 1);
    assert!(
        move_call.is_some(),
        "ShiftN(1) should trigger move_resize_window for the focused window"
    );
    let (_, x, _, w, h) = move_call.unwrap();
    assert_eq!(*x, 1920);
    assert_eq!(*w, 1920);
    assert_eq!(*h, 1080);
}

// ===========================================================================
// 9. Window removed from source desktop stack after move
// ===========================================================================

#[tokio::test]
async fn should_remove_window_from_source_desktop_stack_after_move() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Verify window 1 is in the stack before move
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&1), "window 1 should be in stack before move");

    // Focus and move window 1 to monitor 1
    engine.handle_focus_changed(1);
    engine.move_window_to_monitor(1).await.unwrap();

    // Window 1 should no longer be in the stack (it moved to a different monitor)
    // But window 2 should remain
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(
        desktop.stack_windows.contains(&2),
        "window 2 should still be in the stack"
    );
}

// ===========================================================================
// INTEGRATION TESTS
// ===========================================================================

// 10. Move window from stack screen: window stays tracked, only stack screen is retiled
#[tokio::test]
async fn should_not_retile_moved_window_on_stack_screen() {
    // Two windows on monitor 0 (stack screen)
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Move window 1 to monitor 1
    engine.handle_focus_changed(1);
    engine.move_window_to_monitor(1).await.unwrap();

    // Window 1 remains in stack_windows (all toplevel windows are tracked),
    // but only window 2 should be retiled on the stack screen
    let calls = engine.proxy().move_resize_calls();
    let post_move: Vec<_> = calls.iter().skip(calls_before).collect();

    // Should have: 1 move call (window 1 to monitor 1) + 1 retile call (window 2 on stack screen)
    assert_eq!(post_move.len(), 2, "expected move + retile, got {}", post_move.len());

    // The retile should only position window 2 (on the stack screen), not window 1
    let retile_window_ids: Vec<u64> = post_move.iter().filter(|c| c.1 == 0).map(|c| c.0).collect();
    assert!(
        retile_window_ids.contains(&2),
        "window 2 should be retiled on stack screen"
    );
    assert!(
        !retile_window_ids.contains(&1),
        "window 1 should not be retiled on stack screen after moving away"
    );
}

// 11. Move then close: cleanup works for moved windows
#[tokio::test]
async fn should_handle_close_of_moved_window() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Move window 1 to monitor 1
    engine.handle_focus_changed(1);
    engine.move_window_to_monitor(1).await.unwrap();

    // Close the moved window — should not panic or error
    let result = engine.handle_window_closed(1).await;
    assert!(result.is_ok(), "closing a moved window should succeed");
}

// 12. Move one of multiple windows: source monitor retiles remaining windows
#[tokio::test]
async fn should_retile_source_after_moving_one_of_multiple_windows() {
    // Three windows on monitor 0 (stack screen)
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 3, title: "C".into(), app_class: "c".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy_with_windows(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // After startup, 3 windows were tiled. Record the call count.
    let calls_after_startup = engine.proxy().move_resize_calls().len();

    // Move window 2 to monitor 1
    engine.handle_focus_changed(2);
    engine.move_window_to_monitor(1).await.unwrap();

    // After the move, the source stack screen should have been retiled
    // with the remaining 2 windows (1 and 3)
    let calls = engine.proxy().move_resize_calls();
    let post_move_calls: Vec<_> = calls.iter().skip(calls_after_startup).collect();

    // Should have: 1 call to move window 2, plus retile calls for remaining windows
    assert!(
        post_move_calls.len() >= 3,
        "expected at least 3 calls after move (1 move + 2 retile), got {}",
        post_move_calls.len()
    );

    // Verify window 2 was moved to monitor 1's position
    let move_call = post_move_calls.iter().find(|c| c.0 == 2 && c.1 == 1920);
    assert!(
        move_call.is_some(),
        "window 2 should have been moved to monitor 1 (x=1920)"
    );
}
