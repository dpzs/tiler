use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::menu::state::{MenuInput, MenuState};
use tiler::model::LayoutPreset;
use tiler::tiling::engine::TilingEngine;

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
        proxy.set_window_type(w.id, "normal".into());
    }
    proxy
}

/// Two normal windows on monitor 1, workspace 0.
fn two_windows_on_monitor_1() -> Vec<WindowInfo> {
    vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 1, workspace_id: 0 },
        WindowInfo { id: 2, title: "B".into(), app_class: "b".into(), monitor_id: 1, workspace_id: 0 },
    ]
}

// ===========================================================================
// 1. ToggleMenu opens from Closed
// ===========================================================================

#[tokio::test]
async fn should_transition_to_overview_on_toggle_from_closed() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Menu starts Closed
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Toggle should open to Overview
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);
}

// ===========================================================================
// 2. ToggleMenu closes from Overview (Dismiss)
// ===========================================================================

#[tokio::test]
async fn should_transition_to_closed_on_toggle_from_overview() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open the menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    let calls_before = engine.proxy().move_resize_calls().len();

    // Toggle again should close (Dismiss action — no engine state changes)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Dismiss should not cause any move_resize calls
    let calls_after = engine.proxy().move_resize_calls().len();
    assert_eq!(
        calls_after - calls_before,
        0,
        "dismiss should not trigger any move_resize calls"
    );
}

// ===========================================================================
// 3. PressN zooms into a monitor
// ===========================================================================

#[tokio::test]
async fn should_transition_to_zoomed_in_on_press_n() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu to Overview
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();

    // PressN(1) should zoom into monitor 1
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(1));
}

// ===========================================================================
// 4. ApplyLayout sets preset on desktop and retiles
// ===========================================================================

#[tokio::test]
async fn should_apply_layout_and_retile_on_digit() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Navigate: Closed -> Overview -> ZoomedIn(1)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::ZoomedIn(1));

    let calls_before = engine.proxy().move_resize_calls().len();

    // Digit 2 = SideBySide layout on monitor 1
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap();

    // Menu should transition to Closed
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Layout preset should be set on the desktop
    let desktop = engine.desktop_ref(0).expect("desktop for workspace 0 should exist");
    assert_eq!(
        desktop.get_layout(1),
        Some(LayoutPreset::SideBySide),
        "SideBySide layout should be set for monitor 1"
    );

    // move_resize_window should have been called for the two windows on monitor 1
    let calls_after = engine.proxy().move_resize_calls().len();
    assert!(
        calls_after - calls_before >= 2,
        "expected at least 2 move_resize calls for applying SideBySide to 2 windows, got {}",
        calls_after - calls_before
    );
}

/// Digit 1 = Fullscreen
#[tokio::test]
async fn should_apply_fullscreen_layout_on_digit_1() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Navigate: Closed -> Overview -> ZoomedIn(1)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    // Digit 1 = Fullscreen
    engine.handle_menu_input(MenuInput::Digit(1)).await.unwrap();

    assert_eq!(engine.menu_state(), MenuState::Closed);
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert_eq!(
        desktop.get_layout(1),
        Some(LayoutPreset::Fullscreen),
        "Fullscreen layout should be set for monitor 1"
    );
}

/// Digit 3 = TopBottom
#[tokio::test]
async fn should_apply_top_bottom_layout_on_digit_3() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    engine.handle_menu_input(MenuInput::Digit(3)).await.unwrap();

    assert_eq!(engine.menu_state(), MenuState::Closed);
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert_eq!(
        desktop.get_layout(1),
        Some(LayoutPreset::TopBottom),
        "TopBottom layout should be set for monitor 1"
    );
}

/// Digit 4 = Quadrants
#[tokio::test]
async fn should_apply_quadrants_layout_on_digit_4() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    engine.handle_menu_input(MenuInput::Digit(4)).await.unwrap();

    assert_eq!(engine.menu_state(), MenuState::Closed);
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert_eq!(
        desktop.get_layout(1),
        Some(LayoutPreset::Quadrants),
        "Quadrants layout should be set for monitor 1"
    );
}

// ===========================================================================
// 5. EnforceOn enables enforcement
// ===========================================================================

#[tokio::test]
async fn should_enable_enforcement_on_digit_9() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Enforcement should be off by default
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert!(!desktop.is_enforced(1), "enforcement should be off by default");

    // Navigate: Closed -> Overview -> ZoomedIn(1)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    // Digit 9 = EnforceOn for monitor 1
    engine.handle_menu_input(MenuInput::Digit(9)).await.unwrap();

    // Menu should close
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Enforcement should now be enabled on monitor 1 for workspace 0
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert!(
        desktop.is_enforced(1),
        "enforcement should be enabled on monitor 1 after Digit(9)"
    );
}

// ===========================================================================
// 6. EnforceOff disables enforcement
// ===========================================================================

#[tokio::test]
async fn should_disable_enforcement_on_digit_0() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Pre-enable enforcement via desktop_mut so we can verify it gets disabled
    engine.desktop_mut(0).set_enforcement(1, true);
    let desktop = engine.desktop_ref(0).unwrap();
    assert!(desktop.is_enforced(1), "enforcement should be on after manual set");

    // Navigate: Closed -> Overview -> ZoomedIn(1)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    // Digit 0 = EnforceOff for monitor 1
    engine.handle_menu_input(MenuInput::Digit(0)).await.unwrap();

    // Menu should close
    assert_eq!(engine.menu_state(), MenuState::Closed);

    // Enforcement should now be disabled on monitor 1
    let desktop = engine.desktop_ref(0).expect("desktop should exist");
    assert!(
        !desktop.is_enforced(1),
        "enforcement should be disabled on monitor 1 after Digit(0)"
    );
}

// ===========================================================================
// 7. MoveWindow (ShiftN) transitions menu to Closed
// ===========================================================================

#[tokio::test]
async fn should_handle_move_window_without_panic() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Navigate: Closed -> Overview
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    assert_eq!(engine.menu_state(), MenuState::Overview);

    // ShiftN(1) should move the focused window to monitor 1 and close menu
    let result = engine.handle_menu_input(MenuInput::ShiftN(1)).await;
    assert!(result.is_ok(), "MoveWindow should not panic or error");
    assert_eq!(engine.menu_state(), MenuState::Closed);
}

// ===========================================================================
// 8. Proxy menu calls (show_menu, show_menu_zoomed, hide_menu)
// ===========================================================================

#[tokio::test]
async fn should_call_show_menu_on_toggle_from_closed() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();

    let calls = engine.proxy().show_menu_calls();
    assert_eq!(calls.len(), 1, "show_menu should be called once on toggle from Closed");
    // The JSON should contain monitor data
    assert!(calls[0].contains("DP-1"), "show_menu JSON should contain monitor names");
}

#[tokio::test]
async fn should_call_hide_menu_on_toggle_from_overview() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    // Close menu via toggle
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();

    assert_eq!(engine.proxy().hide_menu_count(), 1, "hide_menu should be called once on toggle from Overview");
}

#[tokio::test]
async fn should_call_hide_menu_on_escape_from_overview() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    // Close menu via Escape
    engine.handle_menu_input(MenuInput::Escape).await.unwrap();

    assert_eq!(engine.proxy().hide_menu_count(), 1, "hide_menu should be called once on Escape from Overview");
}

#[tokio::test]
async fn should_call_show_menu_zoomed_on_press_n() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu then zoom into monitor 1
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();

    let calls = engine.proxy().show_menu_zoomed_calls();
    assert_eq!(calls.len(), 1, "show_menu_zoomed should be called once on PressN");
    assert_eq!(calls[0].0, 1, "show_menu_zoomed should be called with monitor_id == 1");
}

#[tokio::test]
async fn should_call_hide_menu_on_escape_from_zoomed() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu, zoom in, then Escape
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    engine.handle_menu_input(MenuInput::Escape).await.unwrap();

    assert_eq!(engine.proxy().hide_menu_count(), 1, "hide_menu should be called once on Escape from ZoomedIn");
}

#[tokio::test]
async fn should_call_hide_menu_on_digit_from_zoomed() {
    let windows = two_windows_on_monitor_1();
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu, zoom into monitor 1, apply layout via Digit(2)
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::PressN(1)).await.unwrap();
    engine.handle_menu_input(MenuInput::Digit(2)).await.unwrap();

    assert_eq!(engine.proxy().hide_menu_count(), 1, "hide_menu should be called once on Digit from ZoomedIn");
}

#[tokio::test]
async fn should_call_hide_menu_on_shift_n_from_overview() {
    let windows = vec![
        WindowInfo { id: 1, title: "A".into(), app_class: "a".into(), monitor_id: 0, workspace_id: 0 },
    ];
    let proxy = make_proxy(two_monitors(), windows);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Open menu, then ShiftN(1) to move window
    engine.handle_menu_input(MenuInput::ToggleMenu).await.unwrap();
    engine.handle_menu_input(MenuInput::ShiftN(1)).await.unwrap();

    assert_eq!(engine.proxy().hide_menu_count(), 1, "hide_menu should be called once on ShiftN from Overview");
}

#[tokio::test]
async fn should_not_call_show_or_hide_on_noop_input() {
    let proxy = make_proxy(two_monitors(), vec![]);
    let mut engine = TilingEngine::new(proxy, 0);
    engine.startup().await.unwrap();

    // Escape when already Closed — should be a no-op
    engine.handle_menu_input(MenuInput::Escape).await.unwrap();

    assert!(engine.proxy().show_menu_calls().is_empty(), "show_menu should not be called on noop");
    assert_eq!(engine.proxy().hide_menu_count(), 0, "hide_menu should not be called on noop");
}
