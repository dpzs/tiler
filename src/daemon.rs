use std::path::Path;

use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

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
    info!(stack_screen_index, "daemon starting");
    let mut engine = TilingEngine::new(proxy, stack_screen_index);
    engine.startup().await?;
    info!(socket = %socket_path.display(), "startup complete, listening");

    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    let mut shutdown_rx = shutdown;
    let mut event_rx = event_rx;

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let (mut reader, mut writer) = stream.into_split();
                debug!("IPC client connected");

                loop {
                    tokio::select! {
                        cmd = read_message(&mut reader) => {
                            let cmd: Command = match cmd {
                                Ok(cmd) => cmd,
                                Err(_) => {
                                    debug!("IPC client disconnected");
                                    break;
                                }
                            };

                            info!(?cmd, "IPC command received");

                            let response = match cmd {
                                Command::Menu => {
                                    match engine.handle_menu_input(MenuInput::ToggleMenu).await {
                                        Ok(()) => Response::Ok,
                                        Err(e) => {
                                            error!(error = %e, "menu toggle failed");
                                            Response::Error(e.to_string())
                                        }
                                    }
                                }
                                Command::ApplyLayout { monitor, layout } => {
                                    let input = MenuInput::Digit(layout);
                                    engine.set_menu_state(crate::menu::state::MenuState::ZoomedIn(monitor));
                                    match engine.handle_menu_input(input).await {
                                        Ok(()) => {
                                            info!(monitor, layout, "layout applied via CLI");
                                            Response::Ok
                                        }
                                        Err(e) => {
                                            error!(monitor, layout, error = %e, "apply layout failed");
                                            Response::Error(e.to_string())
                                        }
                                    }
                                }
                                Command::Windows => {
                                    match engine.proxy().list_windows().await {
                                        Ok(windows) => {
                                            let json = serde_json::to_string_pretty(&windows)
                                                .unwrap_or_else(|e| format!("serialize error: {e}"));
                                            Response::Windows(json)
                                        }
                                        Err(e) => Response::Error(e.to_string()),
                                    }
                                }
                                Command::Status => Response::Ok,
                                Command::Shutdown => {
                                    info!("shutdown requested via IPC");
                                    let _ = send_message(&mut writer, &Response::Ok).await;
                                    let _ = std::fs::remove_file(socket_path);
                                    return Ok(());
                                }
                            };

                            debug!(?response, "IPC response");
                            if send_message(&mut writer, &response).await.is_err() {
                                warn!("failed to send IPC response, client gone");
                                break;
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
                info!("shutdown signal received");
                let _ = std::fs::remove_file(socket_path);
                return Ok(());
            }
        }
    }
}

async fn dispatch_event<P: GnomeProxy>(engine: &mut TilingEngine<P>, event: Event) {
    match event {
        Event::WindowOpened { window_id, title, app_class, monitor_id } => {
            info!(window_id, %title, %app_class, monitor_id, "WindowOpened");
            if let Err(e) = engine.handle_window_opened(window_id, title, app_class, monitor_id).await {
                error!(window_id, error = %e, "handle WindowOpened failed");
            }
        }
        Event::WindowClosed { window_id } => {
            info!(window_id, "WindowClosed");
            if let Err(e) = engine.handle_window_closed(window_id).await {
                error!(window_id, error = %e, "handle WindowClosed failed");
            }
        }
        Event::WindowFocusChanged { window_id } => {
            debug!(window_id, "FocusChanged");
            engine.handle_focus_changed(window_id);
        }
        Event::WorkspaceChanged { workspace_id } => {
            info!(workspace_id, "WorkspaceChanged");
            if let Err(e) = engine.handle_workspace_changed(workspace_id).await {
                error!(workspace_id, error = %e, "handle WorkspaceChanged failed");
            }
        }
        Event::WindowFullscreenChanged { window_id, is_fullscreen } => {
            info!(window_id, is_fullscreen, "FullscreenChanged");
            if let Err(e) = engine.handle_fullscreen_changed(window_id, is_fullscreen).await {
                error!(window_id, error = %e, "handle FullscreenChanged failed");
            }
        }
        Event::WindowGeometryChanged { window_id, x, y, width, height } => {
            debug!(window_id, x, y, width, height, "GeometryChanged");
            if let Err(e) = engine.handle_geometry_changed(window_id, x, y, width, height).await {
                error!(window_id, error = %e, "handle GeometryChanged failed");
            }
        }
        Event::MenuKeyPressed { key, modifiers } => {
            info!(%key, %modifiers, "MenuKeyPressed");
            if let Some(input) = crate::menu::key_parse::parse_menu_key(
                &key, &modifiers, engine.menu_state()
            ) {
                if let Err(e) = engine.handle_menu_input(input).await {
                    error!(%key, error = %e, "handle MenuKeyPressed failed");
                }
            }
        }
    }
}
