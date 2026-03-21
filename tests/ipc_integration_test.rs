use std::path::PathBuf;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};
use tiler::ipc::server::run_server;

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tiler-integ-{}-{}.sock", name, std::process::id()))
}

#[tokio::test]
async fn should_roundtrip_all_commands_through_server_and_client() {
    // Arrange — start server
    let sock = test_socket_path("roundtrip");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act — send Menu
    let menu_resp = send_command(&sock, Command::Menu)
        .await
        .expect("Menu roundtrip should succeed");

    // Act — send Status
    let status_resp = send_command(&sock, Command::Status)
        .await
        .expect("Status roundtrip should succeed");

    // Act — send Shutdown
    let shutdown_resp = send_command(&sock, Command::Shutdown)
        .await
        .expect("Shutdown roundtrip should succeed");

    // Assert
    assert_eq!(menu_resp, Response::Ok, "Menu should return Ok");
    assert_eq!(status_resp, Response::Ok, "Status should return Ok");
    assert_eq!(shutdown_resp, Response::Ok, "Shutdown should return Ok");

    // Server should have exited
    let server_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        server,
    ).await;
    assert!(server_result.is_ok(), "server should exit after Shutdown");

    // Socket file should be cleaned up
    assert!(!sock.exists(), "socket file should be removed after shutdown");
}

#[tokio::test]
async fn should_handle_rapid_sequential_client_requests() {
    // Arrange
    let sock = test_socket_path("rapid");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act — send 10 rapid Menu commands from separate connections
    for i in 0..10 {
        let resp = send_command(&sock, Command::Menu)
            .await
            .unwrap_or_else(|e| panic!("request {i} failed: {e}"));
        assert_eq!(resp, Response::Ok, "request {i} should return Ok");
    }

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn should_fail_client_when_no_server_is_running() {
    // Arrange — no server, no socket file
    let sock = test_socket_path("noserver");
    let _ = std::fs::remove_file(&sock);

    // Act
    let result = send_command(&sock, Command::Status).await;

    // Assert
    assert!(result.is_err(), "client should error when no server is listening");
}
