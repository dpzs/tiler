//! Lane 5 integration tests — verify cross-slice behavior between the tiling
//! engine, menu command flow, layout enforcement, and daemon IPC routing.

use std::path::PathBuf;
use std::time::Duration;

use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};
use tiler::menu::state::{MenuInput, MenuState};
use tiler::model::{LayoutPreset, Rect};
use tiler::tiling::engine::TilingEngine;
use tiler::tiling::preset::apply_side_by_side;

// ===========================================================================
// Helpers
// ===========================================================================

fn two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]
}

fn monitor_rect(m: &MonitorInfo) -> Rect {
    Rect { x: m.x, y: m.y, width: m.width, height: m.height }
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

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tiler-lane5-integ-{}-{}.sock",
        name,
        std::process::id()
    ))
}

// ===========================================================================
// 1. Full menu → layout → enforcement flow at engine level
// ===========================================================================

/// End-to-end flow: open menu, zoom into monitor, apply SideBySide layout,
/// enable enforcement, then change a window's geometry and verify snap-back.
#[tokio::test]
async fn should_snap_back_after_menu_applies_layout_and_enables_enforcement() {
    // Arrange — 2 windows on monitor 1
    let monitors = two_monitors();
    let mon1 = monitors[1].clone();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);
    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();
    engine.desktop_mut(0).append_window(1);
    engine.desktop_mut(0).append_window(2);

    // Act — use menu to apply SideBySide on monitor 1
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(1));

    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap(); // SideBySide
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Verify layout was set
    let desktop = engine.desktop_ref(0).unwrap();
    assert_eq!(desktop.get_layout(1), Some(LayoutPreset::SideBySide));

    // Record move_resize calls so far (startup stack tiling + layout application)
    let calls_after_layout = engine.proxy().move_resize_calls().len();

    // Now enable enforcement via menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    engine.handle_menu_input(MenuInput::Digit(9)).await.unwrap(); // EnforceOn
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Verify enforcement is enabled
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.is_enforced(1));

    // Record baseline
    let calls_before_geo = engine.proxy().move_resize_calls().len();
    assert_eq!(calls_before_geo, calls_after_layout, "enforcement toggle alone should not produce move_resize calls");

    // Compute expected position for window 1
    let mon1_rect = monitor_rect(&mon1);
    let expected = apply_side_by_side(&[1, 2], mon1_rect);
    let expected_rect = expected[0].1;

    // Manually "move" window 1 to a wrong position — should trigger snap-back
    engine
        .handle_geometry_changed(1, 100, 200, 500, 400)
        .await
        .unwrap();

    let calls_after_geo = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after_geo - calls_before_geo, 1,
        "exactly one snap-back call expected"
    );

    // Verify snap-back coordinates
    let snap = &engine.proxy().move_resize_calls()[calls_before_geo];
    assert_eq!(snap.0, 1, "snap-back targets window 1");
    assert_eq!(snap.1, expected_rect.x);
    assert_eq!(snap.2, expected_rect.y);
    assert_eq!(snap.3, expected_rect.width);
    assert_eq!(snap.4, expected_rect.height);
}

// ===========================================================================
// 2. Window lifecycle with enforcement active
// ===========================================================================

/// With enforcement and layout set up front, opening a new window should
/// position it according to the layout. Closing a window should reposition
/// the remaining ones. Geometry changes should snap back.
#[tokio::test]
async fn should_enforce_layout_through_window_open_close_and_geometry_change() {
    // Arrange — start with 1 window on monitor 1, set SideBySide + enforcement
    let monitors = two_monitors();
    let mon1 = monitors[1].clone();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);
    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();
    engine.desktop_mut(0).append_window(1);

    // Set layout and enforcement directly
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);
    engine.desktop_mut(0).set_enforcement(1, true);

    let calls_baseline = engine.proxy().move_resize_calls().len();

    // Open a second window on monitor 1
    engine.proxy_mut().set_window_type(2, "toplevel".into());
    engine
        .handle_window_opened(2, "B".into(), "b".into(), 1)
        .await
        .unwrap();

    // Window 2 was opened, which triggers stack retiling
    let calls_after_open = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after_open > calls_baseline,
        "opening a window should trigger tiling"
    );

    // Now change geometry of window 1 — enforcement should snap it back
    let calls_before_snap = engine.proxy().move_resize_calls().len();
    engine
        .handle_geometry_changed(1, 0, 0, 100, 100)
        .await
        .unwrap();

    let calls_after_snap = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after_snap - calls_before_snap, 1,
        "enforcement should snap window 1 back"
    );

    // Close window 2 — remaining window should be retiled
    let calls_before_close = engine.proxy().move_resize_calls().len();
    engine.handle_window_closed(2).await.unwrap();

    let calls_after_close = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after_close > calls_before_close,
        "closing a window should retile remaining windows"
    );

    // After closing, verify geometry change on window 1 still enforces
    // (with only 1 window in SideBySide, it gets the left-half slot)
    let mon1_rect = monitor_rect(&mon1);
    let expected_single = apply_side_by_side(&[1], mon1_rect);
    let expected_rect = expected_single[0].1;

    let calls_before_final = engine.proxy().move_resize_calls().len();
    engine
        .handle_geometry_changed(1, 999, 999, 10, 10)
        .await
        .unwrap();

    let calls_after_final = engine.proxy().move_resize_calls().len();
    assert_eq!(calls_after_final - calls_before_final, 1, "snap-back after close");

    let final_snap = &engine.proxy().move_resize_calls()[calls_before_final];
    assert_eq!(final_snap.0, 1);
    assert_eq!(final_snap.1, expected_rect.x);
    assert_eq!(final_snap.2, expected_rect.y);
    assert_eq!(final_snap.3, expected_rect.width);
    assert_eq!(final_snap.4, expected_rect.height);
}

// ===========================================================================
// 3. Daemon multiple command sequence
// ===========================================================================

/// Send a complete sequence of commands through the daemon IPC:
/// Menu (open) → Menu (close) → Status → Menu (open again) → Shutdown.
/// All should return Ok and the daemon should exit cleanly.
#[tokio::test]
async fn should_handle_full_ipc_command_sequence() {
    let sock = test_socket_path("seq");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let proxy = make_proxy(two_monitors(), vec![]);
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Menu (open)
    let r1 = send_command(&sock, Command::Menu).await.expect("Menu open");
    assert_eq!(r1, Response::Ok);

    // Menu (close)
    let r2 = send_command(&sock, Command::Menu).await.expect("Menu close");
    assert_eq!(r2, Response::Ok);

    // Status
    let r3 = send_command(&sock, Command::Status).await.expect("Status");
    assert_eq!(r3, Response::Ok);

    // Menu (open again)
    let r4 = send_command(&sock, Command::Menu).await.expect("Menu reopen");
    assert_eq!(r4, Response::Ok);

    // Shutdown
    let r5 = send_command(&sock, Command::Shutdown).await.expect("Shutdown");
    assert_eq!(r5, Response::Ok);

    // Daemon should exit
    let result = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    assert!(result.is_ok(), "daemon should exit after Shutdown");
    assert!(!sock.exists(), "socket should be removed");
}

// ===========================================================================
// 4. Layout persistence across workspace changes
// ===========================================================================

/// Set layout on workspace 0, switch to workspace 1 and set a different
/// layout, switch back to workspace 0, and verify the original layout
/// is preserved.
#[tokio::test]
async fn should_preserve_layout_across_workspace_switches() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 1 },
    ];
    let proxy = make_proxy(monitors, windows);
    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    // Set SideBySide on workspace 0, monitor 1
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap(); // SideBySide

    // Verify
    assert_eq!(
        engine.desktop_ref(0).unwrap().get_layout(1),
        Some(LayoutPreset::SideBySide)
    );

    // Switch to workspace 1
    engine.handle_workspace_changed(1).await.unwrap();
    assert_eq!(engine.active_workspace(), 1);

    // Set Quadrants on workspace 1, monitor 1
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    engine.handle_menu_input(MenuInput::Digit(4)).await.unwrap(); // Quadrants

    // Verify workspace 1 layout
    assert_eq!(
        engine.desktop_ref(1).unwrap().get_layout(1),
        Some(LayoutPreset::Quadrants)
    );

    // Switch back to workspace 0
    engine.handle_workspace_changed(0).await.unwrap();
    assert_eq!(engine.active_workspace(), 0);

    // Workspace 0 layout should still be SideBySide
    assert_eq!(
        engine.desktop_ref(0).unwrap().get_layout(1),
        Some(LayoutPreset::SideBySide),
        "workspace 0 layout should be preserved after switching away and back"
    );

    // Workspace 1 layout should still be Quadrants
    assert_eq!(
        engine.desktop_ref(1).unwrap().get_layout(1),
        Some(LayoutPreset::Quadrants),
        "workspace 1 layout should be preserved"
    );
}

/// Enable enforcement on workspace 0, switch to workspace 1 (no enforcement),
/// switch back and verify enforcement is still active.
#[tokio::test]
async fn should_preserve_enforcement_across_workspace_switches() {
    let monitors = two_monitors();
    let mon1 = monitors[1].clone();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 3, title: "C".into(), app_class: "c".into(), monitor_id: 1, workspace_id: 1 },
    ];
    let proxy = make_proxy(monitors, windows);
    let mut engine = TilingEngine::new(proxy, 1);
    engine.startup().await.unwrap();

    // Set SideBySide + enforcement on workspace 0, monitor 1
    engine.desktop_mut(0).append_window(1);
    engine.desktop_mut(0).append_window(2);
    engine.desktop_mut(1).append_window(3);
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);
    engine.desktop_mut(0).set_enforcement(1, true);

    // Switch to workspace 1 — no enforcement set here
    engine.handle_workspace_changed(1).await.unwrap();
    assert!(!engine.desktop_ref(1).map_or(false, |d| d.is_enforced(1)));

    // Geometry change on workspace 1 window should NOT snap back
    let calls_before = engine.proxy().move_resize_calls().len();
    engine.handle_geometry_changed(3, 500, 500, 200, 200).await.unwrap();
    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(calls_after - calls_before, 0, "no enforcement on workspace 1");

    // Switch back to workspace 0
    engine.handle_workspace_changed(0).await.unwrap();

    // Enforcement should still be active — geometry change should snap back
    let calls_before_snap = engine.proxy().move_resize_calls().len();
    engine.handle_geometry_changed(1, 0, 0, 100, 100).await.unwrap();
    let calls_after_snap = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after_snap - calls_before_snap, 1,
        "enforcement should still be active on workspace 0 after switching back"
    );

    // Verify snap-back target
    let mon1_rect = monitor_rect(&mon1);
    let expected = apply_side_by_side(&[1, 2], mon1_rect);
    let snap = &engine.proxy().move_resize_calls()[calls_before_snap];
    assert_eq!(snap.0, 1);
    assert_eq!(snap.1, expected[0].1.x);
    assert_eq!(snap.2, expected[0].1.y);
}

// ===========================================================================
// 5. Daemon with signal shutdown after IPC activity
// ===========================================================================

/// Send several IPC commands, then trigger external shutdown via the
/// oneshot channel. Daemon should exit cleanly after having processed
/// real commands.
#[tokio::test]
async fn should_shutdown_via_signal_after_processing_commands() {
    let sock = test_socket_path("sig-after");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let proxy = make_proxy(two_monitors(), vec![]);
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, Some(rx), None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Process some commands
    let r1 = send_command(&sock, Command::Status).await.expect("Status");
    assert_eq!(r1, Response::Ok);

    let r2 = send_command(&sock, Command::Menu).await.expect("Menu");
    assert_eq!(r2, Response::Ok);

    // Shutdown via signal channel (not IPC Shutdown)
    tx.send(()).expect("signal send");

    let result = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    assert!(result.is_ok(), "daemon should exit on signal after processing commands");
    assert!(!sock.exists(), "socket should be cleaned up");
}
