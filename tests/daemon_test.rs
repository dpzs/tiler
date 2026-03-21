use std::path::PathBuf;
use std::time::Duration;

use tiler::daemon::run_daemon;
use tiler::gnome::dbus_proxy::{MockGnomeProxy, MonitorInfo};
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};

// --- Helpers ---

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tiler-daemon-test-{}-{}.sock",
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

/// The daemon should process a Menu command via IPC and respond Ok.
#[tokio::test]
async fn should_process_menu_command_and_respond_ok() {
    // Arrange
    let sock = test_socket_path("menu");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act
    let resp = send_command(&sock, Command::Menu)
        .await
        .expect("Menu command should succeed");

    // Assert
    assert_eq!(resp, Response::Ok, "Menu command should return Response::Ok");

    // Cleanup — send Shutdown to stop the daemon
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// The daemon should respond Ok to Shutdown and exit cleanly,
/// removing the socket file.
#[tokio::test]
async fn should_handle_shutdown_gracefully() {
    // Arrange
    let sock = test_socket_path("shutdown");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Socket should exist while daemon is running
    assert!(sock.exists(), "socket file should exist while daemon is running");

    // Act
    let resp = send_command(&sock, Command::Shutdown)
        .await
        .expect("Shutdown command should succeed");

    // Assert — response is Ok
    assert_eq!(resp, Response::Ok, "Shutdown should return Response::Ok");

    // Daemon task should exit cleanly
    let result = tokio::time::timeout(Duration::from_secs(2), daemon).await;
    assert!(result.is_ok(), "daemon should exit after Shutdown command");

    // Socket file should be cleaned up
    assert!(!sock.exists(), "socket file should be removed after shutdown");
}

/// The daemon should respond Ok to a Status command.
#[tokio::test]
async fn should_route_status_command() {
    // Arrange
    let sock = test_socket_path("status");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act
    let resp = send_command(&sock, Command::Status)
        .await
        .expect("Status command should succeed");

    // Assert
    assert_eq!(resp, Response::Ok, "Status command should return Response::Ok");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}

/// After startup, the daemon should have called engine.startup() which
/// queries monitors and windows from the proxy. We verify this indirectly:
/// the daemon starts without error and can serve IPC requests, which
/// requires a functioning engine. We also send multiple commands to
/// confirm the event loop is processing correctly after initialization.
#[tokio::test]
async fn should_initialize_engine_on_startup() {
    // Arrange
    let sock = test_socket_path("init");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();

    let proxy = make_proxy();
    let daemon = tokio::spawn(async move {
        run_daemon(proxy, &sock2, 0, None, None).await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Act — send Status to verify daemon is alive and engine is initialized
    let resp1 = send_command(&sock, Command::Status)
        .await
        .expect("first Status should succeed — engine should be initialized");
    assert_eq!(resp1, Response::Ok);

    // Send Menu to exercise engine routing
    let resp2 = send_command(&sock, Command::Menu)
        .await
        .expect("Menu after startup should succeed");
    assert_eq!(resp2, Response::Ok);

    // Send another Status to confirm continued operation
    let resp3 = send_command(&sock, Command::Status)
        .await
        .expect("second Status should succeed");
    assert_eq!(resp3, Response::Ok);

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = daemon.await;
    let _ = std::fs::remove_file(&sock);
}
