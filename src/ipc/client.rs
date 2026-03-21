use std::path::Path;
use tokio::net::UnixStream;

use super::protocol::{Command, Response, read_message, send_message};

pub async fn send_command(
    socket_path: &Path,
    command: Command,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let stream = UnixStream::connect(socket_path).await?;
    let (mut reader, mut writer) = stream.into_split();
    send_message(&mut writer, &command)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;
    let response: Response = read_message(&mut reader)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;
    Ok(response)
}
