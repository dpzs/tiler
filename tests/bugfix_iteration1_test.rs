//! Tests for bug fixes in iteration 1 of the code quality improvement loop.
//!
//! Covers:
//! - move_window_to_monitor no-op when source == target
//! - handle_fullscreen_changed re-applies layout presets on non-stack monitors
//! - handle_window_closed avoids double-tiling when stack monitor has a preset
//! - handle_window_closed clears focused_window_id when the closed window was focused
//! - VirtualDesktop::prune_orphaned_windows removes stale entries
//! - handle_window_opened prefers preset over stack tiling

use std::collections::HashSet;

use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::model::{LayoutPreset, VirtualDesktop};
use tiler::tiling::engine::TilingEngine;
use tiler::config::StackScreenPosition;

// ===========================================================================
// Helpers
// ===========================================================================

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

// ===========================================================================
// 1. move_window_to_monitor: no-op when source == target
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

    // Move window to the same monitor it's already on
    engine.move_window_to_monitor(0).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before, 0,
        "moving a window to its current monitor should produce no move_resize calls"
    );
}

// ===========================================================================
// 2. handle_fullscreen_changed re-applies layout presets
// ===========================================================================

#[tokio::test]
async fn fullscreen_on_reapplies_preset_on_non_stack_monitor() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);
    // stack screen is monitor 0, windows are on monitor 1
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a layout preset on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Window 1 goes fullscreen: it should be removed from the stack,
    // and the preset should be re-applied for the remaining window (2).
    engine.handle_fullscreen_changed(1, true).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after > calls_before,
        "going fullscreen should re-apply the preset on the window's monitor"
    );

    // The re-applied layout should position window 2 (the only remaining
    // non-fullscreen window on monitor 1) using the SideBySide preset.
    let last_call = &engine.proxy().move_resize_calls()[calls_before];
    assert_eq!(last_call.0, 2, "re-applied layout should target window 2");
    assert_eq!(last_call.1, 1920, "window 2 should be on monitor 1 (x=1920)");
}

#[tokio::test]
async fn fullscreen_off_reapplies_preset_on_non_stack_monitor() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let mut proxy = make_proxy(monitors, windows);
    proxy.set_fullscreen(1, true);

    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a layout preset on monitor 1
    engine.desktop_mut(0).set_layout(1, LayoutPreset::Fullscreen);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Window 1 exits fullscreen
    engine.handle_fullscreen_changed(1, false).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after > calls_before,
        "exiting fullscreen should re-apply the preset on the window's monitor"
    );

    // The re-applied Fullscreen preset should position window 1
    let last_call = &engine.proxy().move_resize_calls()[calls_before];
    assert_eq!(last_call.0, 1, "re-applied layout should target window 1");
    assert_eq!(last_call.1, 1920, "window should be on monitor 1 (x=1920)");
}

// ===========================================================================
// 3. handle_window_closed: no double-tiling when stack monitor has preset
// ===========================================================================

#[tokio::test]
async fn window_closed_on_stack_with_preset_applies_preset_only() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);
    // Stack screen is monitor 0
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a preset on the stack monitor (unusual but possible)
    engine.desktop_mut(0).set_layout(0, LayoutPreset::SideBySide);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Close window 1
    engine.handle_window_closed(1).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();

    // Should only have layout re-application calls, not both tile_stack
    // AND apply_layout (which would cause conflicting positioning).
    // With 1 remaining window, SideBySide gives 1 call.
    let new_calls = calls_after - calls_before;
    assert_eq!(
        new_calls, 1,
        "closing a window on stack monitor with preset should only apply preset (got {} calls)",
        new_calls
    );

    // Verify the call targets window 2 on monitor 0
    let call = &engine.proxy().move_resize_calls()[calls_before];
    assert_eq!(call.0, 2, "remaining window 2 should be positioned");
}

// ===========================================================================
// 4. handle_window_closed clears focused_window_id
// ===========================================================================

#[tokio::test]
async fn window_closed_clears_focused_window_id() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Focus window 1, then close it
    engine.handle_focus_changed(1);
    assert_eq!(engine.focused_window_id(), Some(1));

    engine.handle_window_closed(1).await.unwrap();

    // focused_window_id should be cleared
    assert_eq!(
        engine.focused_window_id(),
        None,
        "closing the focused window should clear focused_window_id"
    );
}

#[tokio::test]
async fn window_closed_does_not_clear_focus_for_other_window() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Focus window 1, then close window 2
    engine.handle_focus_changed(1);
    engine.handle_window_closed(2).await.unwrap();

    // Focus should still be on window 1
    assert_eq!(
        engine.focused_window_id(),
        Some(1),
        "closing a different window should not clear focused_window_id"
    );
}

// ===========================================================================
// 5. VirtualDesktop::prune_orphaned_windows
// ===========================================================================

#[test]
fn prune_orphaned_windows_removes_stale_ids() {
    let mut desktop = VirtualDesktop::new(0);
    desktop.push_window(1);
    desktop.push_window(2);
    desktop.push_window(3);

    // Only window 2 is still alive
    let live: HashSet<u64> = [2].into_iter().collect();
    desktop.prune_orphaned_windows(&live);

    assert_eq!(
        desktop.stack_windows, vec![2],
        "only live windows should remain after pruning"
    );
}

#[test]
fn prune_orphaned_windows_noop_when_all_live() {
    let mut desktop = VirtualDesktop::new(0);
    desktop.push_window(1);
    desktop.push_window(2);

    let live: HashSet<u64> = [1, 2].into_iter().collect();
    desktop.prune_orphaned_windows(&live);

    assert_eq!(desktop.stack_windows.len(), 2, "no windows should be removed");
}

#[test]
fn prune_orphaned_windows_handles_empty_stack() {
    let mut desktop = VirtualDesktop::new(0);
    let live: HashSet<u64> = [1, 2].into_iter().collect();
    desktop.prune_orphaned_windows(&live);

    assert!(desktop.stack_windows.is_empty(), "empty stack should remain empty");
}

// ===========================================================================
// 6. Orphaned windows are pruned during tile_stack
// ===========================================================================

#[tokio::test]
async fn tile_stack_prunes_orphaned_window_ids() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Manually inject an orphaned window ID into the desktop stack
    engine.desktop_mut(0).push_window(999);

    // Verify orphan is there
    assert!(engine.desktop_ref(0).unwrap().stack_windows.contains(&999));

    // Open a new window to trigger tile_stack
    engine.handle_window_opened(2, "B".into(), "b".into(), 0).await.unwrap();

    // After tiling, the orphan should be pruned
    assert!(
        !engine.desktop_ref(0).unwrap().stack_windows.contains(&999),
        "orphaned window ID should be pruned after tile_stack"
    );
}

// ===========================================================================
// 7. handle_window_opened: preset takes priority over stack tiling
// ===========================================================================

#[tokio::test]
async fn new_window_on_stack_with_preset_uses_preset() {
    let monitors = two_monitors();
    let proxy = make_proxy(monitors, vec![]);
    // stack screen is monitor 0
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Set a preset on the stack monitor
    engine.desktop_mut(0).set_layout(0, LayoutPreset::SideBySide);

    // Open a window on the stack monitor (which also has a preset)
    engine.handle_window_opened(1, "A".into(), "a".into(), 0).await.unwrap();

    // The SideBySide preset should be applied. With 1 window, it gets the
    // left half (x=0, width=960).
    let calls = engine.proxy().move_resize_calls();
    let last = calls.last().unwrap();
    assert_eq!(last.0, 1, "preset should position window 1");
    assert_eq!(last.3, 960, "SideBySide with 1 window should give width=960");
}

// ===========================================================================
// 8. handle_fullscreen_changed: only tile_stack for stack-monitor windows
// ===========================================================================

#[tokio::test]
async fn fullscreen_change_on_non_stack_monitor_does_not_tile_stack() {
    let monitors = two_monitors();
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ];
    let proxy = make_proxy(monitors, windows);
    let mut engine = TilingEngine::new(proxy, StackScreenPosition::Left);
    engine.startup().await.unwrap();

    // Window 1 is on the stack screen (monitor 0), window 2 is on monitor 1
    let calls_before = engine.proxy().move_resize_calls().len();

    // Window 2 (on monitor 1, no preset) goes fullscreen
    engine.handle_fullscreen_changed(2, true).await.unwrap();

    let calls_after = engine.proxy().move_resize_calls().len();
    // No preset on monitor 1, and window 2 is not on the stack screen,
    // so no tiling should occur.
    assert_eq!(
        calls_after - calls_before, 0,
        "fullscreen change on non-stack monitor without preset should not trigger tiling"
    );
}
