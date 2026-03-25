use futures_lite::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info};
use zbus::proxy;

use super::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult, WindowInfo};
use super::event::Event;

#[proxy(
    interface = "org.gnome.Shell.Extensions.Tiler",
    default_service = "org.gnome.Shell.Extensions.Tiler",
    default_path = "/org/gnome/Shell/Extensions/Tiler"
)]
trait Tiler {
    fn list_windows(&self) -> zbus::Result<String>;
    fn move_resize_window(
        &self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> zbus::Result<()>;
    fn get_monitors(&self) -> zbus::Result<String>;
    fn get_active_workspace(&self) -> zbus::Result<u32>;
    fn get_window_type(&self, window_id: u64) -> zbus::Result<String>;
    fn is_fullscreen(&self, window_id: u64) -> zbus::Result<bool>;
    fn show_menu(&self, monitors_json: &str) -> zbus::Result<()>;
    fn show_menu_zoomed(&self, monitor_id: u32, layouts_json: &str) -> zbus::Result<()>;
    fn hide_menu(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn window_opened(
        &self,
        window_id: u64,
        title: String,
        app_class: String,
        monitor_id: u32,
    ) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn window_closed(&self, window_id: u64) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn window_focus_changed(&self, window_id: u64) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn workspace_changed(&self, workspace_id: u32) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn window_fullscreen_changed(
        &self,
        window_id: u64,
        is_fullscreen: bool,
    ) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn window_geometry_changed(
        &self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn menu_key_pressed(&self, key: String, modifiers: String) -> zbus::fdo::Result<()>;
}

/// Production D-Bus proxy wrapping the zbus-generated `TilerProxy`.
pub struct ZbusGnomeProxy {
    proxy: TilerProxy<'static>,
}

impl ZbusGnomeProxy {
    /// Connect to the GNOME Shell extension over the session D-Bus bus.
    pub async fn connect() -> zbus::Result<Self> {
        let connection = zbus::Connection::session().await?;
        let proxy = TilerProxy::new(&connection).await?;
        Ok(Self { proxy })
    }

    /// Spawn a background task that subscribes to all D-Bus signals and
    /// forwards them as `Event` variants through `tx`.
    pub fn spawn_signal_listener(&self, tx: mpsc::UnboundedSender<Event>) {
        let proxy = self.proxy.clone();
        tokio::spawn(async move {
            let (
                mut window_opened,
                mut window_closed,
                mut focus_changed,
                mut workspace_changed,
                mut fullscreen_changed,
                mut geometry_changed,
                mut menu_key,
            ) = match tokio::try_join!(
                proxy.receive_window_opened(),
                proxy.receive_window_closed(),
                proxy.receive_window_focus_changed(),
                proxy.receive_workspace_changed(),
                proxy.receive_window_fullscreen_changed(),
                proxy.receive_window_geometry_changed(),
                proxy.receive_menu_key_pressed(),
            ) {
                Ok(streams) => {
                    info!("D-Bus signal subscriptions established");
                    streams
                }
                Err(e) => {
                    error!(error = %e, "FATAL: failed to subscribe to D-Bus signals");
                    return;
                }
            };

            loop {
                tokio::select! {
                    Some(signal) = window_opened.next() => {
                        match signal.args() {
                            Ok(args) => {
                                let _ = tx.send(Event::WindowOpened {
                                    window_id: *args.window_id(),
                                    title: args.title().to_string(),
                                    app_class: args.app_class().to_string(),
                                    monitor_id: *args.monitor_id(),
                                });
                            }
                            Err(e) => {
                                error!(error = %e, "failed to parse WindowOpened signal");
                            }
                        }
                    }
                    Some(signal) = window_closed.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::WindowClosed {
                                window_id: *args.window_id(),
                            });
                        }
                    }
                    Some(signal) = focus_changed.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::WindowFocusChanged {
                                window_id: *args.window_id(),
                            });
                        }
                    }
                    Some(signal) = workspace_changed.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::WorkspaceChanged {
                                workspace_id: *args.workspace_id(),
                            });
                        }
                    }
                    Some(signal) = fullscreen_changed.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::WindowFullscreenChanged {
                                window_id: *args.window_id(),
                                is_fullscreen: *args.is_fullscreen(),
                            });
                        }
                    }
                    Some(signal) = geometry_changed.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::WindowGeometryChanged {
                                window_id: *args.window_id(),
                                x: *args.x(),
                                y: *args.y(),
                                width: *args.width(),
                                height: *args.height(),
                            });
                        }
                    }
                    Some(signal) = menu_key.next() => {
                        if let Ok(args) = signal.args() {
                            let _ = tx.send(Event::MenuKeyPressed {
                                key: args.key().to_string(),
                                modifiers: args.modifiers().to_string(),
                            });
                        }
                    }
                    else => break,
                }
            }
        });
    }
}

impl GnomeProxy for ZbusGnomeProxy {
    async fn list_windows(&self) -> ProxyResult<Vec<WindowInfo>> {
        let json = self.proxy.list_windows().await?;
        let windows: Vec<WindowInfo> = serde_json::from_str(&json)?;
        Ok(windows)
    }

    async fn move_resize_window(
        &mut self,
        window_id: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> ProxyResult<()> {
        self.proxy
            .move_resize_window(window_id, x, y, width, height)
            .await?;
        Ok(())
    }

    async fn get_monitors(&self) -> ProxyResult<Vec<MonitorInfo>> {
        let json = self.proxy.get_monitors().await?;
        let monitors: Vec<MonitorInfo> = serde_json::from_str(&json)?;
        Ok(monitors)
    }

    async fn get_active_workspace(&self) -> ProxyResult<u32> {
        let workspace = self.proxy.get_active_workspace().await?;
        Ok(workspace)
    }

    async fn get_window_type(&self, window_id: u64) -> ProxyResult<String> {
        let wtype = self.proxy.get_window_type(window_id).await?;
        Ok(wtype)
    }

    async fn is_fullscreen(&self, window_id: u64) -> ProxyResult<bool> {
        let fs = self.proxy.is_fullscreen(window_id).await?;
        Ok(fs)
    }

    async fn show_menu(&mut self, monitors_json: &str) -> ProxyResult<()> {
        self.proxy.show_menu(monitors_json).await?;
        Ok(())
    }

    async fn show_menu_zoomed(&mut self, monitor_id: u32, layouts_json: &str) -> ProxyResult<()> {
        self.proxy.show_menu_zoomed(monitor_id, layouts_json).await?;
        Ok(())
    }

    async fn hide_menu(&mut self) -> ProxyResult<()> {
        self.proxy.hide_menu().await?;
        Ok(())
    }
}
