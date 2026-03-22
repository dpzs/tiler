use tiler::gnome::dbus_proxy::{GnomeProxy, MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::gnome::event::Event;

#[tokio::test]
async fn mock_proxy_list_windows_returns_set_windows() {
    let mut mock = MockGnomeProxy::new();
    mock.set_windows(vec![
        WindowInfo {
            id: 1,
            title: "Terminal".into(),
            app_class: "gnome-terminal".into(),
            monitor_id: 0,
            workspace_id: 0,
        },
        WindowInfo {
            id: 2,
            title: "Firefox".into(),
            app_class: "firefox".into(),
            monitor_id: 1,
            workspace_id: 0,
        },
    ]);

    let windows = mock.list_windows().await.unwrap();
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].id, 1);
    assert_eq!(windows[1].title, "Firefox");
}

#[tokio::test]
async fn mock_proxy_get_monitors() {
    let mut mock = MockGnomeProxy::new();
    mock.set_monitors(vec![MonitorInfo {
        id: 0,
        name: "HDMI-1".into(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    }]);

    let monitors = mock.get_monitors().await.unwrap();
    assert_eq!(monitors.len(), 1);
    assert_eq!(monitors[0].name, "HDMI-1");
    assert_eq!(monitors[0].width, 1920);
}

#[tokio::test]
async fn mock_proxy_move_resize_records_call() {
    let mut mock = MockGnomeProxy::new();
    mock.move_resize_window(42, 0, 0, 800, 600).await.unwrap();

    let calls = mock.move_resize_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (42, 0, 0, 800, 600));
}

#[tokio::test]
async fn mock_proxy_get_active_workspace() {
    let mut mock = MockGnomeProxy::new();
    mock.set_active_workspace(3);

    let ws = mock.get_active_workspace().await.unwrap();
    assert_eq!(ws, 3);
}

#[tokio::test]
async fn mock_proxy_get_window_type() {
    let mut mock = MockGnomeProxy::new();
    mock.set_window_type(1, "normal".into());

    let wt = mock.get_window_type(1).await.unwrap();
    assert_eq!(wt, "normal");
}

#[tokio::test]
async fn mock_proxy_is_fullscreen() {
    let mut mock = MockGnomeProxy::new();
    mock.set_fullscreen(1, true);

    assert!(mock.is_fullscreen(1).await.unwrap());
    assert!(!mock.is_fullscreen(999).await.unwrap());
}

#[test]
fn event_enum_has_all_signal_variants() {
    // Test that all event variants can be constructed
    let events = vec![
        Event::WindowOpened { window_id: 1, title: "t".into(), app_class: "c".into(), monitor_id: 0 },
        Event::WindowClosed { window_id: 1 },
        Event::WindowFocusChanged { window_id: 1 },
        Event::WorkspaceChanged { workspace_id: 0 },
        Event::WindowFullscreenChanged { window_id: 1, is_fullscreen: true },
        Event::WindowGeometryChanged { window_id: 1, x: 0, y: 0, width: 100, height: 100 },
        Event::MenuKeyPressed { key: "n".into(), modifiers: "shift".into() },
    ];
    assert_eq!(events.len(), 7);
}

#[test]
fn event_debug_impl() {
    let event = Event::WindowOpened {
        window_id: 42,
        title: "Test".into(),
        app_class: "test".into(),
        monitor_id: 0,
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("WindowOpened"));
    assert!(debug.contains("42"));
}

#[test]
fn window_info_clone_and_eq() {
    let w = WindowInfo {
        id: 1,
        title: "T".into(),
        app_class: "C".into(),
        monitor_id: 0,
        workspace_id: 0,
    };
    assert_eq!(w, w.clone());
}

#[test]
fn monitor_info_clone_and_eq() {
    let m = MonitorInfo {
        id: 0,
        name: "DP-1".into(),
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };
    assert_eq!(m, m.clone());
}

#[tokio::test]
async fn mock_proxy_show_menu_records_json() {
    let mut mock = MockGnomeProxy::new();
    mock.show_menu(r#"[{"id":0}]"#).await.unwrap();

    let calls = mock.show_menu_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], r#"[{"id":0}]"#);
}

#[tokio::test]
async fn mock_proxy_hide_menu_increments_count() {
    let mut mock = MockGnomeProxy::new();
    mock.hide_menu().await.unwrap();
    mock.hide_menu().await.unwrap();

    assert_eq!(mock.hide_menu_count(), 2);
}

#[tokio::test]
async fn mock_proxy_show_menu_zoomed_records_call() {
    let mut mock = MockGnomeProxy::new();
    mock.show_menu_zoomed(1, "[]").await.unwrap();

    let calls = mock.show_menu_zoomed_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], (1, "[]".to_string()));
}
