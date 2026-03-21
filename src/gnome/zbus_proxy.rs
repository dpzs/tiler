use zbus::proxy;

use super::dbus_proxy::{GnomeProxy, MonitorInfo, ProxyResult, WindowInfo};

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

    #[zbus(signal)]
    fn window_opened(
        &self,
        window_id: u64,
        title: String,
        app_class: String,
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
}
