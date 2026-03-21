use tiler::ipc::protocol::{Command, Response, decode_frame, encode_frame, read_message, send_message};

// --- Command serialization ---

#[test]
fn should_serialize_command_menu_to_json() {
    // Arrange
    let cmd = Command::Menu;

    // Act
    let json = serde_json::to_string(&cmd).expect("should serialize Command::Menu");

    // Assert
    assert_eq!(json, "\"Menu\"", "unit variant should serialize as a JSON string");
}

#[test]
fn should_serialize_command_status_to_json() {
    // Arrange
    let cmd = Command::Status;

    // Act
    let json = serde_json::to_string(&cmd).expect("should serialize Command::Status");

    // Assert
    assert_eq!(json, "\"Status\"");
}

#[test]
fn should_serialize_command_shutdown_to_json() {
    // Arrange
    let cmd = Command::Shutdown;

    // Act
    let json = serde_json::to_string(&cmd).expect("should serialize Command::Shutdown");

    // Assert
    assert_eq!(json, "\"Shutdown\"");
}

#[test]
fn should_roundtrip_command_menu_through_json() {
    // Arrange
    let cmd = Command::Menu;

    // Act
    let json = serde_json::to_string(&cmd).expect("serialize");
    let deserialized: Command = serde_json::from_str(&json).expect("deserialize");

    // Assert
    assert!(
        matches!(deserialized, Command::Menu),
        "expected Command::Menu, got {:?}",
        deserialized
    );
}

#[test]
fn should_roundtrip_command_status_through_json() {
    // Arrange
    let cmd = Command::Status;

    // Act
    let json = serde_json::to_string(&cmd).expect("serialize");
    let deserialized: Command = serde_json::from_str(&json).expect("deserialize");

    // Assert
    assert!(
        matches!(deserialized, Command::Status),
        "expected Command::Status, got {:?}",
        deserialized
    );
}

#[test]
fn should_roundtrip_command_shutdown_through_json() {
    // Arrange
    let cmd = Command::Shutdown;

    // Act
    let json = serde_json::to_string(&cmd).expect("serialize");
    let deserialized: Command = serde_json::from_str(&json).expect("deserialize");

    // Assert
    assert!(
        matches!(deserialized, Command::Shutdown),
        "expected Command::Shutdown, got {:?}",
        deserialized
    );
}

// --- Response serialization ---

#[test]
fn should_serialize_response_ok_to_json() {
    // Arrange
    let resp = Response::Ok;

    // Act
    let json = serde_json::to_string(&resp).expect("should serialize Response::Ok");

    // Assert
    assert_eq!(json, "\"Ok\"", "unit variant should serialize as a JSON string");
}

#[test]
fn should_serialize_response_error_to_json() {
    // Arrange
    let resp = Response::Error("something went wrong".into());

    // Act
    let json = serde_json::to_string(&resp).expect("should serialize Response::Error");

    // Assert
    assert_eq!(
        json,
        r#"{"Error":"something went wrong"}"#,
        "newtype variant should serialize as a JSON object"
    );
}

#[test]
fn should_roundtrip_response_ok_through_json() {
    // Arrange
    let resp = Response::Ok;

    // Act
    let json = serde_json::to_string(&resp).expect("serialize");
    let deserialized: Response = serde_json::from_str(&json).expect("deserialize");

    // Assert
    assert!(
        matches!(deserialized, Response::Ok),
        "expected Response::Ok, got {:?}",
        deserialized
    );
}

#[test]
fn should_roundtrip_response_error_through_json() {
    // Arrange
    let resp = Response::Error("fail".into());

    // Act
    let json = serde_json::to_string(&resp).expect("serialize");
    let deserialized: Response = serde_json::from_str(&json).expect("deserialize");

    // Assert
    match deserialized {
        Response::Error(msg) => assert_eq!(msg, "fail"),
        other => panic!("expected Response::Error(\"fail\"), got {:?}", other),
    }
}

// --- encode_frame ---

#[test]
fn should_encode_frame_with_length_prefix() {
    // Arrange
    let payload = b"hello";

    // Act
    let frame = encode_frame(payload).expect("encode");

    // Assert — 4-byte big-endian length (5) + payload
    assert_eq!(frame.len(), 4 + 5, "frame should be 9 bytes total");
    let length_bytes = &frame[..4];
    assert_eq!(
        length_bytes,
        &[0, 0, 0, 5],
        "first 4 bytes should be big-endian u32 length of 5"
    );
    assert_eq!(&frame[4..], b"hello", "remaining bytes should be the payload");
}

#[test]
fn should_encode_empty_payload() {
    // Arrange
    let payload = b"";

    // Act
    let frame = encode_frame(payload).expect("encode");

    // Assert — 4-byte big-endian length (0) + empty payload
    assert_eq!(frame.len(), 4);
    assert_eq!(&frame[..4], &[0, 0, 0, 0]);
}

#[test]
fn should_encode_larger_payload() {
    // Arrange
    let payload = vec![0xAB; 300];

    // Act
    let frame = encode_frame(&payload).expect("encode");

    // Assert — length 300 = 0x0000012C in big-endian
    assert_eq!(frame.len(), 4 + 300);
    assert_eq!(&frame[..4], &[0, 0, 1, 44]); // 0x012C = 300
}

// --- decode_frame ---

#[tokio::test]
async fn should_decode_frame_from_reader() {
    // Arrange — build a valid frame: 4-byte length + payload
    let payload = b"world";
    let mut data = Vec::new();
    data.extend_from_slice(&5u32.to_be_bytes());
    data.extend_from_slice(payload);
    let mut reader = &data[..];

    // Act
    let result = decode_frame(&mut reader).await;

    // Assert
    let decoded = result.expect("should decode a valid frame");
    assert_eq!(decoded, b"world");
}

#[tokio::test]
async fn should_return_error_on_eof_before_length() {
    // Arrange — empty reader
    let data: &[u8] = &[];
    let mut reader = data;

    // Act
    let result = decode_frame(&mut reader).await;

    // Assert
    assert!(result.is_err(), "should error when reader is empty (EOF before length)");
}

#[tokio::test]
async fn should_return_error_on_truncated_payload() {
    // Arrange — length says 10 but only 3 bytes follow
    let mut data = Vec::new();
    data.extend_from_slice(&10u32.to_be_bytes());
    data.extend_from_slice(b"abc");
    let mut reader = &data[..];

    // Act
    let result = decode_frame(&mut reader).await;

    // Assert
    assert!(
        result.is_err(),
        "should error when payload is shorter than declared length"
    );
}

#[tokio::test]
async fn should_decode_zero_length_frame() {
    // Arrange — length 0, no payload
    let data = 0u32.to_be_bytes();
    let mut reader = &data[..];

    // Act
    let result = decode_frame(&mut reader).await;

    // Assert
    let decoded = result.expect("should decode a zero-length frame");
    assert!(decoded.is_empty());
}

// --- send_message + read_message roundtrip ---

#[tokio::test]
async fn should_roundtrip_command_menu_through_framed_message() {
    // Arrange
    let (mut writer, mut reader) = tokio::io::duplex(1024);
    let cmd = Command::Menu;

    // Act
    send_message(&mut writer, &cmd).await.expect("send_message should succeed");
    drop(writer); // close write half so reader sees EOF after the frame
    let received: Command = read_message(&mut reader).await.expect("read_message should succeed");

    // Assert
    assert!(
        matches!(received, Command::Menu),
        "expected Command::Menu, got {:?}",
        received
    );
}

#[tokio::test]
async fn should_roundtrip_command_shutdown_through_framed_message() {
    // Arrange
    let (mut writer, mut reader) = tokio::io::duplex(1024);
    let cmd = Command::Shutdown;

    // Act
    send_message(&mut writer, &cmd).await.expect("send_message should succeed");
    drop(writer);
    let received: Command = read_message(&mut reader).await.expect("read_message should succeed");

    // Assert
    assert!(
        matches!(received, Command::Shutdown),
        "expected Command::Shutdown, got {:?}",
        received
    );
}

#[tokio::test]
async fn should_roundtrip_response_ok_through_framed_message() {
    // Arrange
    let (mut writer, mut reader) = tokio::io::duplex(1024);
    let resp = Response::Ok;

    // Act
    send_message(&mut writer, &resp).await.expect("send_message should succeed");
    drop(writer);
    let received: Response = read_message(&mut reader).await.expect("read_message should succeed");

    // Assert
    assert!(
        matches!(received, Response::Ok),
        "expected Response::Ok, got {:?}",
        received
    );
}

#[tokio::test]
async fn should_roundtrip_response_error_through_framed_message() {
    // Arrange
    let (mut writer, mut reader) = tokio::io::duplex(1024);
    let resp = Response::Error("fail".into());

    // Act
    send_message(&mut writer, &resp).await.expect("send_message should succeed");
    drop(writer);
    let received: Response = read_message(&mut reader).await.expect("read_message should succeed");

    // Assert
    match received {
        Response::Error(msg) => assert_eq!(msg, "fail"),
        other => panic!("expected Response::Error(\"fail\"), got {:?}", other),
    }
}

#[tokio::test]
async fn should_roundtrip_multiple_messages_on_same_channel() {
    // Arrange
    let (mut writer, mut reader) = tokio::io::duplex(4096);

    // Act — send two commands in sequence
    send_message(&mut writer, &Command::Menu).await.expect("send Menu");
    send_message(&mut writer, &Command::Status).await.expect("send Status");
    send_message(&mut writer, &Command::Shutdown).await.expect("send Shutdown");
    drop(writer);

    let first: Command = read_message(&mut reader).await.expect("read first");
    let second: Command = read_message(&mut reader).await.expect("read second");
    let third: Command = read_message(&mut reader).await.expect("read third");

    // Assert
    assert!(matches!(first, Command::Menu), "first should be Menu, got {:?}", first);
    assert!(matches!(second, Command::Status), "second should be Status, got {:?}", second);
    assert!(matches!(third, Command::Shutdown), "third should be Shutdown, got {:?}", third);
}
