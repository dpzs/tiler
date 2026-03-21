use std::path::Path;
use tokio::net::UnixListener;

use super::protocol::{Command, Response, read_message, send_message};

pub async fn run_server(socket_path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Remove stale socket file if it exists
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _addr) = listener.accept().await?;
        let (mut reader, mut writer) = stream.into_split();

        // Handle commands on this connection until the client disconnects
        loop {
            let cmd: Command = match read_message(&mut reader).await {
                Ok(cmd) => cmd,
                Err(_) => break, // Client disconnected
            };

            let response = match cmd {
                Command::Menu => Response::Ok,
                Command::Status => Response::Ok,
                Command::Shutdown => {
                    let _ = send_message(&mut writer, &Response::Ok).await;
                    // Clean up socket file and exit
                    let _ = std::fs::remove_file(socket_path);
                    return Ok(());
                }
            };

            if send_message(&mut writer, &response).await.is_err() {
                break; // Client disconnected
            }
        }
    }
}
