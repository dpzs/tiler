use tiler::gnome::dbus_proxy::{GnomeProxy, MonitorInfo, WindowInfo};
use tiler::gnome::zbus_proxy::ZbusGnomeProxy;

// --- WindowInfo serde roundtrip ---

#[test]
fn should_roundtrip_window_info_through_json() {
    // Arrange
    let window = WindowInfo {
        id: 42,
        title: "Terminal".into(),
        app_class: "gnome-terminal".into(),
        monitor_id: 1,
        workspace_id: 2,
    };

    // Act
    let json = serde_json::to_string(&window).expect("serialize WindowInfo");
    let deserialized: WindowInfo =
        serde_json::from_str(&json).expect("deserialize WindowInfo");

    // Assert
    assert_eq!(deserialized, window);
}

#[test]
fn should_roundtrip_window_info_with_empty_strings() {
    // Arrange
    let window = WindowInfo {
        id: 0,
        title: String::new(),
        app_class: String::new(),
        monitor_id: 0,
        workspace_id: 0,
    };

    // Act
    let json = serde_json::to_string(&window).expect("serialize WindowInfo");
    let deserialized: WindowInfo =
        serde_json::from_str(&json).expect("deserialize WindowInfo");

    // Assert
    assert_eq!(deserialized, window);
}

// --- MonitorInfo serde roundtrip ---

#[test]
fn should_roundtrip_monitor_info_through_json() {
    // Arrange
    let monitor = MonitorInfo {
        id: 0,
        name: "HDMI-1".into(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    // Act
    let json = serde_json::to_string(&monitor).expect("serialize MonitorInfo");
    let deserialized: MonitorInfo =
        serde_json::from_str(&json).expect("deserialize MonitorInfo");

    // Assert
    assert_eq!(deserialized, monitor);
}

#[test]
fn should_roundtrip_monitor_info_with_negative_coordinates() {
    // Arrange — monitors to the left/above origin have negative x/y
    let monitor = MonitorInfo {
        id: 1,
        name: "DP-2".into(),
        x: -2560,
        y: -1440,
        width: 2560,
        height: 1440,
    };

    // Act
    let json = serde_json::to_string(&monitor).expect("serialize MonitorInfo");
    let deserialized: MonitorInfo =
        serde_json::from_str(&json).expect("deserialize MonitorInfo");

    // Assert
    assert_eq!(deserialized, monitor);
}

// --- Vec<WindowInfo> deserialization (mimics D-Bus ListWindows return) ---

#[test]
fn should_deserialize_json_array_to_vec_window_info() {
    // Arrange
    let json = r#"[
        {"id": 1, "title": "Terminal", "app_class": "gnome-terminal", "monitor_id": 0, "workspace_id": 0},
        {"id": 2, "title": "Firefox", "app_class": "firefox", "monitor_id": 1, "workspace_id": 0}
    ]"#;

    // Act
    let windows: Vec<WindowInfo> =
        serde_json::from_str(json).expect("deserialize Vec<WindowInfo>");

    // Assert
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].id, 1);
    assert_eq!(windows[0].title, "Terminal");
    assert_eq!(windows[1].id, 2);
    assert_eq!(windows[1].app_class, "firefox");
}

// --- Vec<MonitorInfo> deserialization (mimics D-Bus GetMonitors return) ---

#[test]
fn should_deserialize_json_array_to_vec_monitor_info() {
    // Arrange
    let json = r#"[
        {"id": 0, "name": "HDMI-1", "x": 0, "y": 0, "width": 1920, "height": 1080},
        {"id": 1, "name": "DP-1", "x": 1920, "y": 0, "width": 2560, "height": 1440}
    ]"#;

    // Act
    let monitors: Vec<MonitorInfo> =
        serde_json::from_str(json).expect("deserialize Vec<MonitorInfo>");

    // Assert
    assert_eq!(monitors.len(), 2);
    assert_eq!(monitors[0].name, "HDMI-1");
    assert_eq!(monitors[0].width, 1920);
    assert_eq!(monitors[1].name, "DP-1");
    assert_eq!(monitors[1].x, 1920);
}

// --- ZbusGnomeProxy compile-time verification ---

fn assert_gnome_proxy<T: GnomeProxy>() {}

fn assert_send<T: Send>() {}

#[test]
fn should_implement_gnome_proxy_trait() {
    assert_gnome_proxy::<ZbusGnomeProxy>();
}

#[test]
fn should_implement_send() {
    assert_send::<ZbusGnomeProxy>();
}

// --- Event variant construction (one per D-Bus signal) ---

use tiler::gnome::event::Event;

#[test]
fn should_construct_window_opened_event() {
    let event = Event::WindowOpened {
        window_id: 42,
        title: "Terminal".into(),
        app_class: "gnome-terminal".into(),
    };
    assert_eq!(event, Event::WindowOpened {
        window_id: 42,
        title: "Terminal".into(),
        app_class: "gnome-terminal".into(),
    });
}

#[test]
fn should_construct_window_closed_event() {
    let event = Event::WindowClosed { window_id: 42 };
    assert_eq!(event, Event::WindowClosed { window_id: 42 });
}

#[test]
fn should_construct_window_focus_changed_event() {
    let event = Event::WindowFocusChanged { window_id: 42 };
    assert_eq!(event, Event::WindowFocusChanged { window_id: 42 });
}

#[test]
fn should_construct_workspace_changed_event() {
    let event = Event::WorkspaceChanged { workspace_id: 1 };
    assert_eq!(event, Event::WorkspaceChanged { workspace_id: 1 });
}

#[test]
fn should_construct_fullscreen_changed_event() {
    let event = Event::WindowFullscreenChanged { window_id: 42, is_fullscreen: true };
    assert_eq!(event, Event::WindowFullscreenChanged { window_id: 42, is_fullscreen: true });
}

#[test]
fn should_construct_geometry_changed_event() {
    let event = Event::WindowGeometryChanged { window_id: 42, x: 0, y: 0, width: 800, height: 600 };
    assert_eq!(event, Event::WindowGeometryChanged { window_id: 42, x: 0, y: 0, width: 800, height: 600 });
}

#[test]
fn should_construct_menu_key_pressed_event() {
    let event = Event::MenuKeyPressed { key: "1".into(), modifiers: "Super".into() };
    assert_eq!(event, Event::MenuKeyPressed { key: "1".into(), modifiers: "Super".into() });
}

// --- spawn_signal_listener compile-time verification ---

/// Compile-time check that ZbusGnomeProxy has spawn_signal_listener.
/// Uses a function pointer cast to verify the method signature exists.
fn assert_has_spawn_signal_listener() {
    // Verify spawn_signal_listener exists on ZbusGnomeProxy by referencing it.
    // This will fail to compile until the method is implemented.
    let _: fn(&ZbusGnomeProxy, tokio::sync::mpsc::UnboundedSender<Event>) =
        ZbusGnomeProxy::spawn_signal_listener;
}

#[test]
fn should_have_spawn_signal_listener_method() {
    assert_has_spawn_signal_listener();
}
