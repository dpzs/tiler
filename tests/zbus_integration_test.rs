use std::path::PathBuf;
use std::time::Duration;

use tiler::config::StackScreenPosition;
use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo, WindowInfo};
use tiler::gnome::event::Event;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};
use tokio::sync::mpsc;

// --- Helpers ---

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tiler-integration-test-{}-{}.sock",
        name,
        std::process::id()
    ))
}

fn make_proxy() -> MockGnomeProxy {
    let mut proxy = MockGnomeProxy::new();
    proxy.set_monitors(vec![
        MonitorInfo { id: 0, name: "DP-1".into(), x: 0, y: 0, width: 1920, height: 1080 },
        MonitorInfo { id: 1, name: "DP-2".into(), x: 1920, y: 0, width: 1920, height: 1080 },
    ]);
    proxy.set_active_workspace(0);
    proxy
}

// --- Integration tests ---

/// Full event pipeline: send multiple events in sequence (WindowOpened, WindowClosed,
/// WorkspaceChanged), verify daemon stays alive and responds to IPC after all events.
#[tokio::test]
async fn should_handle_full_event_pipeline() {
    // Arrange
    let sock = test_socket_path("pipeline");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send a sequence of events through the channel
    tx.send(Event::WindowOpened {
        window_id: 1,
        title: "Terminal".into(),
        app_class: "gnome-terminal".into(),
        monitor_id: 0,
    })
    .unwrap();

    tx.send(Event::WindowOpened {
        window_id: 2,
        title: "Firefox".into(),
        app_class: "firefox".into(),
        monitor_id: 0,
    })
    .unwrap();

    tx.send(Event::WindowClosed { window_id: 1 }).unwrap();

    tx.send(Event::WorkspaceChanged { workspace_id: 1 }).unwrap();

    tx.send(Event::WindowOpened {
        window_id: 3,
        title: "Code".into(),
        app_class: "code".into(),
        monitor_id: 0,
    })
    .unwrap();

    // Allow daemon time to process all events
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Assert — daemon is still alive and IPC works after processing the event burst
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok, "daemon should respond Ok after processing full event pipeline");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Event dispatch affects engine state: open a window then close it, verify the
/// daemon handles the lifecycle sequence without error.
#[tokio::test]
async fn should_handle_window_open_close_lifecycle() {
    // Arrange
    let sock = test_socket_path("lifecycle");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — open a window, focus it, then close it
    tx.send(Event::WindowOpened {
        window_id: 42,
        title: "Editor".into(),
        app_class: "vim".into(),
        monitor_id: 0,
    })
    .unwrap();

    tx.send(Event::WindowFocusChanged { window_id: 42 }).unwrap();

    tx.send(Event::WindowFullscreenChanged {
        window_id: 42,
        is_fullscreen: true,
    })
    .unwrap();

    tx.send(Event::WindowFullscreenChanged {
        window_id: 42,
        is_fullscreen: false,
    })
    .unwrap();

    tx.send(Event::WindowClosed { window_id: 42 }).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Assert — daemon survived the full lifecycle
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok, "daemon should survive window open/focus/fullscreen/close lifecycle");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Mixed IPC and events: interleave IPC commands with events, verify all IPC
/// commands return Ok.
#[tokio::test]
async fn should_handle_mixed_ipc_and_events() {
    // Arrange
    let sock = test_socket_path("mixed");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — interleave IPC and events

    // IPC: Status
    let resp1 = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp1, Response::Ok, "first Status should succeed");

    // Event: WindowOpened
    tx.send(Event::WindowOpened {
        window_id: 10,
        title: "App1".into(),
        app_class: "app1".into(),
        monitor_id: 0,
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // IPC: Menu
    let resp2 = send_command(&sock, Command::Menu).await.unwrap();
    assert_eq!(resp2, Response::Ok, "Menu after event should succeed");

    // Event: WorkspaceChanged
    tx.send(Event::WorkspaceChanged { workspace_id: 2 }).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // IPC: Status again
    let resp3 = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp3, Response::Ok, "Status after workspace change event should succeed");

    // Event: WindowGeometryChanged
    tx.send(Event::WindowGeometryChanged {
        window_id: 10,
        x: 100,
        y: 100,
        width: 800,
        height: 600,
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // IPC: Status once more
    let resp4 = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp4, Response::Ok, "Status after geometry event should succeed");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Serde + Event integration: deserialize a JSON string to Vec<WindowInfo>, then
/// construct WindowOpened events from the deserialized data — proving the serde
/// types and Event enum work together end-to-end.
#[test]
fn should_construct_events_from_deserialized_window_info() {
    // Arrange — JSON mimicking D-Bus ListWindows response
    let json = r#"[
        {"id": 1, "title": "Terminal", "app_class": "gnome-terminal", "monitor_id": 0, "workspace_id": 0},
        {"id": 2, "title": "Firefox", "app_class": "firefox", "monitor_id": 1, "workspace_id": 0},
        {"id": 3, "title": "Code", "app_class": "code", "monitor_id": 0, "workspace_id": 1}
    ]"#;

    // Act — deserialize then map to events
    let windows: Vec<WindowInfo> =
        serde_json::from_str(json).expect("deserialize Vec<WindowInfo>");

    let events: Vec<Event> = windows
        .iter()
        .map(|w| Event::WindowOpened {
            window_id: w.id,
            title: w.title.clone(),
            app_class: w.app_class.clone(),
            monitor_id: w.monitor_id,
        })
        .collect();

    // Assert — events match the deserialized data
    assert_eq!(events.len(), 3);

    assert_eq!(
        events[0],
        Event::WindowOpened {
            window_id: 1,
            title: "Terminal".into(),
            app_class: "gnome-terminal".into(),
        monitor_id: 0,
        }
    );

    assert_eq!(
        events[1],
        Event::WindowOpened {
            window_id: 2,
            title: "Firefox".into(),
            app_class: "firefox".into(),
        monitor_id: 1,
        }
    );

    assert_eq!(
        events[2],
        Event::WindowOpened {
            window_id: 3,
            title: "Code".into(),
            app_class: "code".into(),
        monitor_id: 0,
        }
    );

    // Also verify we can roundtrip the WindowInfo back to JSON
    let roundtripped = serde_json::to_string(&windows).expect("serialize Vec<WindowInfo>");
    let reparsed: Vec<WindowInfo> =
        serde_json::from_str(&roundtripped).expect("re-deserialize Vec<WindowInfo>");
    assert_eq!(reparsed, windows, "JSON roundtrip should preserve all WindowInfo fields");
}
