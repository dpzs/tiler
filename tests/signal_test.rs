use std::path::PathBuf;
use std::time::Duration;

use tiler::config::StackScreenPosition;
use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo};
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};

// --- Helpers ---

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tiler-signal-test-{}-{}.sock",
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

/// The daemon should exit cleanly when an external shutdown signal is sent
/// via the oneshot channel, without needing an IPC Shutdown command.
#[tokio::test]
async fn should_exit_on_external_shutdown_signal() {
    // Arrange
    let sock = test_socket_path("signal");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, Some(rx), None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify daemon is alive by sending a Status command
    let resp = send_command(&sock, Command::Status)
        .await
        .expect("Status should succeed while daemon is running");
    assert_eq!(resp, Response::Ok);

    // Act — send shutdown signal via the channel (simulates SIGTERM/SIGINT)
    tx.send(()).expect("shutdown signal should be sent successfully");

    // Assert — daemon should exit within 2 seconds
    let result = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    assert!(
        result.is_ok(),
        "daemon should exit within 2s after receiving shutdown signal"
    );

    // Socket file should be cleaned up
    assert!(
        !sock.exists(),
        "socket file should be removed after signal-triggered shutdown"
    );
}

/// The daemon should still handle IPC commands normally when given a
/// shutdown channel that has not fired yet.
#[tokio::test]
async fn should_serve_ipc_normally_with_pending_shutdown_channel() {
    // Arrange
    let sock = test_socket_path("pending");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, Some(rx), None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send multiple IPC commands while shutdown channel is idle
    let resp1 = send_command(&sock, Command::Status)
        .await
        .expect("Status should succeed");
    assert_eq!(resp1, Response::Ok);

    let resp2 = send_command(&sock, Command::Menu)
        .await
        .expect("Menu should succeed");
    assert_eq!(resp2, Response::Ok);

    let resp3 = send_command(&sock, Command::Status)
        .await
        .expect("second Status should succeed");
    assert_eq!(resp3, Response::Ok);

    // Cleanup — use the shutdown channel instead of IPC Shutdown
    tx.send(()).expect("shutdown signal should send");
    let _ = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    let _ = std::fs::remove_file(&sock);
}

/// When the shutdown channel sender is dropped without sending,
/// the daemon should detect the closed channel and exit cleanly
/// (the sender being dropped means the signal source is gone).
#[tokio::test]
async fn should_exit_when_shutdown_sender_is_dropped() {
    // Arrange
    let sock = test_socket_path("drop");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, StackScreenPosition::Left, Some(rx), None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify daemon is alive
    let resp = send_command(&sock, Command::Status)
        .await
        .expect("Status should succeed");
    assert_eq!(resp, Response::Ok);

    // Act — drop the sender without sending a value
    drop(tx);

    // Assert — daemon should detect the dropped sender and exit
    let result = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    assert!(
        result.is_ok(),
        "daemon should exit when shutdown sender is dropped"
    );

    // Socket should be cleaned up
    assert!(
        !sock.exists(),
        "socket file should be removed after sender-dropped shutdown"
    );
}
