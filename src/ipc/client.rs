use std::path::Path;
use tokio::net::UnixStream;

use super::protocol::{Command, Response, read_message, send_message};

/// Send a command to the tiler daemon over its IPC socket and return the response.
///
/// # Errors
///
/// Returns an error if the daemon is not running (socket not found), if the
/// connection fails, or if message serialization/deserialization fails.
pub async fn send_command(
    socket_path: &Path,
    command: Command,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let stream = UnixStream::connect(socket_path).await.map_err(|e| {
        format!(
            "cannot connect to daemon at {}: {e} (is the daemon running?)",
            socket_path.display()
        )
    })?;
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &command).await?;
    let response: Response = read_message(&mut reader).await?;
    Ok(response)
}
