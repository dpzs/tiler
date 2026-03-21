use std::path::PathBuf;
use tiler::ipc::protocol::{Command, Response, read_message, send_message};
use tiler::ipc::server::run_server;
use tokio::net::UnixStream;

fn test_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tiler-test-{}-{}.sock", name, std::process::id()))
}

#[tokio::test]
async fn should_respond_ok_to_menu_command() {
    // Arrange
    let sock = test_socket_path("menu");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });

    // Give server time to bind
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let stream = UnixStream::connect(&sock).await.expect("should connect");
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &Command::Menu).await.expect("send Menu");
    let resp: Response = read_message(&mut reader).await.expect("read response");

    // Assert
    assert_eq!(resp, Response::Ok, "Menu command should get Response::Ok");

    // Cleanup — send Shutdown to stop server
    send_message(&mut writer, &Command::Shutdown).await.ok();
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn should_respond_ok_to_status_command() {
    // Arrange
    let sock = test_socket_path("status");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let stream = UnixStream::connect(&sock).await.expect("should connect");
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &Command::Status).await.expect("send Status");
    let resp: Response = read_message(&mut reader).await.expect("read response");

    // Assert
    assert_eq!(resp, Response::Ok, "Status command should get Response::Ok");

    // Cleanup
    send_message(&mut writer, &Command::Shutdown).await.ok();
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn should_shutdown_on_shutdown_command() {
    // Arrange
    let sock = test_socket_path("shutdown");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act
    let stream = UnixStream::connect(&sock).await.expect("should connect");
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &Command::Shutdown).await.expect("send Shutdown");
    let resp: Response = read_message(&mut reader).await.expect("read response");

    // Assert
    assert_eq!(resp, Response::Ok, "Shutdown command should get Response::Ok");

    // Server task should finish
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        server,
    ).await;
    assert!(result.is_ok(), "server should exit after Shutdown command");
}

#[tokio::test]
async fn should_remove_socket_file_on_shutdown() {
    // Arrange
    let sock = test_socket_path("cleanup");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    assert!(sock.exists(), "socket file should exist while server is running");

    // Act
    let stream = UnixStream::connect(&sock).await.expect("should connect");
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &Command::Shutdown).await.expect("send Shutdown");
    let _resp: Response = read_message(&mut reader).await.expect("read response");
    let _ = server.await;

    // Assert
    assert!(!sock.exists(), "socket file should be removed after shutdown");
}

#[tokio::test]
async fn should_handle_multiple_sequential_connections() {
    // Arrange
    let sock = test_socket_path("multi");
    let _ = std::fs::remove_file(&sock);
    let sock2 = sock.clone();
    let server = tokio::spawn(async move { run_server(&sock2).await });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Act — first connection
    let stream1 = UnixStream::connect(&sock).await.expect("connect 1");
    let (mut r1, mut w1) = stream1.into_split();
    send_message(&mut w1, &Command::Menu).await.expect("send Menu");
    let resp1: Response = read_message(&mut r1).await.expect("read resp 1");
    drop(r1);
    drop(w1);

    // Small delay for server to accept next connection
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // Act — second connection
    let stream2 = UnixStream::connect(&sock).await.expect("connect 2");
    let (mut r2, mut w2) = stream2.into_split();
    send_message(&mut w2, &Command::Status).await.expect("send Status");
    let resp2: Response = read_message(&mut r2).await.expect("read resp 2");

    // Assert
    assert_eq!(resp1, Response::Ok, "first connection should get Ok");
    assert_eq!(resp2, Response::Ok, "second connection should get Ok");

    // Cleanup
    send_message(&mut w2, &Command::Shutdown).await.ok();
    let _ = server.await;
    let _ = std::fs::remove_file(&sock);
}
