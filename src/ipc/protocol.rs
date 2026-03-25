use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum frame size: 16 MiB.
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Commands sent from the GNOME extension to the tiler daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Menu,
    Status,
    Shutdown,
    ApplyLayout { monitor: u32, layout: u8 },
    Windows,
}

/// Responses sent from the tiler daemon back to the extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Error(String),
    Windows(String),
}

/// Encode a payload with a u32 big-endian length prefix.
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let len = u32::try_from(payload.len())
        .map_err(|_| format!("payload too large: {} bytes", payload.len()))?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Decode a length-prefixed frame from an async reader.
pub async fn decode_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_FRAME_SIZE {
        return Err(format!("frame too large: {len} bytes (max {MAX_FRAME_SIZE})").into());
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

/// Serialize a value to JSON and write it as a length-prefixed frame.
pub async fn send_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    value: &impl Serialize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_vec(value)?;
    let frame = encode_frame(&json)?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a length-prefixed frame and deserialize from JSON.
pub async fn read_message<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    let payload = decode_frame(reader).await?;
    let value = serde_json::from_slice(&payload)?;
    Ok(value)
}
