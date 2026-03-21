use std::path::Path;

use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};

use crate::gnome::dbus_proxy::GnomeProxy;
use crate::gnome::event::Event;
use crate::ipc::protocol::{Command, Response, read_message, send_message};
use crate::menu::state::MenuInput;
use crate::tiling::engine::TilingEngine;

/// Run the tiler daemon.
///
/// Initializes the tiling engine, binds a Unix socket at `socket_path`, and
/// serves IPC commands in a loop. Exits cleanly on a `Shutdown` IPC command or
/// when the optional `shutdown` channel receives a value (used for OS signal
/// handling).
///
/// Any existing file at `socket_path` is removed before binding.
pub async fn run_daemon<P: GnomeProxy + 'static>(
    proxy: P,
    socket_path: &Path,
    stack_screen_index: usize,
    shutdown: Option<oneshot::Receiver<()>>,
    event_rx: Option<mpsc::UnboundedReceiver<Event>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = TilingEngine::new(proxy, stack_screen_index);
    engine.startup().await?;

    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    let mut shutdown_rx = shutdown;
    let mut event_rx = event_rx;

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
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
            Some(event) = async {
                match event_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                dispatch_event(&mut engine, event).await;
            }
            _ = async {
                match shutdown_rx.as_mut() {
                    Some(rx) => { let _ = rx.await; },
                    None => std::future::pending::<()>().await,
                }
            } => {
                let _ = std::fs::remove_file(socket_path);
                return Ok(());
            }
        }
    }
}

async fn dispatch_event<P: GnomeProxy>(engine: &mut TilingEngine<P>, event: Event) {
    match event {
        Event::WindowOpened { window_id, title, app_class } => {
            let _ = engine.handle_window_opened(window_id, title, app_class).await;
        }
        Event::WindowClosed { window_id } => {
            let _ = engine.handle_window_closed(window_id).await;
        }
        Event::WindowFocusChanged { window_id } => {
            let _ = engine.handle_focus_changed(window_id).await;
        }
        Event::WorkspaceChanged { workspace_id } => {
            let _ = engine.handle_workspace_changed(workspace_id).await;
        }
        Event::WindowFullscreenChanged { window_id, is_fullscreen } => {
            let _ = engine.handle_fullscreen_changed(window_id, is_fullscreen).await;
        }
        Event::WindowGeometryChanged { window_id, x, y, width, height } => {
            let _ = engine.handle_geometry_changed(window_id, x, y, width, height).await;
        }
        Event::MenuKeyPressed { key: _, modifiers: _ } => {
            // TODO: Parse key into MenuInput and dispatch
        }
    }
}
