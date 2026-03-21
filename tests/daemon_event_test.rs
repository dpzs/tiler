use std::path::PathBuf;
use std::time::Duration;

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
        run_daemon(proxy, &sock2, 0, None, Some(rx)).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send an event through the channel
    tx.send(Event::WindowOpened {
        window_id: 100,
        title: "Test".into(),
        app_class: "test-app".into(),
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
        run_daemon(proxy, &sock2, 0, None, Some(rx)).await
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
        run_daemon(proxy, &sock2, 0, None, Some(rx)).await
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

/// Backward compatibility: start daemon with None for the event receiver, verify IPC still works.
#[tokio::test]
async fn should_work_without_event_receiver() {
    // Arrange
    let sock = test_socket_path("event-none");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let proxy = make_proxy();

    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
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
