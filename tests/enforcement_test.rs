use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::model::LayoutPreset;
use tiler::tiling::engine::TilingEngine;
use tiler::tiling::preset::apply_side_by_side;
use tiler::model::Rect;

// --- Helpers ---

fn two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
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

fn monitor_rect(monitor: &MonitorInfo) -> Rect {
    Rect {
        x: monitor.x,
        y: monitor.y,
        width: monitor.width,
        height: monitor.height,
    }
}

// --- Tests ---

/// When enforcement is disabled (default), geometry changes should be ignored.
/// No move_resize_window calls should be made.
#[tokio::test]
async fn should_not_snap_back_when_enforcement_disabled() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Enforcement is disabled by default — geometry change should be a no-op
    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "no snap-back should occur when enforcement is disabled"
    );
}

/// When enforcement is enabled but no layout preset is assigned,
/// geometry changes should be ignored.
#[tokio::test]
async fn should_not_snap_back_when_enforcement_enabled_but_no_layout() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    // Enable enforcement on monitor 1 but set no layout preset
    engine.desktop_mut(0).set_enforcement(1, true);

    let calls_before = engine.proxy().move_resize_calls().len();

    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "no snap-back when enforcement is on but no layout preset is set"
    );
}

/// When enforcement is enabled and a layout preset is assigned,
/// a window that has been manually moved should be snapped back
/// to its assigned layout position.
#[tokio::test]
async fn should_snap_back_when_enforcement_enabled_and_layout_set() {
    let monitors = two_monitors();
    let mon1 = monitors[1].clone();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    // Enable enforcement and set SideBySide layout on monitor 1
    engine.desktop_mut(0).set_enforcement(1, true);
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    // Clear the post-tiling grace period from startup so enforcement is active
    engine.clear_tiling_grace();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Compute expected positions using the same preset function the engine should use
    let mon1_rect = monitor_rect(&mon1);
    let expected = apply_side_by_side(&[1, 2], mon1_rect);
    let expected_rect = expected[0].1; // window 1's expected position

    // Send a geometry change that differs from the expected layout position
    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        1,
        "should make exactly one move_resize call to snap window back"
    );

    // Verify the snap-back call matches the expected layout position
    let snap_call = &engine.proxy().move_resize_calls()[calls_before];
    assert_eq!(snap_call.0, 1, "snap-back should target window 1");
    assert_eq!(snap_call.1, expected_rect.x, "snap-back x");
    assert_eq!(snap_call.2, expected_rect.y, "snap-back y");
    assert_eq!(snap_call.3, expected_rect.width, "snap-back width");
    assert_eq!(snap_call.4, expected_rect.height, "snap-back height");
}

/// When handle_geometry_changed is called with an unknown window_id,
/// it should return Ok(()) without panicking or making any calls.
#[tokio::test]
async fn should_not_crash_for_unknown_window() {
    let monitors = two_monitors();
    let proxy = make_proxy(monitors, vec![]);

    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Unknown window — should be a safe no-op
    let result = engine
        .handle_geometry_changed(999, 0, 0, 100, 100)
        .await;
    assert!(result.is_ok(), "unknown window should return Ok, not error");

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "no move_resize calls for unknown window"
    );
}

/// When a window's current geometry already matches the expected
/// layout position, no unnecessary move_resize call should be made.
#[tokio::test]
async fn should_not_snap_when_geometry_already_correct() {
    let monitors = two_monitors();
    let mon1 = monitors[1].clone();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);

    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    // Enable enforcement and set SideBySide layout on monitor 1
    engine.desktop_mut(0).set_enforcement(1, true);
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Compute expected position for window 1
    let mon1_rect = monitor_rect(&mon1);
    let expected = apply_side_by_side(&[1, 2], mon1_rect);
    let expected_rect = expected[0].1;

    // Send a geometry change that exactly matches the expected layout position
    engine
        .handle_geometry_changed(
            1,
            expected_rect.x,
            expected_rect.y,
            expected_rect.width,
            expected_rect.height,
        )
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "no snap-back needed when geometry already matches layout"
    );
}
