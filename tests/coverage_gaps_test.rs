//! Tests filling coverage gaps identified during test quality review (iteration 9).
//!
//! Each section targets a specific untested scenario in the tiling engine,
//! model layer, or config module.

use std::collections::HashSet;

use tiler::config::StackScreenPosition;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::model::{LayoutPreset, VirtualDesktop};
use tiler::tiling::engine::TilingEngine;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn two_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]
}

fn three_monitors() -> Vec<MonitorInfo> {
    vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 2560, height: 1440 },
        MonitorInfo { id: 2, name: "DP-3".into(), x: 4480, y: 0, width: 1920, height: 1080 },
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

// ===========================================================================
// 1. Engine: startup() called twice should re-initialize cleanly
// ===========================================================================

#[tokio::test]
async fn startup_called_twice_should_reinitialize() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);

    // First startup
    engine.startup().await.unwrap();
    let calls_after_first = engine.proxy().move_resize_calls().len();
    assert_eq!(calls_after_first, 1, "first startup should tile 1 window");

    // Second startup: should not panic, should re-tile
    engine.startup().await.unwrap();
    let calls_after_second = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after_second > calls_after_first,
        "second startup should produce additional move_resize calls"
    );
}

// ===========================================================================
// 2. Engine: handle_workspace_changed with the *same* workspace
// ===========================================================================

#[tokio::test]
async fn workspace_changed_to_same_workspace_no_reflow() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // "Switch" to the same workspace — already visited, should NOT retile
    engine.handle_workspace_changed(0).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after, calls_before,
        "workspace_changed to already-visited workspace should NOT retile"
    );
    assert_eq!(engine.active_workspace(), 0);
}

// ===========================================================================
// 3. Engine: workspace change re-applies layout presets on non-stack monitors
// ===========================================================================

#[tokio::test]
async fn workspace_change_does_not_reapply_layout_presets() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a layout preset on monitor 1 for workspace 0
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Switch away and back — both workspaces already visited after first switch
    engine.handle_workspace_changed(1).await.unwrap();
    // ws 1 is first visit → tiles stack (no windows on stack for ws 1, so no calls)
    let calls_after_first = engine.proxy().move_resize_calls().len();

    engine.handle_workspace_changed(0).await.unwrap();
    let calls_after_second = engine.proxy().move_resize_calls().len();

    // Switching back to ws 0 (already visited) should NOT re-apply presets
    assert_eq!(
        calls_after_second, calls_after_first,
        "returning to already-visited workspace should NOT retile or re-apply presets"
    );
}

// ===========================================================================
// 4. Engine: handle_window_closed clears focused_window_id
// ===========================================================================

#[tokio::test]
async fn close_focused_window_clears_focus() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    engine.handle_focus_changed(1);
    assert_eq!(engine.focused_window_id(), Some(1));

    engine.handle_window_closed(1).await.unwrap();

    assert_eq!(
        engine.focused_window_id(),
        None,
        "closing the focused window should clear focused_window_id"
    );
}

// ===========================================================================
// 5. Engine: move_window_to_monitor when already on target is a no-op
// ===========================================================================

#[tokio::test]
async fn move_window_to_same_monitor_is_noop() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    engine.handle_focus_changed(1);
    let calls_before = engine.proxy().move_resize_calls().len();

    // Move window to its current monitor
    engine.move_window_to_monitor(0).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "moving a window to its current monitor should produce no move_resize calls"
    );
}

// ===========================================================================
// 6. Engine: prune_desktop removes orphaned windows from stack
// ===========================================================================

#[tokio::test]
async fn prune_desktop_removes_orphaned_entries() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Both windows should be in desktop stack
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.stack_windows.contains(&1));
    assert!(desktop.stack_windows.contains(&2));

    // Close window 2 so it's removed from engine tracking
    engine.handle_window_closed(2).await.unwrap();

    // After close, window 2 should be removed from the stack
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(
        !desktop.stack_windows.contains(&2),
        "closed window should be pruned from desktop stack_windows"
    );
    assert!(
        desktop.stack_windows.contains(&1),
        "remaining window should still be in desktop stack_windows"
    );
}

// ===========================================================================
// 7. VirtualDesktop: prune_orphaned_windows
// ===========================================================================

#[test]
fn prune_orphaned_windows_removes_stale_ids() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);
    vd.push_window(3);

    // Only windows 1 and 3 are "live"
    let live: HashSet<u64> = [1, 3].into_iter().collect();
    vd.prune_orphaned_windows(&live);

    assert_eq!(
        vd.stack_windows,
        vec![3, 1],
        "window 2 should be pruned, order of remaining windows preserved"
    );
}

#[test]
fn prune_orphaned_windows_empty_live_set_clears_all() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);

    let live: HashSet<u64> = HashSet::new();
    vd.prune_orphaned_windows(&live);

    assert!(
        vd.stack_windows.is_empty(),
        "empty live set should remove all windows"
    );
}

#[test]
fn prune_orphaned_windows_all_live_is_noop() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);

    let live: HashSet<u64> = [1, 2].into_iter().collect();
    vd.prune_orphaned_windows(&live);

    assert_eq!(
        vd.stack_windows,
        vec![2, 1],
        "when all windows are live, nothing should be pruned"
    );
}

// ===========================================================================
// 8. VirtualDesktop: append_window behavior
// ===========================================================================

#[test]
fn append_window_adds_to_back() {
    let mut vd = VirtualDesktop::new(0);
    vd.append_window(1);
    vd.append_window(2);
    vd.append_window(3);

    assert_eq!(
        vd.stack_windows,
        vec![1, 2, 3],
        "append_window should add to back of the stack"
    );
}

#[test]
fn append_existing_window_moves_to_back() {
    let mut vd = VirtualDesktop::new(0);
    vd.append_window(1);
    vd.append_window(2);
    vd.append_window(3);

    // Re-append window 1 -- should move to back
    vd.append_window(1);

    assert_eq!(
        vd.stack_windows,
        vec![2, 3, 1],
        "re-appending a window should move it to the back without duplicates"
    );
}

// ===========================================================================
// 9. StackScreenPosition::resolve_index edge cases
// ===========================================================================

#[test]
fn resolve_index_three_monitors_left_picks_smallest_x() {
    let monitors = three_monitors();
    let idx = StackScreenPosition::Left.resolve_index(&monitors);
    assert_eq!(idx, Some(0), "left should pick monitor at x=0 (index 0)");
}

#[test]
fn resolve_index_three_monitors_right_picks_largest_x() {
    let monitors = three_monitors();
    let idx = StackScreenPosition::Right.resolve_index(&monitors);
    assert_eq!(idx, Some(2), "right should pick monitor at x=4480 (index 2)");
}

#[test]
fn resolve_index_with_negative_x_coordinates() {
    let monitors = vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: -1920, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 0, y: 0, width: 1920, height: 1080 },
    ];
    let idx = StackScreenPosition::Left.resolve_index(&monitors);
    assert_eq!(
        idx,
        Some(0),
        "left should pick the monitor with x=-1920 (smallest x)"
    );

    let idx = StackScreenPosition::Right.resolve_index(&monitors);
    assert_eq!(
        idx,
        Some(1),
        "right should pick the monitor with x=0 (largest x)"
    );
}

#[test]
fn resolve_index_monitors_with_same_x() {
    // Two monitors stacked vertically at the same x coordinate
    let monitors = vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 0, y: 1080, width: 1920, height: 1080 },
    ];
    // Both have the same x; min_by_key picks first, max_by_key picks last.
    // The important thing is that both resolve to a valid index.
    let left = StackScreenPosition::Left.resolve_index(&monitors);
    let right = StackScreenPosition::Right.resolve_index(&monitors);
    assert_eq!(left, Some(0), "min_by_key returns first element on tie");
    assert_eq!(right, Some(1), "max_by_key returns last element on tie");
}

// ===========================================================================
// 10. Config: StackScreenPosition::parse edge cases
// ===========================================================================

#[test]
fn parse_position_mixed_case() {
    assert_eq!(StackScreenPosition::parse("Left").unwrap(), StackScreenPosition::Left);
    assert_eq!(StackScreenPosition::parse("RIGHT").unwrap(), StackScreenPosition::Right);
    assert_eq!(StackScreenPosition::parse("rIgHt").unwrap(), StackScreenPosition::Right);
}

#[test]
fn parse_position_invalid_returns_error() {
    let err = StackScreenPosition::parse("center").unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("center"),
        "error should include the invalid value, got: {msg}"
    );
}

#[test]
fn parse_position_empty_string_returns_error() {
    assert!(
        StackScreenPosition::parse("").is_err(),
        "empty string should be rejected"
    );
}

// ===========================================================================
// 11. Config: ConfigError Display and source chain
// ===========================================================================

#[test]
fn config_error_display_io() {
    use tiler::config::ConfigError;

    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
    let config_err = ConfigError::from(io_err);
    let msg = format!("{config_err}");
    assert!(
        msg.contains("I/O") || msg.contains("no access"),
        "IO error display should mention the cause, got: {msg}"
    );

    // Verify source chain
    use std::error::Error;
    assert!(
        config_err.source().is_some(),
        "IO variant should have a source"
    );
}

#[test]
fn config_error_display_invalid_position() {
    use tiler::config::ConfigError;
    use std::error::Error;

    let err = ConfigError::InvalidStackScreenPosition("top".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("top"), "should include the invalid value");
    assert!(err.source().is_none(), "InvalidStackScreenPosition has no source");
}

// ===========================================================================
// 12. Engine: fullscreen window on non-stack monitor with preset
// ===========================================================================

#[tokio::test]
async fn fullscreen_change_on_monitor_with_preset_retiles() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a layout on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Window 1 goes fullscreen on monitor 1 which has a preset
    engine.handle_fullscreen_changed(1, true).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after > calls_before,
        "fullscreen change on a monitor with a preset should re-apply layout for remaining windows"
    );
}

// ===========================================================================
// 13. Engine: handle_fullscreen_changed for unknown window is no-op
// ===========================================================================

#[tokio::test]
async fn fullscreen_changed_unknown_window_is_noop() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Unknown window goes fullscreen
    let result = engine.handle_fullscreen_changed(999, true).await;
    assert!(result.is_ok());

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "fullscreen change for unknown window should produce no calls"
    );
}

// ===========================================================================
// 14. Engine: close window on monitor with layout preset re-applies layout
// ===========================================================================

#[tokio::test]
async fn close_window_on_preset_monitor_reapplies_layout() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set SideBySide on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Close window 1 from monitor 1 (which has a preset)
    engine.handle_window_closed(1).await.unwrap();

    // Should re-apply layout for remaining window 2
    let calls_after = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after > calls_before,
        "closing a window on a monitor with a preset should re-apply the layout"
    );

    // Verify the re-apply targets window 2 on monitor 1
    let post_calls: Vec<_> = engine
        .proxy()
        .move_resize_calls()
        .iter()
        .skip(calls_before)
        .collect();
    let targeted_window_2 = post_calls.iter().any(|c| c.0 == 2);
    assert!(
        targeted_window_2,
        "re-applied layout should target remaining window 2"
    );
}

// ===========================================================================
// 15. Engine: geometry changed for window on different workspace is no-op
// ===========================================================================

#[tokio::test]
async fn geometry_changed_respects_workspace_isolation() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 1 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Enable enforcement + layout on ws 0, monitor 0
    engine.desktop_mut(0).set_enforcement(0, true);
    engine.desktop_mut(0).set_layout(0, LayoutPreset::SideBySide);
    engine.clear_tiling_grace();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Window 2 is on workspace 1 which has no enforcement
    engine
        .handle_geometry_changed(2, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "geometry change for window on a different workspace (without enforcement) should be ignored"
    );
}

// ===========================================================================
// 16. Engine: menu set_menu_state bypasses menu transitions
// ===========================================================================

#[tokio::test]
async fn set_menu_state_overrides_state_directly() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    assert_eq!(engine.menu_state(), MenuState::Closed);

    use tiler::menu::state::MenuState;
    engine.set_menu_state(MenuState::ZoomedIn(5));
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(5));

    engine.set_menu_state(MenuState::Closed);
    assert_eq!(engine.menu_state(), MenuState::Closed);
}

// ===========================================================================
// 17. Engine: opening dialog/non-toplevel windows does not affect stack
// ===========================================================================

#[tokio::test]
async fn dialog_window_opened_does_not_affect_stack() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Open a dialog window
    engine.proxy_mut().set_window_type(10, "dialog".into());
    engine
        .handle_window_opened(10, "Dialog".into(), "dialog-app".into(), 0)
        .await
        .unwrap();

    // Dialog should not appear in desktop stack
    let desktop = engine.desktop_ref(0);
    let has_window = desktop
        .map(|d| d.stack_windows.contains(&10))
        .unwrap_or(false);
    assert!(
        !has_window,
        "dialog window should not be added to desktop stack_windows"
    );
}

// ===========================================================================
// 18. Engine: closing a non-toplevel window does not retile
// ===========================================================================

#[tokio::test]
async fn closing_dialog_window_does_not_retile() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Open a dialog, then a normal window
    engine.proxy_mut().set_window_type(10, "dialog".into());
    engine
        .handle_window_opened(10, "Dialog".into(), "d".into(), 0)
        .await
        .unwrap();

    engine
        .handle_window_opened(20, "Normal".into(), "n".into(), 0)
        .await
        .unwrap();

    let calls_before = engine.proxy().move_resize_calls().len();

    // Close the dialog
    engine.handle_window_closed(10).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "closing a dialog window should not trigger retiling"
    );
}

// ===========================================================================
// 19. VirtualDesktop: push then remove then push same window
// ===========================================================================

#[test]
fn push_remove_push_same_window() {
    let mut vd = VirtualDesktop::new(0);
    vd.push_window(1);
    vd.push_window(2);

    vd.remove_window(1);
    assert_eq!(vd.stack_windows, vec![2]);

    vd.push_window(1);
    assert_eq!(
        vd.stack_windows,
        vec![1, 2],
        "window 1 should be re-added at the front after removal"
    );
}

// ===========================================================================
// 20. Engine: multiple windows open/close rapidly (stress test)
// ===========================================================================

#[tokio::test]
async fn rapid_open_close_cycle_does_not_corrupt_state() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Open 10 windows rapidly
    for i in 1..=10u64 {
        engine
            .handle_window_opened(i, format!("Win{i}"), "app".into(), 0)
            .await
            .unwrap();
    }

    // Close odd-numbered windows
    for i in (1..=10u64).step_by(2) {
        engine.handle_window_closed(i).await.unwrap();
    }

    // Only even-numbered windows should remain in the stack
    let desktop = engine.desktop_ref(0).unwrap();
    let remaining: Vec<u64> = desktop.stack_windows.clone();
    for id in &remaining {
        assert!(
            id % 2 == 0,
            "only even-numbered windows should remain, found {id}"
        );
    }
    assert_eq!(remaining.len(), 5, "5 even-numbered windows should remain");
}

// ===========================================================================
// 21. Engine: handle_geometry_changed while is_tiling=true is suppressed
//     (verifying the tiling grace period flag behavior end-to-end)
// ===========================================================================

#[tokio::test]
async fn geometry_changed_during_grace_period_is_suppressed() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set enforcement so snap-back would fire
    engine.desktop_mut(0).set_enforcement(0, true);
    engine.desktop_mut(0).set_layout(0, LayoutPreset::Fullscreen);

    // startup() sets last_tiling_end, so we are within the grace period
    // (tests run much faster than 500ms)
    let calls_before = engine.proxy().move_resize_calls().len();

    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "geometry changes within the grace period should be suppressed"
    );

    // Now clear the grace period and verify snap-back fires
    engine.clear_tiling_grace();

    engine
        .handle_geometry_changed(1, 100, 100, 800, 600)
        .await
        .unwrap();

    let calls_final = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_final - calls_after,
        1,
        "geometry change after clearing grace period should trigger snap-back"
    );
}
