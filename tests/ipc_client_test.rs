use std::path::PathBuf;
use tiler::ipc::client::send_command;
use tiler::ipc::protocol::{Command, Response};
use tiler::ipc::server::run_server;

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tiler-client-test-{}-{}.sock", name, std::process::id()))
}

#[tokio::test]
async fn should_send_menu_command_and_receive_ok() {
    // Arrange
    let sock = test_socket_path("menu");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let resp = send_command(&sock, Command::Menu).await.expect("should send Menu");

    // Assert
    assert_eq!(resp, Response::Ok, "Menu should return Ok");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn should_send_status_command_and_receive_ok() {
    // Arrange
    let sock = test_socket_path("status");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let resp = send_command(&sock, Command::Status).await.expect("should send Status");

    // Assert
    assert_eq!(resp, Response::Ok, "Status should return Ok");

    // Cleanup
    let _ = send_command(&sock, Command::Shutdown).await;
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn should_error_when_daemon_not_running() {
    // Arrange — no server started, socket doesn't exist
    let sock = test_socket_path("noserver");
    let _ = std::fs::remove_file(&sock);

    // Act
    let result = send_command(&sock, Command::Menu).await;

    // Assert
    assert!(result.is_err(), "should error when daemon is not running");
}

#[tokio::test]
async fn should_send_shutdown_and_receive_ok() {
    // Arrange
    let sock = test_socket_path("shutdown");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let resp = send_command(&sock, Command::Shutdown).await.expect("should send Shutdown");

    // Assert
    assert_eq!(resp, Response::Ok, "Shutdown should return Ok");
    let _ = server.await;
}
