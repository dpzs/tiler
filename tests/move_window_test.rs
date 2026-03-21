use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo};
use tiler::tiling::engine::TilingEngine;

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
    let mut engine = TilingEngine::new(proxy, 0);
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
    // Arrange
    let proxy = make_proxy(two_monitors());
    let mut engine = TilingEngine::new(proxy, 0);
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
    // Arrange
    let proxy = make_proxy(two_monitors());
    let mut engine = TilingEngine::new(proxy, 0);
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
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Act — window 12345 is not tracked by the engine
    engine.handle_focus_changed(12345);

    // Assert — should still store it; filtering happens elsewhere
    assert_eq!(
        engine.focused_window_id(),
        Some(12345),
        "handle_focus_changed should work even for windows not in the tracked set"
    );
}
