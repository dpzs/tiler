use std::path::Path;

use tokio::net::UnixListener;

use crate::gnome::dbus_proxy::GnomeProxy;
use crate::ipc::protocol::{Command, Response, read_message, send_message};
use crate::menu::state::MenuInput;
use crate::tiling::engine::TilingEngine;

/// Run the tiler daemon: initialize the tiling engine, then serve IPC commands
/// over a Unix socket until a Shutdown command is received.
pub async fn run_daemon<P: GnomeProxy + 'static>(
    proxy: P,
    socket_path: &Path,
    stack_screen_index: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = TilingEngine::new(proxy, stack_screen_index);
    engine.startup().await?;

    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    loop {
        let (stream, _addr) = listener.accept().await?;
        let (mut reader, mut writer) = stream.into_split();

        loop {
            let cmd: Command = match read_message(&mut reader).await {
                Ok(cmd) => cmd,
                Err(_) => break,
            };

            let response = match cmd {
                Command::Menu => {
                    let _ = engine.handle_menu_input(MenuInput::ToggleMenu).await;
                    Response::Ok
                }
                Command::Status => Response::Ok,
                Command::Shutdown => {
                    let _ = send_message(&mut writer, &Response::Ok).await;
                    let _ = std::fs::remove_file(socket_path);
                    return Ok(());
                }
            };

            if send_message(&mut writer, &response).await.is_err() {
                break;
            }
        }
    }
}
