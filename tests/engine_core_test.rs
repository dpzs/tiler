use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::model::LayoutPreset;
use tiler::tiling::engine::TilingEngine;

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

// --- Startup tiling ---

#[tokio::test]
async fn startup_tiles_existing_windows_on_stack_screen() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    // stack_screen_index=0 means monitor 0 is the stack screen
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Should have called move_resize for both windows on the stack screen
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 2);

    // Both windows should be on monitor 0 (stack screen)
    // Window 1 on top half, window 2 on bottom half (stack layout)
    assert_eq!(calls[0].0, 1);
    assert_eq!(calls[1].0, 2);
}

#[tokio::test]
async fn startup_skips_fullscreen_windows() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "FS".into(), app_class: "fs".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors, windows);
    proxy.set_fullscreen(2, true);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Only window 1 should be tiled
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

#[tokio::test]
async fn startup_skips_dialog_windows() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "Dialog".into(), app_class: "d".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors, windows);
    proxy.set_window_type(2, "dialog".into());

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

// --- New window ---

#[tokio::test]
async fn new_window_added_to_stack_and_retiled() {
    let monitors = two_monitors();
    let proxy = make_proxy(monitors, vec![]);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open a new normal window
    engine.handle_window_opened(1, "Term".into(), "terminal".into(), 0).await.unwrap();

    let calls = engine.proxy().move_resize_calls();
    // Single window = single move_resize
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

#[tokio::test]
async fn new_window_retiles_all_stack_windows() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // 1 call for startup (window 1)
    let startup_calls = engine.proxy().move_resize_calls().len();
    assert_eq!(startup_calls, 1);

    // Open window 2
    engine.handle_window_opened(2, "B".into(), "b".into(), 0).await.unwrap();

    // Should now have 1 (startup) + 2 (retile both) = 3 calls
    let total_calls = engine.proxy().move_resize_calls().len();
    assert_eq!(total_calls, 3);
}

#[tokio::test]
async fn new_fullscreen_window_ignored() {
    let monitors = two_monitors();
    let proxy = make_proxy(monitors, vec![]);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Mark window as fullscreen before opening
    engine.proxy_mut().set_fullscreen(1, true);
    engine.handle_window_opened(1, "FS".into(), "fs".into(), 0).await.unwrap();

    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 0);
}

// --- Close window ---

#[tokio::test]
async fn close_window_retiles_remaining() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // 2 calls from startup
    let startup_calls = engine.proxy().move_resize_calls().len();
    assert_eq!(startup_calls, 2);

    // Close window 1
    engine.handle_window_closed(1).await.unwrap();

    // Should retile window 2 alone = 1 more call
    let total_calls = engine.proxy().move_resize_calls().len();
    assert_eq!(total_calls, 3);
}

#[tokio::test]
async fn close_unknown_window_is_noop() {
    let monitors = two_monitors();
    let proxy = make_proxy(monitors, vec![]);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    engine.handle_window_closed(999).await.unwrap();
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 0);
}

// --- Virtual desktop isolation ---

#[tokio::test]
async fn windows_on_different_workspaces_isolated() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Only workspace 0 is active by default, so only window 1 should be tiled
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

// --- Workspace change ---

#[tokio::test]
async fn workspace_change_retiles_new_workspace() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // 1 call for window 1 on ws 0
    assert_eq!(engine.proxy().move_resize_calls().len(), 1);

    // Switch to workspace 1
    engine.handle_workspace_changed(1).await.unwrap();

    // Should tile window 2 on ws 1
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].0, 2);
}

// --- Fullscreen change ---

#[tokio::test]
async fn fullscreen_on_removes_from_stack_and_retiles() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();
    assert_eq!(engine.proxy().move_resize_calls().len(), 2);

    // Window 1 goes fullscreen
    engine.handle_fullscreen_changed(1, true).await.unwrap();

    // Window 2 should be retiled alone
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[2].0, 2);
}

#[tokio::test]
async fn fullscreen_off_adds_back_to_stack_and_retiles() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors, windows);
    proxy.set_fullscreen(1, true);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();
    // Fullscreen window not tiled on startup
    assert_eq!(engine.proxy().move_resize_calls().len(), 0);

    // Window exits fullscreen
    engine.handle_fullscreen_changed(1, false).await.unwrap();

    // Should now be tiled
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, 1);
}

// --- is_tiling guard ---

#[tokio::test]
async fn is_tiling_is_false_after_startup() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // After startup completes, is_tiling should be false
    assert!(!engine.is_tiling(), "is_tiling should be false after startup completes");
}

#[tokio::test]
async fn is_tiling_is_false_after_tile_stack_completes() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap(); // startup calls tile_stack internally

    // After tile_stack has completed (via startup), is_tiling should be false
    assert!(!engine.is_tiling(), "is_tiling should be false after tile_stack completes");
}

#[tokio::test]
async fn geometry_changed_during_tiling_is_suppressed() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Set up enforcement + layout on monitor 0 so snap-back would normally fire
    engine.desktop_mut(0).set_enforcement(0, true);
    engine.desktop_mut(0).set_layout(0, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Simulate being mid-tiling: set the guard
    engine.set_tiling(true);

    // Geometry change while tiling — should be suppressed (no snap-back)
    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_during = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_during - calls_before,
        0,
        "no snap-back should occur while is_tiling is true"
    );

    // Clear the guard and the grace period
    engine.set_tiling(false);
    engine.clear_tiling_grace();

    // Now the same geometry change should trigger a snap-back
    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_during,
        1,
        "snap-back should occur when is_tiling is false and enforcement is active"
    );
}

// --- Monitor guard in startup ---

#[tokio::test]
async fn startup_only_stacks_windows_on_stack_screen() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 3, title: "C".into(), app_class: "c".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    // stack_screen_index=0 means only monitor 0 windows should be tiled
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Only windows 1 and 2 (on monitor 0) should have been moved
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 2, "only stack-screen windows should be tiled at startup");
}

#[tokio::test]
async fn startup_tracks_all_toplevel_windows_in_desktop() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Both windows should be in the desktop stack (all toplevel windows are tracked)
    let desktop = engine.desktop_mut(0);
    assert_eq!(desktop.stack_windows, vec![1, 2], "all toplevel windows should be in desktop stack");

    // But only window 1 (on stack screen 0) should have been tiled
    let calls = engine.proxy().move_resize_calls();
    assert_eq!(calls.len(), 1, "only stack-screen window should be tiled at startup");
}

