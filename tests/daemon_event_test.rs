use std::path::PathBuf;
use std::time::Duration;

use tiler::config::StackScreenPosition;
use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo};
use tiler::gnome::event::Event;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};
use tokio::sync::mpsc;

// --- Helpers (mirrored from daemon_test.rs) ---

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tiler-daemon-event-test-{}-{}.sock",
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

// --- Tests ---

/// Verify run_daemon compiles with the new 5th parameter Option<mpsc::UnboundedReceiver<Event>>.
/// Start daemon with Some(rx), send a WindowOpened event, then shut down cleanly.
#[tokio::test]
async fn should_accept_event_receiver_parameter() {
    // Arrange
    let sock = test_socket_path("event-rx");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send an event through the channel
    tx.send(Event::WindowOpened {
        window_id: 100,
        title: "Test".into(),
        app_class: "test-app".into(),
        monitor_id: 0,
    })
    .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon is still alive and responds to IPC
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Send a WindowClosed event and verify the daemon doesn't crash — it still responds to IPC.
#[tokio::test]
async fn should_dispatch_window_closed_event() {
    // Arrange
    let sock = test_socket_path("event-close");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send WindowClosed event
    tx.send(Event::WindowClosed { window_id: 1 }).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Send a WorkspaceChanged event and verify the daemon doesn't crash — it still responds to IPC.
#[tokio::test]
async fn should_dispatch_workspace_changed_event() {
    // Arrange
    let sock = test_socket_path("event-ws");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send WorkspaceChanged event
    tx.send(Event::WorkspaceChanged { workspace_id: 1 }).unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

// ===========================================================================
// MenuKeyPressed dispatch
// ===========================================================================

/// Send a MenuKeyPressed event with Escape and verify daemon stays alive.
#[tokio::test]
async fn should_dispatch_menu_key_pressed_escape() {
    // Arrange
    let sock = test_socket_path("event-menu-esc");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send MenuKeyPressed Escape
    tx.send(Event::MenuKeyPressed {
        key: "Escape".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Send a MenuKeyPressed event with digit "1" and verify daemon stays alive.
#[tokio::test]
async fn should_dispatch_menu_key_pressed_digit() {
    // Arrange
    let sock = test_socket_path("event-menu-digit");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send MenuKeyPressed digit
    tx.send(Event::MenuKeyPressed {
        key: "1".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Send a MenuKeyPressed event with unknown key "F5" and verify daemon stays alive.
#[tokio::test]
async fn should_ignore_unknown_menu_key() {
    // Arrange
    let sock = test_socket_path("event-menu-unknown");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send MenuKeyPressed with unknown key
    tx.send(Event::MenuKeyPressed {
        key: "F5".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

// ===========================================================================
// End-to-end MenuKeyPressed dispatch (menu opened via IPC first)
// ===========================================================================

/// Open menu via Command::Menu (Closed->Overview), then send Escape via
/// MenuKeyPressed event. Exercises the full dispatch path: parse_menu_key
/// sees Overview state, returns Some(Escape), engine transitions Overview->Closed.
#[tokio::test]
async fn should_dispatch_escape_from_overview_state() {
    // Arrange
    let sock = test_socket_path("event-e2e-esc");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — open menu via IPC, then close via MenuKeyPressed Escape
    let resp = send_command(&sock, Command::Menu).await.unwrap();
    assert_eq!(resp, Response::Ok);
    tokio::time::sleep(Duration::from_millis(50)).await;

    tx.send(Event::MenuKeyPressed {
        key: "Escape".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive after full open->close cycle
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Open menu via Command::Menu (Closed->Overview), then send digit "1" via
/// MenuKeyPressed event. In Overview state, "1" maps to PressN(0) which
/// transitions Overview->ZoomedIn(0) and calls proxy.show_menu_zoomed().
#[tokio::test]
async fn should_dispatch_digit_press_from_overview_state() {
    // Arrange
    let sock = test_socket_path("event-e2e-digit");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — open menu, then press "1" to zoom into monitor 0
    let resp = send_command(&sock, Command::Menu).await.unwrap();
    assert_eq!(resp, Response::Ok);
    tokio::time::sleep(Duration::from_millis(50)).await;

    tx.send(Event::MenuKeyPressed {
        key: "1".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon still alive after Overview->ZoomedIn transition
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Full 3-step flow: open menu (Closed->Overview), press "1" (Overview->ZoomedIn(0)),
/// press "2" (ZoomedIn(0)->Closed via ApplyLayout). Exercises the entire dispatch
/// chain through all three menu states.
#[tokio::test]
async fn should_dispatch_full_menu_flow_open_zoom_apply() {
    // Arrange
    let sock = test_socket_path("event-e2e-full");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();
    let (tx, rx) = mpsc::unbounded_channel();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — step 1: open menu via IPC (Closed -> Overview)
    let resp = send_command(&sock, Command::Menu).await.unwrap();
    assert_eq!(resp, Response::Ok);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — step 2: press "1" via event (Overview -> ZoomedIn(0))
    tx.send(Event::MenuKeyPressed {
        key: "1".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — step 3: press "2" via event (ZoomedIn(0) -> Closed, ApplyLayout(0, 2))
    tx.send(Event::MenuKeyPressed {
        key: "2".into(),
        modifiers: "".into(),
    })
    .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert — daemon survived the full Closed->Overview->ZoomedIn->Closed cycle
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// Backward compatibility: start daemon with None for the event receiver, verify IPC still works.
#[tokio::test]
async fn should_work_without_event_receiver() {
    // Arrange
    let sock = test_socket_path("event-none");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — verify IPC works without an event channel
    let resp = send_command(&sock, Command::Status).await.unwrap();
    assert_eq!(resp, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}
